use super::*;
use crate::protocol::NativeProtocol;
use std::{
    ffi::OsString,
    io::{BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

/// One official VectorCraft automation session over the MCP stdio tool surface. The same
/// 32 tools answer on both transports, so editing semantics never live in FileAtlas:
/// `headless` runs silently in this process, `live` drives the user's running GUI.
pub(crate) struct McpSession {
    child: Child,
    stdin: Option<ChildStdin>,
    replies: Receiver<Result<String, String>>,
    request_id: u64,
    timeout: Duration,
    poisoned: bool,
    /// Native document this session is allowed to write. A live GUI holds whatever the
    /// user left open, so every mutation re-checks the active document before running.
    pub(crate) bound_path: Option<PathBuf>,
    live: bool,
}

/// Commands that only read state; anything else can change the active document.
const READ_COMMANDS: [&str; 5] = [
    "document.inspect",
    "document.node",
    "document.find",
    "document.formats",
    "attributes.info",
];
const READ_TOOLS: [&str; 5] = [
    "command_list",
    "doc_inspect",
    "render_preview",
    "ui_inspect",
    "open_file",
];

impl McpSession {
    pub(crate) fn headless(
        executable: &Path,
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
        config_dir: &Path,
    ) -> Result<Self, BoardsError> {
        std::fs::create_dir_all(config_dir)?;
        let mut args = vec!["mcp".into(), "--headless".into()];
        for root in read_roots {
            args.push("--automation-read-root".into());
            args.push(root.as_os_str().to_os_string());
        }
        for root in write_roots {
            args.push("--automation-write-root".into());
            args.push(root.as_os_str().to_os_string());
        }
        Self::start(executable, args, Some(config_dir), false)
    }

    pub(crate) fn live(executable: &Path, port: u16) -> Result<Self, BoardsError> {
        Self::start(
            executable,
            vec![
                "mcp".into(),
                "--connect".into(),
                format!("127.0.0.1:{port}").into(),
            ],
            None,
            true,
        )
    }

    fn start(
        executable: &Path,
        args: Vec<OsString>,
        config_dir: Option<&Path>,
        live: bool,
    ) -> Result<Self, BoardsError> {
        let mut command = Command::new(executable);
        for arg in args {
            command.arg(arg);
        }
        if let Some(dir) = config_dir {
            command.env("VECTORCRAFT_CONFIG_DIR", dir);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BoardsError::Protocol("MCP stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BoardsError::Protocol("MCP stdout unavailable".into()))?;
        let (send, replies) = mpsc::channel();
        std::thread::spawn(move || read_lines(stdout, send));
        let mut session = Self {
            child,
            stdin: Some(stdin),
            replies,
            request_id: 1,
            timeout: Duration::from_secs(65),
            poisoned: false,
            bound_path: None,
            live,
        };
        session.rpc(
            "initialize",
            json!({
                "protocolVersion":"2024-11-05",
                "capabilities":{},
                "clientInfo":{"name":"FileAtlas WorkCrate","version":"0.1"}
            }),
        )?;
        session.rpc("tools/list", json!({}))?;
        Ok(session)
    }

    fn rpc(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        if self.poisoned {
            return Err(BoardsError::Protocol(
                "MCP process invalidated after uncertain outcome".into(),
            ));
        }
        let id = self.request_id;
        self.request_id += 1;
        let mut request = serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":id, "method":method, "params":params
        }))?;
        request.push(b'\n');
        let Some(stdin) = self.stdin.as_mut() else {
            self.poison();
            return Err(BoardsError::Protocol("MCP stdin is closed".into()));
        };
        if let Err(error) = stdin.write_all(&request).and_then(|_| stdin.flush()) {
            self.poison();
            return Err(error.into());
        }
        let line = match self.replies.recv_timeout(self.timeout) {
            Ok(Ok(line)) => line,
            Ok(Err(error)) => {
                self.poison();
                return Err(BoardsError::Protocol(error));
            }
            Err(_) => {
                self.poison();
                return Err(BoardsError::Protocol("MCP operation timed out".into()));
            }
        };
        if line.len() > 32 * 1024 * 1024 {
            self.poison();
            return Err(BoardsError::Protocol("MCP reply exceeds 32 MiB".into()));
        }
        let reply: Value = match serde_json::from_str(&line) {
            Ok(reply) => reply,
            Err(error) => {
                self.poison();
                return Err(error.into());
            }
        };
        if reply["id"].as_u64() != Some(id) {
            self.poison();
            return Err(BoardsError::Protocol("MCP response ID mismatch".into()));
        }
        if let Some(error) = reply.get("error") {
            return Err(BoardsError::Remote(error.to_string()));
        }
        Ok(reply.get("result").cloned().unwrap_or(Value::Null))
    }

    fn poison(&mut self) {
        self.poisoned = true;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, BoardsError> {
        self.assert_bound(name, &Value::Null)?;
        let result = self.rpc("tools/call", json!({"name":name,"arguments":arguments}))?;
        if result["isError"] == true {
            // A rejected command is a well-formed reply: the official server keeps serving,
            // so an interactive session must survive it.
            return Err(BoardsError::Remote(
                tool_text(&result).unwrap_or_else(|| result.to_string()),
            ));
        }
        Ok(result)
    }

    /// A live session shares the user's GUI, so a mutation may never land on whatever
    /// document happens to be active there.
    fn assert_bound(&mut self, name: &str, arguments: &Value) -> Result<(), BoardsError> {
        let Some(path) = self.bound_path.clone() else {
            return Ok(());
        };
        if READ_TOOLS.contains(&name) {
            return Ok(());
        }
        if name == "command_run" {
            if let Some(id) = arguments["id"].as_str() {
                if READ_COMMANDS.contains(&id) {
                    return Ok(());
                }
            }
        }
        let active = self.inspect_active()?;
        if same_native_path(active["path"].as_str().unwrap_or(""), &path) {
            return Ok(());
        }
        Err(BoardsError::Invalid(format!(
            "active document changed during transaction: {}",
            path.display()
        )))
    }

    fn inspect_active(&mut self) -> Result<Value, BoardsError> {
        let result = self.rpc(
            "tools/call",
            json!({"name":"doc_inspect","arguments":{"depth":0}}),
        )?;
        let text = tool_text(&result).ok_or_else(|| {
            self.poison();
            BoardsError::Protocol("MCP inspect returned no text result".into())
        })?;
        serde_json::from_str(&text).map_err(Into::into)
    }

    fn tool(&mut self, name: &str, arguments: Value) -> Result<Value, BoardsError> {
        let result = self.call_tool(name, arguments)?;
        let Some(text) = tool_text(&result) else {
            if result["content"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["type"] == "image"))
            {
                return Ok(Value::Null);
            }
            self.poison();
            return Err(BoardsError::Protocol(
                "MCP tool returned no text result".into(),
            ));
        };
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.poison();
                Err(error.into())
            }
        }
    }

    pub(crate) fn bind(&mut self, path: &Path) {
        self.bound_path = Some(path.to_owned());
    }

    pub(crate) fn call(&mut self, name: &str, arguments: Value) -> Result<Value, BoardsError> {
        self.tool(name, arguments)
    }

    pub(crate) fn inspect(&mut self) -> Result<Value, BoardsError> {
        self.tool("doc_inspect", json!({"depth":0}))
    }

    pub(crate) fn select_tool(&mut self, tool: &str) -> Result<Value, BoardsError> {
        self.tool("select_tool", json!({"tool":tool}))
    }

    /// The official tool runs the same code path as the mouse, including hit testing.
    pub(crate) fn pointer_gesture(&mut self, events: Value) -> Result<Value, BoardsError> {
        self.tool("pointer_gesture", json!({"events":events}))
    }

    /// Make `path` the session's active document. A live GUI may already hold it open with
    /// unsaved edits, so an existing window is activated instead of re-read from disk. A
    /// session with no document open is not an error: opening `path` is the next step.
    pub(crate) fn open_document(&mut self, path: &Path) -> Result<(), BoardsError> {
        let active = self.inspect_optional()?;
        if active.as_ref().is_some_and(|inspect| {
            inspect["path"]
                .as_str()
                .is_some_and(|p| native::same_path(p, path))
        }) {
            self.bind(path);
            return Ok(());
        }
        if self.live
            && active.is_some()
            && let Some(index) = self.document_index_by_title(path)?
        {
            self.execute("document.activate", json!({"index":index}))?;
            if self
                .active_path()?
                .is_some_and(|p| native::same_path(&p, path))
            {
                self.bind(path);
                return Ok(());
            }
        }
        self.open_file(path)?;
        if !self
            .active_path()?
            .is_some_and(|p| native::same_path(&p, path))
        {
            return Err(BoardsError::Invalid(
                "官方程序未能打开该原生文档；请确认文件仍在关联文件夹中且未被占用".into(),
            ));
        }
        Ok(())
    }

    /// The active document, or `None` when the official program has no document open.
    /// Opening a document is how that state is repaired, so it is not an error here.
    fn active_path(&mut self) -> Result<Option<String>, BoardsError> {
        Ok(self
            .inspect_optional()?
            .and_then(|inspect| inspect["path"].as_str().map(str::to_owned)))
    }

    /// A snapshot of the bound document. A live GUI can close it while FileAtlas still shows
    /// the page editor, so reopen the bound file before reporting failure.
    pub(crate) fn inspect_bound(&mut self) -> Result<Value, BoardsError> {
        if let Some(inspect) = self.inspect_optional()? {
            return Ok(inspect);
        }
        let Some(path) = self.bound_path.clone() else {
            return Err(BoardsError::Invalid(
                "官方程序中没有打开文档；请在 WorkCrate 中重新打开该页面编辑".into(),
            ));
        };
        self.open_file(&path)?;
        self.inspect()
    }

    fn inspect_optional(&mut self) -> Result<Option<Value>, BoardsError> {
        match self.inspect() {
            Ok(inspect) => Ok(Some(inspect)),
            Err(error) if no_document_open(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn document_index_by_title(&mut self, path: &Path) -> Result<Option<u64>, BoardsError> {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        if stem.is_empty() {
            return Ok(None);
        }
        let ui = self.tool("inspect_ui", json!({}))?;
        Ok(ui["documents"]
            .as_array()
            .and_then(|documents| {
                documents
                    .iter()
                    .position(|document| document["title"].as_str() == Some(stem))
            })
            .map(|index| index as u64))
    }

    pub(crate) fn open_file(&mut self, path: &Path) -> Result<Value, BoardsError> {
        let result = self.tool("open_file", json!({"path":path}))?;
        self.bound_path = Some(path.to_owned());
        Ok(result)
    }

    pub(crate) fn save_file(&mut self, path: &Path) -> Result<Value, BoardsError> {
        let result = self.tool("save_file", json!({"path":path}))?;
        if !std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0) {
            self.poison();
            return Err(BoardsError::Invalid(format!(
                "official CLI produced no native output: {}",
                path.display()
            )));
        }
        Ok(result)
    }

    pub(crate) fn render_preview(
        &mut self,
        artboard: usize,
        max_side: u32,
        path: &Path,
    ) -> Result<(), BoardsError> {
        let result = self.call_tool(
            "render_preview",
            json!({"artboard":artboard,"max_side":max_side.clamp(64, 2048)}),
        )?;
        let image = result["content"]
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item["type"] == "image" && item["mimeType"] == "image/png")
            })
            .ok_or_else(|| {
                self.poison();
                BoardsError::Protocol("MCP preview did not return a PNG image".into())
            })?;
        let encoded = image["data"].as_str().ok_or_else(|| {
            self.poison();
            BoardsError::Protocol("MCP preview has no image data".into())
        })?;
        if encoded.len() > 48 * 1024 * 1024 {
            self.poison();
            return Err(BoardsError::Protocol(
                "MCP preview exceeds the encoded size limit".into(),
            ));
        }
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| {
                self.poison();
                BoardsError::Protocol(format!("invalid preview base64: {error}"))
            })?;
        if bytes.len() > 32 * 1024 * 1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            self.poison();
            return Err(BoardsError::Protocol(
                "MCP preview is oversized or not a PNG".into(),
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| BoardsError::Invalid("preview output has no directory".into()))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
}

