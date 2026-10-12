//! Cordis-owned PhoTag settings persistence service.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use cordis::{Context, Plugin, Service};

pub mod settings;
pub use settings::*;

/// Shared, mutable user preferences backed by the original settings JSON file.
pub struct SettingsService {
    state: RwLock<Settings>,
    path: PathBuf,
}

impl Service for SettingsService {
    const NAME: &'static str = "photag/settings";
}

impl SettingsService {
    /// Load settings using PhoTag's default-on-missing-or-invalid behavior.
    pub fn open_default() -> Self {
        Self::open_at(default_settings_path())
    }

    /// Load settings from an explicit path, preserving the original JSON shape.
    pub fn open_at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            state: RwLock::new(state),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn snapshot(&self) -> Result<Settings, SettingsServiceError> {
        self.state
            .read()
            .map(|state| state.clone())
            .map_err(|_| SettingsServiceError::Poisoned)
    }

    /// Replace preferences and persist them to the source-compatible file.
    pub fn replace(&self, settings: Settings) -> Result<(), SettingsServiceError> {
        {
            let mut state = self
                .state
                .write()
                .map_err(|_| SettingsServiceError::Poisoned)?;
            *state = settings.clone();
        }
        self.persist(&settings)
    }

    /// Apply one preference edit and persist the resulting snapshot.
    pub fn update(
        &self,
        edit: impl FnOnce(&mut Settings),
    ) -> Result<Settings, SettingsServiceError> {
        let settings = {
            let mut state = self
                .state
                .write()
                .map_err(|_| SettingsServiceError::Poisoned)?;
            edit(&mut state);
            state.clone()
        };
        self.persist(&settings)?;
        Ok(settings)
    }

    fn persist(&self, settings: &Settings) -> Result<(), SettingsServiceError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(settings)?;
        std::fs::write(&self.path, serialized)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsServiceError {
    #[error("settings lock is poisoned")]
    Poisoned,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default)]
pub struct SettingsConfig {
    pub path: Option<PathBuf>,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    type Config = SettingsConfig;
    type Input = SettingsConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = SettingsPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.settings".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let service = Arc::new(match &config.path {
            Some(path) => SettingsService::open_at(path.clone()),
            None => SettingsService::open_default(),
        });
        context
            .provide(service)
            .map_err(|error| SettingsPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsPluginError {
    #[error("settings service publication failed: {0}")]
    Publish(String),
}
