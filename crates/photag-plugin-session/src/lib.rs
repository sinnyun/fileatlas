//! Cordis-owned persistence for PhoTag's last-opened navigation and view state.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use cordis::{Context, Plugin, Service};

pub mod session;
pub use session::*;

pub struct SessionService {
    state: RwLock<SessionState>,
    path: PathBuf,
}

impl Service for SessionService {
    const NAME: &'static str = "photag/session";
}

impl SessionService {
    /// Load the original `%LOCALAPPDATA%/PhoTag/session.json` file.
    pub fn open_default() -> Self {
        Self::open_at(default_session_path())
    }

    /// Load a session file, defaulting when the file is missing or invalid.
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

    pub fn snapshot(&self) -> Result<SessionState, SessionServiceError> {
        self.state
            .read()
            .map(|state| state.clone())
            .map_err(|_| SessionServiceError::Poisoned)
    }

    /// Replace the saved session and serialize it using the original field names.
    pub fn replace(&self, session: SessionState) -> Result<(), SessionServiceError> {
        {
            let mut state = self
                .state
                .write()
                .map_err(|_| SessionServiceError::Poisoned)?;
            *state = session.clone();
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(&session)?;
        std::fs::write(&self.path, serialized)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionServiceError {
    #[error("session lock is poisoned")]
    Poisoned,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    pub path: Option<PathBuf>,
}

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    type Config = SessionConfig;
    type Input = SessionConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = SessionPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.session".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let service = Arc::new(match &config.path {
            Some(path) => SessionService::open_at(path.clone()),
            None => SessionService::open_default(),
        });
        let _publication = context
            .provide(service)
            .map_err(|error| SessionPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionPluginError {
    #[error("session service publication failed: {0}")]
    Publish(String),
}
