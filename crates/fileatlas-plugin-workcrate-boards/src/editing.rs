use super::*;
use serde::Serialize;

/// The page editor's canvas editing session. One long-lived official MCP session holds
/// the native document; every editing semantic (hit test, selection, move, scale, image
/// link, text) runs inside VectorCraft. FileAtlas only forwards input and displays what
/// the official program reports back.
pub(crate) struct CanvasState {
    session: mcp::McpSession,
    live: bool,
    document: PathBuf,
    project_id: String,
    board_id: String,
    native_id: u64,
    artboard: usize,
    preview: PathBuf,
    revision: u64,
    /// Leaves the session was launched with read access to. The official engine fixes its
    /// automation folders at launch, so a new folder means a new session.
    reads: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CanvasGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Everything the overlay needs for one frame: the official render, the official
/// selection box and the artboard size that maps one onto the other.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CanvasSnapshot {
    pub board_id: String,
    pub live: bool,
    pub tool: String,
    pub revision: u64,
    pub dirty: bool,
    pub selection: Vec<u64>,
    pub selection_bounds: Option<CanvasGeometry>,
    pub artboard: CanvasGeometry,
    pub preview: Option<String>,
    pub preview_size: Option<[u32; 2]>,
    pub last_step: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CanvasObject {
    pub id: u64,
    pub kind: String,
    pub name: String,
    pub bounds: Option<CanvasGeometry>,
    pub text: String,
    pub linked: bool,
    pub status: String,
    pub link_path: String,
}

fn geometry(value: &Value) -> Option<CanvasGeometry> {
    let read = |key: &str| value[key].as_f64();
    Some(CanvasGeometry {
        x: read("x")?,
        y: read("y")?,
        width: read("width")?,
        height: read("height")?,
    })
}

fn png_size(path: &Path) -> Option<[u32; 2]> {
    let header = {
        let mut file = std::fs::File::open(path).ok()?;
        let mut bytes = [0u8; 24];
        std::io::Read::read_exact(&mut file, &mut bytes).ok()?;
        bytes
    };
    if &header[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Some([width, height])
}

impl WorkCrateBoardsService {
    /// Start (or replace) the canvas session for one page. Uses the live GUI whenever its
    /// control port answers, otherwise the silent headless session; a running GUI is never
    /// a reason to refuse editing.
    pub fn canvas_open(&self, board_id: &str) -> Result<CanvasSnapshot, BoardsError> {
        let board = self.board(board_id)?;
        let project = board.project_id.clone();
        let document = self
            .document_path(&project)?
            .ok_or(BoardsError::NoDocument)?;
        let directory = self.projects.project_directory(&project)?;
        let preview_dir = directory.join("canvas");
        std::fs::create_dir_all(&preview_dir)?;
        let preview = preview_dir.join(format!("{}.png", board.native_id));
        let mut reads = vec![document.clone()];
        reads.extend(self.local_links(&project, &document)?);
        let (session, live) =
            self.canvas_session(&reads, &document, &directory.join("cli-config"))?;
        let mut state = CanvasState::new(session, live, &document, reads);
        state.preview = preview;
        state.board_id = board.id.clone();
        state.project_id = project;
        state.native_id = board.native_id;
        state.artboard = board.index;
        let snapshot = state.snapshot_after(true)?;
        let mut slot = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        *slot = Some(state);
        Ok(snapshot)
    }

    /// Native documents link their placed files in place, so a session that cannot read a
    /// link folder cannot redraw the page. Only references under the document's own folder
    /// are granted up front; anything else is granted when the user points at it.
    fn local_links(&self, project: &str, document: &Path) -> Result<Vec<PathBuf>, BoardsError> {
        let directory = document
            .parent()
            .map_or_else(|| PathBuf::from("."), |parent| parent.to_path_buf());
        let directory = std::fs::canonicalize(&directory).unwrap_or(directory);
        Ok(self
            .projects
            .references(project)?
            .into_iter()
            .map(|reference| PathBuf::from(reference.path))
            .filter(|path| path.is_file() && path.starts_with(&directory))
            .collect())
    }

    /// One official canvas session: the user's running GUI when its control port answers,
    /// otherwise the silent headless session confined to the one folder holding `reads`
    /// and writing only the document's folder.
    fn canvas_session(
        &self,
        reads: &[PathBuf],
        document: &Path,
        config_dir: &Path,
    ) -> Result<(mcp::McpSession, bool), BoardsError> {
        let config = self.configuration()?;
        let live = mcp::port_is_listening(config.port);
        let mut session = if live {
            mcp::McpSession::live(&self.cli_executable(&config)?, config.port)?
        } else {
            let writes = [document.to_path_buf()];
            self.headless_client_in(reads, &writes, config_dir)?
        };
        session.open_document(document)?;
        session.select_tool("selection")?;
        Ok((session, live))
    }

    /// Reopen the silent session with one more folder granted. A live GUI keeps the roots
    /// its owner launched it with, so a refused path there stays a visible error instead of
    /// silently restarting the user's program.
    fn canvas_grant(&self, path: &Path) -> Result<(), BoardsError> {
        let restart = self.canvas_call(|state| Ok(!state.live && !state.grants(path)))?;
        if !restart {
            return Ok(());
        }
        let resume = self.canvas_call(|state| {
            Ok(CanvasResume {
                document: state.document.clone(),
                project_id: state.project_id.clone(),
                board_id: state.board_id.clone(),
                native_id: state.native_id,
                artboard: state.artboard,
                preview: state.preview.clone(),
                reads: state.reads.clone(),
            })
        })?;
        self.canvas_commit()?;
        let mut reads = resume.reads;
        reads.push(std::fs::canonicalize(path)?);
        // Close the running session before launching the next one: the official engine owns
        // one preferences folder, so a second headless process may not start beside it.
        drop(
            self.canvas
                .lock()
                .map_err(|_| BoardsError::Poisoned)?
                .take(),
        );
        let config_dir = self
            .projects
            .project_directory(&resume.project_id)?
            .join("cli-config");
        let (session, live) = self.canvas_session(&reads, &resume.document, &config_dir)?;
        let mut replacement = CanvasState::new(session, live, &resume.document, reads);
        replacement.project_id = resume.project_id;
        replacement.board_id = resume.board_id;
        replacement.native_id = resume.native_id;
        replacement.artboard = resume.artboard;
        replacement.preview = resume.preview;
        *self.canvas.lock().map_err(|_| BoardsError::Poisoned)? = Some(replacement);
        Ok(())
    }

    pub fn canvas_close(&self) -> Result<(), BoardsError> {
        let mut slot = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        drop(slot.take());
        Ok(())
    }

    pub fn canvas_is_open(&self) -> bool {
        self.canvas.lock().is_ok_and(|slot| slot.is_some())
    }

    /// Cheap change probe for a UI observer: no render, no file write.
    ///
    /// `known` is the revision that observer last displayed, because an edit made through
    /// this service by another client (an automation call, a second panel) advances the
    /// session without that observer ever seeing the returned snapshot.
    pub fn canvas_probe(&self, known: u64) -> Result<Option<CanvasSnapshot>, BoardsError> {
        let mut slot = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        let Some(state) = slot.as_mut() else {
            return Ok(None);
        };
        let inspect = state.session.inspect()?;
        let revision = inspect["revision"].as_u64().unwrap_or(0);
        if revision == known {
            return Ok(None);
        }
        state.snapshot_with(&inspect, false).map(Some)
    }

    fn canvas_call<T>(
        &self,
        run: impl FnOnce(&mut CanvasState) -> Result<T, BoardsError>,
    ) -> Result<T, BoardsError> {
        let mut slot = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        let Some(state) = slot.as_mut() else {
            return Err(BoardsError::Invalid("canvas editing is not open".into()));
        };
        run(state)
    }

    pub fn canvas_state(&self) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_call(|state| state.snapshot_after(false))
    }

    /// Re-read and re-render the page after an official change FileAtlas did not cause.
    /// No save: the document already holds whatever the official program reports.
    pub fn canvas_refresh(&self) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_call(|state| state.snapshot_after(true))
    }

