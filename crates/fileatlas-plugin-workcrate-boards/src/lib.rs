//! WorkCrate native VectorCraft session and artboard service.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_workcrate_projects::{WorkCrateProjectsError, WorkCrateProjectsService};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicU64},
};
use thiserror::Error;
mod autosync;
mod editing;
mod header;
mod header_layout;
mod mcp;
mod native;
mod operations;
mod page_order;
mod portable;
pub use page_order::{PageOrderSave, page_title};
mod protocol;
pub use autosync::{AutoSync, DocumentStamp, PreviewState};
pub use editing::{CanvasGeometry, CanvasObject, CanvasSnapshot};
use protocol::Client;
use protocol::NativeProtocol;
/// The outcome of saving one page's fields: the new card record, plus the official frame the
/// editor's own session already rendered. A reused canvas session hands its frame straight to
/// the overlay, so the page is not rendered a second time by the change probe.
#[derive(Clone, Debug)]
pub struct InfoSave {
    pub board: WorkCrateBoard,
    pub frame: Option<CanvasSnapshot>,
    /// False when the draft equalled the stored page, so nothing was written and no frame needs
    /// re-reading.
    pub changed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorCraftConfig {
    pub executable: String,
    #[serde(default)]
    pub cli_executable: String,
    pub port: u16,
}
impl Default for VectorCraftConfig {
    fn default() -> Self {
        Self {
            executable: String::new(),
            cli_executable: String::new(),
            port: 17979,
        }
    }
}
/// The one folder the official launcher can be handed for these leaves: the deepest folder
/// holding them all. The protocol takes a single read folder and a single write folder, so
/// inputs that only meet at a drive root are refused instead of granting the volume.
fn automation_folder(leaves: &[PathBuf]) -> Result<Option<PathBuf>, BoardsError> {
    let mut resolved: Vec<PathBuf> = Vec::new();
    for leaf in leaves {
        let candidate = if leaf.is_file() {
            leaf.parent().unwrap_or(leaf.as_path())
        } else {
            leaf.as_path()
        };
        let canonical = std::fs::canonicalize(candidate)?;
        if !resolved.contains(&canonical) {
            resolved.push(canonical);
        }
    }
    if resolved.len() <= 1 {
        return Ok(resolved.pop());
    }
    let mut common = resolved[0].as_path();
    while !resolved.iter().all(|root| root.starts_with(common)) {
        common = common.parent().ok_or_else(|| BoardsError::Invalid(
            "CLI inputs do not share a safe folder; place the source on the native document's volume and under a shared project folder".into(),
        ))?;
    }
    if common.components().count() <= 2 {
        return Err(BoardsError::Invalid(
            "CLI inputs span unrelated folders; choose files under one shared project folder"
                .into(),
        ));
    }
    Ok(Some(common.to_owned()))
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageInfo {
    pub order: u32,
    pub auto_from_first: bool,
    pub pixel_size: Option<[u32; 2]>,
    pub first_image: Option<String>,
    pub physical_size_cm: Option<[f64; 2]>,
    pub size_label: String,
    pub project_name_override: Option<String>,
    pub material_category: String,
    pub material: String,
    pub size: String,
    pub quantity: String,
    pub extra: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkCrateBoard {
    pub id: String,
    pub project_id: String,
    pub native_id: u64,
    pub index: usize,
    pub title: String,
    pub width_pt: f64,
    pub height_pt: f64,
    pub info: PageInfo,
}
pub struct WorkCrateBoardsService {
    projects: Arc<WorkCrateProjectsService>,
    gate: Mutex<()>,
    status: Mutex<Value>,
    diagnostics: Arc<Mutex<BTreeMap<PathBuf, Value>>>,
    canvas: Mutex<Option<editing::CanvasState>>,
    /// Identity of every official canvas process this service has started. A page switch
    /// inside one document keeps its id; only a new process gets a new one.
    canvas_sessions: AtomicU64,
    header_report: Mutex<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BoardCache {
    version: u32,
    binding: String,
    boards: Vec<WorkCrateBoard>,
    previews: BTreeMap<String, String>,
    /// Saved-document stamp this catalogue was built from. Caches written before this field
    /// existed have no source and are refreshed once.
    #[serde(default)]
    source: Option<DocumentStamp>,
}
impl Service for WorkCrateBoardsService {
    const NAME: &'static str = "fileatlas/workcrate-boards";
}
impl WorkCrateBoardsService {
    pub fn new(projects: Arc<WorkCrateProjectsService>) -> Self {
        Self {
            projects,
            gate: Mutex::new(()),
            status: Mutex::new(Value::Null),
            diagnostics: Arc::new(Mutex::new(BTreeMap::new())),
            canvas: Mutex::new(None),
            canvas_sessions: AtomicU64::new(1),
            header_report: Mutex::new(Value::Null),
        }
    }
    pub fn configuration(&self) -> Result<VectorCraftConfig, BoardsError> {
        let p = self.projects.root().join("vectorcraft.json");
        let mut c: VectorCraftConfig = if p.exists() {
            serde_json::from_slice(&std::fs::read(p)?)?
        } else {
            Default::default()
        };
        if c.executable.trim().is_empty() {
            c.executable = std::env::var("FILEATLAS_VECTORCRAFT_EXE").unwrap_or_default()
        }
        Ok(c)
    }
    pub fn configure(&self, config: VectorCraftConfig) -> Result<(), BoardsError> {
        if config.port == 0 {
            return Err(BoardsError::Invalid("port must be nonzero".into()));
        }
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let p = self.projects.root().join("vectorcraft.json");
        use std::io::Write;
        let mut t = tempfile::NamedTempFile::new_in(self.projects.root())?;
        t.write_all(&serde_json::to_vec_pretty(&config)?)?;
        t.as_file().sync_all()?;
        if p.exists() {
            std::fs::copy(&p, p.with_extension("json.bak"))?;
        }
        t.persist(p).map_err(|e| e.error)?;
        Ok(())
    }
    fn client(&self) -> Result<Client, BoardsError> {
        let mut client = Client::connect(self.configuration()?.port)?;
        client.diagnostics = self.diagnostics.clone();
        Ok(client)
    }
    fn cli_executable(&self, config: &VectorCraftConfig) -> Result<PathBuf, BoardsError> {
        let cli = if !config.cli_executable.trim().is_empty() {
            PathBuf::from(config.cli_executable.as_str())
        } else {
            let executable = PathBuf::from(&config.executable);
            if executable
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("vectorcraft-cli.exe"))
            {
                executable
            } else {
                executable.with_file_name("vectorcraft-cli.exe")
            }
        };
        if !cli.is_file() {
            return Err(BoardsError::Invalid(format!(
                "official VectorCraft CLI not found: {}",
                cli.display()
            )));
        }
        Ok(cli)
    }
    fn headless_client(
        &self,
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
    ) -> Result<mcp::McpSession, BoardsError> {
        let config_dir = self.projects.root().join("cli-config");
        self.headless_client_in(read_roots, write_roots, &config_dir)
    }
    /// The official launcher accepts exactly one read folder and one write folder, so every
    /// input leaf collapses to the single deepest folder that holds them all. A long-lived
    /// canvas session keeps its own preferences folder beside the project instead of the
    /// shared one, so it never starts a second headless engine over the same settings.
    fn headless_client_in(
        &self,
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
        config_dir: &Path,
    ) -> Result<mcp::McpSession, BoardsError> {
        let config = self.configuration()?;
        let cli = self.cli_executable(&config)?;
        let folder = |leaves: &[PathBuf]| -> Result<Vec<PathBuf>, BoardsError> {
            Ok(automation_folder(leaves)?.into_iter().collect())
        };
        let read_roots = folder(read_roots)?;
        let write_roots = folder(write_roots)?;
        mcp::McpSession::headless(&cli, &read_roots, &write_roots, config_dir)
    }
    /// One official session for any work on a native document: the user's running GUI when
    /// its control port answers, otherwise the silent headless session. A running GUI is
    /// never a reason to refuse an edit, and an edit never requires closing the GUI first.
    fn official_session(
        &self,
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
    ) -> Result<mcp::McpSession, BoardsError> {
        if self.canvas_is_open() {
            self.canvas_commit()?;
            self.canvas_close()?;
        }
        let config = self.configuration()?;
        if mcp::port_is_listening(config.port) {
            return mcp::McpSession::live(&self.cli_executable(&config)?, config.port);
        }
        self.headless_client(read_roots, write_roots)
    }
    pub fn connect(&self) -> Result<Value, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let cfg = self.configuration()?;
        let mut c = match Client::connect(cfg.port) {
            Ok(c) => c,
            Err(e) => {
                if cfg.executable.trim().is_empty() {
                    return Err(e);
                }
                std::process::Command::new(&cfg.executable)
                    .args(["--control", &cfg.port.to_string()])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()?;
                let start = std::time::Instant::now();
                loop {
                    match Client::connect(cfg.port) {
                        Ok(c) => break c,
                        Err(_) if start.elapsed().as_secs() < 20 => {
                            std::thread::sleep(std::time::Duration::from_millis(200))
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        };
        let commands = c.request("engine.commands", json!({}))?;
        let command_ids = commands
            .as_array()
            .ok_or_else(|| BoardsError::Protocol("command catalogue is not an array".into()))?
            .iter()
            .filter_map(|c| c["id"].as_str())
            .collect::<std::collections::HashSet<_>>();
        for required in [
            "document.json",
            "text.create",
            "attributes.set",
            "view.goToArtboard",
            "edit.undoGroup.begin",
        ] {
            if !command_ids.contains(required) {
                return Err(BoardsError::Protocol(format!(
                    "official server missing {required}"
                )));
            }
        }
        let status = json!({"connected":true,"port":cfg.port,"capabilities":commands});
        *self.status.lock().map_err(|_| BoardsError::Poisoned)? = status.clone();
        Ok(status)
    }
    pub fn document_path(&self, project: &str) -> Result<Option<PathBuf>, BoardsError> {
        Ok(self.projects.document_path(project)?)
    }
    pub fn attach_document(&self, project: &str, path: &Path) -> Result<(), BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        if !path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("vectorcraft"))
        {
            return Err(BoardsError::Invalid(
                "native .vectorcraft file required".into(),
            ));
        }
        self.projects.project_directory(project)?;
        let path = std::fs::canonicalize(path)?;
        let mut client = self.headless_client(std::slice::from_ref(&path), &[])?;
        native::revalidate_open_headless(&mut client, &path, &self.diagnostics)?;
        drop(client);
        let project_dir = self.projects.project_directory(project)?;
        let old_previews = std::fs::read(self.cache_path(project)?)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BoardCache>(&bytes).ok())
            .map(|cache| cache_preview_paths(&project_dir, &cache))
            .unwrap_or_default();
        self.projects.set_document_path(project, &path)?;
        let _ = std::fs::remove_file(self.cache_path(project)?);
        for preview in old_previews {
            remove_owned_preview(&project_dir, &preview);
        }
        Ok(())
    }
    pub fn board_catalog(&self, project: &str) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        if self.document_path(project)?.is_none() {
            return Ok(vec![]);
        }
        Ok(self
            .read_cache(project)?
            .map(|cache| cache.boards)
            .unwrap_or_default())
    }

    fn cache_path(&self, project: &str) -> Result<PathBuf, BoardsError> {
        Ok(self
            .projects
            .project_directory(project)?
            .join("boards-cache.json"))
    }

    fn read_cache(&self, project: &str) -> Result<Option<BoardCache>, BoardsError> {
        let path = self.cache_path(project)?;
        if !path.is_file() {
            return Ok(None);
        }
        let mut cache: BoardCache = serde_json::from_slice(&std::fs::read(path)?)?;
        for board in &mut cache.boards {
            board.title = page_title(board.index);
            if board.info.material_category.is_empty() {
                if let Some(value) = board.info.project_name_override.take() {
                    board.info.material_category = value;
                }
            }
        }
        if cache.version != 1 {
            return Err(BoardsError::Invalid(
                "unsupported native board cache version".into(),
            ));
        }
        let binding = match self.projects.document_binding(project) {
            Ok(binding) => binding,
            Err(_) => return Ok(None),
        };
        if cache.binding != binding || cache.boards.iter().any(|b| b.project_id != project) {
            return Ok(None);
        }
        self.resolve_cache_paths(project, &mut cache)?;
        Ok(Some(cache))
    }

    fn write_cache(&self, project: &str, cache: &BoardCache) -> Result<(), BoardsError> {
        let mut stored = cache.clone();
        self.store_cache_paths(project, &mut stored)?;
        let path = self.cache_path(project)?;
        let project_dir = self.projects.project_directory(project)?;
        let old_paths = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BoardCache>(&bytes).ok())
            .map(|old| cache_preview_paths(&project_dir, &old))
            .unwrap_or_default();
        let new_paths = cache_preview_paths(&project_dir, cache);

        let publication = (|| {
            let mut file =
                tempfile::NamedTempFile::new_in(path.parent().ok_or_else(|| {
                    BoardsError::Invalid("native board cache has no parent".into())
                })?)?;
            use std::io::Write;
            file.write_all(&serde_json::to_vec_pretty(&stored)?)?;
            file.as_file().sync_all()?;
            file.persist(path).map_err(|e| e.error)?;
            Ok::<_, BoardsError>(())
        })();

        if let Err(error) = publication {
            for preview in new_paths.difference(&old_paths) {
                remove_owned_preview(&project_dir, preview);
            }
            return Err(error);
        }

        for preview in old_paths.difference(&new_paths) {
            remove_owned_preview(&project_dir, preview);
        }
        Ok(())
    }
    pub fn load_board(&self, project: &str, id: &str) -> Result<WorkCrateBoard, BoardsError> {
        self.board_catalog(project)?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| BoardsError::BoardNotFound(id.into()))
    }
    pub fn board(&self, id: &str) -> Result<WorkCrateBoard, BoardsError> {
        let (project, _) = native::split_id(id)?;
        self.load_board(project, id)
    }

