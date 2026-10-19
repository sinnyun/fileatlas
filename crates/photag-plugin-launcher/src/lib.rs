#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Personal program launcher backed by its feature-owned JSON data file.

mod discovery;
mod icons;
mod mcp;
mod view;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cordis::{Context, Plugin, Service};
use photag_plugin_database::{DatabaseAccessError, DatabaseService, LauncherDiscoveredProgram};
use photag_plugin_settings::{PluginLaunchMode, SettingsService};
use photag_ui_bridge::{
    ContributionKind, UiBridgeService, UiCommandInvoked, UiContribution, register_for_plugin,
    register_views_for_plugin,
};

const OWNER: &str = "photag.launcher";
const PANEL_ID: &str = "program-launcher/panel";
const OPEN_COMMAND: &str = "program-launcher/open";

fn discovery_key(path: &Path) -> String {
    format!(
        "path:{}",
        path.to_string_lossy().replace('/', "\\").to_lowercase()
    )
}

fn persist_launcher(path: &Path, snapshot: &LauncherSnapshot) -> Result<(), LauncherError> {
    let bytes = serde_json::to_vec_pretty(snapshot)?;
    let parent = path
        .parent()
        .ok_or_else(|| LauncherError::Io(std::io::Error::other("invalid launcher data path")))?;
    std::fs::create_dir_all(parent)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes)?;
    if let Err(error) = std::fs::rename(&temp, path) {
        if path.exists() {
            let backup = path.with_extension("json.bak");
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(path, &backup)?;
            if let Err(replace_error) = std::fs::rename(&temp, path) {
                let _ = std::fs::rename(&backup, path);
                return Err(LauncherError::Io(replace_error));
            }
            let _ = std::fs::remove_file(backup);
        } else {
            return Err(LauncherError::Io(error));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LauncherCategory {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    #[serde(default)]
    pub description: String,
    pub pin_scope: PinScope,
    #[serde(default)]
    discovery_key: String,
    #[serde(default)]
    custom_name: bool,
    #[serde(default)]
    custom_category: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LauncherSnapshot {
    #[serde(default = "launcher_data_version")]
    pub version: u32,
    pub categories: Vec<LauncherCategory>,
    pub entries: Vec<LauncherEntry>,
    pub scanning: bool,
    pub revision: u64,
}

fn launcher_data_version() -> u32 {
    1
}

pub struct LauncherService {
    data_path: PathBuf,
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
        let root = if let Some(profile) = std::env::var_os("PHOTAG_DEV_TEST_DIR") {
            PathBuf::from(profile)
        } else {
            database
                .path()
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        };
        let feature_dir = root.join("launcher");
        std::fs::create_dir_all(&feature_dir)?;
        let data_path = feature_dir.join("launcher.json");
        let icon_cache_dir = feature_dir.join("icons");
        let legacy_icon_dir = root.join("program-icons");
        if legacy_icon_dir.is_dir() {
            std::fs::create_dir_all(&icon_cache_dir)?;
            for item in std::fs::read_dir(&legacy_icon_dir)?.flatten() {
                let source = item.path();
                let destination = icon_cache_dir.join(item.file_name());
                if source.is_file() && !destination.exists() {
                    std::fs::copy(source, destination)?;
                }
            }
        }
        let snapshot = if data_path.is_file() {
            let mut loaded =
                serde_json::from_slice::<LauncherSnapshot>(&std::fs::read(&data_path)?)?;
            if loaded.version != 1 {
                return Err(LauncherError::UnsupportedDataVersion(loaded.version));
            }
            loaded.entries.retain(|entry| {
                !discovery::is_non_program_entry(&entry.display_name, &entry.launch_path)
            });
            for entry in &mut loaded.entries {
                if entry.discovery_key.is_empty() {
                    entry.discovery_key = discovery_key(&entry.launch_path);
                }
                let suggested_category =
                    discovery::classify_program(&entry.display_name, &entry.launch_path);
                if !entry.custom_category {
                    if let Some(category) = loaded
                        .categories
                        .iter()
                        .find(|category| category.name == suggested_category)
                    {
                        entry.category_id = category.id;
                    }
                }
                let category = loaded
                    .categories
                    .iter()
                    .find(|category| category.id == entry.category_id)
                    .map(|category| category.name.as_str())
                    .unwrap_or("其他");
                if discovery::has_product_description(&entry.display_name)
                    || entry.description.trim().is_empty()
                {
                    entry.description = discovery::program_description(
                        &entry.display_name,
                        &entry.launch_path,
                        category,
                    );
                }
            }
            loaded
        } else {
            database.launcher_ensure_default_category()?;
            let (categories, entries) = database.launcher_snapshot()?;
            let snapshot = LauncherSnapshot {
                version: 1,
                categories: categories
                    .into_iter()
                    .map(|c| LauncherCategory {
                        id: c.id,
                        name: c.name,
                        position: c.position,
                    })
                    .collect(),
                entries: entries
                    .into_iter()
                    .map(|e| {
                        let description = discovery::program_description(
                            &e.display_name,
                            Path::new(&e.launch_path),
                            "其他",
                        );
                        LauncherEntry {
                            available: Path::new(&e.launch_path).exists(),
                            id: e.id,
                            display_name: e.display_name,
                            launch_path: PathBuf::from(e.launch_path),
                            uninstall_command: e.uninstall_command,
                            category_id: e.category_id,
                            hidden: e.hidden,
                            source: e.source,
                            installed_on: e.installed_on,
                            size_bytes: e.size_bytes,
                            description,
                            pin_scope: PinScope::from_db(e.pin_scope),
                            discovery_key: e.discovery_key,
                            custom_name: e.custom_name,
                            custom_category: e.custom_category,
                        }
                    })
                    .collect(),
                scanning: false,
                revision: 0,
            };
            persist_launcher(&data_path, &snapshot)?;
            snapshot
        };
        let service = Arc::new(Self {
            data_path,
            icon_cache_dir,
            state: Mutex::new(snapshot),
            scanning: AtomicBool::new(false),
            changed: Default::default(),
            icon_cache: Default::default(),
        });
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
        let mut state = self
            .state
            .lock()
            .map_err(|_| LauncherError::StatePoisoned)?;
        for entry in &mut state.entries {
            entry.available = entry.launch_path.exists();
        }
        state.scanning = self.scanning.load(Ordering::Acquire);
        state.revision = state.revision.wrapping_add(1);
        persist_launcher(&self.data_path, &state)?;
        drop(state);
        self.changed.trigger();
        Ok(())
    }

    fn edit(
        &self,
        f: impl FnOnce(&mut LauncherSnapshot) -> Result<(), LauncherError>,
    ) -> Result<(), LauncherError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| LauncherError::StatePoisoned)?;
        f(&mut state)?;
        persist_launcher(&self.data_path, &state)?;
        drop(state);
        self.reload()
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
            .map(|program| {
                (
                    LauncherDiscoveredProgram {
                        discovery_key: program.key,
                        display_name: program.name,
                        launch_path: program.launch_path.to_string_lossy().into_owned(),
                        uninstall_command: program.uninstall_command,
                        category_name: program.category_name,
                        source: program.source,
                        installed_on: program.installed_on,
                        size_bytes: program.size_bytes,
                    },
                    program.description,
                )
            })
            .collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .map_err(|_| LauncherError::StatePoisoned)?;
        state.entries.retain(|entry| {
            !discovery::is_non_program_entry(&entry.display_name, &entry.launch_path)
        });
        for (record, description) in records {
            let category_id = if let Some(category) = state
                .categories
                .iter()
                .find(|c| c.name == record.category_name)
            {
                category.id
            } else {
                let id = state.categories.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                let position = state
                    .categories
                    .iter()
                    .map(|c| c.position)
                    .max()
                    .unwrap_or(-1)
                    + 1;
                state.categories.push(LauncherCategory {
                    id,
                    name: record.category_name.clone(),
                    position,
                });
                id
            };
            if let Some(entry) = state
                .entries
                .iter_mut()
                .find(|e| e.discovery_key == record.discovery_key)
            {
                if !entry.custom_name {
                    entry.display_name = record.display_name;
                }
                entry.launch_path = PathBuf::from(record.launch_path);
                entry.uninstall_command =
                    record.uninstall_command.or(entry.uninstall_command.take());
                if !entry.custom_category {
                    entry.category_id = category_id;
                }
                entry.source = record.source;
                entry.installed_on = record.installed_on;
                entry.size_bytes = record.size_bytes;
                entry.description = description;
            } else {
                let id = state.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                state.entries.push(LauncherEntry {
                    available: Path::new(&record.launch_path).exists(),
                    id,
                    display_name: record.display_name,
                    launch_path: PathBuf::from(record.launch_path),
                    uninstall_command: record.uninstall_command,
                    category_id,
                    hidden: false,
                    source: record.source,
                    installed_on: record.installed_on,
                    size_bytes: record.size_bytes,
                    description,
                    pin_scope: PinScope::None,
                    discovery_key: record.discovery_key,
                    custom_name: false,
                    custom_category: false,
                });
            }
        }
        persist_launcher(&self.data_path, &state)?;
        Ok(())
    }

    pub fn add_category(&self, name: &str) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.edit(|state| {
            if state
                .categories
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(name))
            {
                return Err(LauncherError::EmptyName);
            }
            let id = state.categories.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            let position = state
                .categories
                .iter()
                .map(|c| c.position)
                .max()
                .unwrap_or(-1)
                + 1;
            state.categories.push(LauncherCategory {
                id,
                name: name.into(),
                position,
            });
            Ok(())
        })
    }

    pub fn rename_category(&self, id: i64, name: &str) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.edit(|state| {
            let category = state
                .categories
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or(LauncherError::EntryMissing)?;
            category.name = name.into();
            Ok(())
        })
    }

    pub fn delete_category(&self, id: i64) -> Result<(), LauncherError> {
        self.edit(|state| {
            if state
                .categories
                .iter()
                .find(|c| c.id == id)
                .is_some_and(|c| c.name == "General")
            {
                return Err(LauncherError::EntryMissing);
            }
            let general = state
                .categories
                .iter()
                .find(|c| c.name == "General")
                .map(|c| c.id)
                .ok_or(LauncherError::EntryMissing)?;
            state.categories.retain(|c| c.id != id);
            for e in &mut state.entries {
                if e.category_id == id {
                    e.category_id = general;
                    e.custom_category = true;
                }
            }
            Ok(())
        })
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
        self.edit(|state| {
            if !state.categories.iter().any(|c| c.id == category_id) {
                return Err(LauncherError::EntryMissing);
            }
            let category_name = state
                .categories
                .iter()
                .find(|c| c.id == category_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "其他".into());
            let e = state
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(LauncherError::EntryMissing)?;
            if e.display_name != name {
                e.custom_name = true;
            }
            if e.category_id != category_id {
                e.custom_category = true;
            }
            e.display_name = name.into();
            e.category_id = category_id;
            e.hidden = hidden;
            e.description =
                discovery::program_description(&e.display_name, &e.launch_path, &category_name);
            Ok(())
        })
    }

    pub fn rename_entry(&self, id: i64, name: &str) -> Result<(), LauncherError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LauncherError::EmptyName);
        }
        self.edit(|state| {
            let category_name = state
                .entries
                .iter()
                .find(|entry| entry.id == id)
                .and_then(|entry| {
                    state
                        .categories
                        .iter()
                        .find(|category| category.id == entry.category_id)
                })
                .map(|category| category.name.clone())
                .unwrap_or_else(|| "其他".into());
            let e = state
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(LauncherError::EntryMissing)?;
            e.display_name = name.into();
            e.custom_name = true;
            e.description =
                discovery::program_description(&e.display_name, &e.launch_path, &category_name);
            Ok(())
        })
    }

    pub fn move_entry(&self, id: i64, category_id: i64) -> Result<(), LauncherError> {
        self.edit(|state| {
            let category_name = state
                .categories
                .iter()
                .find(|category| category.id == category_id)
                .map(|category| category.name.clone())
                .ok_or(LauncherError::EntryMissing)?;
            let e = state
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(LauncherError::EntryMissing)?;
            e.category_id = category_id;
            e.custom_category = true;
            e.description =
                discovery::program_description(&e.display_name, &e.launch_path, &category_name);
            Ok(())
        })
    }

    pub fn set_hidden(&self, id: i64, hidden: bool) -> Result<(), LauncherError> {
        self.edit(|state| {
            state
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(LauncherError::EntryMissing)?
                .hidden = hidden;
            Ok(())
        })
    }

    pub fn set_pin_scope(&self, id: i64, pin_scope: PinScope) -> Result<(), LauncherError> {
        self.edit(|state| {
            state
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(LauncherError::EntryMissing)?
                .pin_scope = pin_scope;
            Ok(())
        })
    }

    pub fn remove_entry(&self, id: i64) -> Result<(), LauncherError> {
        self.edit(|state| {
            let before = state.entries.len();
            state.entries.retain(|e| e.id != id);
            if before == state.entries.len() {
                return Err(LauncherError::EntryMissing);
            }
            Ok(())
        })
    }

    pub fn add_manual(&self, path: &Path) -> Result<(), LauncherError> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !path.is_file() {
            return Err(LauncherError::InvalidPath(path));
        }
        if !discovery::is_launch_target(&path) {
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
        self.edit(|state| {
            if state
                .entries
                .iter()
                .any(|e| discovery_key(&e.launch_path) == key)
            {
                return Ok(());
            }
            let id = state.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
            let category = state
                .categories
                .iter()
                .find(|c| c.name == "General")
                .map(|c| c.id)
                .ok_or(LauncherError::EntryMissing)?;
            state.entries.push(LauncherEntry {
                available: true,
                id,
                display_name: name.into(),
                launch_path: path.clone(),
                uninstall_command: None,
                category_id: category,
                hidden: false,
                source: "manual".into(),
                installed_on: None,
                size_bytes: None,
                description: discovery::program_description(name, &path, "常用"),
                pin_scope: PinScope::None,
                discovery_key: key,
                custom_name: false,
                custom_category: false,
            });
            Ok(())
        })
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
    #[error(transparent)]
    Json(#[from] serde_json::Error),
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
    #[error("unsupported launcher JSON version: {0}")]
    UnsupportedDataVersion(u32),
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
        if let Ok(registry) = context.try_service::<photag_contracts::mcp::McpToolRegistry>() {
            let snapshot_service = Arc::clone(&service);
            let snapshot = photag_contracts::mcp::McpToolDefinition {
                name: "launcher.snapshot".into(),
                title: "Read program launcher".into(),
                description: "Read the launcher's current categories, programs, and scan state.".into(),
                input_schema: serde_json::json!({"type":"object","properties":{},"additionalProperties":false})
                    .as_object().cloned().unwrap_or_default(),
                mutating: false,
                confirmation_required: false,
                handler: Arc::new(move |_| {
                    let service = Arc::clone(&snapshot_service);
                    Box::pin(async move {
                        serde_json::to_value(service.snapshot()).map_err(|error| error.to_string())
                    })
                }),
            };
            let launch_service = Arc::clone(&service);
            let launch = photag_contracts::mcp::McpToolDefinition {
                name: "launcher.launch".into(),
                title: "Launch a registered program".into(),
                description: "Launch one program entry by its launcher entry ID.".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"id":{"type":"integer","minimum":1}},"required":["id"],"additionalProperties":false})
                    .as_object().cloned().unwrap_or_default(),
                mutating: true,
                confirmation_required: true,
                handler: Arc::new(move |arguments| {
                    let service = Arc::clone(&launch_service);
                    Box::pin(async move {
                        let id = arguments.get("id").and_then(serde_json::Value::as_i64)
                            .ok_or_else(|| "id must be a positive integer".to_owned())?;
                        service.launch(id).map_err(|error| error.to_string())?;
                        Ok(serde_json::json!({"ok":true,"id":id}))
                    })
                }),
            };
            let mut tools = mcp::tools(&service);
            tools.extend([snapshot, launch]);
            let registration = registry
                .register(OWNER, tools)
                .map_err(|error| LauncherPluginError::Ui(error.to_string()))?;
            let _mcp_registration = context
                .provide(Arc::new(LauncherMcpRegistration {
                    _registration: registration,
                }))
                .map_err(|error| LauncherPluginError::Publish(error.to_string()))?;
        }
        let bridge = context
            .try_service::<UiBridgeService>()
            .map_err(|error| LauncherPluginError::Dependency(error.to_string()))?;
        let settings = context
            .try_service::<SettingsService>()
            .map_err(|error| LauncherPluginError::Dependency(error.to_string()))?;
        let panels = Arc::clone(&bridge.panels);
        let command_panels = Arc::clone(&panels);
        let _listener = context
            .on::<UiCommandInvoked, _>(cordis::event::observer_sync(move |_, command_id| {
                if command_id == OPEN_COMMAND {
                    let mode = settings
                        .snapshot()
                        .map(|settings| settings.plugin_launch_mode(OWNER))
                        .unwrap_or_default();
                    let _ = command_panels
                        .open_presented(PANEL_ID, mode == PluginLaunchMode::PanelOnly);
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
                    "program-launcher/shortcut-open",
                    ContributionKind::Keybinding {
                        key: "Ctrl+F".into(),
                        command_id: OPEN_COMMAND.into(),
                        scope: photag_ui_bridge::ShortcutScope::System,
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

struct LauncherMcpRegistration {
    _registration: Arc<photag_contracts::mcp::McpToolRegistration>,
}

impl Service for LauncherMcpRegistration {
    const NAME: &'static str = "photag/launcher-mcp-registration";
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
