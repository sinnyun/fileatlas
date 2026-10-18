#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! PhoTag launcher and output-folder manager for the upstream Shotoshot app.
//! Shotoshot owns the capture overlay, annotation editor, and save workflow.

mod shotoshot_settings;
mod view;

use cordis::{Context, Plugin, Service};
use photag_plugin_settings::{ScreenshotToolSettings, SettingsService};
use photag_ui_bridge::{
    ContributionKind, UiBridgeService, UiCommandInvoked, UiContribution, register_for_plugin,
    register_views_for_plugin,
};
use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};

const OWNER: &str = "photag.screenshot";
const PANEL: &str = "screenshot-tool/panel";
const OPEN_PANEL: &str = "screenshot-tool/open";
const CAPTURE: &str = "screenshot-tool/capture";

pub struct ScreenshotService {
    settings: Arc<SettingsService>,
    startup_error: Mutex<Option<String>>,
    installed_executable: Mutex<Option<PathBuf>>,
    last_capture_action: Mutex<Option<String>>,
    last_capture_error: Mutex<Option<String>>,
}

impl Service for ScreenshotService {
    const NAME: &'static str = "photag/screenshot";
}

impl ScreenshotService {
    fn new(settings: Arc<SettingsService>) -> Self {
        Self {
            settings,
            startup_error: Mutex::new(None),
            installed_executable: Mutex::new(None),
            last_capture_action: Mutex::new(None),
            last_capture_error: Mutex::new(None),
        }
    }

    fn initialize(&self) {
        // Resolve installation once per PhoTag launch. Rendering and capture
        // commands read this snapshot instead of rescanning disk locations.
        let configured = self
            .configuration()
            .ok()
            .and_then(|settings| settings.executable);
        let executable = environment_executable()
            .filter(|path| path.is_file())
            .or_else(|| configured.filter(|path| path.is_file()))
            .or_else(discover_shotoshot);
        if let Some(path) = executable.as_ref() {
            if let Ok(mut current) = self.installed_executable.lock() {
                *current = Some(path.clone());
            }
            let _ = self.set_executable(path.clone());
        }

        let result = self.ensure_output_directory();
        if let Err(error) = result {
            tracing::warn!(%error, "could not initialize Shotoshot output directory");
            if let Ok(mut current) = self.startup_error.lock() {
                *current = Some(error);
            }
        }
    }

    pub fn installed_executable(&self) -> Option<PathBuf> {
        self.installed_executable
            .lock()
            .ok()
            .and_then(|path| path.clone())
    }

    pub fn configuration(&self) -> Result<ScreenshotToolSettings, String> {
        self.settings
            .snapshot()
            .map(|settings| settings.screenshot_tool)
            .map_err(|error| error.to_string())
    }

    pub fn startup_error(&self) -> Option<String> {
        self.startup_error
            .lock()
            .ok()
            .and_then(|error| error.clone())
    }

    pub fn set_executable(&self, path: PathBuf) -> Result<(), String> {
        if !path.is_file() {
            return Err("所选文件不存在".into());
        }
        if path
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("exe"))
        {
            return Err("请选择 Shotoshot 的 Windows 可执行文件".into());
        }
        self.settings
            .update(|settings| settings.screenshot_tool.executable = Some(path))
            .map_err(|error| error.to_string())?;
        if let Ok(mut current) = self.installed_executable.lock() {
            *current = self
                .configuration()
                .ok()
                .and_then(|settings| settings.executable);
        }
        Ok(())
    }

    pub fn set_save_directory(&self, path: PathBuf) -> Result<(), String> {
        std::fs::create_dir_all(&path).map_err(|error| format!("创建截图目录失败：{error}"))?;
        if std::env::var_os("PHOTAG_DEV_TEST_DIR").is_none() {
            shotoshot_settings::set_save_folder(&path)?;
        }
        self.settings
            .update(|settings| settings.screenshot_tool.save_directory = Some(path))
            .map_err(|error| error.to_string())?;
        if let Ok(mut current) = self.startup_error.lock() {
            *current = None;
        }
        Ok(())
    }

    pub fn launch_capture(&self) -> Result<(), String> {
        tracing::info!("screenshot capture requested");
        if std::env::var_os("PHOTAG_DEV_TEST_DIR").is_some() {
            return self.capture_failed("隔离开发配置中已禁用真实桌面截图启动".into());
        }
        let executable = self
            .installed_executable()
            .ok_or_else(|| {
                "尚未检测到 Shotoshot。请先安装 Shotoshot，然后在截图工具设置中选择 shotoshot.exe。"
                    .to_owned()
            })
            .or_else(|error| self.capture_failed(error))?;
        let config = self
            .configuration()
            .or_else(|error| self.capture_failed(error))?;
        let directory = config
            .save_directory
            .filter(|path| path.is_dir())
            .unwrap_or(default_screenshot_directory()?);
        self.set_save_directory(directory)
            .or_else(|error| self.capture_failed(error))?;
        let forwarded =
            forward_capture_to_running_shotoshot().or_else(|error| self.capture_failed(error))?;
        if forwarded {
            self.record_capture_action("delivered_to_running_shotoshot");
            tracing::info!("capture request delivered to the running Shotoshot window");
            return Ok(());
        }
        Command::new(&executable)
            .arg("--capture")
            .spawn()
            .map(|_| {
                self.record_capture_action("started_shotoshot_process");
                tracing::info!(executable = %executable.display(), "started Shotoshot capture process");
            })
            .map_err(|error| format!("启动 Shotoshot 失败：{error}"))
            .or_else(|error| self.capture_failed(error))
    }

    fn capture_failed<T>(&self, error: String) -> Result<T, String> {
        if let Ok(mut current) = self.last_capture_error.lock() {
            *current = Some(error.clone());
        }
        tracing::error!(%error, "screenshot capture request failed");
        Err(error)
    }

    fn record_capture_action(&self, action: &str) {
        if let Ok(mut current) = self.last_capture_action.lock() {
            *current = Some(action.to_owned());
        }
        if let Ok(mut current) = self.last_capture_error.lock() {
            *current = None;
        }
    }

    fn ensure_output_directory(&self) -> Result<(), String> {
        let path = self
            .configuration()?
            .save_directory
            .unwrap_or(default_screenshot_directory()?);
        self.set_save_directory(path)
    }
}

