#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Cordis service for FileAtlas's local MobileNetV2 tag suggestions.
//!
//! Inference and model download run on worker threads, matching the source
//! application's asynchronous contract.

use std::sync::{Arc, mpsc};

use cordis::{Context, Plugin, Service};
use fileatlas_plugin_settings::{AiSettings, AiTagGroup};

mod engine;
mod imagenet;

/// One source-compatible AI tag suggestion.
#[derive(Debug, Clone)]
pub struct TagSuggestion {
    pub label: String,
    pub confidence: f32,
    /// Present for the selected suggestion from a mandatory tag group.
    pub group_name: Option<String>,
}

/// Shared local inference API.
pub struct AiService;

impl Service for AiService {
    const NAME: &'static str = "fileatlas/ai";
}

impl AiService {
    pub fn model_exists(&self) -> bool {
        engine::model_exists()
    }

    /// Download the ONNX Model Zoo model. Optional messages report progress.
    pub fn download_model(&self, progress: Option<mpsc::Sender<String>>) -> Result<(), String> {
        engine::download_model(progress)
    }

    /// Start model download on a worker thread and receive its result.
    pub fn download_model_async(&self) -> mpsc::Receiver<Result<(), String>> {
        engine::download_model_async()
    }

    /// Start inference with the configured mandatory groups on a worker thread.
    pub fn suggest_tags(
        &self,
        image_path: String,
        top_n: usize,
        settings: &AiSettings,
    ) -> mpsc::Receiver<Result<Vec<TagSuggestion>, String>> {
        self.suggest_tags_with_groups(image_path, top_n, settings.mandatory_groups.clone())
    }

    /// Source-compatible entry point for callers that already hold the
    /// mandatory groups rather than a full persisted settings snapshot.
    pub fn suggest_tags_with_groups(
        &self,
        image_path: String,
        top_n: usize,
        mandatory_groups: Vec<AiTagGroup>,
    ) -> mpsc::Receiver<Result<Vec<TagSuggestion>, String>> {
        engine::suggest_tags_async(image_path, top_n, mandatory_groups)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AiConfig;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    type Config = AiConfig;
    type Input = AiConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = AiPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.ai".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let _publication = context
            .provide(Arc::new(AiService))
            .map_err(|error| AiPluginError::Publish(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AiPluginError {
    #[error("AI service publication failed: {0}")]
    Publish(String),
}
