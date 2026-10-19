#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Windows Explorer and application drag-out for indexed PhoTag media.
//!
//! The host supplies selected filesystem paths from its current media model.
//! This plugin delegates platform drag sessions and file data objects to the
//! open-source `drag` crate.

#[cfg(target_os = "windows")]
mod dispatch;
mod plugin;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use cordis::Service;

pub use plugin::{DragDropConfig, DragDropPlugin, DragDropPluginError};

/// Resolved UI colors supplied by the host; the platform adapter owns no palette.
#[derive(Clone, Copy)]
pub struct DragImageStyle {
    pub background: [u8; 3],
    pub foreground: [u8; 3],
    pub border: [u8; 3],
    pub accent: [u8; 3],
}

/// Stable Cordis service for starting source-compatible OS file drags.
#[derive(Default)]
pub struct DragDropService {
    status: Arc<AtomicU8>,
}

impl DragDropService {
    const IDLE: u8 = 0;
    const QUEUED: u8 = 1;
    const ACTIVE: u8 = 2;
    const DROPPED: u8 = 3;
    const CANCELLED: u8 = 4;
    const FAILED: u8 = 5;

    /// Returns the latest native drag lifecycle for same-process diagnostics.
    pub fn status(&self) -> &'static str {
        match self.status.load(Ordering::Acquire) {
            Self::IDLE => "idle",
            Self::QUEUED => "queued",
            Self::ACTIVE => "active",
            Self::DROPPED => "dropped",
            Self::CANCELLED => "cancelled",
            Self::FAILED => "failed",
            _ => "unknown",
        }
    }
}

impl Service for DragDropService {
    const NAME: &'static str = "photag/drag-drop";
}

impl DragDropService {
    /// Starts a copy-only platform drag using the source window and file paths.
    pub fn start_file_drag<W: raw_window_handle::HasWindowHandle>(
        &self,
        window: &W,
        paths: &[PathBuf],
        style: DragImageStyle,
    ) -> Result<(), DragDropError> {
        if paths.is_empty() {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        return dispatch::queue_file_drag(&self.status, window, paths, style);

        #[cfg(not(target_os = "windows"))]
        Err(DragDropError::UnsupportedPlatform)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragDropError {
    UnsupportedPlatform,
    Platform(String),
}

impl std::fmt::Display for DragDropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => f.write_str("file drag-out is supported on Windows only"),
            Self::Platform(error) => write!(f, "platform file drag failed: {error}"),
        }
    }
}

impl std::error::Error for DragDropError {}
