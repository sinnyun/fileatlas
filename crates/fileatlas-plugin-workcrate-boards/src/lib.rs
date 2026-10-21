//! WorkCrate native VectorCraft session and artboard service.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_workcrate_projects::{WorkCrateProjectsError, WorkCrateProjectsService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use thiserror::Error;
mod headless;
mod native;
mod operations;
mod protocol;
use protocol::Client;
use protocol::NativeProtocol;
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
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BoardCache {
    version: u32,
    binding: String,
    boards: Vec<WorkCrateBoard>,
    previews: BTreeMap<String, String>,
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
    fn headless_client(
        &self,
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
    ) -> Result<headless::HeadlessClient, BoardsError> {
        let config = self.configuration()?;
        let cli = if !config.cli_executable.trim().is_empty() {
            PathBuf::from(config.cli_executable)
        } else {
            let executable = PathBuf::from(config.executable);
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
        let config_dir = self.projects.root().join("cli-config");
        let directories = |roots: &[PathBuf]| -> Result<Vec<PathBuf>, BoardsError> {
            let mut resolved = Vec::new();
            for root in roots {
                let candidate = if root.is_file() {
                    root.parent().unwrap_or(root)
                } else {
                    root.as_path()
                };
                let canonical = std::fs::canonicalize(candidate)?;
                if !resolved.contains(&canonical) {
                    resolved.push(canonical);
                }
            }
            if resolved.len() <= 1 {
                return Ok(resolved);
            }
            let mut common = resolved[0].as_path();
            while !resolved.iter().all(|root| root.starts_with(common)) {
                common = common.parent().ok_or_else(|| {
                    BoardsError::Invalid(
                        "CLI inputs do not share a safe folder; place the source on the native document's volume and under a shared project folder".into(),
                    )
                })?;
            }
            if common.components().count() <= 2 {
                return Err(BoardsError::Invalid(
                    "CLI inputs span unrelated folders; choose files under one shared project folder".into()
                ));
            }
            Ok(vec![common.to_owned()])
        };
        let read_roots = directories(read_roots)?;
        let write_roots = directories(write_roots)?;
        headless::HeadlessClient::start(&cli, &read_roots, &write_roots, &config_dir)
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
        let mut c = self.client()?;
        native::revalidate_open(&mut c, &path)?;
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
        let cache: BoardCache = serde_json::from_slice(&std::fs::read(path)?)?;
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
        Ok(Some(cache))
    }

    fn write_cache(&self, project: &str, cache: &BoardCache) -> Result<(), BoardsError> {
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
            file.write_all(&serde_json::to_vec_pretty(cache)?)?;
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
        self.refuse_if_gui_has_document(project, false)?;
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        let mut client = self.headless_client(std::slice::from_ref(&path), &[])?;
        client.open_file(&path)?;
        let boards = native::catalog(&mut client, project, &binding)?;
        let previews = self.make_previews(&mut client, &boards)?;
        let cache = BoardCache {
            version: 1,
            binding,
            boards: boards.clone(),
            previews,
        };
        self.write_cache(project, &cache)?;
        Ok(boards)
    }

    fn make_previews(
        &self,
        client: &mut headless::HeadlessClient,
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

    fn refresh_cache_from_client(
        &self,
        client: &mut headless::HeadlessClient,
        project: &str,
        binding: String,
    ) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        let boards = native::catalog(client, project, &binding)?;
        let previews = self.make_previews(client, &boards)?;
        self.write_cache(
            project,
            &BoardCache {
                version: 1,
                binding,
                boards: boards.clone(),
                previews,
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
            self.write_info(&mut client, &board, &board.title, PageInfo::default())?;
            client.save_file(&path)?;
            boards = native::catalog(&mut client, project, &binding)?;
            let previews = self.make_previews(&mut client, &boards)?;
            self.write_cache(
                project,
                &BoardCache {
                    version: 1,
                    binding,
                    boards: boards.clone(),
                    previews,
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
        if let Some(n) = &board.info.project_name_override {
            return Ok(n.clone());
        }
        self.projects
            .projects(true)?
            .into_iter()
            .find(|p| p.id == board.project_id)
            .map(|p| p.name)
            .ok_or_else(|| BoardsError::Invalid("project missing".into()))
    }
    fn target(&self, c: &mut Client, id: &str) -> Result<WorkCrateBoard, BoardsError> {
        let (project, _) = native::split_id(id)?;
        let binding = self.projects.document_binding(project)?;
        if id.split(':').nth(1) != Some(binding.as_str()) {
            return Err(BoardsError::Invalid(
                "native document association changed; refresh pages".into(),
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

    fn refuse_if_gui_has_document(&self, project: &str, writes: bool) -> Result<(), BoardsError> {
        let Some(path) = self.document_path(project)? else {
            return Ok(());
        };
        let config = self.configuration()?;
        let Ok(mut client) = Client::connect(config.port) else {
            return Ok(());
        };
        let Some(inspect) = native::inspect_optional(&mut client)? else {
            return Ok(());
        };
        let active = inspect["path"].as_str().unwrap_or("");
        let normalize = |raw: &str| {
            raw.trim_start_matches(r"\\?\\")
                .replace('\\', "/")
                .to_lowercase()
        };
        if normalize(active) != normalize(&path.to_string_lossy()) {
            return Ok(());
        }
        if inspect["dirty"] == true {
            return Err(BoardsError::DirtyDocument);
        }
        if writes {
            return Err(BoardsError::Invalid(
                "close the open VectorCraft document before a background write".into(),
            ));
        }
        Ok(())
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
    #[error("different active document has unsaved changes; save or switch it explicitly")]
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
mod integration_tests {
    use super::*;
    #[test]
    fn uncached_catalogue_and_preview_reads_stay_offline() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?;
        let projects = Arc::new(WorkCrateProjectsService::open_at(root.path().join("data"))?);
        let project = projects.create_project("Offline reads")?;
        let native = root.path().join("existing.vectorcraft");
        std::fs::write(&native, b"native")?;
        projects.set_document_path(&project.id, &native)?;
        let service = WorkCrateBoardsService::new(projects);
        service.configure(VectorCraftConfig {
            executable: "unavailable-vectorcraft.exe".into(),
            cli_executable: String::new(),
            port: unused_local_port(),
        })?;

        assert!(service.board_catalog(&project.id)?.is_empty());
        let board_id = format!(
            "{}:{}:42",
            project.id,
            service.projects.document_binding(&project.id)?
        );
        assert!(matches!(
            service.render_preview(&board_id, 1600),
            Err(BoardsError::PreviewUnavailable(_))
        ));
        Ok(())
    }

    #[test]
    fn cached_catalogue_and_preview_survive_restart_without_editor(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let data = root.path().join("data");
        let projects = Arc::new(WorkCrateProjectsService::open_at(&data)?);
        let project = projects.create_project("Persistent cache")?;
        let native_path = root.path().join("document.vectorcraft");
        std::fs::write(&native_path, b"native")?;
        projects.set_document_path(&project.id, &native_path)?;
        let binding = projects.document_binding(&project.id)?;
        let service = WorkCrateBoardsService::new(projects.clone());
        let board = WorkCrateBoard {
            id: format!("{}:{binding}:12", project.id),
            project_id: project.id.clone(),
            native_id: 12,
            index: 0,
            title: "Cached page".into(),
            width_pt: 420.,
            height_pt: 297.,
            info: PageInfo::default(),
        };
        let preview_dir = projects
            .project_directory(&project.id)?
            .join("native-previews");
        std::fs::create_dir_all(&preview_dir)?;
        let preview_path = preview_dir.join("cached.png");
        std::fs::write(&preview_path, b"png")?;
        service.write_cache(
            &project.id,
            &BoardCache {
                version: 1,
                binding: binding.clone(),
                boards: vec![board.clone()],
                previews: BTreeMap::from([(
                    format!("{}@1600", board.id),
                    "native-previews/cached.png".into(),
                )]),
            },
        )?;
        drop(service);
        drop(projects);

        let reopened = Arc::new(WorkCrateProjectsService::open_at(&data)?);
        let offline = WorkCrateBoardsService::new(reopened.clone());
        let cached = offline.board_catalog(&project.id)?;
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].id, board.id);
        assert_eq!(cached[0].title, board.title);
        assert_eq!(
            offline.render_preview(&board.id, 1600)?,
            std::fs::canonicalize(preview_path)?
        );
        reopened.set_document_path(&project.id, &native_path)?;
        assert!(
            offline.board_catalog(&project.id)?.is_empty(),
            "a new association invalidates stale IDs"
        );
        assert!(matches!(
            offline.render_preview(&board.id, 1600),
            Err(BoardsError::PreviewUnavailable(_))
        ));
        Ok(())
    }

    fn unused_local_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }
    #[test]
    #[ignore = "requires isolated official VectorCraft instance and WRITE_ROOT environment"]
    fn official_native_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let root = PathBuf::from(std::env::var("WORKCRATE_NATIVE_TEST_ROOT")?);
        let projects = Arc::new(WorkCrateProjectsService::open_at(root)?);
        let p = projects.create_project(&format!(
            "Native {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis()
        ))?;
        let service = WorkCrateBoardsService::new(projects.clone());
        service.connect()?;
        let b = service.create_page(&p.id)?;
        assert_eq!(service.board_catalog(&p.id)?.len(), 1);
        let mut info = b.info.clone();
        info.material = "中文材料123".into();
        info.quantity = "20套".into();
        info.extra.insert("工艺".into(), "切割".into());
        service.update_info(&b.id, "真实页面", info)?;
        let read = service.board(&b.id)?;
        assert_eq!(read.title, "真实页面");
        assert_eq!(read.info.material, "中文材料123");
        assert_eq!(read.info.extra["工艺"], "切割");
        let mut c = service.client()?;
        let fields = native::header_fields(&mut c)?;
        let material = fields[&b.native_id]["material"].0;
        let before = c.execute("document.node", json!({"id":material}))?;
        let mut edited = read.info;
        edited.extra.insert("AAA".into(), "added".into());
        service.update_info(&b.id, "真实页面", edited)?;
        let after = c.execute("document.node", json!({"id":material}))?;
        assert_ne!(
            before["kind"]["xf"], after["kind"]["xf"],
            "adding earlier extra must reflow existing material"
        );
        let mut edited = service.board(&b.id)?.info;
        edited.extra.remove("AAA");
        service.update_info(&b.id, "真实页面", edited)?;
        let restored = c.execute("document.node", json!({"id":material}))?;
        assert_eq!(before["kind"]["xf"], restored["kind"]["xf"]);
        let second = service.create_page(&p.id)?;
        assert_ne!(second.native_id, b.native_id);
        assert_eq!(service.board_catalog(&p.id)?.len(), 2);
        let preview = service.render_preview(&b.id, 640)?;
        assert!(preview.is_file());
        let reference = projects
            .add_references(&p.id, std::slice::from_ref(&preview))?
            .into_iter()
            .next()
            .ok_or("saved reference missing")?;
        service.place_reference(&b.id, &reference.id, 24., 150., 200., false)?;
        let placed = c.execute("document.find", json!({"kind":"Image","limit":10}))?;
        assert!(
            placed["total"].as_u64().is_some_and(|n| n > 0),
            "official placement must create native image"
        );
        let out = projects.project_directory(&p.id)?;
        service.export_native(&[b.id.clone(), second.id], &out.join("pages.pdf"), "pdf")?;
        service.export_native(&[b.id.clone()], &out.join("page.png"), "png")?;
        service.save_project(&p.id)?;
        let original = service.document_path(&p.id)?.ok_or("native path missing")?;
        let replacement = out.join("replacement.vectorcraft");
        std::fs::copy(original, &replacement)?;
        service.attach_document(&p.id, &replacement)?;
        assert!(
            service
                .update_info(&b.id, "stale mutation", PageInfo::default())
                .is_err(),
            "old document binding must not modify copied replacement"
        );
        let rebound = service
            .board_catalog(&p.id)?
            .into_iter()
            .next()
            .ok_or("replacement board missing")?;
        service.attach_document(&p.id, &replacement)?;
        assert!(
            service
                .update_info(&rebound.id, "stale same path", PageInfo::default())
                .is_err(),
            "same-path reattach must invalidate old binding"
        );
        Ok(())
    }
}
