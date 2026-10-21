//! WorkCrate native VectorCraft session and artboard service.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_workcrate_projects::{WorkCrateProjectsError, WorkCrateProjectsService};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use thiserror::Error;
mod native;
mod operations;
mod protocol;
use protocol::Client;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorCraftConfig {
    pub executable: String,
    pub port: u16,
}
impl Default for VectorCraftConfig {
    fn default() -> Self {
        Self {
            executable: String::new(),
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
        self.projects.set_document_path(project, &path)?;
        Ok(())
    }
    pub fn board_catalog(&self, project: &str) -> Result<Vec<WorkCrateBoard>, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        let Some(path) = self.document_path(project)? else {
            return Ok(vec![]);
        };
        let mut c = self.client()?;
        native::open_safely(&mut c, &path)?;
        native::catalog(&mut c, project, &self.projects.document_binding(project)?)
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
