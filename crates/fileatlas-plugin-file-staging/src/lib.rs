//! Persistent path staging basket; actual transfers use FileOperationsService.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod mcp;
mod view;
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_file_operations::FileOperationsService;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
use thiserror::Error;

const OWNER: &str = "fileatlas.file-staging";
const PANEL: &str = "file-staging/panel";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedFile {
    pub path: String,
    pub added_at: u64,
    #[serde(default = "available_by_default")]
    pub available: bool,
}
fn available_by_default() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Document {
    #[serde(default = "version")]
    version: u32,
    #[serde(default)]
    files: Vec<StagedFile>,
}
fn version() -> u32 {
    1
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            files: Vec::new(),
        }
    }
}

pub struct FileStagingService {
    path: PathBuf,
    document: RwLock<Document>,
    revision: AtomicU64,
    wake: fileatlas_ui_bridge::refresh::Wake,
}
impl Service for FileStagingService {
    const NAME: &'static str = "fileatlas/file-staging";
}
impl FileStagingService {
    fn open() -> Result<Arc<Self>, StagingError> {
        let root = std::env::var_os("FILEATLAS_DEV_TEST_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::data_local_dir().map(|p| p.join("FileAtlas")))
            .ok_or(StagingError::NoDataDirectory)?;
        let dir = root.join("file-staging");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("staging.json");
        let backup = path.with_extension("json.bak");
        let source = if path.is_file() {
            Some(path.clone())
        } else if backup.is_file() {
            Some(backup.clone())
        } else {
            None
        };
        let document = if let Some(source) = source {
            let mut d: Document = serde_json::from_slice(&std::fs::read(&source)?)?;
            if d.version != 1 {
                return Err(StagingError::UnsupportedVersion(d.version));
            }
            d.files.retain(|item| !item.path.trim().is_empty());
            if source == backup {
                let _ = std::fs::copy(&backup, &path);
            }
            d
        } else {
            Document::default()
        };
        Ok(Arc::new(Self {
            path,
            document: RwLock::new(document),
            revision: AtomicU64::new(0),
            wake: Default::default(),
        }))
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    pub fn wake(&self) -> fileatlas_ui_bridge::refresh::Wake {
        self.wake.clone()
    }
    pub fn snapshot(&self) -> Vec<StagedFile> {
        self.document
            .read()
            .map(|d| d.files.clone())
            .unwrap_or_default()
    }
    pub fn add_paths(
        &self,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Vec<StagedFile>, StagingError> {
        let mut guard = self.document.write().map_err(|_| StagingError::Poisoned)?;
        let mut next = guard.clone();
        for raw in paths {
            let path = std::fs::canonicalize(&raw)?;
            if !path.is_file() {
                continue;
            }
            let value = path.to_string_lossy().into_owned();
            if !next
                .files
                .iter()
                .any(|entry| same_path(&entry.path, &value))
            {
                next.files.push(StagedFile {
                    path: value,
                    added_at: now(),
                    available: true,
                });
            }
        }
        if next.files == guard.files {
            return Ok(guard.files.clone());
        }
        self.persist(&next)?;
        *guard = next;
        self.changed();
        Ok(guard.files.clone())
    }
    pub fn remove(&self, path: &str) -> Result<Vec<StagedFile>, StagingError> {
        self.update(|d| d.files.retain(|f| !same_path(&f.path, path)))
    }
    pub fn remove_paths(&self, paths: &[String]) -> Result<Vec<StagedFile>, StagingError> {
        let paths = paths.iter().map(String::as_str).collect::<Vec<_>>();
        self.update(|d| {
            d.files
                .retain(|file| !paths.iter().any(|path| same_path(&file.path, path)));
        })
    }
    pub fn clear(&self) -> Result<Vec<StagedFile>, StagingError> {
        self.update(|d| d.files.clear())
    }
    pub fn remove_missing(&self) -> Result<Vec<StagedFile>, StagingError> {
        self.update(|d| {
            for file in &mut d.files {
                file.available = Path::new(&file.path).is_file();
            }
            d.files.retain(|f| f.available);
        })
    }
    pub fn refresh_status(&self) -> Result<Vec<StagedFile>, StagingError> {
        self.update(|d| {
            for file in &mut d.files {
                file.available = Path::new(&file.path).is_file();
            }
        })
    }
    fn update(&self, change: impl FnOnce(&mut Document)) -> Result<Vec<StagedFile>, StagingError> {
        let mut guard = self.document.write().map_err(|_| StagingError::Poisoned)?;
        let mut next = guard.clone();
        change(&mut next);
        if next.files == guard.files {
            return Ok(guard.files.clone());
        }
        self.persist(&next)?;
        *guard = next;
        self.changed();
        Ok(guard.files.clone())
    }
    fn changed(&self) {
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
    }
    fn persist(&self, document: &Document) -> Result<(), StagingError> {
        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(document)?)?;
        if let Err(error) = std::fs::rename(&temp, &self.path) {
            if !self.path.exists() {
                return Err(error.into());
            }
            let backup = self.path.with_extension("json.bak");
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(&self.path, &backup)?;
            if let Err(replace) = std::fs::rename(&temp, &self.path) {
                let _ = std::fs::rename(&backup, &self.path);
                return Err(replace.into());
            }
            let _ = std::fs::remove_file(backup);
        }
        Ok(())
    }
}
fn same_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(b.replace('/', "\\").trim_end_matches('\\'))
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or_default()
}

#[derive(Debug, Error)]
pub enum StagingError {
    #[error("no local application data directory is available")]
    NoDataDirectory,
    #[error("unsupported staging list version {0}")]
    UnsupportedVersion(u32),
    #[error("staging list lock was poisoned")]
    Poisoned,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("plugin dependency or registration failed: {0}")]
    Plugin(String),
}

