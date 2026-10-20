//! Sidebar drive/quick-access/favorites configuration owned by Navigation
//! and persisted by Settings through `SidebarFoldersChanged`.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use fileatlas_contracts::SidebarFoldersConfig;

#[cfg(test)]
#[path = "sidebar_folders_tests.rs"]
mod tests;

#[derive(Default)]
pub struct SidebarFoldersService {
    config: RwLock<SidebarFoldersConfig>,
    revision: AtomicU64,
}

impl SidebarFoldersService {
    /// Seed the service with the persisted document before views mount.
    pub fn with_config(config: SidebarFoldersConfig) -> Self {
        Self {
            config: RwLock::new(config),
            revision: AtomicU64::new(1),
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    pub fn snapshot(&self) -> SidebarFoldersConfig {
        self.config
            .read()
            .map(|config| config.clone())
            .unwrap_or_default()
    }

    /// Apply one edit and report the resulting document, or `None` when the
    /// edit produced no change. The caller emits `SidebarFoldersChanged`.
    pub fn update(
        &self,
        edit: impl FnOnce(&mut SidebarFoldersConfig),
    ) -> Option<SidebarFoldersConfig> {
        let updated = {
            let mut config = self.config.write().ok()?;
            let previous = config.clone();
            edit(&mut config);
            if *config == previous {
                return None;
            }
            config.clone()
        };
        self.revision.fetch_add(1, Ordering::AcqRel);
        Some(updated)
    }

    /// Import each registered library bookmark once, preserving existing aliases,
    /// group order, library rows and user removals. No filesystem/database IO.
    pub fn import_library_folders(
        &self,
        roots: &[(i64, String, String)],
        name: &str,
    ) -> Option<SidebarFoldersConfig> {
        self.update(|config| {
            for (id, path, alias) in roots {
                let key = sidebar_path_key(path);
                if config.imported_library_paths.contains(&key) {
                    continue;
                }
                config.imported_library_paths.push(key);
                let sequence = config.imported_library_paths.len();
                if config.favorite_groups.iter().any(|group| {
                    group
                        .folders
                        .iter()
                        .any(|folder| same_sidebar_path(&folder.path, path))
                }) {
                    continue;
                }
                let group_id = config
                    .favorite_groups
                    .iter()
                    .find(|group| group.id == "unified-library-favorites" || group.name == name)
                    .map(|group| group.id.clone())
                    .unwrap_or_else(|| "unified-library-favorites".into());
                if !config
                    .favorite_groups
                    .iter()
                    .any(|group| group.id == group_id)
                {
                    config.favorite_groups.insert(
                        0,
                        fileatlas_contracts::FavoriteGroup {
                            id: group_id.clone(),
                            name: name.into(),
                            folders: Vec::new(),
                        },
                    );
                }
                if let Some(group) = config.favorite_groups.iter_mut().find(|g| g.id == group_id) {
                    group.folders.push(fileatlas_contracts::FavoriteFolder {
                        id: format!("library-{id}-{sequence}"),
                        path: path.clone(),
                        display_name: Some(alias.clone()),
                    });
                }
            }
        })
    }

    /// Install the persisted document at startup without announcing a change.
    pub fn restore(&self, config: SidebarFoldersConfig) {
        if let Ok(mut current) = self.config.write() {
            if *current == config {
                return;
            }
            *current = config;
            self.revision.fetch_add(1, Ordering::AcqRel);
        }
    }
}

pub(crate) fn sidebar_path_key(path: &str) -> String {
    let path = if let Some(tail) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{tail}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    };
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}
pub(crate) fn same_sidebar_path(left: &str, right: &str) -> bool {
    sidebar_path_key(left) == sidebar_path_key(right)
}
pub(crate) fn in_sidebar_root(path: &str, root: &str) -> bool {
    let path = sidebar_path_key(path);
    let root = sidebar_path_key(root);
    path == root || path.starts_with(&(root + "/"))
}
