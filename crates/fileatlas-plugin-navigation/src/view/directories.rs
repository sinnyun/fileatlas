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
            self.directory_revision += 1;
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

impl SidebarView {
    pub(super) fn refresh_selected_directory(&mut self) {
        let Ok(saved) = self.session.snapshot() else {
            return;
        };
        let path = saved.selected_path.or_else(|| {
            self.snapshot
                .as_ref()
                .ok()?
                .folders
                .iter()
                .find(|node| Some(node.folder.id) == saved.selected_folder_id)
                .map(|node| node.folder.path.clone())
        });
        if let Some(path) = path.filter(|path| self.expanded_directories.contains_key(path)) {
            self.queue_directory(&path);
        }
    }
}

impl SidebarView {
    pub(super) fn toggle_directory(&mut self, path: &str) {
        if self.expanded_directories.remove(path).is_none() {
            if self.service.browser_enabled() {
                self.queue_directory(path);
            } else {
                self.expanded_directories
                    .insert(path.to_owned(), self.service.subdirectories(path));
            }
        }
        self.persist_directory_expansion();
    }
    pub(super) fn expand_directory(&mut self, path: &str) {
        if !self.expanded_directories.contains_key(path) {
            if self.service.browser_enabled() {
                self.queue_directory(path);
            } else {
                let children = self.service.subdirectories(path);
                self.expanded_directories.insert(path.to_owned(), children);
            }
        }
        self.persist_directory_expansion();
    }
    pub(super) fn persist_directory_expansion(&self) {
        if let Ok(mut session) = self.session.snapshot() {
            session.expanded_subdirs = self.expanded_directories.keys().cloned().collect();
            if let Err(error) = self.session.replace(session) {
                tracing::error!("directory expansion session save failed: {error}");
            }
        }
    }
}