    /// Run one official editing command, then immediately write the native document and
    /// re-read the official truth. Every edit therefore reaches the same `.vectorcraft`
    /// file the GUI edits, instead of living only inside this session.
    fn canvas_edit(
        &self,
        run: impl FnOnce(&mut CanvasState) -> Result<(), BoardsError>,
    ) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_call(|state| {
            run(state)?;
            if state.snapshot_after(false)?.dirty {
                let document = state.document.clone();
                state.session.save_file(&document)?;
            }
            state.snapshot_after(true)
        })
    }

    /// Forward a pointer gesture; the official tool performs hit testing and the edit.
    pub fn canvas_gesture(&self, events: Value) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_edit(|state| {
            state.session.pointer_gesture(events)?;
            Ok(())
        })
    }

    pub fn canvas_tool(&self, tool: &str) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_call(|state| {
            state.session.select_tool(tool)?;
            state.snapshot_after(false)
        })
    }

    pub fn canvas_move(
        &self,
        ids: &[u64],
        dx: f64,
        dy: f64,
    ) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_edit(|state| {
            state
                .session
                .execute("object.move", json!({"ids":ids,"dx":dx,"dy":dy}))?;
            Ok(())
        })
    }

    pub fn canvas_set_bounds(
        &self,
        ids: &[u64],
        bounds: &CanvasGeometry,
    ) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_edit(|state| {
            // `reference: 0` makes x/y the box's top-left; the official default is the
            // centre, which shifts the object by half its size.
            state.session.execute(
                "object.setBounds",
                json!({"ids":ids,"reference":0,"x":bounds.x,"y":bounds.y,"width":bounds.width,"height":bounds.height}),
            )?;
            Ok(())
        })
    }

    pub fn canvas_object(&self, id: u64) -> Result<CanvasObject, BoardsError> {
        self.canvas_call(|state| state.object(id))
    }

    pub fn canvas_set_text(&self, id: u64, text: &str) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_edit(|state| {
            state
                .session
                .execute("text.setText", json!({"id":id,"text":text}))?;
            Ok(())
        })
    }

    /// Place a raster image or native document; `link` keeps it linked to its file so
    /// update and replace work on the same object. `centre` is the document point the
    /// placed box is centred on. A linked file is registered as a project reference, which
    /// is also how the next session learns the folder it has to grant.
    pub fn canvas_place(
        &self,
        path: &Path,
        centre: [f64; 2],
        link: bool,
    ) -> Result<CanvasSnapshot, BoardsError> {
        let path = std::fs::canonicalize(path)?;
        self.canvas_grant(&path)?;
        if link {
            let project = self.canvas_call(|state| Ok(state.project_id.clone()))?;
            self.projects
                .add_references(&project, std::slice::from_ref(&path))?;
        }
        self.canvas_edit(|state| {
            state
                .session
                .execute("file.place", json!({"path":&path,"link":link,"at":centre}))?;
            Ok(())
        })
    }

    /// Re-read every selected link from disk, granting the folders the official session has
    /// not read yet, so a link placed or relinked in an earlier session still refreshes.
    pub fn canvas_links_update(&self, ids: &[u64]) -> Result<CanvasSnapshot, BoardsError> {
        for id in ids {
            let link = self.canvas_object(*id)?;
            if !link.link_path.is_empty() {
                self.canvas_grant(Path::new(&link.link_path))?;
            }
        }
        self.canvas_edit(|state| {
            state.session.execute("links.update", json!({"ids":ids}))?;
            Ok(())
        })
    }

    pub fn canvas_links_relink(
        &self,
        ids: &[u64],
        path: &Path,
    ) -> Result<CanvasSnapshot, BoardsError> {
        let path = std::fs::canonicalize(path)?;
        self.canvas_grant(&path)?;
        let project = self.canvas_call(|state| Ok(state.project_id.clone()))?;
        self.projects
            .add_references(&project, std::slice::from_ref(&path))?;
        self.canvas_edit(|state| {
            state
                .session
                .execute("links.relink", json!({"ids":ids,"path":&path}))?;
            Ok(())
        })
    }

    pub fn canvas_undo(&self) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_step("undo")
    }

    pub fn canvas_redo(&self) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_step("redo")
    }

    fn canvas_step(&self, tool: &str) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_edit(|state| {
            state.session.call(tool, json!({}))?;
            Ok(())
        })
    }

    /// Write the native document so the official file, the GUI and every cached preview
    /// describe the same page. Edits already save themselves; this covers leaving the
    /// canvas with an official change that no command reported as dirty.
    pub fn canvas_commit(&self) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_call(|state| {
            let document = state.document.clone();
            state.session.save_file(&document)?;
            state.snapshot_after(true)
        })
    }

    /// Add a native text object at one document point.
    pub fn canvas_create_text(
        &self,
        x: f64,
        y: f64,
        text: &str,
        size: f64,
    ) -> Result<CanvasSnapshot, BoardsError> {
        self.canvas_edit(|state| {
            state
                .session
                .execute("text.create", json!({"x":x,"y":y,"text":text,"size":size}))?;
            Ok(())
        })
    }
}

