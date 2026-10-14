use crate::{PreviewService, ai_suggestions, preview_view};
use cordis::{Context, Plugin};
use photag_plugin_ai::AiService;
use photag_plugin_database::DatabaseService;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_history::HistoryService;
use photag_plugin_session::SessionService;
use photag_plugin_settings::SettingsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::{
    ContributionKind, UiContribution, register_for_plugin, register_views_for_plugin,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("preview source does not exist: {0}")]
    MissingSource(PathBuf),
    #[error("preview state lock is poisoned")]
    StatePoisoned,
    #[error("could not decode preview image {0}: {1}")]
    Decode(PathBuf, String),
    #[error("could not write transformed preview {0}: {1}")]
    Write(PathBuf, String),
    #[error("could not initialize preview cache: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not publish preview service: {0}")]
    Publish(String),
    #[error("session service is unavailable")]
    SessionUnavailable,
    #[error("required preview integration service is unavailable: {0}")]
    IntegrationServiceUnavailable(&'static str),
    #[error("could not register preview UI: {0}")]
    Ui(String),
}

#[derive(Debug, Clone, Default)]
pub struct PreviewConfig {
    pub cache_dir: Option<PathBuf>,
}

pub struct PreviewPlugin;
impl Plugin for PreviewPlugin {
    type Config = PreviewConfig;
    type Input = PreviewConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = PreviewError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.preview".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let directory = config.cache_dir.clone().unwrap_or_else(default_cache_dir);
        let session = context
            .try_service::<SessionService>()
            .map_err(|_| PreviewError::SessionUnavailable)?;
        let ai = context
            .try_service::<AiService>()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("photag/ai"))?;
        let settings = context
            .try_service::<SettingsService>()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("photag/settings"))?;
        let tagging = context
            .try_service::<TaggingService>()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("photag/tagging"))?;
        let history = context
            .try_service::<HistoryService>()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("photag/history"))?;
        let file_operations = context
            .try_service::<FileOperationsService>()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("photag/file-operations"))?;
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("photag/database"))?;
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| PreviewError::IntegrationServiceUnavailable("tokio runtime"))?;
        let service = Arc::new(PreviewService::open_with_session_at(
            directory,
            Arc::clone(&session),
        )?);
        service.configure_database(database);
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| PreviewError::Publish(error.to_string()))?;
        register_for_plugin(
            &context,
            "photag.preview",
            [
                UiContribution::new(
                    "preview/image",
                    ContributionKind::Panel {
                        label: "Image preview".into(),
                        slot: "workspace.modal".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "preview/ai-suggestions",
                    ContributionKind::Panel {
                        label: "AI tag suggestions".into(),
                        slot: "workspace.main".into(),
                        order: 1,
                    },
                ),
            ],
        )
        .map_err(|error| PreviewError::Ui(error.to_string()))?;
        register_views_for_plugin(
            &context,
            "photag.preview",
            [
                preview_view(
                    Arc::clone(&service),
                    Arc::clone(&settings),
                    Arc::clone(&session),
                    Arc::clone(&tagging),
                    Some(Arc::clone(&history)),
                    context.clone(),
                    file_operations,
                    runtime,
                ),
                ai_suggestions::view(service, ai, settings, tagging, history),
            ],
        )
        .map_err(|error| PreviewError::Ui(error.to_string()))
    }
}

fn default_cache_dir() -> PathBuf {
    dirs_cache_dir().join("PhoTag").join("preview")
}
fn dirs_cache_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}
