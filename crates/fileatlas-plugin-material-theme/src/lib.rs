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
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            seed: "#6686a3".into(),
            variant: "neutral".into(),
            contrast: 0.0,
        }
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
            Ok(text) => match serde_json::from_str::<Preferences>(&text) {
                Ok(p) if p.schema_version == 1 => (p, None),
                Ok(_) => (
                    Preferences::default(),
                    Some("Unsupported saved Material schema; file retained".into()),
                ),
                Err(e) => (Preferences::default(), Some(e.to_string())),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Preferences::default(), None),
            Err(e) => (Preferences::default(), Some(e.to_string())),
        };
        let output = root.join("material.json");
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
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "Material operation lock")?;
        let result: Result<State, String> = (|| {
            let json = generate::generate(&preferences)?;
            std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
            let output = self.root.join("material.json");
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
                std::fs::rename(staging, path).map_err(|e| e.to_string())?;
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
