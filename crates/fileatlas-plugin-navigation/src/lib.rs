//! FileAtlas library sidebar data and tag tree operations, owned by Cordis.

mod drives;
mod favorites;
mod folder_picker;
mod location;
mod mcp;
mod sidebar_folders;
mod ui_requests;
mod view;
pub use drives::{DriveInfo, DriveKind, enumerate_drives};
pub use sidebar_folders::SidebarFoldersService;
pub use ui_requests::{
    AddressBarRequest, NavigationUiRequest, NavigationUiRequestError, SidebarEditorRequest,
    SidebarUiRequest,
};

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use cordis::{Context, Plugin, Service};
use fileatlas_plugin_database::DatabaseService;
use fileatlas_plugin_database::{Folder, Tag, TagDropPos};
use fileatlas_plugin_indexing::{IndexingService, ScanProgress};
use fileatlas_plugin_watcher::WatcherService;
use fileatlas_ui_bridge::{
    ContributionKind, UiContribution, register_for_plugin, register_views_for_plugin,
};

#[derive(Debug, Clone)]
pub struct FolderNode {
    pub folder: Folder,
    pub media_count: i64,
}

#[derive(Debug, Clone)]
pub struct NavigationSnapshot {
    pub folders: Vec<FolderNode>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubdirectoryNode {
    pub name: String,
    pub path: String,
    pub has_children: bool,
    pub media_count: i64,
}

pub struct NavigationService {
    ui_wake: fileatlas_ui_bridge::refresh::Wake,
    plugin_views_revision: std::sync::atomic::AtomicU64,
    tag_revision: std::sync::atomic::AtomicU64,
    database: Arc<DatabaseService>,
    indexing: Arc<IndexingService>,
    watcher: Option<Arc<WatcherService>>,
    scans: Mutex<HashMap<i64, Arc<AtomicBool>>>,
    ui_request: Mutex<Option<NavigationUiRequest>>,
    picker: Arc<folder_picker::FolderPickerState>,
    folders: Arc<SidebarFoldersService>,
    sidebar_state: Mutex<Option<serde_json::Value>>,
}

impl Service for NavigationService {
    const NAME: &'static str = "fileatlas/navigation";
}

impl NavigationService {
    pub fn new(
        database: Arc<DatabaseService>,
        indexing: Arc<IndexingService>,
        folders: Arc<SidebarFoldersService>,
    ) -> Self {
        Self {
            ui_wake: Default::default(),
            plugin_views_revision: std::sync::atomic::AtomicU64::new(0),
            tag_revision: std::sync::atomic::AtomicU64::new(0),
            database,
            indexing,
            watcher: None,
            scans: Mutex::new(HashMap::new()),
            ui_request: Mutex::new(None),
            picker: Arc::default(),
            folders,
            sidebar_state: Mutex::new(None),
        }
    }

    /// Last snapshot the mounted sidebar view published for read-only debugging.
    pub fn sidebar_state(&self) -> Option<serde_json::Value> {
        self.sidebar_state
            .lock()
            .ok()
            .and_then(|state| state.clone())
    }

    pub(crate) fn publish_sidebar_state(&self, state: serde_json::Value) {
        if let Ok(mut current) = self.sidebar_state.lock() {
            if current.as_ref() != Some(&state) {
                *current = Some(state);
            }
        }
    }
    pub fn sidebar_folders(&self) -> &Arc<SidebarFoldersService> {
        &self.folders
    }
    pub fn notify_plugin_views_changed(&self) {
        self.plugin_views_revision.fetch_add(1, Ordering::AcqRel);
        self.ui_wake.trigger();
    }
    pub(crate) fn plugin_views_revision(&self) -> u64 {
        self.plugin_views_revision.load(Ordering::Acquire)
    }
    pub fn browser_enabled(&self) -> bool {
        self.database.is_browser()
    }

