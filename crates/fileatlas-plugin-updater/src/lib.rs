//! FileAtlas self-update release checking, staged download verification, and install swap.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cordis::Service;

mod plugin;
mod update;

pub use plugin::{UpdaterConfig, UpdaterPlugin, UpdaterPluginError};
pub use update::{
    UpdateInfo, check, check_async, cleanup_previous, current_version, download, download_async,
    install, releases_url,
};

/// Cordis-owned entry point for source-compatible self-updating behavior.
#[derive(Default)]
pub struct UpdaterService {
    pending_install: Mutex<Option<PathBuf>>,
}

impl Service for UpdaterService {
    const NAME: &'static str = "fileatlas/updater";
}

impl UpdaterService {
    /// Ask the shell to install this staged binary after the GPUI app exits.
    pub fn schedule_install(&self, staged: PathBuf) -> Result<(), String> {
        let mut pending = self
            .pending_install
            .lock()
            .map_err(|_| "updater pending-install lock is poisoned".to_owned())?;
        if pending.is_some() {
            return Err("an update install is already scheduled".into());
        }
        *pending = Some(staged);
        Ok(())
    }

    /// Take the staged binary requested by Settings after the application exits.
    pub fn take_pending_install(&self) -> Result<Option<PathBuf>, String> {
        self.pending_install
            .lock()
            .map(|mut pending| pending.take())
            .map_err(|_| "updater pending-install lock is poisoned".to_owned())
    }

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
