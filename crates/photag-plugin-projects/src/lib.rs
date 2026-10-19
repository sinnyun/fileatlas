//! Project folder workbench. The plugin owns its JSON list and sidebar page;
//! navigation is dispatched through Navigation's public request contract.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod view;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use cordis::{Context, Plugin, Service};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const OWNER: &str = "photag.projects";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFolder {
    pub path: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProjectDocument {
    #[serde(default = "version")]
    version: u32,
    #[serde(default)]
    projects: Vec<ProjectFolder>,
}

impl Default for ProjectDocument {
    fn default() -> Self {
        Self {
            version: version(),
            projects: Vec::new(),
        }
    }
}

fn version() -> u32 {
    1
}

pub struct ProjectsService {
    path: PathBuf,
    document: RwLock<ProjectDocument>,
}

impl Service for ProjectsService {
    const NAME: &'static str = "photag/projects";
}

impl ProjectsService {
    fn open() -> Result<Arc<Self>, ProjectsError> {
        let root = std::env::var_os("PHOTAG_DEV_TEST_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|path| path.join("PhoTag")))
            .ok_or(ProjectsError::NoDataDirectory)?;
        let dir = root.join("projects");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("projects.json");
        let backup = path.with_extension("json.bak");
        let source = if path.is_file() {
            Some(path.clone())
        } else if backup.is_file() {
            Some(backup.clone())
        } else {
            None
        };
        let document = if let Some(source) = source {
            let document: ProjectDocument = serde_json::from_slice(&std::fs::read(&source)?)?;
            if document.version != 1 {
                return Err(ProjectsError::UnsupportedVersion(document.version));
            }
            if source == backup {
                let _ = std::fs::copy(&backup, &path);
            }
            document
        } else {
            ProjectDocument::default()
        };
        Ok(Arc::new(Self {
            path,
            document: RwLock::new(document),
        }))
    }

    pub fn snapshot(&self) -> Vec<ProjectFolder> {
        self.document
            .read()
            .map(|doc| doc.projects.clone())
            .unwrap_or_default()
    }

    pub fn add(&self, raw_path: &str) -> Result<Vec<ProjectFolder>, ProjectsError> {
        let path = Path::new(raw_path.trim());
        if !path.is_dir() {
            return Err(ProjectsError::MissingFolder);
        }
        let path = std::fs::canonicalize(path)?.to_string_lossy().into_owned();
        let name = Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let mut doc = self.document.write().map_err(|_| ProjectsError::Poisoned)?;
        if !doc.projects.iter().any(|p| same_path(&p.path, &path)) {
            let mut updated = doc.clone();
            updated.projects.push(ProjectFolder { path, name });
            self.persist(&updated)?;
            *doc = updated;
        }
        Ok(doc.projects.clone())
    }

    pub fn remove(&self, path: &str) -> Result<Vec<ProjectFolder>, ProjectsError> {
        let mut doc = self.document.write().map_err(|_| ProjectsError::Poisoned)?;
        let mut updated = doc.clone();
        let old_len = updated.projects.len();
        updated
            .projects
            .retain(|project| !same_path(&project.path, path));
        if old_len != updated.projects.len() {
            self.persist(&updated)?;
            *doc = updated;
        }
        Ok(doc.projects.clone())
    }

    fn persist(&self, doc: &ProjectDocument) -> Result<(), ProjectsError> {
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
    a.replace('/', "\\")
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(b.replace('/', "\\").trim_end_matches('\\'))
}

#[derive(Debug, Error)]
pub enum ProjectsError {
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
pub struct ProjectsConfig;
pub struct ProjectsPlugin;

impl Plugin for ProjectsPlugin {
    type Config = ProjectsConfig;
    type Input = ProjectsConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = ProjectsError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, _: &Self::Input) -> Result<(), Self::ApplyError> {
        let navigation = context
            .try_service::<photag_plugin_navigation::NavigationService>()
            .map_err(|error| ProjectsError::Plugin(error.to_string()))?;
        let service = ProjectsService::open()?;
        let _service = context
            .provide(Arc::clone(&service))
            .map_err(|error| ProjectsError::Plugin(error.to_string()))?;
        let page_service = Arc::clone(&service);
        let page_navigation = Arc::clone(&navigation);
        photag_ui_bridge::register_views_for_plugin(
            &context,
            OWNER,
            [view::sidebar_page(page_service, page_navigation)],
        )
        .map_err(|error| ProjectsError::Plugin(error.to_string()))?;
        Ok(())
    }
}
