//! PhoTag's complete metadata backup export exposed as a Cordis service.

mod view;

use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cordis::event::observer_sync;
use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::db::queries::{self, MediaExport};
use photag_ui_bridge::{
    ContributionKind, UiCommandInvoked, UiContribution, register_for_plugin,
    register_views_for_plugin,
};

const EXPORT_COMMAND_ID: &str = "metadata-export/export";

pub struct MetadataExportService {
    database: Arc<DatabaseService>,
    destination: Mutex<Option<PathBuf>>,
    status: Mutex<MetadataExportStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MetadataExportStatus {
    #[default]
    Idle,
    Exporting,
    Succeeded {
        path: PathBuf,
        records: usize,
    },
    Failed(String),
}

impl Service for MetadataExportService {
    const NAME: &'static str = "photag/metadata-export";
}

impl MetadataExportService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self {
            database,
            destination: Mutex::new(None),
            status: Mutex::new(MetadataExportStatus::Idle),
        }
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

    /// Set the path used by the lifecycle-registered export command.
    pub fn set_destination(&self, path: impl Into<PathBuf>) -> Result<(), MetadataExportError> {
        *self
            .destination
            .lock()
            .map_err(|_| MetadataExportError::StatePoisoned)? = Some(path.into());
        let mut status = self
            .status
            .lock()
            .map_err(|_| MetadataExportError::StatePoisoned)?;
        if !matches!(&*status, MetadataExportStatus::Exporting) {
            *status = MetadataExportStatus::Idle;
        }
        Ok(())
    }

    /// Snapshot the current command progress for the GPUI panel.
    pub fn status(&self) -> Result<MetadataExportStatus, MetadataExportError> {
        self.status
            .lock()
            .map(|status| status.clone())
            .map_err(|_| MetadataExportError::StatePoisoned)
    }

    fn start_export(self: &Arc<Self>) -> Result<(), MetadataExportError> {
        let path = self
            .destination
            .lock()
            .map_err(|_| MetadataExportError::StatePoisoned)?
            .clone()
            .ok_or(MetadataExportError::DestinationMissing)?;
        let mut status = self
            .status
            .lock()
            .map_err(|_| MetadataExportError::StatePoisoned)?;
        if matches!(&*status, MetadataExportStatus::Exporting) {
            return Err(MetadataExportError::AlreadyRunning);
        }
        *status = MetadataExportStatus::Exporting;
        drop(status);
        let service = Arc::clone(self);
        if let Err(error) = std::thread::Builder::new()
            .name("photag-metadata-export".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    service.records().and_then(|records| {
                        let count = records.len();
                        let json = serde_json::to_string_pretty(&records)?;
                        std::fs::write(&path, json)?;
                        Ok(count)
                    })
                }));
                let status = match result {
                    Ok(Ok(records)) => MetadataExportStatus::Succeeded { path, records },
                    Ok(Err(error)) => MetadataExportStatus::Failed(error.to_string()),
                    Err(_) => MetadataExportStatus::Failed(
                        "export worker stopped unexpectedly; see application logs for details"
                            .into(),
                    ),
                };
                if let Ok(mut current) = service.status.lock() {
                    *current = status;
                }
            })
        {
            // A failed thread spawn must not strand the panel in an endless
            // Exporting state. The listener exposes this same error to users.
            let mut current = self
                .status
                .lock()
                .map_err(|_| MetadataExportError::StatePoisoned)?;
            if matches!(&*current, MetadataExportStatus::Exporting) {
                *current = MetadataExportStatus::Failed(error.to_string());
            }
            return Err(MetadataExportError::Io(error));
        }
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
    #[error("metadata export destination has not been selected")]
    DestinationMissing,
    #[error("a metadata export is already running")]
    AlreadyRunning,
    #[error("metadata export state lock is poisoned")]
    StatePoisoned,
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
        let service = Arc::new(MetadataExportService::new(database));
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| MetadataExportPluginError::Publish(error.to_string()))?;
        let command_service = Arc::clone(&service);
        let _command_listener = context
            .on::<UiCommandInvoked, _>(observer_sync(move |_, command_id| {
                if command_id == EXPORT_COMMAND_ID
                    && let Err(error) = command_service.start_export()
                {
                    if !matches!(&error, MetadataExportError::AlreadyRunning)
                        && let Ok(mut status) = command_service.status.lock()
                        && !matches!(&*status, MetadataExportStatus::Exporting)
                    {
                        *status = MetadataExportStatus::Failed(error.to_string());
                    }
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| MetadataExportPluginError::EventRegistration(error.to_string()))?;
        register_for_plugin(
            &context,
            "photag.metadata-export",
            [
                UiContribution::new(
                    "metadata-export/panel-slot",
                    ContributionKind::Slot {
                        label: "workspace.metadata-export".into(),
                        order: 20,
                    },
                ),
                UiContribution::new(
                    "metadata-export/panel",
                    ContributionKind::Panel {
                        label: "Metadata Backup Export".into(),
                        slot: "workspace.metadata-export".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    EXPORT_COMMAND_ID,
                    ContributionKind::Command {
                        label: "Export metadata…".into(),
                    },
                ),
                UiContribution::new(
                    "metadata-export/file-menu-item",
                    ContributionKind::MenuItem {
                        label: "Export metadata…".into(),
                        menu: "File".into(),
                        command_id: EXPORT_COMMAND_ID.into(),
                        order: 80,
                    },
                ),
            ],
        )
        .map_err(|error| MetadataExportPluginError::UiRegistration(error.to_string()))?;
        register_views_for_plugin(
            &context,
            "photag.metadata-export",
            [view::export_panel(service, context.clone())],
        )
        .map_err(|error| MetadataExportPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataExportPluginError {
    #[error("required metadata export dependency unavailable: {0}")]
    Dependency(String),
    #[error("metadata export service publication failed: {0}")]
    Publish(String),
    #[error("metadata export UI registration failed: {0}")]
    UiRegistration(String),
    #[error("metadata export command event registration failed: {0}")]
    EventRegistration(String),
}