/// Shotoshot 0.3.7 has no IPC flag for an existing resident. Forward its pinned
/// WM_APP+2 capture request directly, then use --capture only for first launch.
#[cfg(windows)]
fn forward_capture_to_running_shotoshot() -> Result<bool, String> {
    use windows::{
        Win32::{Foundation::*, UI::WindowsAndMessaging::*},
        core::w,
    };

    let Ok(window) = (unsafe { FindWindowW(w!("Shotoshot.Window"), w!("Shotoshot")) }) else {
        return Ok(false);
    };
    let mut result = 0;
    let delivered = unsafe {
        SendMessageTimeoutW(
            window,
            WM_APP + 2,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            3_000,
            Some(&mut result),
        )
    };
    if delivered.0 == 0 {
        let error = unsafe { GetLastError() };
        return Err(format!(
            "Shotoshot 未能处理截图请求（Windows 错误 {error:?}）"
        ));
    }
    Ok(true)
}

#[cfg(not(windows))]
fn forward_capture_to_running_shotoshot() -> Result<bool, String> {
    Ok(false)
}

impl ScreenshotService {
    pub fn mcp_state(&self) -> serde_json::Value {
        let config = self.configuration().ok();
        let configured = config.as_ref().and_then(|c| c.executable.clone());
        let executable = self.installed_executable();
        let directory = config
            .as_ref()
            .and_then(|c| c.save_directory.clone())
            .or_else(|| default_screenshot_directory().ok());
        serde_json::json!({
            "manager": "photag",
            "capture_editor": "shotoshot",
            "configured_executable": configured,
            "effective_executable": executable,
            "executable_available": executable.is_some(),
            "save_directory": directory,
            "save_directory_exists": directory.as_ref().is_some_and(|path| path.is_dir()),
            "isolated_profile": std::env::var_os("PHOTAG_DEV_TEST_DIR").is_some(),
            "capture_argument": "--capture",
            "last_capture_action": self.last_capture_action.lock().ok().and_then(|value| value.clone()),
            "last_capture_error": self.last_capture_error.lock().ok().and_then(|value| value.clone()),
            "startup_error": self.startup_error(),
        })
    }
}

fn default_screenshot_directory() -> Result<PathBuf, String> {
    if let Some(profile) = std::env::var_os("PHOTAG_DEV_TEST_DIR") {
        return Ok(PathBuf::from(profile).join("screenshots"));
    }
    dirs::data_local_dir()
        .map(|path| path.join("PhoTag").join("Screenshots"))
        .ok_or_else(|| "无法确定 Windows 本地应用数据目录".into())
}

fn environment_executable() -> Option<PathBuf> {
    std::env::var_os("PHOTAG_SHOTOSHOT_EXE").map(PathBuf::from)
}

