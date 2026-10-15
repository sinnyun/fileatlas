//! Sidebar drive/quick-access/favorites configuration owned by Navigation
//! and persisted by Settings through `SidebarFoldersChanged`.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use photag_contracts::SidebarFoldersConfig;

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
