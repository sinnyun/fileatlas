//! Lazy sidebar directory reads use the shared keyed reader, never child probes.
use super::*;
impl SidebarView {
    pub(super) fn queue_directory(&mut self, path: &str) {
        self.expanded_directories.entry(path.into()).or_default();
        if !self.pending_directories.iter().any(|p| p == path) {
            self.pending_directories.push_back(path.into());
        }
    }
    pub(super) fn poll_directories(&mut self) -> bool {
        let mut changed = false;
        if self.directory_read.poll() {
            if let (Some(path), Some(result)) =
                (self.directory_read.key(), self.directory_read.value())
            {
                match result {
                    Ok(nodes) if self.expanded_directories.contains_key(path) => {
                        self.expanded_directories
                            .insert(path.clone(), nodes.clone());
                    }
                    Err(error) => {
                        self.scan_failure =
                            Some((ScanRetryTarget::Directory(path.clone()), error.clone()))
                    }
                    _ => {}
                }
            }
            self.directory_read = Default::default();
            changed = true;
        }
        if !self.directory_read.is_pending() {
            if let Some(path) = self.pending_directories.pop_front() {
                let service = self.service.clone();
                let target = path.clone();
                self.directory_read.request(path, move || {
                    service
                        .browse_subdirectories(&target)
                        .map_err(|e| e.to_string())
                });
            }
        }
        changed
    }
}