    pub fn refresh_project(&self, project: &str) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        // `official_session` may commit a warm canvas session first, so sample the saved
        // document only after that write.
        let client = self.official_session(std::slice::from_ref(&path), &[])?;
        let source = autosync::stamp_of(&path);
        self.rebuild_cache(project, &path, binding, source, client)
    }

    fn make_previews(
        &self,
        client: &mut mcp::McpSession,
        boards: &[WorkCrateBoard],
    ) -> Result<BTreeMap<String, String>, BoardsError> {
        let mut previews = BTreeMap::new();
        let mut created = Vec::new();
        let result = (|| {
            for board in boards {
                let width = 1600u32;
                let scale = (width as f64 / board.width_pt).min(2048. / board.height_pt);
                let max_side = (scale * board.width_pt)
                    .max(scale * board.height_pt)
                    .round() as u32;
                let directory = self
                    .projects
                    .project_directory(&board.project_id)?
                    .join("native-previews");
                std::fs::create_dir_all(&directory)?;
                let path = directory.join(format!("{}.png", unique_token()));
                client.render_preview(board.index, max_side, &path)?;
                created.push(path.clone());
                let relative = path
                    .strip_prefix(self.projects.project_directory(&board.project_id)?)
                    .map_err(|_| BoardsError::Invalid("preview path escaped project cache".into()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                previews.insert(format!("{}@{width}", board.id), relative);
            }
            Ok(())
        })();
        if let Err(error) = result {
            for path in created {
                let _ = std::fs::remove_file(path);
            }
            return Err(error);
        }
        Ok(previews)
    }

    /// 尺寸标记 is panel-only since the compact band stopped painting it: the official
    /// document carries no node for it, so a catalogue rebuild must keep the cached value.
    fn merge_panel_fields(&self, project: &str, boards: &mut [WorkCrateBoard]) {
        let Ok(Some(old)) = self.read_cache(project) else {
            return;
        };
        for board in boards {
            if let Some(old_board) = old.boards.iter().find(|b| b.id == board.id) {
                board.info.size_label = old_board.info.size_label.clone();
            }
        }
    }

    fn refresh_cache_from_client(
        &self,
        client: &mut mcp::McpSession,
        project: &str,
        binding: String,
    ) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        let mut boards = native::catalog(client, project, &binding)?;
        self.merge_panel_fields(project, &mut boards);
        let previews = self.make_previews(client, &boards)?;
        self.write_cache(
            project,
            &BoardCache {
                version: 1,
                binding,
                boards: boards.clone(),
                previews,
                source: self.source_stamp(project),
            },
        )?;
        Ok(boards)
    }

    pub fn create_project_document(
        &self,
        project: &str,
        directory: &Path,
    ) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let project_data = self
            .projects
            .projects(true)?
            .into_iter()
            .find(|item| item.id == project)
            .ok_or_else(|| BoardsError::Invalid("project missing".into()))?;
        if self.projects.document_path(project)?.is_some() {
            return Err(BoardsError::Invalid(
                "project already has an associated native document".into(),
            ));
        }
        std::fs::create_dir_all(directory)?;
        let directory = std::fs::canonicalize(directory)?;
        let name = safe_document_name(&project_data.name);
        let path = directory.join(format!("{name}.vectorcraft"));
        if path.parent() != Some(directory.as_path()) || path.exists() {
            return Err(BoardsError::Invalid(format!(
                "native file already exists: {}",
                path.display()
            )));
        }
        let mut client = self.headless_client(&[directory.clone()], &[directory.clone()])?;
        let reservation = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        drop(reservation);
        let w = 420. * 72. / 25.4;
        let h = 297. * 72. / 25.4;
        let created = (|| {
            client.execute("file.new", json!({"name":project_data.name,"width":w,"height":h,"units":"Points","artboards":1,"backgroundContents":"white"}))?;
            client.save_file(&path)?;
            self.projects.set_document_path(project, &path)?;
            let binding = self.projects.document_binding(project)?;
            let mut boards = native::catalog(&mut client, project, &binding)?;
            let board = boards.first().cloned().ok_or(BoardsError::NoDocument)?;
            self.write_info(
                &mut client,
                &board,
                &page_title(board.index),
                PageInfo::default(),
            )?;
            client.save_file(&path)?;
            boards = native::catalog(&mut client, project, &binding)?;
            self.projects.relocate_project_data(project)?;
            let previews = self.make_previews(&mut client, &boards)?;
            self.write_cache(
                project,
                &BoardCache {
                    version: 1,
                    binding,
                    boards: boards.clone(),
                    previews,
                    source: autosync::stamp_of(&path),
                },
            )?;
            Ok::<_, BoardsError>(boards)
        })();
        match created {
            Ok(boards) => Ok(boards),
            Err(error) => {
                let _ = self.projects.clear_document_path_if_matches(project, &path);
                let _ = std::fs::remove_file(self.cache_path(project)?);
                let _ = std::fs::remove_file(&path);
                Err(error)
            }
        }
    }
    pub fn boards_for_project(&self, project: &str) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        self.board_catalog(project)
    }
    pub fn project_name(&self, board: &WorkCrateBoard) -> Result<String, BoardsError> {
        Ok(board.info.material_category.clone())
    }

    fn target(&self, c: &mut Client, id: &str) -> Result<WorkCrateBoard, BoardsError> {
        let (project, _) = native::split_id(id)?;
        let binding = self.projects.document_binding(project)?;
        if id.split(':').nth(1) != Some(binding.as_str()) {
            return Err(BoardsError::Invalid(
                "本页所属原生工程的关联已变化，请先同步工程与预览后再保存".into(),
            ));
        }
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        native::open_safely(c, &path)?;
        native::assert_path(c, &path)?;
        native::catalog(c, project, &binding)?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| BoardsError::BoardNotFound(id.into()))
    }
}

