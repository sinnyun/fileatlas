//! Cordis lifecycle adapter for the file operation service.

use std::sync::Arc;

use cordis::{Context, Plugin};
use photag_plugin_database::DatabaseService;

use crate::FileOperationsService;

#[derive(Debug, Clone, Default)]
pub struct FileOperationsConfig;

pub struct FileOperationsPlugin;

impl Plugin for FileOperationsPlugin {
    type Config = FileOperationsConfig;
    type Input = FileOperationsConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = FileOperationsPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.file-operations".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| FileOperationsPluginError::Dependency(error.to_string()))?;
        let _publication = context
            .provide(Arc::new(FileOperationsService::new(
                database,
                context.clone(),
            )))
            .map_err(|error| FileOperationsPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileOperationsPluginError {
    #[error("required file operations dependency unavailable: {0}")]
    Dependency(String),
    #[error("file operations service publication failed: {0}")]
    Publish(String),
}
