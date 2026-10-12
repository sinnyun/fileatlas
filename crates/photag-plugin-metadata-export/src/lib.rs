//! PhoTag's complete metadata backup export exposed as a Cordis service.

use std::path::Path;
use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::db::queries::{self, MediaExport};

pub struct MetadataExportService {
    database: Arc<DatabaseService>,
}

impl Service for MetadataExportService {
    const NAME: &'static str = "photag/metadata-export";
}

impl MetadataExportService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self { database }
    }

    /// Return all media rows in source order with source-compatible tag paths.
    pub fn records(&self) -> Result<Vec<MediaExport>, MetadataExportError> {
        Ok(self.database.with_reader(queries::export_all_metadata)?)
    }

    /// Serialize the same top-level JSON array and field names as PhoTag.
    pub fn to_pretty_json(&self) -> Result<String, MetadataExportError> {
        Ok(serde_json::to_string_pretty(&self.records()?)?)
    }

    /// Write a metadata backup to a caller-selected destination.
    ///
    /// Invoke this away from the GPUI render thread for large libraries. The
    /// caller owns path selection and reports completion/errors to the user.
    pub fn write_to(&self, path: impl AsRef<Path>) -> Result<(), MetadataExportError> {
        std::fs::write(path, self.to_pretty_json()?)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataExportError {
    #[error(transparent)]
    Database(#[from] photag_plugin_database::DatabaseAccessError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct MetadataExportConfig;

pub struct MetadataExportPlugin;

impl Plugin for MetadataExportPlugin {
    type Config = MetadataExportConfig;
    type Input = MetadataExportConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = MetadataExportPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.metadata-export".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| MetadataExportPluginError::Dependency(error.to_string()))?;
        context
            .provide(Arc::new(MetadataExportService::new(database)))
            .map_err(|error| MetadataExportPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataExportPluginError {
    #[error("required metadata export dependency unavailable: {0}")]
    Dependency(String),
    #[error("metadata export service publication failed: {0}")]
    Publish(String),
}