    pub fn library_refresh(&self) -> Option<fileatlas_plugin_watcher::LibraryRefreshState> {
        self.watcher
            .as_ref()
            .map(|watcher| watcher.library_refresh())
    }
    pub fn folder_picker_pending(&self) -> bool {
        self.picker.pending()
    }
    pub fn folder_picker_window(&self) -> Option<isize> {
        self.picker.window()
    }
    pub fn cancel_folder_picker(&self) -> Result<(), String> {
        self.picker.cancel()
    }

    /// Queue one typed action for the mounted navigation view.
    #[tracing::instrument(skip_all, err, fields(request=?request))]
    pub fn request_ui(&self, request: NavigationUiRequest) -> Result<(), NavigationUiRequestError> {
        tracing::info!(?request, "navigation request");
        if let Some(id) = request.target_id() {
            if id <= 0 {
                return Err(NavigationUiRequestError::InvalidTagId(id));
            }
            let snapshot = self
                .snapshot()
                .map_err(|_| NavigationUiRequestError::TagLookupFailed)?;
            if !tree_contains_tag(&snapshot.tags, id) {
                return Err(NavigationUiRequestError::UnknownTag(id));
            }
        }
        match &request {
            NavigationUiRequest::ExpandFolder(id) => {
                if *id <= 0 {
                    return Err(NavigationUiRequestError::InvalidFolderId(*id));
                }
                let snapshot = self
                    .snapshot()
                    .map_err(|_| NavigationUiRequestError::UnknownFolder(*id))?;
                if !snapshot.folders.iter().any(|node| node.folder.id == *id) {
                    return Err(NavigationUiRequestError::UnknownFolder(*id));
                }
            }
            NavigationUiRequest::SelectPath(path)
            | NavigationUiRequest::SelectPathPreservingPage(path)
            | NavigationUiRequest::ExpandPath(path) => {
                let target = std::fs::canonicalize(path)
                    .ok()
                    .filter(|path| path.is_dir())
                    .ok_or_else(|| NavigationUiRequestError::InvalidDirectoryPath(path.clone()))?;
                if !self.database.is_browser() {
                    let snapshot = self.snapshot().map_err(|_| {
                        NavigationUiRequestError::InvalidDirectoryPath(path.clone())
                    })?;
                    let inside_registered_folder = snapshot.folders.iter().any(|node| {
                        std::fs::canonicalize(&node.folder.path)
                            .is_ok_and(|root| target.starts_with(root))
                    });
                    if !inside_registered_folder && !self.database.is_browser() {
                        return Err(NavigationUiRequestError::InvalidDirectoryPath(path.clone()));
                    }
                }
            }
            _ => {}
        }
        if let NavigationUiRequest::Sidebar(request) = &request {
            ui_requests::validate_sidebar_request(&self.folders, request)?;
        }
        let mut pending = self
            .ui_request
            .lock()
            .map_err(|_| NavigationUiRequestError::QueuePoisoned)?;
        if pending.is_some() {
            return Err(NavigationUiRequestError::Busy);
        }
        *pending = Some(request);
        drop(pending);
        self.ui_wake.trigger();
        Ok(())
    }

    pub(crate) fn has_ui_request(&self) -> bool {
        self.ui_request
            .lock()
            .is_ok_and(|pending| pending.is_some())
    }

    pub(crate) fn take_ui_request(&self) -> Option<NavigationUiRequest> {
        self.ui_request
            .lock()
            .ok()
            .and_then(|mut pending| pending.take())
    }

    fn with_watcher(mut self, watcher: Arc<WatcherService>) -> Self {
        self.watcher = Some(watcher);
        self
    }

