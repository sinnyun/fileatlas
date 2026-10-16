#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Windows Explorer and application drag-out for indexed PhoTag media.
//!
//! The host supplies selected filesystem paths from its current media model.
//! This plugin advertises them as `CF_HDROP` and performs copy-only OLE drag
//! operations on the calling UI thread.

mod plugin;
#[cfg(target_os = "windows")]
mod win32;

use std::path::PathBuf;

use cordis::Service;

pub use plugin::{DragDropConfig, DragDropPlugin, DragDropPluginError};

/// Stable Cordis service for starting source-compatible OS file drags.
pub struct DragDropService;

impl Service for DragDropService {
    const NAME: &'static str = "photag/drag-drop";
}

impl DragDropService {
    /// Starts a blocking, copy-only drag operation on the caller's thread.
    ///
    /// The caller must invoke this from the UI thread in response to a drag
    /// gesture. Paths are passed directly to the shell data object; this
    /// service does not move, copy, or otherwise mutate the underlying files.
    pub fn start_file_drag(&self, paths: &[PathBuf]) -> Result<bool, DragDropError> {
        if paths.is_empty() {
            return Ok(false);
        }

        #[cfg(target_os = "windows")]
        {
            win32::start_file_drag(paths)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = paths;
            Err(DragDropError::UnsupportedPlatform)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragDropError {
    UnsupportedPlatform,
    OleInitialization(i32),
    DataObjectAllocation,
    DropSourceAllocation,
}

impl std::fmt::Display for DragDropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => f.write_str("file drag-out is supported on Windows only"),
            Self::OleInitialization(hr) => write!(f, "OleInitialize failed (HRESULT {hr:#010x})"),
            Self::DataObjectAllocation => {
                f.write_str("could not allocate the CF_HDROP data object")
            }
            Self::DropSourceAllocation => f.write_str("could not allocate the OLE drop source"),
        }
    }
}

impl std::error::Error for DragDropError {}
