//! Update-check/download lifecycle for the settings surface.

use gpui_kit::*;
use std::sync::Arc;
use std::time::Duration;

use super::SettingsView;

impl SettingsView {
    pub(super) fn begin_update_check(&mut self, manual: bool) {
        let Some(updater) = &self.updater else {
            self.update_error = Some("Updater service is unavailable.".into());
            return;
        };
        self.update_error = None;
        self.update_status = Some("Checking GitHub for a newer release…".into());
        self.update_check = Some(updater.check_async());
        if manual {
            self.update_info = None;
        }
    }

    pub(super) fn begin_update_download(&mut self) {
        let (Some(updater), Some(info)) = (&self.updater, self.update_info.clone()) else {
            return;
        };
        self.update_error = None;
        self.update_status = Some(format!("Downloading PhoTag {}…", info.version));
        self.update_progress
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.update_download =
            Some(updater.download_async(info, Arc::clone(&self.update_progress)));
    }

    pub(super) fn monitor_updates(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let active = this
                    .update(cx, |view, cx| {
                        view.poll_updates();
                        cx.notify();
                        view.update_check.is_some() || view.update_download.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
            }
        })
        .detach();
    }

    fn poll_updates(&mut self) {
        let check_result = self
            .update_check
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = check_result {
            self.update_check = None;
            match result {
                Ok(Some(info)) => {
                    self.update_status = Some(format!(
                        "PhoTag {} is available ({:.1} MB).",
                        info.version,
                        info.size_mb()
                    ));
                    self.update_info = Some(info);
                }
                Ok(None) => {
                    self.update_info = None;
                    self.update_status = Some(format!(
                        "PhoTag {} is up to date.",
                        photag_plugin_updater::current_version()
                    ));
                }
                Err(error) => {
                    self.update_status = None;
                    self.update_error = Some(error);
                }
            }
        }
        let download_result = self
            .update_download
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = download_result {
            self.update_download = None;
            match result {
                Ok(path) => {
                    self.update_staged = Some(path);
                    self.confirm_install = false;
                    self.update_status =
                        Some("Download verified. PhoTag is ready to install and restart.".into());
                }
                Err(error) => {
                    self.update_status = None;
                    self.update_error = Some(error);
                }
            }
        }
    }

    pub(super) fn install_update(&mut self, cx: &mut Context<Self>) {
        if !self.confirm_install {
            self.confirm_install = true;
            self.update_status = Some("Confirm restart and install the verified update.".into());
            cx.notify();
            return;
        }
        let Some(path) = self.update_staged.clone() else {
            return;
        };
        let session_save = self
            .session
            .as_ref()
            .ok_or_else(|| {
                "Session service is unavailable; update installation was stopped.".to_owned()
            })
            .and_then(|session| {
                let snapshot = session.snapshot().map_err(|error| error.to_string())?;
                session.replace(snapshot).map_err(|error| error.to_string())
            });
        if let Err(error) = session_save {
            self.update_error = Some(error);
            cx.notify();
            return;
        }
        let result = self
            .updater
            .as_ref()
            .ok_or_else(|| "Updater service is unavailable.".to_owned())
            .and_then(|updater| updater.schedule_install(path.clone()));
        match result {
            Ok(()) => {
                self.update_staged = None;
                self.update_status = Some("Closing PhoTag to install the verified update…".into());
                cx.quit();
            }
            Err(error) => {
                let log = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("PhoTag")
                    .join("update-error.txt");
                let _ = std::fs::create_dir_all(
                    log.parent().unwrap_or_else(|| std::path::Path::new(".")),
                );
                let _ = std::fs::write(
                    &log,
                    format!(
                        "PhoTag {} could not install update from {}\n\n{}\n",
                        photag_plugin_updater::current_version(),
                        path.display(),
                        error
                    ),
                );
                self.update_error = Some(format!("{error} (details: {})", log.display()));
            }
        }
        cx.notify();
    }
}
