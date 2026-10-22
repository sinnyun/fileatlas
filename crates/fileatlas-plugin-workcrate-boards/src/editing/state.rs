use super::*;

impl CanvasState {
    pub(crate) fn prepare_relocation(
        &mut self,
        project: Option<&str>,
    ) -> Result<bool, BoardsError> {
        if project.is_some_and(|id| id != self.project_id) {
            return Ok(false);
        }
        // A moved folder must not be recreated by saving to the stale old path.
        if self.document.is_file() && self.session.inspect_bound()?["dirty"].as_bool() == Some(true)
        {
            self.session.save_file(&self.document)?;
        }
        Ok(true)
    }
    pub(super) fn new(
        session: mcp::McpSession,
        live: bool,
        document: &Path,
        reads: Vec<PathBuf>,
        session_id: u64,
    ) -> Self {
        Self {
            session,
            session_id,
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

    /// Whether the official session still shows this session's own document.
    pub(super) fn holds_document(&mut self, document: &Path) -> bool {
        self.session.inspect_bound().is_ok_and(|inspect| {
            inspect["path"]
                .as_str()
                .is_some_and(|p| native::same_path(p, document))
        })
    }

    /// Point the same official session at another page of its document. The session, its
    /// process and its history stay; only the artboard the overlay renders moves. The old
    /// page's selection is cleared because those objects do not exist on this one. Nothing
    /// is written here: the document is saved when the editor closes, as before.
    pub(super) fn switch_to(
        &mut self,
        board: &WorkCrateBoard,
        preview: PathBuf,
        render: bool,
    ) -> Result<CanvasSnapshot, BoardsError> {
        if self.board_id != board.id {
            self.session.execute("select.none", json!({}))?;
            self.board_id = board.id.clone();
            self.native_id = board.native_id;
            self.artboard = board.index;
            self.preview = preview;
        }
        self.snapshot_after(render)
    }

    /// Whether the session already reads the folder that would be granted for `path` too.
    /// The launcher collapses its inputs into one folder, so widening that folder is what
    /// forces a new session; a file already inside it never does.
    pub(super) fn grants(&self, path: &Path) -> bool {
        let mut widened = self.reads.clone();
        widened.push(path.to_path_buf());
        match (automation_folder(&self.reads), automation_folder(&widened)) {
            (Ok(current), Ok(widened)) => current == widened,
            _ => false,
        }
    }

    pub(super) fn snapshot_after(&mut self, render: bool) -> Result<CanvasSnapshot, BoardsError> {
        let inspect = self.session.inspect_bound()?;
        self.snapshot_with(&inspect, render)
    }

    pub(super) fn snapshot_with(
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
            session_id: self.session_id,
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

    pub(super) fn object(&mut self, id: u64) -> Result<CanvasObject, BoardsError> {
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
                object.font_family = range["runs"][0]["style"]["font_family"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
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
