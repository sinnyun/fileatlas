//! Program workspaces: versioned launch layouts, independent from photo data.
mod ui;
use cordis::{Context, Plugin, Service};
use fileatlas_ui_bridge::refresh::Wake;
use serde::{Deserialize, Serialize};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

const FORMAT_VERSION: u32 = 1;
/// UI surface value written by the live pane host before the user saves a workspace.
pub const CURRENT_SNAPSHOT_SURFACE_KEY: &str = "workspace.current_snapshot";
/// UI surface value updated by the manager after choosing a workspace.
pub const SELECTION_SURFACE_KEY: &str = "workspace.selection";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnLayout {
    #[default]
    Single,
    Dual,
    Quad,
}

impl ColumnLayout {
    pub const fn column_count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Dual => 2,
            Self::Quad => 4,
        }
    }
}

/// State needed to reopen one FileManager column. The values are configuration only;
/// media rows remain owned by FileManager and the shared database service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnDefinition {
    pub address: Option<String>,
    pub folder_id: Option<i64>,
    pub address_history: Vec<String>,
    pub address_history_index: usize,
    pub search: String,
    pub search_in_name: bool,
    pub search_in_path: bool,
    pub search_in_tags: bool,
    pub search_in_time: bool,
    pub media_type: String,
    pub included_tag_ids: Vec<i64>,
    pub excluded_tag_ids: Vec<i64>,
    pub included_extensions: Vec<String>,
    pub excluded_extensions: Vec<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub rating_min: u8,
    pub rating_max: u8,
    pub sort_order: String,
    pub grouping: String,
    pub list_layout: bool,
    pub show_subfolders: bool,
    pub thumbnails_enabled: bool,
    pub thumbnail_size: f32,
    pub selected_file_paths: Vec<String>,
    pub scroll_offset: f32,
}