    fn start_scan(
        self: &Arc<Self>,
        folder: &Folder,
    ) -> Result<mpsc::Receiver<Option<ScanProgress>>, NavigationError> {
        self.cancel_scan(folder.id)?;
        let cancel = Arc::new(AtomicBool::new(false));
        self.scans
            .lock()
            .map_err(|_| NavigationError::Poisoned)?
            .insert(folder.id, Arc::clone(&cancel));
        Ok(self
            .indexing
            .scan_folder(folder.id, folder.path.clone(), cancel))
    }

    /// Cancel a running scan for one registered folder. Returns success when no scan is active.
    pub fn cancel_scan(&self, folder_id: i64) -> Result<(), NavigationError> {
        if let Some(cancel) = self
            .scans
            .lock()
            .map_err(|_| NavigationError::Poisoned)?
            .remove(&folder_id)
        {
            cancel.store(true, Ordering::Release);
        }
        Ok(())
    }

    /// Register a local folder, begin recursive indexing and watch it for later changes.
    #[tracing::instrument(skip_all, err)]
    pub fn add_folder(
        self: &Arc<Self>,
        path: impl AsRef<Path>,
    ) -> Result<(Folder, mpsc::Receiver<Option<ScanProgress>>), NavigationError> {
        let path = path.as_ref();
        if !path.is_dir() {
            return Err(NavigationError::NotDirectory(
                path.to_string_lossy().into_owned(),
            ));
        }
        let path = path.to_string_lossy().into_owned();
        let folder = self.database.add_folder(&path)?;
        if !self.database.is_browser()
            && let Some(watcher) = &self.watcher
        {
            watcher.watch(&folder.path);
        }
        let progress = if self.database.is_browser() {
            let (tx, rx) = mpsc::channel();
            let _ = tx.send(None);
            rx
        } else {
            self.start_scan(&folder)?
        };
        Ok((folder, progress))
    }

    /// Cancel any active scan, clear this folder's indexed rows and start a fresh scan.
    pub fn rescan(
        self: &Arc<Self>,
        folder_id: i64,
    ) -> Result<mpsc::Receiver<Option<ScanProgress>>, NavigationError> {
        self.cancel_scan(folder_id)?;
        let folder = self
            .database
            .list_folders()?
            .into_iter()
            .find(|folder| folder.id == folder_id)
            .ok_or(fileatlas_plugin_database::DatabaseAccessError::NotFound)?;
        if self.database.is_browser() {
            self.request_ui(NavigationUiRequest::SelectPath(folder.path.clone()))
                .map_err(|e| NavigationError::NotDirectory(e.to_string()))?;
            if let Some(watcher) = &self.watcher {
                watcher.invalidate_browse_directory();
            }
            let (tx, rx) = mpsc::channel();
            let _ = tx.send(None);
            return Ok(rx);
        }
        self.database.clear_folder_media(folder_id)?;
        self.start_scan(&folder)
    }

    /// Stop indexing and monitoring a folder, then remove its library rows.
    #[tracing::instrument(skip_all, err)]
    pub fn remove_folder(&self, folder_id: i64) -> Result<Folder, NavigationError> {
        self.cancel_scan(folder_id)?;
        let folder = self
            .database
            .list_folders()?
            .into_iter()
            .find(|folder| folder.id == folder_id)
            .ok_or(fileatlas_plugin_database::DatabaseAccessError::NotFound)?;
        self.database.remove_library_folder(folder_id)?;
        if let Some(watcher) = &self.watcher {
            watcher.unwatch(&folder.path);
        }
        Ok(folder)
    }

    pub fn snapshot(&self) -> Result<NavigationSnapshot, NavigationError> {
        let data = self.database.navigation_snapshot()?;
        let mut folders: Vec<FolderNode> = data
            .folders
            .into_iter()
            .map(|folder| FolderNode {
                media_count: data.folder_counts.get(&folder.id).copied().unwrap_or(0),
                folder,
            })
            .collect();
        if self.database.is_browser() {
            let metadata = data
                .favorite_meta
                .into_iter()
                .map(|(id, name, position)| (id, (name, position)))
                .collect::<HashMap<_, _>>();
            for node in &mut folders {
                if let Some((name, _)) = metadata.get(&node.folder.id) {
                    node.folder.name = name.clone();
                }
                node.media_count = 0;
            }
            folders.sort_by_key(|node| {
                metadata
                    .get(&node.folder.id)
                    .map(|(_, position)| *position)
                    .unwrap_or(node.folder.id)
            });
        }
        Ok(NavigationSnapshot {
            folders,
            tags: data.tags,
        })
    }

