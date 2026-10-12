//! PhoTag self-update release checking, staged download verification, and install swap.

use std::sync::Arc;

use cordis::{Context, Service};

mod plugin;
mod update;

pub use plugin::{UpdaterConfig, UpdaterPlugin, UpdaterPluginError};
pub use update::{
    UpdateInfo, check, check_async, cleanup_previous, current_version, download, download_async,
    install, releases_url,
};

/// Cordis-owned entry point for source-compatible self-updating behavior.
pub struct UpdaterService;

impl Service for UpdaterService {
    const NAME: &'static str = "photag/updater";
}

impl UpdaterService {
    pub fn check(&self) -> Result<Option<UpdateInfo>, String> {
        update::check()
    }

    pub fn check_async(&self) -> std::sync::mpsc::Receiver<Result<Option<UpdateInfo>, String>> {
        update::check_async()
    }

    pub fn download_async(
        &self,
        info: UpdateInfo,
        progress: Arc<std::sync::atomic::AtomicU64>,
    ) -> std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>> {
        update::download_async(info, progress)
    }

    pub fn install(&self, staged: &std::path::Path) -> Result<(), String> {
        update::install(staged)
    }

    pub fn cleanup_previous(&self) {
        update::cleanup_previous();
    }
}