impl Default for ColumnDefinition {
    fn default() -> Self {
        Self {
            address: None,
            folder_id: None,
            address_history: Vec::new(),
            address_history_index: 0,
            search: String::new(),
            search_in_name: true,
            search_in_path: true,
            search_in_tags: true,
            search_in_time: true,
            media_type: "all".into(),
            included_tag_ids: Vec::new(),
            excluded_tag_ids: Vec::new(),
            included_extensions: Vec::new(),
            excluded_extensions: Vec::new(),
            date_from: None,
            date_to: None,
            rating_min: 0,
            rating_max: 5,
            sort_order: "newest".into(),
            grouping: "month".into(),
            list_layout: false,
            show_subfolders: false,
            thumbnails_enabled: true,
            thumbnail_size: 160.0,
            selected_file_paths: Vec::new(),
            scroll_offset: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramWorkspace {
    pub id: String,
    pub name: String,
    pub layout: ColumnLayout,
    pub columns: Vec<ColumnDefinition>,
}

impl ProgramWorkspace {
    pub fn validate(&self) -> Result<(), WorkspaceError> {
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err(WorkspaceError::Invalid(
                "workspace id and name must be non-empty",
            ));
        }
        if self.columns.len() != self.layout.column_count() {
            return Err(WorkspaceError::Invalid(
                "column count does not match layout",
            ));
        }
        if self.columns.iter().any(|column| {
            column.rating_min > 5 || column.rating_max > 5 || column.rating_min > column.rating_max
        }) {
            return Err(WorkspaceError::Invalid(
                "rating range must be between 0 and 5",
            ));
        }
        if self
            .columns
            .iter()
            .any(|column| !column.scroll_offset.is_finite() || column.scroll_offset < 0.0)
        {
            return Err(WorkspaceError::Invalid(
                "column scroll offset must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceDocument {
    pub version: u32,
    pub default_workspace_id: String,
    pub workspaces: Vec<ProgramWorkspace>,
}

impl Default for WorkspaceDocument {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            default_workspace_id: "default".into(),
            workspaces: vec![ProgramWorkspace {
                id: "default".into(),
                name: "Default workspace".into(),
                layout: ColumnLayout::Single,
                columns: vec![ColumnDefinition::default()],
            }],
        }
    }
}

pub struct WorkspaceService {
    document: RwLock<WorkspaceDocument>,
    active_id: RwLock<String>,
    wake: Wake,
    path: PathBuf,
}

impl Service for WorkspaceService {
    const NAME: &'static str = "fileatlas/workspaces";
}

impl WorkspaceService {
    pub fn open_at(path: impl Into<PathBuf>) -> Result<Self, WorkspaceError> {
        let path = path.into();
        let document = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                WorkspaceDocument::default()
            }
            Err(error) => return Err(error.into()),
        };
        validate_document(&document)?;
        Ok(Self {
            active_id: RwLock::new(document.default_workspace_id.clone()),
            document: RwLock::new(document),
            wake: Wake::default(),
            path,
        })
    }

    pub fn open_default() -> Result<Self, WorkspaceError> {
        Self::open_at(default_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn wake(&self) -> Wake {
        self.wake.clone()
    }

    pub fn snapshot(&self) -> Result<WorkspaceDocument, WorkspaceError> {
        self.document
            .read()
            .map(|document| document.clone())
            .map_err(|_| WorkspaceError::Poisoned)
    }

    pub fn active(&self) -> Result<ProgramWorkspace, WorkspaceError> {
        let document = self.document.read().map_err(|_| WorkspaceError::Poisoned)?;
        let active_id = self
            .active_id
            .read()
            .map_err(|_| WorkspaceError::Poisoned)?;
        document
            .workspaces
            .iter()
            .find(|workspace| workspace.id == *active_id)
            .cloned()
            .ok_or(WorkspaceError::MissingDefault)
    }

    pub fn activate(&self, id: &str) -> Result<ProgramWorkspace, WorkspaceError> {
        let document = self.snapshot()?;
        let workspace = document
            .workspaces
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .ok_or_else(|| WorkspaceError::NotFound(id.into()))?;
        let mut active = self
            .active_id
            .write()
            .map_err(|_| WorkspaceError::Poisoned)?;
        if *active != id {
            *active = id.into();
            drop(active);
            self.wake.trigger();
        }
        Ok(workspace)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<(), WorkspaceError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(WorkspaceError::Invalid("workspace name must be non-empty"));
        }
        let mut workspace = self
            .snapshot()?
            .workspaces
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| WorkspaceError::NotFound(id.into()))?;
        workspace.name = name.into();
        self.upsert(workspace)
    }

    pub fn save_active(&self, workspace: ProgramWorkspace) -> Result<(), WorkspaceError> {
        let active_id = self
            .active_id
            .read()
            .map_err(|_| WorkspaceError::Poisoned)?
            .clone();
        if workspace.id != active_id {
            return Err(WorkspaceError::NotFound(active_id));
        }
        self.upsert(workspace)
    }

    pub fn save(&self, mut document: WorkspaceDocument) -> Result<(), WorkspaceError> {
        document.version = FORMAT_VERSION;
        validate_document(&document)?;
        let mut current = self
            .document
            .write()
            .map_err(|_| WorkspaceError::Poisoned)?;
        if *current == document {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&document)?;
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path)
            .map_err(|error| WorkspaceError::Io(error.error))?;
        *current = document;
        Ok(())
    }

    pub fn upsert(&self, workspace: ProgramWorkspace) -> Result<(), WorkspaceError> {
        workspace.validate()?;
        let mut document = self.snapshot()?;
        if let Some(existing) = document
            .workspaces
            .iter_mut()
            .find(|item| item.id == workspace.id)
        {
            *existing = workspace;
        } else {
            document.workspaces.push(workspace);
        }
        self.save(document)
    }

    pub fn set_default(&self, id: &str) -> Result<(), WorkspaceError> {
        let mut document = self.snapshot()?;
        if !document
            .workspaces
            .iter()
            .any(|workspace| workspace.id == id)
        {
            return Err(WorkspaceError::NotFound(id.into()));
        }
        document.default_workspace_id = id.into();
        self.save(document)
    }

    pub fn remove(&self, id: &str) -> Result<(), WorkspaceError> {
        let mut document = self.snapshot()?;
        if id == document.default_workspace_id {
            return Err(WorkspaceError::CannotRemoveDefault);
        }
        let was_active = self
            .active_id
            .read()
            .map_err(|_| WorkspaceError::Poisoned)?
            .as_str()
            == id;
        let default_id = document.default_workspace_id.clone();
        let before = document.workspaces.len();
        document.workspaces.retain(|workspace| workspace.id != id);
        if before == document.workspaces.len() {
            return Err(WorkspaceError::NotFound(id.into()));
        }
        self.save(document)?;
        if was_active {
            *self
                .active_id
                .write()
                .map_err(|_| WorkspaceError::Poisoned)? = default_id;
            self.wake.trigger();
        }
        Ok(())
    }
}

fn validate_document(document: &WorkspaceDocument) -> Result<(), WorkspaceError> {
    if document.version > FORMAT_VERSION {
        return Err(WorkspaceError::NewerVersion(document.version));
    }
    let mut ids = std::collections::HashSet::new();
    for workspace in &document.workspaces {
        workspace.validate()?;
        if !ids.insert(workspace.id.as_str()) {
            return Err(WorkspaceError::DuplicateId(workspace.id.clone()));
        }
    }
    if !ids.contains(document.default_workspace_id.as_str()) {
        return Err(WorkspaceError::MissingDefault);
    }
    Ok(())
}

fn default_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("FileAtlas")
        .join("workspaces")
        .join("workspaces.json")
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace state lock is poisoned")]
    Poisoned,
    #[error("workspace {0} was not found")]
    NotFound(String),
    #[error("cannot remove the default workspace")]
    CannotRemoveDefault,
    #[error("workspace document has no valid default workspace")]
    MissingDefault,
    #[error("duplicate workspace id: {0}")]
    DuplicateId(String),
    #[error("workspace data uses a newer format version: {0}")]
    NewerVersion(u32),
    #[error("invalid workspace: {0}")]
    Invalid(&'static str),
    #[error("could not publish workspace service: {0}")]
    Publish(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default)]
pub struct WorkspacesConfig {
    pub path: Option<PathBuf>,
}

pub struct WorkspacesPlugin;
impl Plugin for WorkspacesPlugin {
    type Config = WorkspacesConfig;
    type Input = WorkspacesConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = WorkspaceError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.workspaces".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let service = Arc::new(match &config.path {
            Some(path) => WorkspaceService::open_at(path)?,
            None => WorkspaceService::open_default()?,
        });
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| WorkspaceError::Publish(error.to_string()))?;
        let bridge = context
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .map_err(|error| WorkspaceError::Publish(error.to_string()))?;
        let surfaces = Arc::clone(&bridge.surfaces);
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            "fileatlas.workspaces",
            [crate::ui::manager_view(service, surfaces)],
        )
        .map_err(|error| WorkspaceError::Publish(error.to_string()))?;
        Ok(())
    }
}