fn discover_shotoshot() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("PHOTAG_SHOTOSHOT_EXE") {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(current) = std::env::current_exe()
        && let Some(directory) = current.parent()
    {
        candidates.push(directory.join("shotoshot.exe"));
        if let Some(target) = directory.parent() {
            for profile in ["debug", "release"] {
                candidates.push(target.join("shotoshot").join(profile).join("shotoshot.exe"));
            }
        }
    }
    if let Some(local) = dirs::data_local_dir() {
        candidates.push(
            local
                .join("Programs")
                .join("Shotoshot")
                .join("shotoshot.exe"),
        );
    }
    candidates.into_iter().find(|path| path.is_file())
}

#[derive(Default)]
pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = ScreenshotPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }

    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }

    async fn apply(&self, context: Context, _: &()) -> Result<(), ScreenshotPluginError> {
        let settings = context
            .try_service::<SettingsService>()
            .map_err(|error| ScreenshotPluginError::Dependency(error.to_string()))?;
        let service = Arc::new(ScreenshotService::new(settings));
        service.initialize();
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| ScreenshotPluginError::Publish(error.to_string()))?;
        let bridge = context
            .try_service::<UiBridgeService>()
            .map_err(|error| ScreenshotPluginError::Dependency(error.to_string()))?;
        let panels = Arc::clone(&bridge.panels);
        let command_panels = Arc::clone(&panels);
        let capture_service = Arc::clone(&service);
        let _listener = context
            .on::<UiCommandInvoked, _>(cordis::event::observer_sync(move |_, id: String| {
                match id.as_str() {
                    OPEN_PANEL => {
                        let _ = command_panels.open(PANEL);
                    }
                    CAPTURE => {
                        if let Err(error) = capture_service.launch_capture() {
                            tracing::error!(%error, "screenshot capture could not start");
                        }
                    }
                    _ => {}
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| ScreenshotPluginError::EventRegistration(error.to_string()))?;

        register_for_plugin(
            &context,
            OWNER,
            [
                UiContribution::new(
                    "screenshot-tool/panel-slot",
                    ContributionKind::Slot {
                        label: "workspace.screenshot-tool".into(),
                        order: 30,
                    },
                ),
                UiContribution::new(
                    "screenshot-tool/panel-contribution",
                    ContributionKind::Panel {
                        label: "截图工具".into(),
                        slot: "workspace.screenshot-tool".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    OPEN_PANEL,
                    ContributionKind::Command {
                        label: "打开截图工具管理".into(),
                        needs: photag_ui_bridge::CommandNeeds::None,
                    },
                ),
                UiContribution::new(
                    CAPTURE,
                    ContributionKind::Command {
                        label: "启动截图".into(),
                        needs: photag_ui_bridge::CommandNeeds::None,
                    },
                ),
                UiContribution::new(
                    "screenshot-tool/capture-key",
                    ContributionKind::Keybinding {
                        key: "Ctrl+Shift+S".into(),
                        command_id: CAPTURE.into(),
                        scope: photag_ui_bridge::ShortcutScope::System,
                    },
                ),
                UiContribution::new(
                    "screenshot-tool/sidebar-button",
                    ContributionKind::Widget {
                        label: "截图".into(),
                        slot: "navigation.launcher-shortcuts".into(),
                        order: 7,
                    },
                ),
            ],
        )
        .map_err(|error| ScreenshotPluginError::UiRegistration(error.to_string()))?;

        let panel_service = Arc::clone(&service);
        let surfaces = Arc::clone(&bridge.surfaces);
        photag_ui_bridge::panel_catalog::register_panel_for_plugin(
            &context,
            OWNER,
            photag_ui_bridge::panel_catalog::PanelDefinition {
                options: photag_ui_bridge::panels::PanelOptions {
                    id: PANEL.into(),
                    title: "截图工具".into(),
                    default_size: (640., 420.),
                    min_size: (540., 360.),
                },
                multiplicity: photag_ui_bridge::panel_catalog::PanelMultiplicity::Single,
                factory: Arc::new(move |window, app| {
                    view::panel(
                        Arc::clone(&panel_service),
                        Arc::clone(&surfaces),
                        window,
                        app,
                    )
                }),
            },
        )
        .map_err(|error| ScreenshotPluginError::UiRegistration(error.to_string()))?;
        register_views_for_plugin(&context, OWNER, [view::sidebar_button(panels)])
            .map_err(|error| ScreenshotPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScreenshotPluginError {
    #[error("screenshot plugin dependency unavailable: {0}")]
    Dependency(String),
    #[error("screenshot service publication failed: {0}")]
    Publish(String),
    #[error("screenshot plugin event registration failed: {0}")]
    EventRegistration(String),
    #[error("screenshot plugin UI registration failed: {0}")]
    UiRegistration(String),
}