    pub fn browse_subdirectories(&self, path: &str) -> std::io::Result<Vec<SubdirectoryNode>> {
        let mut nodes = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() || kind.is_symlink() && entry.path().is_dir() {
                nodes.push(SubdirectoryNode {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path().to_string_lossy().into_owned(),
                    has_children: true,
                    media_count: 0,
                });
            }
        }
        nodes.sort_by_cached_key(|node| (node.name.to_lowercase(), node.path.clone()));
        Ok(nodes)
    }

    /// Expand a directory lazily, including dot-directories under the all-file policy.
    pub fn subdirectories(&self, path: &str) -> Vec<SubdirectoryNode> {
        if self.database.is_browser() {
            return self.browse_subdirectories(path).unwrap_or_default();
        }
        self.indexing
            .list_subdirectories(path)
            .into_iter()
            .map(|(name, path, has_children)| {
                let media_count = self.database.count_media_under_path(&path).unwrap_or(0);
                SubdirectoryNode {
                    name,
                    path,
                    has_children,
                    media_count,
                }
            })
            .collect()
    }

    /// In-memory invalidation identity for tag-choice readers; no database I/O.
    pub fn tag_revision(&self) -> u64 {
        self.tag_revision.load(Ordering::Acquire)
    }

    pub fn create_tag(&self, name: &str, parent: Option<i64>) -> Result<Tag, NavigationError> {
        let tag = self.database.create_tag(name, parent)?;
        self.tag_revision.fetch_add(1, Ordering::Release);
        Ok(tag)
    }
    pub fn rename_tag(&self, id: i64, name: &str) -> Result<(), NavigationError> {
        self.database.rename_tag(id, name)?;
        self.tag_revision.fetch_add(1, Ordering::Release);
        Ok(())
    }
    /// Delete a tag and optionally remove its ancestors from media that had
    /// the tag, matching FileAtlas's explicit "clean parents" confirmation.
    pub fn delete_tag(&self, id: i64, clean_parents: bool) -> Result<Vec<i64>, NavigationError> {
        let affected = self.database.delete_tag_with_cleanup(id, clean_parents)?;
        self.tag_revision.fetch_add(1, Ordering::Release);
        Ok(affected)
    }
    pub fn move_tag(&self, id: i64, drop: TagDropPos) -> Result<Vec<i64>, NavigationError> {
        let affected = self.database.move_tag(id, drop)?;
        self.tag_revision.fetch_add(1, Ordering::Release);
        Ok(affected)
    }
    pub fn set_tag_appearance(
        &self,
        id: i64,
        color: Option<&str>,
        icon: Option<&str>,
    ) -> Result<(), NavigationError> {
        self.database.update_tag_appearance(id, color, icon)?;
        self.tag_revision.fetch_add(1, Ordering::Release);
        Ok(())
    }
    pub fn set_tag_display(&self, id: i64, display: bool) -> Result<(), NavigationError> {
        if !self.database.set_tag_display_subtree(id, display)? {
            return Err(NavigationError::TagNotFound(id));
        }
        self.tag_revision.fetch_add(1, Ordering::Release);
        Ok(())
    }
}

fn tree_contains_tag(tags: &[Tag], id: i64) -> bool {
    tags.iter()
        .any(|tag| tag.id == id || tree_contains_tag(&tag.children, id))
}