/// The page editor's canvas session fields that survive a folder being granted: everything
/// except the official session process itself.
struct CanvasResume {
    document: PathBuf,
    project_id: String,
    board_id: String,
    native_id: u64,
    artboard: usize,
    preview: PathBuf,
    reads: Vec<PathBuf>,
}

impl CanvasState {
    fn new(session: mcp::McpSession, live: bool, document: &Path, reads: Vec<PathBuf>) -> Self {
        Self {
            session,
            live,
            document: document.to_path_buf(),
            project_id: String::new(),
            board_id: String::new(),
            native_id: 0,
            artboard: 0,
            preview: PathBuf::new(),
            revision: u64::MAX,
            reads,
        }
    }

    /// Whether the session already reads the folder that would be granted for `path` too.
    /// The launcher collapses its inputs into one folder, so widening that folder is what
    /// forces a new session; a file already inside it never does.
    fn grants(&self, path: &Path) -> bool {
        let mut widened = self.reads.clone();
        widened.push(path.to_path_buf());
        match (automation_folder(&self.reads), automation_folder(&widened)) {
            (Ok(current), Ok(widened)) => current == widened,
            _ => false,
        }
    }

    fn snapshot_after(&mut self, render: bool) -> Result<CanvasSnapshot, BoardsError> {
        let inspect = self.session.inspect()?;
        self.snapshot_with(&inspect, render)
    }

