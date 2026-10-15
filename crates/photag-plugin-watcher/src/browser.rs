//! Local notifications patch the session cache; annotations are never deleted by inference.
use super::*;
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
                    files.push((
                        path.clone(),
                        meta.len() as i64,
                        time.as_secs() as i64,
                        format!("{}:{}", meta.len(), time.as_nanos()),
                    ));
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
        self.database.with_cache_writer(|conn| {
            for path in removed {
                let known: bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM media WHERE full_path=?1)",[path],|r|r.get(0))?;
                if !known { self.directory_dirty.store(true,Ordering::Release); }
                conn.execute("DELETE FROM live_paths WHERE full_path=?1",[path])?;
            }
            for (path,size,mtime,version) in &files {
                let previous: Option<String>=conn.query_row("SELECT version FROM file_versions WHERE full_path=?1",[path],|r|r.get(0)).ok();
                if previous.as_ref()!=Some(version) {
                    conn.execute("UPDATE media SET thumb_path=NULL,micro_thumb=NULL,width=NULL,height=NULL WHERE full_path=?1",[path])?;
                }
                let file=Path::new(path);
                let filename=file.file_name().unwrap_or_default().to_string_lossy();
                let extension=file.extension().unwrap_or_default().to_string_lossy();
                queries::scan_upsert_media(conn,0,&filename,path,classify_media_type(&extension),*size,*mtime,None)?;
                conn.execute("INSERT OR IGNORE INTO live_paths VALUES(?1)",[path])?;
                conn.execute("INSERT OR REPLACE INTO file_versions VALUES(?1,?2)",rusqlite::params![path,version])?;
            }
            Ok(())
        })?;
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