impl Drop for NavigationService {
    fn drop(&mut self) {
        if let Ok(scans) = self.scans.get_mut() {
            for cancel in scans.values() {
                cancel.store(true, Ordering::Release);
            }
        }
    }
}

/// The `#RRGGBB` parser is a shared theme token facility in the UI bridge.
pub use fileatlas_ui_bridge::theme::parse_hex_color;

pub fn format_hex_color(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

#[derive(Debug, thiserror::Error)]
pub enum NavigationError {
    #[error(transparent)]
    DatabaseAccess(#[from] fileatlas_plugin_database::DatabaseAccessError),
    #[error("folder path is not a directory: {0}")]
    NotDirectory(String),
    #[error("tag does not exist: {0}")]
    TagNotFound(i64),
    #[error("folder scan registry lock is poisoned")]
    Poisoned,
}

#[derive(Debug, Clone, Default)]
pub struct NavigationConfig;
pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    type Config = NavigationConfig;
    type Input = NavigationConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = NavigationPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.navigation".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let preferences = Arc::new(std::sync::RwLock::new(
            fileatlas_contracts::TagInteractionPreferences::default(),
        ));
        let preference_state = Arc::clone(&preferences);
        let _preference_listener = context
            .on::<fileatlas_contracts::TagInteractionPreferencesChanged, _>(
                cordis::event::observer_sync(
                    move |_, updated: fileatlas_contracts::TagInteractionPreferences| {
                        if let Ok(mut preferences) = preference_state.write() {
                            *preferences = updated;
                        }
                        Ok::<_, std::convert::Infallible>(())
                    },
                ),
            )
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let indexing = context
            .try_service::<IndexingService>()
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let watcher = context
            .try_service::<WatcherService>()
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let session_service = context
            .try_service::<fileatlas_plugin_session::SessionService>()
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let session = session_service
            .snapshot()
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let folders_service = Arc::new(SidebarFoldersService::default());
        let service = Arc::new(
            NavigationService::new(database, indexing, folders_service).with_watcher(watcher),
        );
        let folders = service
            .snapshot()
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?
            .folders;
        let selected_folder_id = session
            .selected_folder_id
            .filter(|id| folders.iter().any(|folder| folder.folder.id == *id));
        let selected_path = session.selected_path.filter(|path| {
            if service.browser_enabled() {
                return Path::new(path).is_dir();
            }
            let selected = Path::new(path).canonicalize();
            selected.is_ok_and(|selected| {
                folders.iter().any(|folder| {
                    Path::new(&folder.folder.path)
                        .canonicalize()
                        .is_ok_and(|root| selected.starts_with(root))
                })
            })
        });
        let selected_folder_id = if selected_path.is_some() {
            None
        } else {
            selected_folder_id
        };
        let expanded_subdirs = session.expanded_subdirs;
        let add_folder_requested = Arc::new(AtomicBool::new(false));
        let index_changed = Arc::new(AtomicBool::new(false));
        let dirty = Arc::clone(&index_changed);
        let tag_invalidation = Arc::downgrade(&service);
        let _index_listener = context
            .on::<fileatlas_contracts::MediaIndexChanged, _>(cordis::event::observer_sync(
                move |_, ()| {
                    dirty.store(true, Ordering::Release);
                    // Tagging merge/reparent and metadata reconciliation also
                    // publish this event; invalidate tag choices without a read.
                    if let Some(service) = tag_invalidation.upgrade() {
                        service.tag_revision.fetch_add(1, Ordering::Release);
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let pending = Arc::clone(&add_folder_requested);
        let _command_listener = context
            .on::<fileatlas_ui_bridge::UiCommandInvoked, _>(cordis::event::observer_sync(
                move |_, command: String| {
                    if command == "navigation/add-folder" {
                        pending.store(true, Ordering::Release);
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|error| NavigationPluginError::Dependency(error.to_string()))?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| NavigationPluginError::Publish(error.to_string()))?;
        if let Ok(registry) = context.try_service::<fileatlas_contracts::mcp::McpToolRegistry>() {
            let navigation = Arc::clone(&service);
            let browser_mode = service.browser_enabled();
            let tool = fileatlas_contracts::mcp::McpToolDefinition {
                name: "navigation.list_library".into(),
                title: if browser_mode {
                    "List folder favorites and tags".into()
                } else {
                    "List indexed folders and tags".into()
                },
                description: if browser_mode {
                    "Read saved folder favorites and the hierarchical tag tree. Folder entries are shortcuts, not indexed library roots.".into()
                } else {
                    "Read registered indexed folders, scan metadata, media counts, and the hierarchical tag tree.".into()
                },
                input_schema: serde_json::json!({"type":"object","properties":{},"additionalProperties":false})
                    .as_object().cloned().unwrap_or_default(),
                mutating: false,
                confirmation_required: false,
                handler: Arc::new(move |_| {
                    let navigation = Arc::clone(&navigation);
                    Box::pin(async move {
                        let snapshot = navigation.snapshot().map_err(|error| error.to_string())?;
                        let folders = snapshot.folders.into_iter().map(|node| serde_json::json!({
                            "id":node.folder.id,
                            "path":node.folder.path,
                            "name":node.folder.name,
                            "last_scan":node.folder.last_scan,
                            "media_count":node.media_count,
                        })).collect::<Vec<_>>();
                        Ok(serde_json::json!({"folders":folders,"tags":snapshot.tags}))
                    })
                }),
            };
            let mut tools = mcp::tools(&service, &context);
            tools.push(tool);
            let registration = registry
                .register("fileatlas.navigation", tools)
                .map_err(|error| NavigationPluginError::Publish(error.to_string()))?;
            let _mcp_registration = context
                .provide(Arc::new(NavigationMcpRegistration {
                    _registration: registration,
                }))
                .map_err(|error| NavigationPluginError::Publish(error.to_string()))?;
        }
        register_for_plugin(
            &context,
            "fileatlas.navigation",
            [
                UiContribution::new(
                    "navigation/sidebar-slot",
                    ContributionKind::Slot {
                        label: "workspace.sidebar".into(),
                        order: 10,
                    },
                ),
                UiContribution::new(
                    "navigation/sidebar",
                    ContributionKind::Panel {
                        label: "File Navigation".into(),
                        slot: "workspace.sidebar".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "navigation/folders-tab",
                    ContributionKind::Widget {
                        label: "Folders and Tags".into(),
                        slot: "workspace.sidebar".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "navigation/launcher-shortcuts-slot",
                    ContributionKind::Slot {
                        label: "navigation.launcher-shortcuts".into(),
                        order: 5,
                    },
                ),
            ],
        )
        .map_err(|error| NavigationPluginError::UiRegistration(error.to_string()))?;
        let sidebar_folders = Arc::clone(service.sidebar_folders());
        register_views_for_plugin(
            &context,
            "fileatlas.navigation",
            view::sidebar(
                service,
                context.clone(),
                session_service,
                selected_folder_id,
                selected_path,
                expanded_subdirs,
                preferences,
                add_folder_requested,
                index_changed,
                sidebar_folders,
            ),
        )
        .map_err(|error| NavigationPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

struct NavigationMcpRegistration {
    _registration: Arc<fileatlas_contracts::mcp::McpToolRegistration>,
}

impl Service for NavigationMcpRegistration {
    const NAME: &'static str = "fileatlas/navigation-mcp-registration";
}

#[derive(Debug, thiserror::Error)]
pub enum NavigationPluginError {
    #[error("required navigation dependency unavailable: {0}")]
    Dependency(String),
    #[error("navigation service publication failed: {0}")]
    Publish(String),
    #[error("navigation UI registration failed: {0}")]
    UiRegistration(String),
}

#[cfg(test)]
mod tests;
