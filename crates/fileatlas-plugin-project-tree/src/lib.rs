//! Project folder workbench. The plugin owns its JSON list and sidebar page;
//! navigation is dispatched through Navigation's public request contract.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod view;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use cordis::{Context, Plugin, Service};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const OWNER: &str = "fileatlas.project-tree";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTreeRoot {
    pub path: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProjectTreeDocument {
    #[serde(default = "version")]
    version: u32,
    #[serde(default, rename = "roots", alias = "projects")]
    roots: Vec<ProjectTreeRoot>,
}

impl Default for ProjectTreeDocument {
    fn default() -> Self {
        Self {
            version: version(),
            roots: Vec::new(),
        }
    }
}

fn version() -> u32 {
    1
}

pub struct ProjectTreeService {
    path: PathBuf,
    document: RwLock<ProjectTreeDocument>,
}

impl Service for ProjectTreeService {
    const NAME: &'static str = "fileatlas/project-tree";
}

impl ProjectTreeService {
    fn open() -> Result<Arc<Self>, ProjectTreeError> {
        let root = std::env::var_os("FILEATLAS_DEV_TEST_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|path| path.join("FileAtlas")))
            .ok_or(ProjectTreeError::NoDataDirectory)?;
        let dir = root.join("project-tree");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("project-tree.json");
        let legacy_path = root.join("projects").join("projects.json");
        let legacy_backup = legacy_path.with_extension("json.bak");
        let backup = path.with_extension("json.bak");
        let source = if path.is_file() {
            Some(path.clone())
        } else if backup.is_file() {
            Some(backup.clone())
        } else if legacy_path.is_file() {
            Some(legacy_path.clone())
        } else if legacy_backup.is_file() {
            Some(legacy_backup.clone())
        } else {
            None
        };
        let migrating_legacy = source
            .as_ref()
            .is_some_and(|source| source == &legacy_path || source == &legacy_backup);
        let document = if let Some(source) = source {
            let document: ProjectTreeDocument = serde_json::from_slice(&std::fs::read(&source)?)?;
            if document.version != 1 {
                return Err(ProjectTreeError::UnsupportedVersion(document.version));
            }
            if source == backup {
                let _ = std::fs::copy(&backup, &path);
            }
            document
        } else {
            ProjectTreeDocument::default()
        };
        let service = Arc::new(Self {
            path,
            document: RwLock::new(document),
        });
        if migrating_legacy {
            let migrated = service
                .document
                .read()
                .map_err(|_| ProjectTreeError::Poisoned)?
                .clone();
            service.persist(&migrated)?;
        }
        Ok(service)
    }

    pub fn snapshot(&self) -> Vec<ProjectTreeRoot> {
        self.document
            .read()
            .map(|doc| doc.roots.clone())
            .unwrap_or_default()
    }

    pub fn add(&self, raw_path: &str) -> Result<Vec<ProjectTreeRoot>, ProjectTreeError> {
        let path = Path::new(raw_path.trim());
        if !path.is_dir() {
            return Err(ProjectTreeError::MissingFolder);
        }
        let path = std::fs::canonicalize(path)?.to_string_lossy().into_owned();
        let name = Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let mut doc = self
            .document
            .write()
            .map_err(|_| ProjectTreeError::Poisoned)?;
        if !doc.roots.iter().any(|p| same_path(&p.path, &path)) {
            let mut updated = doc.clone();
            updated.roots.push(ProjectTreeRoot { path, name });
            self.persist(&updated)?;
            *doc = updated;
        }
        Ok(doc.roots.clone())
    }

    pub fn remove(&self, path: &str) -> Result<Vec<ProjectTreeRoot>, ProjectTreeError> {
        let mut doc = self
            .document
            .write()
            .map_err(|_| ProjectTreeError::Poisoned)?;
        let mut updated = doc.clone();
        let old_len = updated.roots.len();
        updated
            .roots
            .retain(|project| !same_path(&project.path, path));
        if old_len != updated.roots.len() {
            self.persist(&updated)?;
            *doc = updated;
        }
        Ok(doc.roots.clone())
    }

    fn persist(&self, doc: &ProjectTreeDocument) -> Result<(), ProjectTreeError> {
        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(doc)?)?;
        if let Err(error) = std::fs::rename(&temp, &self.path) {
            if self.path.exists() {
                let backup = self.path.with_extension("json.bak");
                let _ = std::fs::remove_file(&backup);
                std::fs::rename(&self.path, &backup)?;
                if let Err(replace_error) = std::fs::rename(&temp, &self.path) {
                    let _ = std::fs::rename(&backup, &self.path);
                    return Err(replace_error.into());
                }
                let _ = std::fs::remove_file(backup);
            } else {
                return Err(error.into());
            }
        }
        Ok(())
    }
}

fn same_path(a: &str, b: &str) -> bool {
    normalize_path(a) == normalize_path(b)
}

fn normalize_path(path: &str) -> String {
    let path = path
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase();
    path.strip_prefix("\\\\?\\").unwrap_or(&path).to_owned()
}

#[derive(Debug, Error)]
pub enum ProjectTreeError {
    #[error("no local application data directory is available")]
    NoDataDirectory,
    #[error("project folder does not exist")]
    MissingFolder,
    #[error("unsupported project list version {0}")]
    UnsupportedVersion(u32),
    #[error("project list lock was poisoned")]
    Poisoned,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("plugin dependency or registration failed: {0}")]
    Plugin(String),
}

#[derive(Debug, Clone, Default)]
pub struct ProjectTreeConfig;
pub struct ProjectTreePlugin;

impl Plugin for ProjectTreePlugin {
    type Config = ProjectTreeConfig;
    type Input = ProjectTreeConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = ProjectTreeError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, _: &Self::Input) -> Result<(), Self::ApplyError> {
        let navigation = context
            .try_service::<fileatlas_plugin_navigation::NavigationService>()
            .map_err(|error| ProjectTreeError::Plugin(error.to_string()))?;
        let service = ProjectTreeService::open()?;
        let _service = context
            .provide(Arc::clone(&service))
            .map_err(|error| ProjectTreeError::Plugin(error.to_string()))?;
        let page_service = Arc::clone(&service);
        let page_navigation = Arc::clone(&navigation);
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            OWNER,
            [view::sidebar_page(page_service, page_navigation)],
        )
        .map_err(|error| ProjectTreeError::Plugin(error.to_string()))?;
        Ok(())
    }
}