impl NativeProtocol for McpSession {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        let (tool, args) = match method {
            "engine.commands" => ("command_list", params),
            "document.inspect" => ("doc_inspect", params),
            "app.open" => ("open_file", params),
            "app.save" => ("save_file", params),
            "app.export" => ("export", params),
            "ui.render" => (
                "render_preview",
                json!({"artboard":params["artboard"],"max_side":1600}),
            ),
            _ => {
                return Err(BoardsError::Protocol(format!(
                    "unsupported MCP method: {method}"
                )));
            }
        };
        self.tool(tool, args)
    }

    fn execute(&mut self, command: &str, params: Value) -> Result<Value, BoardsError> {
        self.assert_bound("command_run", &json!({"id":command}))?;
        self.tool("command_run", json!({"id":command,"params":params}))
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        self.stdin.take();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn same_native_path(raw: &str, path: &Path) -> bool {
    native::same_path(raw, path)
}

/// The official program answers a document query with this error while no document is open.
fn no_document_open(error: &BoardsError) -> bool {
    matches!(
        error,
        BoardsError::Remote(message)
            if message.contains("document.inspect") && message.contains("no document open")
    )
}

/// The live transport is only possible while the official GUI answers on its control port.
pub(crate) fn port_is_listening(port: u16) -> bool {
    use std::{
        net::{SocketAddr, TcpStream},
        time::Duration,
    };
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(250),
    )
    .is_ok()
}

