//! Cordis feature plugin: Material JSON producer for the existing GPUI theme system.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_settings::SettingsService;
use fileatlas_ui_bridge::UiBridgeService;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
mod generate;
mod mcp;
mod view;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub schema_version: u32,
    pub seed: String,
    pub variant: String,
    pub contrast: f64,
    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default)]
    pub manual_colors: std::collections::BTreeMap<String, String>,
}
fn default_theme_name() -> String {
    "material".into()
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 3,
            seed: "#6686a3".into(),
            variant: "neutral".into(),
            contrast: 0.0,
            theme_name: default_theme_name(),
            manual_colors: Default::default(),
        }
    }
}
const PALETTE_ROLES: &[&str] = &[
    "background",
    "foreground",
    "secondary.background",
    "success.background",
    "warning.background",
    "danger.background",
    "info.background",
];

fn migrate_preferences(mut preferences: Preferences) -> Result<Preferences, String> {
    match preferences.schema_version {
        1 | 2 => {
            let legacy = std::mem::take(&mut preferences.manual_colors);
            let mut migrated = std::collections::BTreeMap::new();
            // Old primary/accent overrides become the single primary seed input.
            if let Some(color) = legacy
                .get("accent.background")
                .or_else(|| legacy.get("primary.background"))
                .or_else(|| legacy.get("button.primary.background"))
            {
                preferences.seed = color.clone();
            }
            for (family, roles) in [
                ("background", &["background"][..]),
                ("foreground", &["foreground"][..]),
                (
                    "secondary.background",
                    &["secondary.background", "button.secondary.background"][..],
                ),
                ("success.background", &["success.background"][..]),
                ("warning.background", &["warning.background"][..]),
                ("danger.background", &["danger.background"][..]),
                ("info.background", &["info.background"][..]),
            ] {
                if let Some(color) = roles.iter().find_map(|role| legacy.get(*role)) {
                    migrated.insert(family.to_owned(), color.clone());
                }
            }
            // Preserve older advanced edits by folding them into their nearest semantic family.
            for (role, color) in legacy {
                let family = if role.contains("success") {
                    Some("success.background")
                } else if role.contains("warning") {
                    Some("warning.background")
                } else if role.contains("danger") || role.contains("error") {
                    Some("danger.background")
                } else if role.contains("info") {
                    Some("info.background")
                } else if role.starts_with("secondary.") || role.starts_with("button.secondary.") {
                    Some("secondary.background")
                } else if role.starts_with("primary.")
                    || role.starts_with("button.primary.")
                    || role.starts_with("accent.")
                {
                    None
                } else if role.ends_with(".foreground") || role == "foreground" {
                    Some("foreground")
                } else if role.ends_with(".background")
                    && !role.starts_with("primary.")
                    && !role.starts_with("accent.")
                {
                    Some("background")
                } else {
                    None
                };
                if let Some(family) = family.filter(|family| !migrated.contains_key(*family)) {
                    migrated.insert(family.to_owned(), color);
                }
            }
            preferences.manual_colors = migrated;
            preferences.schema_version = 3;
            Ok(preferences)
        }
        3 => {
            if let Some(role) = preferences
                .manual_colors
                .keys()
                .find(|role| !PALETTE_ROLES.contains(&role.as_str()))
            {
                return Err(format!("Unknown semantic palette role: {role}"));
            }
            Ok(preferences)
        }
        _ => Err("Unsupported Material preferences version".into()),
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct State {
    pub preferences: Preferences,
    pub output: PathBuf,
    #[serde(rename = "last_error")]
    pub error: Option<String>,
    pub generated: bool,
}
pub struct MaterialThemeService {
    settings: Arc<SettingsService>,
    root: PathBuf,
    state: RwLock<State>,
    operation: std::sync::Mutex<()>,
}
impl Service for MaterialThemeService {
    const NAME: &'static str = "fileatlas/material-theme";
}
impl MaterialThemeService {
    fn open(settings: Arc<SettingsService>) -> Self {
        let root = settings
            .path()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("themes");
        let loaded = std::fs::read_to_string(root.join("material-settings.json"));
        let (preferences, error) = match loaded {
            Ok(text) => match serde_json::from_str::<Preferences>(&text)
                .map_err(|error| error.to_string())
                .and_then(migrate_preferences)
            {
                Ok(p) => (p, None),
                Err(error) => (Preferences::default(), Some(error)),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Preferences::default(), None),
            Err(e) => (Preferences::default(), Some(e.to_string())),
        };
        let stored_name =
            safe_theme_name(&preferences.theme_name).unwrap_or_else(|| "material".into());
        let output = root.join(format!("{stored_name}.json"));
        let generated = std::fs::read_to_string(&output)
            .ok()
            .and_then(|t| serde_json::from_str::<gpui_kit::component::ThemeSet>(&t).ok())
            .is_some();
        Self {
            settings,
            root,
            operation: Default::default(),
            state: RwLock::new(State {
                preferences,
                output,
                error,
                generated,
            }),
        }
    }
    pub fn snapshot(&self) -> State {
        self.state.read().expect("material state").clone()
    }
    pub fn generate(&self, preferences: Preferences, apply: bool) -> Result<State, String> {
        let mut preferences = preferences;
        preferences.theme_name = "material".into();
        self.generate_to(preferences, "material".into(), apply)
    }
    pub fn generate_named(
        &self,
        preferences: Preferences,
        name: String,
        apply: bool,
    ) -> Result<State, String> {
        let safe = safe_theme_name(&name).ok_or("主题名称只能包含字母、数字、连字符或下划线")?;
        let mut preferences = preferences;
        preferences.theme_name = safe.clone();
        self.generate_to(preferences, safe, apply)
    }
    fn generate_to(
        &self,
        preferences: Preferences,
        file_stem: String,
        apply: bool,
    ) -> Result<State, String> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "Material operation lock")?;
        let result: Result<State, String> = (|| {
            let preferences = migrate_preferences(preferences)?;
            let json = generate::generate(&preferences)?;
            std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
            let output = self.root.join(format!("{file_stem}.json"));
            for (path, text) in [
                (&output, json),
                (
                    &self.root.join("material-settings.json"),
                    serde_json::to_string_pretty(&preferences).map_err(|e| e.to_string())?,
                ),
            ] {
                let staging = path.with_extension("json.tmp");
                std::fs::write(&staging, text).map_err(|e| e.to_string())?;
                if path.exists() {
                    std::fs::copy(path, path.with_extension("json.bak"))
                        .map_err(|e| e.to_string())?;
                }
                replace_file(&staging, path).map_err(|e| e.to_string())?;
            }
            if apply {
                self.settings
                    .update(|s| s.theme_file = Some(output.clone()))
                    .map_err(|e| e.to_string())?;
            }
            let next = State {
                preferences,
                output,
                error: None,
                generated: true,
            };
            *self.state.write().map_err(|_| "Material state lock")? = next.clone();
            Ok(next)
        })();
        if let Err(error) = &result {
            self.state.write().map_err(|_| "Material state lock")?.error = Some(error.clone());
        }
        result
    }
    pub fn apply_saved(&self) -> Result<(), String> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "Material operation lock")?;
        let state = self.snapshot();
        if !state.generated {
            return Err("Generate a theme first".into());
        }
        let text = std::fs::read_to_string(&state.output).map_err(|e| e.to_string())?;
        let mode = self
            .settings
            .snapshot()
            .map_err(|e| e.to_string())?
            .color_mode;
        fileatlas_ui_bridge::theme::request_theme(&mode, Some(text))?;
        self.settings
            .update(|s| s.theme_file = Some(state.output))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn replace_file(staging: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let wide = |path: &std::path::Path| {
            path.as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let staging = wide(staging);
        let target = wide(target);
        unsafe {
            MoveFileExW(
                windows::core::PCWSTR(staging.as_ptr()),
                windows::core::PCWSTR(target.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(std::io::Error::other)
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(staging, target)
    }
}

fn safe_theme_name(name: &str) -> Option<String> {
    (!name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'))
    .then(|| name.to_owned())
}
pub struct MaterialThemePlugin;
impl Plugin for MaterialThemePlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = std::io::Error;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.material-theme".into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), std::io::Error> {
        let settings = context
            .try_service::<SettingsService>()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let _bridge = context
            .try_service::<UiBridgeService>()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let service = Arc::new(MaterialThemeService::open(settings));
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            "fileatlas.material-theme",
            [view::contribution(service.clone())],
        )
        .map_err(|e| std::io::Error::other(e.to_string()))?;
        mcp::register(&context, service.clone()).map_err(std::io::Error::other)?;
        let _service = context
            .provide(service)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
