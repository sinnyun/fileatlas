//! Cordis publication lifecycle for the drag-out service.

use std::sync::Arc;

use cordis::{Context, Plugin};

use crate::DragDropService;

#[derive(Debug, Clone, Default)]
pub struct DragDropConfig;

pub struct DragDropPlugin;

impl Plugin for DragDropPlugin {
    type Config = DragDropConfig;
    type Input = DragDropConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = DragDropPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.drag-drop".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let _publication = context
            .provide(Arc::new(DragDropService::default()))
            .map_err(|error| DragDropPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DragDropPluginError {
    #[error("could not publish drag-drop service: {0}")]
    Publish(String),
}
