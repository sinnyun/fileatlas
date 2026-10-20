//! Main-window column layout and active-column routing state.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_workspaces::ColumnLayout;
use fileatlas_ui_bridge::refresh::Wake;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

mod view;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnState {
    pub layout: ColumnLayout,
    pub active_index: usize,
    pub revision: u64,
}

impl Default for ColumnState {
    fn default() -> Self {
        Self {
            layout: ColumnLayout::Single,
            active_index: 0,
            revision: 0,
        }
    }
}

pub struct ColumnLayoutService {
    state: RwLock<ColumnState>,
    wake: Wake,
}

impl Service for ColumnLayoutService {
    const NAME: &'static str = "fileatlas/columns";
}

impl ColumnLayoutService {
    pub fn snapshot(&self) -> Result<ColumnState, ColumnError> {
        self.state
            .read()
            .map(|state| *state)
            .map_err(|_| ColumnError::Poisoned)
    }
    pub fn wake(&self) -> Wake {
        self.wake.clone()
    }
    pub fn set_layout(&self, layout: ColumnLayout) -> Result<ColumnState, ColumnError> {
        let mut state = self.state.write().map_err(|_| ColumnError::Poisoned)?;
        if state.layout == layout {
            return Ok(*state);
        }
        state.layout = layout;
        state.active_index = state
            .active_index
            .min(layout.column_count().saturating_sub(1));
        state.revision = state.revision.wrapping_add(1);
        let updated = *state;
        self.wake.trigger();
        Ok(updated)
    }
    pub fn activate(&self, index: usize) -> Result<ColumnState, ColumnError> {
        let mut state = self.state.write().map_err(|_| ColumnError::Poisoned)?;
        if index >= state.layout.column_count() {
            return Err(ColumnError::InvalidColumn(index));
        }
        if state.active_index != index {
            state.active_index = index;
            state.revision = state.revision.wrapping_add(1);
            self.wake.trigger();
        }
        Ok(*state)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ColumnError {
    #[error("column state lock is poisoned")]
    Poisoned,
    #[error("column index is outside the active layout: {0}")]
    InvalidColumn(usize),
    #[error("workspace service is unavailable: {0}")]
    Dependency(String),
    #[error("could not publish column service: {0}")]
    Publish(String),
    #[error("could not register column UI: {0}")]
    Ui(String),
}

pub struct ColumnsPlugin;
impl Plugin for ColumnsPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = ColumnError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.columns".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let workspace = context
            .try_service::<fileatlas_plugin_workspaces::WorkspaceService>()
            .map_err(|error| ColumnError::Dependency(error.to_string()))?;
        let active = workspace
            .active()
            .map_err(|error| ColumnError::Dependency(error.to_string()))?;
        let service = Arc::new(ColumnLayoutService {
            state: RwLock::new(ColumnState {
                layout: active.layout,
                active_index: 0,
                revision: 0,
            }),
            wake: Wake::default(),
        });
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| ColumnError::Publish(error.to_string()))?;
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            "fileatlas.columns",
            [view::toolbar(service, workspace)],
        )
        .map_err(|error| ColumnError::Ui(error.to_string()))?;
        Ok(())
    }
}