    fn snapshot_with(
        &mut self,
        inspect: &Value,
        render: bool,
    ) -> Result<CanvasSnapshot, BoardsError> {
        let artboards = inspect["artboards"].as_array().cloned().unwrap_or_default();
        let artboard = artboards
            .get(self.artboard)
            .or_else(|| artboards.first())
            .and_then(geometry)
            .unwrap_or_default();
        let history = inspect["history"]
            .as_array()
            .and_then(|steps| steps.last())
            .and_then(Value::as_str)
            .map(str::to_owned);
        let preview = if render {
            self.session
                .render_preview(self.artboard, 1600, &self.preview)?;
            Some(self.preview.to_string_lossy().replace('\\', "/"))
        } else {
            std::fs::metadata(&self.preview)
                .is_ok()
                .then(|| self.preview.to_string_lossy().replace('\\', "/"))
        };
        self.revision = inspect["revision"].as_u64().unwrap_or(0);
        Ok(CanvasSnapshot {
            board_id: self.board_id.clone(),
            live: self.live,
            tool: inspect["tool"].as_str().unwrap_or_default().to_owned(),
            revision: self.revision,
            dirty: inspect["dirty"].as_bool().unwrap_or(false),
            selection: inspect["selection"]
                .as_array()
                .map(|ids| ids.iter().filter_map(Value::as_u64).collect())
                .unwrap_or_default(),
            selection_bounds: geometry(&inspect["selectionBounds"]),
            artboard,
            preview_size: preview.as_ref().and_then(|path| png_size(Path::new(path))),
            preview,
            last_step: history,
        })
    }

    fn object(&mut self, id: u64) -> Result<CanvasObject, BoardsError> {
        let node = self
            .session
            .execute("document.node", json!({"id":id,"summary":true}))?;
        // The official program reports `Type` and `Image`; the panel compares one spelling.
        let kind = node["kind"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let mut object = CanvasObject {
            id,
            kind: kind.clone(),
            name: node["name"].as_str().unwrap_or_default().to_owned(),
            bounds: geometry(&node["bounds"]),
            ..Default::default()
        };
        if kind.eq_ignore_ascii_case("type") {
            if let Ok(range) = self.session.execute("text.getRange", json!({"id":id})) {
                object.text = range["text"].as_str().unwrap_or_default().to_owned();
            }
        }
        if kind.eq_ignore_ascii_case("image") {
            let links = self
                .session
                .execute("links.list", json!({"show":"all"}))
                .unwrap_or(Value::Null);
            if let Some(entry) = links["links"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["id"].as_u64() == Some(id)))
            {
                object.linked = entry["linked"].as_bool().unwrap_or(false);
                object.status = entry["status"].as_str().unwrap_or_default().to_owned();
                object.link_path = entry["path"].as_str().unwrap_or_default().to_owned();
            }
        }
        Ok(object)
    }
}
