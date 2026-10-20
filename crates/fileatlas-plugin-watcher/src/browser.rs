//! Local notifications patch the session cache; annotations are never deleted by inference.
use super::*;
use fileatlas_plugin_database::BrowserFile;
impl WatcherService {
    pub fn take_directory_dirty(&self) -> bool {
        self.directory_dirty.swap(false, Ordering::AcqRel)
    }

    pub(super) fn reconcile_browser(
        &self,
        added: &[String],
        removed: &[String],
    ) -> Result<ReconcileSummary, WatcherError> {
        let mut files = Vec::new();
        for path in added {
            match std::fs::metadata(path) {
                Ok(meta) if meta.is_file() => {
                    let time = meta
                        .modified()
                        .ok()
                        .and_then(|v| v.duration_since(std::time::UNIX_EPOCH).ok())
                        .unwrap_or_default();
                    let file = Path::new(path);
                    let filename = file
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let extension = file.extension().unwrap_or_default().to_string_lossy();
                    files.push(BrowserFile {
                        path: path.clone(),
                        filename,
                        media_type: classify_media_type(&extension).to_owned(),
                        file_size: meta.len() as i64,
                        modified_at: time.as_secs() as i64,
                        version: format!("{}:{}", meta.len(), time.as_nanos()),
                    });
                }
                Ok(meta) if meta.is_dir() => {
                    self.directory_dirty.store(true, Ordering::Release);
                }
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    tracing::warn!(%path,%error,"file notification could not be verified")
                }
                _ => {}
            }
        }
        let unknown_removal = self.database.reconcile_browser_files(&files, removed)?;
        if unknown_removal {
            self.directory_dirty.store(true, Ordering::Release);
        }
        if let (Some(cordis), Some(runtime)) = (&self.cordis, &self.runtime) {
            let cordis = cordis.clone();
            runtime.spawn(async move {
                let _ = cordis
                    .emit::<MediaIndexChanged>(Routing::Unscoped, ())
                    .await;
            });
        }
        Ok(ReconcileSummary {
            added_or_updated: files.len(),
            removed: removed.len(),
        })
    }
}