fn cache_preview_paths(
    project_dir: &Path,
    cache: &BoardCache,
) -> std::collections::BTreeSet<PathBuf> {
    cache
        .previews
        .values()
        .filter_map(|relative| {
            let relative = Path::new(relative);
            let mut components = relative.components();
            if !matches!(components.next(), Some(std::path::Component::Normal(name)) if name == "native-previews")
                || !matches!(components.next(), Some(std::path::Component::Normal(name)) if name.to_string_lossy().ends_with(".png"))
                || components.next().is_some()
            {
                return None;
            }
            Some(project_dir.join(relative))
        })
        .collect()
}

fn remove_owned_preview(project_dir: &Path, preview: &Path) {
    let Ok(project_root) = std::fs::canonicalize(project_dir) else {
        return;
    };
    let Some(parent) = preview.parent() else {
        return;
    };
    let Ok(preview_dir) = std::fs::canonicalize(parent) else {
        return;
    };
    if !preview_dir.starts_with(&project_root)
        || preview_dir
            .file_name()
            .is_none_or(|name| name != "native-previews")
    {
        return;
    }
    let _ = std::fs::remove_file(preview);
}

fn safe_document_name(name: &str) -> String {
    let safe = name
        .trim()
        .trim_end_matches([' ', '.'])
        .chars()
        .map(|c| {
            if matches!(
                c,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'
            ) {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    if safe.is_empty()
        || matches!(
            safe.to_ascii_uppercase().as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "LPT1"
        )
    {
        format!("WorkCrate-{}", unique_token())
    } else {
        safe
    }
}

fn unique_token() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let ticks = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!(
        "{ticks:x}-{:x}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
#[derive(Debug, Error)]
pub enum BoardsError {
    #[error(transparent)]
    Projects(#[from] WorkCrateProjectsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("VectorCraft protocol: {0}")]
    Protocol(String),
    #[error("VectorCraft: {0}")]
    Remote(String),
    #[error("{0}")]
    Invalid(String),
    #[error("board not found: {0}")]
    BoardNotFound(String),
    #[error("reference not found: {0}")]
    ReferenceNotFound(String),
    #[error("no native document associated")]
    NoDocument,
    #[error("cached native preview unavailable for board: {0}")]
    PreviewUnavailable(String),
    #[error("service lock poisoned")]
    Poisoned,
    #[error("plugin: {0}")]
    Plugin(String),
    #[error("官方程序中另一个文档有未保存的修改，请先保存或关闭它后再重试")]
    DirtyDocument,
}
pub struct WorkCrateBoardsPlugin;
impl Plugin for WorkCrateBoardsPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = BoardsError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.workcrate-boards".into()
    }
    fn prepare(&self, c: ()) -> Result<(), Self::PrepareError> {
        Ok(c)
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), BoardsError> {
        let projects = context
            .try_service::<WorkCrateProjectsService>()
            .map_err(|e| BoardsError::Plugin(e.to_string()))?;
        let _publication = context
            .provide(Arc::new(WorkCrateBoardsService::new(projects)))
            .map_err(|e| BoardsError::Plugin(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests;
