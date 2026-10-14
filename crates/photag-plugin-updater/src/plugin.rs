use std::sync::Arc;

use cordis::{Context, Plugin};

use crate::UpdaterService;

#[derive(Debug, Clone, Default)]
pub struct UpdaterConfig;

pub struct UpdaterPlugin;

impl Plugin for UpdaterPlugin {
    type Config = UpdaterConfig;
    type Input = UpdaterConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = UpdaterPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.updater".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        crate::cleanup_previous();
        let _publication = context
            .provide(Arc::new(UpdaterService::default()))
            .map_err(|error| UpdaterPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdaterPluginError {
    #[error("updater service publication failed: {0}")]
    Publish(String),
}
