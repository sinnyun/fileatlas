use super::*;
use crate::protocol::NativeProtocol;
use std::{
    io::{BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

pub(crate) struct HeadlessClient {
    child: Child,
    stdin: Option<ChildStdin>,
    replies: Receiver<Result<String, String>>,
    request_id: u64,
    timeout: Duration,
    poisoned: bool,
}

impl HeadlessClient {
    pub(crate) fn start(
        executable: &Path,
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
        config_dir: &Path,
    ) -> Result<Self, BoardsError> {
        std::fs::create_dir_all(config_dir)?;
        let mut command = Command::new(executable);
        command.arg("mcp").arg("--headless");
        for root in read_roots {
            command.arg("--automation-read-root").arg(root);
        }
        for root in write_roots {
            command.arg("--automation-write-root").arg(root);
        }
        command
            .env("VECTORCRAFT_CONFIG_DIR", config_dir)
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
            .ok_or_else(|| BoardsError::Protocol("headless stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BoardsError::Protocol("headless stdout unavailable".into()))?;
        let (send, replies) = mpsc::channel();
        std::thread::spawn(move || read_lines(stdout, send));
        let mut client = Self {
            child,
            stdin: Some(stdin),
            replies,
            request_id: 1,
            timeout: Duration::from_secs(65),
            poisoned: false,
        };
        client.rpc(
            "initialize",
            json!({
                "protocolVersion":"2024-11-05",
                "capabilities":{},
                "clientInfo":{"name":"FileAtlas WorkCrate","version":"0.1"}
            }),
        )?;
        client.rpc("tools/list", json!({}))?;
        Ok(client)
    }

    fn rpc(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        if self.poisoned {
            return Err(BoardsError::Protocol(
                "headless process invalidated after uncertain outcome".into(),
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
            return Err(BoardsError::Protocol("headless stdin is closed".into()));
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
                return Err(BoardsError::Protocol("headless operation timed out".into()));
            }
        };
        if line.len() > 32 * 1024 * 1024 {
            self.poison();
            return Err(BoardsError::Protocol(
                "headless reply exceeds 32 MiB".into(),
            ));
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
            return Err(BoardsError::Protocol(
                "headless response ID mismatch".into(),
            ));
        }
        if let Some(error) = reply.get("error") {
            self.poison();
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
        let result = self.rpc("tools/call", json!({"name":name,"arguments":arguments}))?;
        if result["isError"] == true {
            self.poison();
            return Err(BoardsError::Remote(
                tool_text(&result).unwrap_or_else(|| result.to_string()),
            ));
        }
        Ok(result)
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
                "headless tool returned no text result".into(),
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

    pub(crate) fn open_file(&mut self, path: &Path) -> Result<Value, BoardsError> {
        self.tool("open_file", json!({"path":path}))
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
                BoardsError::Protocol("headless preview did not return a PNG image".into())
            })?;
        let encoded = image["data"].as_str().ok_or_else(|| {
            self.poison();
            BoardsError::Protocol("headless preview has no image data".into())
        })?;
        if encoded.len() > 48 * 1024 * 1024 {
            self.poison();
            return Err(BoardsError::Protocol(
                "headless preview exceeds the encoded size limit".into(),
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
                "headless preview is oversized or not a PNG".into(),
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

impl NativeProtocol for HeadlessClient {
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
                    "unsupported headless method: {method}"
                )));
            }
        };
        self.tool(tool, args)
    }

    fn execute(&mut self, command: &str, params: Value) -> Result<Value, BoardsError> {
        self.tool("command_run", json!({"id":command,"params":params}))
    }
}

impl Drop for HeadlessClient {
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
                        let _ = send.send(Err("headless reply exceeds 32 MiB".into()));
                        return;
                    }
                    bytes.push(one[0]);
                }
                Err(error) => {
                    let message = if bytes.is_empty() {
                        format!("headless stdout closed: {error}")
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
