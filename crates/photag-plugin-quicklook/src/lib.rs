//! Send the user's QuickLook a request; never start, stop or own that program.
#[cfg(windows)]
pub mod windows;
use cordis::{Context, Plugin, Service};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct LaunchSnapshot {
    pub path: Option<PathBuf>,
    pub viewer_path: Option<PathBuf>,
    pub outcome: Option<String>,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct QuickLookService {
    state: Mutex<LaunchSnapshot>,
    pipe: Mutex<Option<String>>,
}
impl Service for QuickLookService {
    const NAME: &'static str = "photag/quicklook";
}
impl QuickLookService {
    /// Blocking action-time request. Call from a worker, never render.
    #[tracing::instrument(skip_all, fields(path=%path.as_ref().display()), err)]
    pub fn open(&self, path: impl AsRef<Path>) -> Result<(), QuickLookError> {
        let path = path.as_ref();
        let viewer_path = normalize_viewer_path(path);
        let result = if path.is_file() {
            self.dispatch(&viewer_path)
        } else {
            Err(QuickLookError::MissingFile)
        };
        *self.state.lock().expect("QuickLook snapshot") = LaunchSnapshot {
            path: Some(path.to_owned()),
            viewer_path: Some(viewer_path),
            outcome: result.as_ref().ok().cloned(),
            error: result.as_ref().err().map(ToString::to_string),
        };
        result.map(|_| ())
    }

    /// QuickLook's running instance is the only target. Starting QuickLook belongs
    /// to the user, so an unreachable instance produces a prompt, never a process.
    #[cfg(windows)]
    fn dispatch(&self, viewer_path: &Path) -> Result<String, QuickLookError> {
        let Some(pipe_path) = self.pipe_path() else {
            return Err(QuickLookError::NotRunning);
        };
        match windows::send_toggle(&pipe_path, viewer_path) {
            Ok(()) => {
                tracing::info!(%pipe_path, source_path=%viewer_path.display(), outcome="pipe_toggle_sent", "QuickLook request dispatched");
                *self.pipe.lock().expect("QuickLook pipe") = Some(pipe_path);
                Ok("pipe_toggle_sent".to_owned())
            }
            Err(windows::PipeError::Denied) => {
                match windows::higher_integrity_quicklook_process() {
                    Some(process_id) => {
                        tracing::warn!(
                            process_id,
                            "QuickLook runs at a higher Windows integrity level than PhoTag"
                        );
                        Err(QuickLookError::PrivilegeMismatch)
                    }
                    None => {
                        tracing::warn!(%pipe_path, "QuickLook refused this process over its instance pipe");
                        Err(QuickLookError::NotRunning)
                    }
                }
            }
            Err(error) => {
                tracing::info!(%pipe_path, ?error, "No QuickLook instance accepts requests; the user starts QuickLook");
                Err(QuickLookError::NotRunning)
            }
        }
    }

    #[cfg(not(windows))]
    fn dispatch(&self, _: &Path) -> Result<String, QuickLookError> {
        Err(QuickLookError::NotInstalled)
    }

    /// Cached pipe path first: deriving the user SID costs a token lookup per press.
    #[cfg(windows)]
    fn pipe_path(&self) -> Option<String> {
        if let Some(cached) = self.pipe.lock().expect("QuickLook pipe").as_ref().cloned() {
            return Some(cached);
        }
        windows::toggle_pipe_path()
    }

    /// Launch diagnostics only; this does not claim QuickLook rendered the file.
    pub fn snapshot(&self) -> LaunchSnapshot {
        self.state.lock().expect("QuickLook snapshot").clone()
    }
}

/// Third-party viewers expect ordinary DOS/UNC paths, not Rust canonical verbatim paths.
#[cfg(windows)]
fn normalize_viewer_path(path: &Path) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let raw = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let verbatim = [92, 92, 63, 92];
    if raw.starts_with(&verbatim) {
        if raw.get(4..8) == Some(&[85, 78, 67, 92]) {
            let mut unc = vec![92, 92];
            unc.extend_from_slice(&raw[8..]);
            return std::ffi::OsString::from_wide(&unc).into();
        }
        if raw.get(5) == Some(&58) && raw.get(6) == Some(&92) {
            return std::ffi::OsString::from_wide(&raw[4..]).into();
        }
    }
    path.to_owned()
}
#[cfg(not(windows))]
fn normalize_viewer_path(path: &Path) -> PathBuf {
    path.to_owned()
}

#[derive(Debug, thiserror::Error)]
pub enum QuickLookError {
    #[error("QuickLook is not installed. Please install QuickLook and try again.")]
    NotInstalled,
    #[error("The selected file no longer exists.")]
    MissingFile,
    #[error(
        "QuickLook is not responding. Start QuickLook and press Space again. If it is not installed, install QuickLook first."
    )]
    NotRunning,
    #[error(
        "QuickLook is running as administrator while PhoTag is not. Close QuickLook and relaunch it without 'Run as administrator', then try again."
    )]
    PrivilegeMismatch,
}

pub struct QuickLookPlugin;
impl Plugin for QuickLookPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = std::io::Error;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.quicklook".into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), Self::ApplyError> {
        let _publication = context
            .provide(Arc::new(QuickLookService::default()))
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn verbatim_drive_and_unc_paths_are_normalized_without_changing_unicode_or_spaces() {
        assert_eq!(
            normalize_viewer_path(Path::new(r"\\?\D:\资料 & $\video.mp4")),
            PathBuf::from(r"D:\资料 & $\video.mp4")
        );
        assert_eq!(
            normalize_viewer_path(Path::new(r"\\?\UNC\server\share\原图.png")),
            PathBuf::from(r"\\server\share\原图.png")
        );
        assert_eq!(
            normalize_viewer_path(Path::new(r"D:\普通路径\a.jpg")),
            PathBuf::from(r"D:\普通路径\a.jpg")
        );
    }
    #[test]
    fn missing_original_never_launches_and_reports_the_failed_target() {
        let service = QuickLookService::default();
        let path = std::env::temp_dir()
            .join("photag-quicklook-missing-original")
            .join("missing.jpg");
        assert!(matches!(
            service.open(&path),
            Err(QuickLookError::MissingFile)
        ));
        let snapshot = service.snapshot();
        assert_eq!(snapshot.path.as_ref(), Some(&path));
        assert!(snapshot.outcome.is_none());
        assert!(snapshot.error.is_some());
    }
}
