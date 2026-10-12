//! Source-compatible image preview state, non-destructive transforms, and a GPUI preview surface.
//!
//! The source preview owns transient view state rather than changing media files. This crate
//! follows that rule: transformed rasters are materialized under a cache directory and the
//! original path remains the identity used by callers.

mod view;

use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cordis::{Context, Plugin, Service};
use photag_ui_bridge::{
    ContributionKind, UiContribution, register_for_plugin, register_views_for_plugin,
};

pub use view::preview_view;

/// Current preview item and the source transform controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewSnapshot {
    pub source: Option<PathBuf>,
    /// Clockwise quarter-turn count, normalized to 0..=3.
    pub quarter_turns: u8,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub fit_to_window: bool,
    /// Percent zoom when fit-to-window is disabled; source presets are retained.
    pub zoom_percent: u16,
    pub show_info: bool,
    pub revision: u64,
}

impl Default for PreviewSnapshot {
    fn default() -> Self {
        Self {
            source: None,
            quarter_turns: 0,
            flip_horizontal: false,
            flip_vertical: false,
            fit_to_window: true,
            zoom_percent: 100,
            show_info: false,
            revision: 0,
        }
    }
}

/// Cordis service for opening and controlling image previews.
pub struct PreviewService {
    state: Mutex<PreviewSnapshot>,
    cache_dir: PathBuf,
}

impl Service for PreviewService {
    const NAME: &'static str = "photag/preview";
}

impl PreviewService {
    pub fn open_at(cache_dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let cache_dir = cache_dir.into();
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            state: Mutex::new(PreviewSnapshot::default()),
            cache_dir,
        })
    }

    pub fn snapshot(&self) -> PreviewSnapshot {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn open(&self, path: impl Into<PathBuf>) -> Result<(), PreviewError> {
        let path = path.into();
        if !path.is_file() {
            return Err(PreviewError::MissingSource(path));
        }
        let mut state = self.state.lock().map_err(|_| PreviewError::StatePoisoned)?;
        state.source = Some(path);
        state.quarter_turns = 0;
        state.flip_horizontal = false;
        state.flip_vertical = false;
        state.fit_to_window = true;
        state.zoom_percent = 100;
        state.revision = state.revision.wrapping_add(1);
        Ok(())
    }

    pub fn close(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.source = None;
            s.revision = s.revision.wrapping_add(1);
        }
    }
    pub fn rotate_clockwise(&self) {
        self.change(|s| s.quarter_turns = (s.quarter_turns + 1) % 4);
    }
    pub fn rotate_counter_clockwise(&self) {
        self.change(|s| s.quarter_turns = (s.quarter_turns + 3) % 4);
    }
    pub fn flip_horizontal(&self) {
        self.change(|s| s.flip_horizontal = !s.flip_horizontal);
    }
    pub fn flip_vertical(&self) {
        self.change(|s| s.flip_vertical = !s.flip_vertical);
    }
    pub fn toggle_info(&self) {
        self.change(|s| s.show_info = !s.show_info);
    }
    pub fn set_fit(&self) {
        self.change(|s| {
            s.fit_to_window = true;
            s.zoom_percent = 100;
        });
    }
    pub fn set_zoom_percent(&self, percent: u16) {
        if [25, 50, 100, 125, 150, 200].contains(&percent) {
            self.change(|s| {
                s.fit_to_window = false;
                s.zoom_percent = percent;
            });
        }
    }

    /// Return the original path for the identity transform, otherwise make a cached oriented copy.
    pub fn display_path(&self) -> Result<Option<PathBuf>, PreviewError> {
        let s = self.snapshot();
        let Some(source) = s.source else {
            return Ok(None);
        };
        if s.quarter_turns == 0 && !s.flip_horizontal && !s.flip_vertical {
            return Ok(Some(source));
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        s.quarter_turns.hash(&mut hasher);
        s.flip_horizontal.hash(&mut hasher);
        s.flip_vertical.hash(&mut hasher);
        let target = self
            .cache_dir
            .join(format!("preview-{:016x}.png", hasher.finish()));
        if target.is_file() {
            return Ok(Some(target));
        }
        let mut image = image::open(&source)
            .map_err(|error| PreviewError::Decode(source.clone(), error.to_string()))?;
        image = match s.quarter_turns {
            1 => image.rotate90(),
            2 => image.rotate180(),
            3 => image.rotate270(),
            _ => image,
        };
        if s.flip_horizontal {
            image = image.fliph();
        }
        if s.flip_vertical {
            image = image.flipv();
        }
        image
            .save(&target)
            .map_err(|error| PreviewError::Write(target.clone(), error.to_string()))?;
        Ok(Some(target))
    }

    fn change(&self, update: impl FnOnce(&mut PreviewSnapshot)) {
        if let Ok(mut state) = self.state.lock() {
            update(&mut state);
            state.revision = state.revision.wrapping_add(1);
        }
    }
}

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
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let directory = config.cache_dir.clone().unwrap_or_else(default_cache_dir);
        let service = Arc::new(PreviewService::open_at(directory)?);
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| PreviewError::Publish(error.to_string()))?;
        register_for_plugin(
            &context,
            "photag.preview",
            [UiContribution::new(
                "preview/image",
                ContributionKind::Panel {
                    label: "Image preview".into(),
                    slot: "workspace.main".into(),
                    order: 0,
                },
            )],
        )
        .map_err(|error| PreviewError::Ui(error.to_string()))?;
        register_views_for_plugin(
            &context,
            "photag.preview",
            [preview_view(Arc::clone(&service))],
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
