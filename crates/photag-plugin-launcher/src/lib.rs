#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Personal program launcher backed by PhoTag's shared application database.

mod discovery;
mod icons;
mod view;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cordis::{Context, Plugin, Service};
use photag_plugin_database::{DatabaseAccessError, DatabaseService, LauncherDiscoveredProgram};
use photag_ui_bridge::{
    ContributionKind, UiBridgeService, UiCommandInvoked, UiContribution, register_for_plugin,
    register_views_for_plugin,
};

const OWNER: &str = "photag.launcher";
const PANEL_ID: &str = "program-launcher/panel";
const OPEN_COMMAND: &str = "program-launcher/open";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherCategory {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherEntry {
    pub id: i64,
    pub display_name: String,
    pub launch_path: PathBuf,
    pub uninstall_command: Option<String>,
    pub category_id: i64,
    pub hidden: bool,
    pub source: String,
    pub available: bool,
    pub installed_on: Option<String>,
    pub size_bytes: Option<i64>,
    pub pin_scope: PinScope,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PinScope {
    #[default]
    None,
    Category,
    AllPrograms,
}

impl PinScope {
    fn from_db(value: i64) -> Self {
        match value {
            1 => Self::Category,
            2 => Self::AllPrograms,
            _ => Self::None,
        }
    }

    fn to_db(self) -> i64 {
        match self {
            Self::None => 0,
            Self::Category => 1,
            Self::AllPrograms => 2,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LauncherSnapshot {
    pub categories: Vec<LauncherCategory>,
    pub entries: Vec<LauncherEntry>,
    pub scanning: bool,
    pub revision: u64,
}

pub struct LauncherService {
    database: Arc<DatabaseService>,
    icon_cache_dir: PathBuf,
    state: Mutex<LauncherSnapshot>,
    scanning: AtomicBool,
    changed: photag_ui_bridge::refresh::Wake,
    icon_cache: Mutex<icons::IconSet>,
}

impl Service for LauncherService {
    const NAME: &'static str = "photag/launcher";
}

impl LauncherService {
    fn open(database: Arc<DatabaseService>) -> Result<Arc<Self>, LauncherError> {
        database.launcher_ensure_default_category()?;
        let icon_cache_dir = database
            .path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("program-icons");
        let service = Arc::new(Self {
            database,
            icon_cache_dir,
            state: Mutex::new(LauncherSnapshot::default()),
            scanning: AtomicBool::new(false),
            changed: Default::default(),
            icon_cache: Default::default(),
        });
        service.reload()?;
        Ok(service)
    }

    fn read_icons(&self, paths: Vec<PathBuf>) -> Result<icons::IconSet, String> {
        let cached = self
            .icon_cache
            .lock()
            .map_err(|_| "Icon cache unavailable")?
            .clone();
        let icons = icons::load(paths, cached, &self.icon_cache_dir)?;
        *self
            .icon_cache
            .lock()
            .map_err(|_| "Icon cache unavailable")? = icons.clone();
        Ok(icons)
    }

    pub fn wake(&self) -> photag_ui_bridge::refresh::Wake {
        self.changed.clone()
    }

    pub fn snapshot(&self) -> LauncherSnapshot {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    fn reload(&self) -> Result<(), LauncherError> {
        let (categories, entries) = self.database.launcher_snapshot()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| LauncherError::StatePoisoned)?;
        state.categories = categories
            .into_iter()
            .map(|category| LauncherCategory {
                id: category.id,
                name: category.name,
                position: category.position,
            })
            .collect();
        state.entries = entries
            .into_iter()
            .map(|entry| LauncherEntry {
                available: Path::new(&entry.launch_path).exists(),
                id: entry.id,
                display_name: entry.display_name,
                launch_path: PathBuf::from(entry.launch_path),
                uninstall_command: entry.uninstall_command,
                category_id: entry.category_id,
                hidden: entry.hidden,
                source: entry.source,
                installed_on: entry.installed_on,
                size_bytes: entry.size_bytes,
                pin_scope: PinScope::from_db(entry.pin_scope),
            })
            .collect();
        state.scanning = self.scanning.load(Ordering::Acquire);
        state.revision = state.revision.wrapping_add(1);
        drop(state);
        self.changed.trigger();
        Ok(())
    }

    pub fn refresh_discovery(self: &Arc<Self>) -> Result<(), LauncherError> {
        if self.scanning.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        self.reload()?;
        let service = Arc::clone(self);
        if let Err(error) = std::thread::Builder::new()
            .name("photag-program-discovery".into())
            .spawn(move || {
                let result = discovery::discover();
                if let Err(error) = result
                    .map_err(LauncherError::Discovery)
                    .and_then(|found| service.merge_discovered(found))
                {
                    tracing::error!(%error, "program discovery failed");
                }
                service.scanning.store(false, Ordering::Release);
                if let Err(error) = service.reload() {
                    tracing::error!(%error, "program launcher refresh failed");
                }
            })
        {
            self.scanning.store(false, Ordering::Release);
            self.reload()?;
            return Err(LauncherError::Io(error));
        }
        Ok(())
    }

    fn merge_discovered(
        &self,
        found: Vec<discovery::DiscoveredProgram>,
    ) -> Result<(), LauncherError> {
        let records = found
            .into_iter()
            .map(|program| LauncherDiscoveredProgram {
                discovery_key: program.key,
                display_name: program.name,
                launch_path: program.launch_path.to_string_lossy().into_owned(),
                uninstall_command: program.uninstall_command,
                category_name: program.category_name,
                source: program.source,
                installed_on: program.installed_on,
                size_bytes: program.size_bytes,
            })
            .collect::<Vec<_>>();
        Ok(self.database.launcher_merge_discovered(&records)?)
    }

    pub fn add_category(&self, name: &str) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.database.launcher_add_category(name)?;
        self.reload()
    }

    pub fn rename_category(&self, id: i64, name: &str) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.database.launcher_rename_category(id, name)?;
        self.reload()
    }

    pub fn delete_category(&self, id: i64) -> Result<(), LauncherError> {
        self.database.launcher_delete_category(id)?;
        self.reload()
    }

    pub fn update_entry(
        &self,
        id: i64,
        name: &str,
        category_id: i64,
        hidden: bool,
    ) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.database
            .launcher_update_entry(id, name, category_id, hidden)?;
        self.reload()
    }

    pub fn rename_entry(&self, id: i64, name: &str) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.database.launcher_rename_entry(id, name)?;
        self.reload()
    }

    pub fn move_entry(&self, id: i64, category_id: i64) -> Result<(), LauncherError> {
        self.database.launcher_move_entry(id, category_id)?;
        self.reload()
    }

    pub fn set_hidden(&self, id: i64, hidden: bool) -> Result<(), LauncherError> {
        self.database.launcher_set_hidden(id, hidden)?;
        self.reload()
    }

    pub fn set_pin_scope(&self, id: i64, pin_scope: PinScope) -> Result<(), LauncherError> {
        self.database
            .launcher_set_pin_scope(id, pin_scope.to_db())?;
        self.reload()
    }

    pub fn remove_entry(&self, id: i64) -> Result<(), LauncherError> {
        self.database.launcher_remove_entry(id)?;
        self.reload()
    }

    pub fn add_manual(&self, path: &Path) -> Result<(), LauncherError> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !path.is_file() {
            return Err(LauncherError::InvalidPath(path));
        }
        if !path.extension().is_some_and(|extension| {
            ["exe", "com", "bat", "cmd", "lnk"]
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        }) {
            return Err(LauncherError::UnsupportedProgram(path));
        }
        let name = path
            .file_stem()
            .and_then(|part| part.to_str())
            .unwrap_or("Program");
        let key = format!(
            "path:{}",
            path.to_string_lossy().replace('/', "\\").to_lowercase()
        );
        self.database.launcher_add_manual(&path, &key, name)?;
        self.reload()
    }

    pub fn launch(&self, id: i64) -> Result<(), LauncherError> {
        let entry = self
            .snapshot()
            .entries
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or(LauncherError::EntryMissing)?;
        if !entry.launch_path.is_file() {
            return Err(LauncherError::InvalidPath(entry.launch_path));
        }
        match entry
            .launch_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("exe" | "com") => {
                std::process::Command::new(&entry.launch_path)
                    .spawn()
                    .map_err(LauncherError::Io)?;
            }
            Some("bat" | "cmd") => {
                std::process::Command::new("cmd.exe")
                    .args(["/C", "start", ""])
                    .arg(&entry.launch_path)
                    .spawn()
                    .map_err(LauncherError::Io)?;
            }
            _ => {
                std::process::Command::new("explorer.exe")
                    .arg(&entry.launch_path)
                    .spawn()
                    .map_err(LauncherError::Io)?;
            }
        }
        Ok(())
    }

    pub fn uninstall(&self, id: i64) -> Result<(), LauncherError> {
        let entry = self
            .snapshot()
            .entries
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or(LauncherError::EntryMissing)?;
        let command = entry
            .uninstall_command
            .ok_or(LauncherError::UninstallUnavailable)?;
        let confirmed = rfd::MessageDialog::new()
            .set_title(photag_ui_bridge::i18n::tr("Uninstall application"))
            .set_description(format!(
                "确认运行 {} 的卸载程序？\n\n{}",
                entry.display_name, command
            ))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes;
        if !confirmed {
            return Ok(());
        }
        Self::run_uninstaller(&command)
    }

    /// Start the registered uninstaller after the caller has obtained explicit confirmation.
    pub fn uninstall_confirmed(&self, id: i64) -> Result<(), LauncherError> {
        let entry = self
            .snapshot()
            .entries
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or(LauncherError::EntryMissing)?;
        let command = entry
            .uninstall_command
            .ok_or(LauncherError::UninstallUnavailable)?;
        Self::run_uninstaller(&command)
    }

    fn run_uninstaller(command: &str) -> Result<(), LauncherError> {
        std::process::Command::new("cmd.exe")
            .args(["/C", &command])
            .spawn()
            .map_err(LauncherError::Io)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LauncherError {
    #[error(transparent)]
    Database(#[from] DatabaseAccessError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("name cannot be empty")]
    EmptyName,
    #[error("program entry no longer exists")]
    EntryMissing,
    #[error("no registered uninstaller is available for this entry")]
    UninstallUnavailable,
    #[error("path is missing or invalid: {0}")]
    InvalidPath(PathBuf),
    #[error("unsupported program or shortcut: {0}")]
    UnsupportedProgram(PathBuf),
    #[error("program launcher state lock is poisoned")]
    StatePoisoned,
    #[error("program discovery failed: {0}")]
    Discovery(String),
}

#[derive(Debug, Clone, Default)]
pub struct LauncherConfig;
pub struct LauncherPlugin;

impl Plugin for LauncherPlugin {
    type Config = LauncherConfig;
    type Input = LauncherConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = LauncherPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| LauncherPluginError::Dependency(error.to_string()))?;
        let service = LauncherService::open(database)
            .map_err(|error| LauncherPluginError::Start(error.to_string()))?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| LauncherPluginError::Publish(error.to_string()))?;
        let bridge = context
            .try_service::<UiBridgeService>()
            .map_err(|error| LauncherPluginError::Dependency(error.to_string()))?;
        let panels = Arc::clone(&bridge.panels);
        let command_panels = Arc::clone(&panels);
        let _listener = context
            .on::<UiCommandInvoked, _>(cordis::event::observer_sync(move |_, command_id| {
                if command_id == OPEN_COMMAND {
                    let _ = command_panels.open(PANEL_ID);
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| LauncherPluginError::Ui(error.to_string()))?;
        register_for_plugin(
            &context,
            OWNER,
            [
                UiContribution::new(
                    "program-launcher/panel-slot",
                    ContributionKind::Slot {
                        label: PANEL_ID.into(),
                        order: 40,
                    },
                ),
                UiContribution::new(
                    "program-launcher/panel",
                    ContributionKind::Panel {
                        label: "Program Launchpad".into(),
                        slot: PANEL_ID.into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    OPEN_COMMAND,
                    ContributionKind::Command {
                        label: "Open Program Launchpad".into(),
                        needs: photag_ui_bridge::CommandNeeds::None,
                    },
                ),
                UiContribution::new(
                    "program-launcher/sidebar-button",
                    ContributionKind::Widget {
                        label: "Program Launchpad".into(),
                        slot: "navigation.launcher-shortcuts".into(),
                        order: 0,
                    },
                ),
            ],
        )
        .map_err(|error| LauncherPluginError::Ui(error.to_string()))?;
        let panel_service = Arc::clone(&service);
        let panel_surfaces = Arc::clone(&bridge.surfaces);
        photag_ui_bridge::panel_catalog::register_panel_for_plugin(
            &context,
            OWNER,
            photag_ui_bridge::panel_catalog::PanelDefinition {
                options: photag_ui_bridge::panels::PanelOptions {
                    id: PANEL_ID.into(),
                    title: "Program Launchpad".into(),
                    default_size: (1200., 760.),
                    min_size: (900., 540.),
                },
                multiplicity: photag_ui_bridge::panel_catalog::PanelMultiplicity::Single,
                factory: Arc::new(move |window, app| {
                    view::panel(
                        Arc::clone(&panel_service),
                        Arc::clone(&panel_surfaces),
                        window,
                        app,
                    )
                }),
            },
        )
        .map_err(|error| LauncherPluginError::Ui(error))?;
        register_views_for_plugin(&context, OWNER, [view::sidebar_button(panels)])
            .map_err(|error| LauncherPluginError::Ui(error.to_string()))?;
        service
            .refresh_discovery()
            .map_err(|error| LauncherPluginError::Start(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LauncherPluginError {
    #[error("required launcher dependency unavailable: {0}")]
    Dependency(String),
    #[error("could not start launcher plugin: {0}")]
    Start(String),
    #[error("could not publish launcher service: {0}")]
    Publish(String),
    #[error("launcher UI registration failed: {0}")]
    Ui(String),
}