#[derive(Clone, Copy)]
pub struct FileStagingPlugin;
impl Plugin for FileStagingPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = StagingError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), StagingError> {
        let operations = context
            .try_service::<FileOperationsService>()
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        let settings = context
            .try_service::<fileatlas_plugin_settings::SettingsService>()
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        let service = FileStagingService::open()?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        mcp::register(&context, &service, &operations)?;
        let event_service = Arc::clone(&service);
        let _add_listener = context
            .on::<fileatlas_contracts::FileStagingAddRequested, _>(cordis::event::observer_sync(
                move |_, paths: Vec<String>| {
                    let service = Arc::clone(&event_service);
                    tokio::spawn(async move {
                        let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
                        match tokio::task::spawn_blocking(move || service.add_paths(paths)).await {
                            Ok(Ok(_)) => {}
                            Ok(Err(error)) => {
                                tracing::warn!(%error, "could not add paths to staging basket")
                            }
                            Err(error) => tracing::warn!(%error, "staging add worker failed"),
                        }
                    });
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        let moved_service = Arc::clone(&service);
        let _moved_listener = context
            .on::<fileatlas_contracts::FileStagingPathsMoved, _>(
                cordis::event::observer_sync(move |_, paths: Vec<String>| {
                    let service = Arc::clone(&moved_service);
                    tokio::spawn(async move {
                        match tokio::task::spawn_blocking(move || service.remove_paths(&paths)).await
                        {
                            Ok(Ok(_)) => {}
                            Ok(Err(error)) => tracing::warn!(%error, "could not remove moved paths from staging basket"),
                            Err(error) => tracing::warn!(%error, "staging removal worker failed"),
                        }
                    });
                    Ok::<_, std::convert::Infallible>(())
                }),
            )
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        let bridge = context
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        let panels = Arc::clone(&bridge.panels);
        let command_panels = Arc::clone(&panels);
        let settings = Arc::clone(&settings);
        let _listener = context
            .on::<fileatlas_ui_bridge::UiCommandInvoked, _>(cordis::event::observer_sync(
                move |_, id| {
                    if id == "file-staging/open" {
                        let only = settings
                            .snapshot()
                            .map(|s| {
                                s.plugin_launch_mode(OWNER)
                                    == fileatlas_plugin_settings::PluginLaunchMode::PanelOnly
                            })
                            .unwrap_or(false);
                        let _ = command_panels.open_presented(PANEL, only);
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|e| StagingError::Plugin(e.to_string()))?;
        fileatlas_ui_bridge::register_for_plugin(
            &context,
            OWNER,
            [
                fileatlas_ui_bridge::UiContribution::new(
                    PANEL,
                    fileatlas_ui_bridge::ContributionKind::Panel {
                        label: "文件暂存篮".into(),
                        slot: PANEL.into(),
                        order: 0,
                    },
                ),
                fileatlas_ui_bridge::UiContribution::new(
                    "file-staging/open",
                    fileatlas_ui_bridge::ContributionKind::Command {
                        label: "打开文件暂存篮".into(),
                        needs: fileatlas_ui_bridge::CommandNeeds::None,
                    },
                ),
                fileatlas_ui_bridge::UiContribution::new(
                    "file-staging/sidebar",
                    fileatlas_ui_bridge::ContributionKind::Widget {
                        label: "暂存篮".into(),
                        slot: "navigation.launcher-shortcuts".into(),
                        order: 8,
                    },
                ),
            ],
        )
        .map_err(|e| StagingError::Plugin(e.to_string()))?;
        let panel_service = Arc::clone(&service);
        let panel_ops = Arc::clone(&operations);
        let sidebar_panels = Arc::clone(&panels);
        fileatlas_ui_bridge::panel_catalog::register_panel_for_plugin(
            &context,
            OWNER,
            fileatlas_ui_bridge::panel_catalog::PanelDefinition {
                options: fileatlas_ui_bridge::panels::PanelOptions {
                    id: PANEL.into(),
                    title: "文件暂存篮".into(),
                    modal: false,
                    default_size: (760., 560.),
                    min_size: (560., 420.),
                },
                multiplicity: fileatlas_ui_bridge::panel_catalog::PanelMultiplicity::Single,
                factory: Arc::new(move |window, app| {
                    view::panel(
                        Arc::clone(&panel_service),
                        Arc::clone(&panel_ops),
                        window,
                        app,
                    )
                }),
            },
        )
        .map_err(|e| StagingError::Plugin(e.to_string()))?;
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            OWNER,
            [view::sidebar_button(sidebar_panels)],
        )
        .map_err(|e| StagingError::Plugin(e.to_string()))?;
        Ok(())
    }
}