fn read_lines(stdout: ChildStdout, send: mpsc::Sender<Result<String, String>>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut bytes = Vec::new();
        let mut one = [0u8; 1];
        loop {
            match reader.read_exact(&mut one) {
                Ok(()) if one[0] == b'\n' => break,
                Ok(()) => {
                    if bytes.len() >= 32 * 1024 * 1024 {
                        let _ = send.send(Err("MCP reply exceeds 32 MiB".into()));
                        return;
                    }
                    bytes.push(one[0]);
                }
                Err(error) => {
                    let message = if bytes.is_empty() {
                        format!("MCP stdout closed: {error}")
                    } else {
                        error.to_string()
                    };
                    let _ = send.send(Err(message));
                    return;
                }
            }
        }
        let line = match String::from_utf8(bytes) {
            Ok(line) => line,
            Err(error) => {
                let _ = send.send(Err(error.to_string()));
                return;
            }
        };
        if send.send(Ok(line)).is_err() {
            return;
        }
    }
}

fn tool_text(result: &Value) -> Option<String> {
    result["content"].as_array()?.iter().find_map(|item| {
        (item["type"] == "text")
            .then(|| item["text"].as_str().map(str::to_owned))
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, id: Option<&str>) -> (String, Value) {
        (
            name.to_string(),
            id.map(|id| json!({"id": id})).unwrap_or(Value::Null),
        )
    }

    #[test]
    fn only_state_reads_skip_the_bound_document_guard() {
        let session = |name: &str, id: Option<&str>| {
            let (name, args) = tool(name, id);
            READ_TOOLS.contains(&name.as_str())
                || (name == "command_run"
                    && args["id"]
                        .as_str()
                        .is_some_and(|id| READ_COMMANDS.contains(&id)))
        };
        assert!(session("doc_inspect", None));
        assert!(session("render_preview", None));
        assert!(session("command_run", Some("document.node")));
        assert!(!session("command_run", Some("text.setText")));
        assert!(!session("command_run", Some("file.place")));
        assert!(!session("save_file", None));
        assert!(!session("pointer_gesture", None));
    }

    #[test]
    fn an_official_session_without_a_document_is_an_empty_state() {
        assert!(no_document_open(&BoardsError::Remote(
            "command `document.inspect` is not available right now: no document open".into()
        )));
        assert!(no_document_open(&BoardsError::Remote(
            "doc_inspect: command `document.inspect` is not available right now: no document open"
                .into()
        )));
        assert!(!no_document_open(&BoardsError::Remote(
            "command `document.inspect` is not available right now: another dialog is open".into()
        )));
        assert!(!no_document_open(&BoardsError::Invalid(
            "no document open".into()
        )));
    }

    #[test]
    fn bound_path_compare_ignores_separators_and_drive_case() {
        assert!(same_native_path(
            r"\\?\C:\Work\Page.vectorcraft",
            std::path::Path::new("c:/work/page.vectorcraft")
        ));
        assert!(!same_native_path(
            "other.vectorcraft",
            std::path::Path::new("page.vectorcraft")
        ));
    }
}
