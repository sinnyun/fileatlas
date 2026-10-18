//! Native GPUI sidebar rendering for the navigation plugin.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use cordis::{Context as CordisContext, Routing};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::InputState;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::gpui::InteractiveElement as _;
use gpui_kit::gpui::StatefulInteractiveElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use photag_contracts::{
    DirectoryPathSelected, FolderSelectionChanged, MediaIndexChanged, TagDragPayload,
    TagExcludeModifier, TagFilterSelection, TagFilterSelectionChanged, TagInteractionPreferences,
    TagSearchMatch,
};
use photag_plugin_indexing::ScanProgress;
use photag_plugin_session::SessionService;
use photag_ui_bridge::{UiViewContribution, theme};

use crate::{NavigationService, NavigationSnapshot, SubdirectoryNode};

mod address_bar;
mod construction;
mod directories;
mod favorite_groups;
mod favorite_menu;
mod location_actions;
use favorite_menu::favorite_menu;
mod folder_picker;
mod page_state;
mod projections;
mod render;
mod sections;
mod sidebar_persistence;
mod sidebar_state;
mod support;
mod tag_actions;
mod tag_modal;
mod tag_picker;

use page_state::NavigationPage;
use projections::{breadcrumb_view, tag_modal_view};

use support::{
    collect_tag_appearance_descendants, collect_tag_ids, find_tag, restore_expanded_directories,
    tag_matches,
};

/// Bounded container for one sidebar region: drives, quick access or favorites.
/// The theme decides whether regions are separated by a hairline or a shadow.
fn sidebar_region(body: Div) -> Div {
    photag_ui_bridge::components::sidebar_section(body)
}

pub(super) fn sidebar(
    service: Arc<NavigationService>,
    cordis: CordisContext,
    session: Arc<SessionService>,
    selected_folder_id: Option<i64>,
    selected_path: Option<String>,
    expanded_subdirs: HashSet<String>,
    preferences: Arc<RwLock<TagInteractionPreferences>>,
    add_folder_requested: Arc<std::sync::atomic::AtomicBool>,
    index_changed: Arc<std::sync::atomic::AtomicBool>,
    folders: Arc<crate::SidebarFoldersService>,
) -> Vec<UiViewContribution> {
    let cached = Arc::new(std::sync::Mutex::new(None::<Entity<SidebarView>>));
    let create = Arc::new(move |window: &mut Window, app: &mut App| {
        let mut cached = cached.lock().expect("navigation view factory lock");
        if let Some(view) = cached.as_ref() {
            return view.clone();
        }
        let view_service = Arc::clone(&service);
        let view_cordis = cordis.clone();
        let tagging = view_cordis
            .try_service::<photag_plugin_tagging::TaggingService>()
            .ok();
        let view_session = Arc::clone(&session);
        let saved = view_session.snapshot().unwrap_or_default();
        let saved_page = NavigationPage::from_key(&saved.sidebar_tab).unwrap_or_default();
        let bridge = view_cordis
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .ok();
        let extension_views = bridge
            .as_ref()
            .and_then(|bridge| bridge.views.snapshot().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.contribution.slot == "navigation.extensions")
            .map(|entry| (entry.contribution.factory)(window, app))
            .collect::<Vec<_>>();
        let launcher_shortcuts = bridge
            .as_ref()
            .and_then(|bridge| bridge.views.snapshot().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| entry.contribution.slot == "navigation.launcher-shortcuts")
            .map(|entry| (entry.contribution.factory)(window, app))
            .collect::<Vec<_>>();
        let active_page = bridge
            .as_ref()
            .and_then(|bridge| bridge.surfaces.value("navigation.tab"))
            .and_then(|value| NavigationPage::from_key(&value))
            .unwrap_or(saved_page);
        if let Some(bridge) = bridge.as_ref() {
            let _ = bridge
                .surfaces
                .set_value("navigation.tab", active_page.key());
            if bridge.surfaces.value("navigation.folder-search").is_none() {
                let _ = bridge.surfaces.set_value("navigation.folder-search", "");
            }
        }
        let view_selected_folder_id = selected_folder_id;
        let view_selected_path = selected_path.clone();
        let view_expanded_subdirs = expanded_subdirs.clone();
        let view_preferences = Arc::clone(&preferences);
        let folder_path = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Folder path"))
        });
        let folder_search = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Search folders…"))
        });
        let new_folder_name = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Folder name"))
        });
        let tag_name = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("New tag name"))
        });
        let tag_parent = app.new(|cx| {
            InputState::new(window, cx)
                .placeholder(photag_ui_bridge::i18n::tr("Search parent tags…"))
        });
        let tag_edit_name = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Tag name"))
        });
        let tag_color = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr(
                "Color #RRGGBB (empty = default)",
            ))
        });
        let tag_icon = app.new(|cx| {
            InputState::new(window, cx)
                .placeholder(photag_ui_bridge::i18n::tr("Icon / emoji (empty = default)"))
        });
        let tag_move_target = app.new(|cx| {
            InputState::new(window, cx)
                .placeholder(photag_ui_bridge::i18n::tr("Search move targets…"))
        });
        let tag_search = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Search tags…"))
        });
        let merge_search = app.new(|cx| {
            InputState::new(window, cx)
                .placeholder(photag_ui_bridge::i18n::tr("Filter target tags…"))
        });
        let sidebar_editor_input = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Folder path"))
        });
        let address_input = app.new(|cx| {
            InputState::new(window, cx)
                .placeholder(photag_ui_bridge::i18n::tr("Enter folder address"))
        });
        let pending = Arc::clone(&add_folder_requested);
        let dirty = Arc::clone(&index_changed);
        let folders_service = Arc::clone(&folders);
        let poll_folders_service = Arc::clone(&folders);
        let view = app.new(|cx| {
            photag_ui_bridge::refresh::poll(
                cx,
                Duration::from_millis(100),
                move |view: &mut SidebarView, cx| {
                    let requested = pending.swap(false, std::sync::atomic::Ordering::AcqRel);
                    let ui_requested = view.service.has_ui_request();
                    let changed = dirty.swap(false, std::sync::atomic::Ordering::AcqRel);
                    let bridge = view
                        .cordis
                        .try_service::<photag_ui_bridge::UiBridgeService>()
                        .ok();
                    let surface_page = bridge
                        .as_ref()
                        .and_then(|bridge| bridge.surfaces.value("navigation.tab"))
                        .and_then(|value| NavigationPage::from_key(&value));
                    let directory_changed = view.poll_directories();
                    let drives_changed = view.drives_read.poll() | view.system_folders_read.poll();
                    let config_changed =
                        poll_folders_service.revision() != view.sidebar_config_revision;
                    if config_changed {
                        view.sidebar_config_revision = poll_folders_service.revision();
                    }
                    let page_changed = surface_page.is_some_and(|page| page != view.active_page);
                    if let Some(page) = surface_page {
                        if page != view.active_page {
                            view.set_navigation_page(page);
                        }
                    }
                    let folder_search = bridge
                        .as_ref()
                        .and_then(|bridge| bridge.surfaces.value("navigation.folder-search"))
                        .unwrap_or_default();
                    let input_search = view.folder_search.read(cx).value().to_string();
                    let search_changed = if folder_search != view.last_folder_search_surface {
                        view.pending_folder_search = Some(folder_search.clone());
                        view.last_folder_search_surface = folder_search;
                        true
                    } else if input_search != view.last_folder_search_surface {
                        if let Some(bridge) = bridge.as_ref() {
                            let _ = bridge
                                .surfaces
                                .set_value("navigation.folder-search", &input_search);
                        }
                        view.last_folder_search_surface = input_search;
                        false
                    } else {
                        false
                    };
                    if changed {
                        view.refresh();
                        view.refresh_selected_directory();
                    }
                    let favorites_imported = view.import_library_favorites(cx);
                    let address_changed = view.sync_address_location();
                    let changed = favorites_imported
                        || address_changed
                        || requested
                        || ui_requested
                        || changed
                        || directory_changed
                        || drives_changed
                        || config_changed
                        || page_changed
                        || search_changed;
                    if changed {
                        if requested {
                            view.pick_folder(cx);
                        }
                    }
                    view.publish_sidebar_state(cx);
                    changed.into()
                },
            );
            SidebarView::new(
                view_service,
                view_cordis,
                tagging,
                view_session,
                extension_views,
                launcher_shortcuts,
                view_selected_folder_id,
                view_selected_path,
                view_expanded_subdirs,
                saved.expanded_tag_ids,
                saved.selected_tag_ids,
                saved.excluded_tag_ids,
                saved.tag_click_mode,
                active_page,
                folder_path,
                folder_search,
                new_folder_name,
                tag_name,
                tag_parent,
                tag_edit_name,
                tag_color,
                tag_icon,
                tag_move_target,
                tag_search,
                merge_search,
                view_preferences,
                folders_service,
                sidebar_editor_input,
                address_input,
                cx.focus_handle(),
            )
        });
        address_bar::subscribe(&view, window, app);
        view.update(app, |view, cx| {
            let wake = view.service.ui_wake.clone();
            view._ui_ready_task = Some(photag_ui_bridge::refresh::listen(cx, wake, |view, _| {
                view.service.has_ui_request()
            }));
        });
        view.update(app, |view, cx| {
            view._window_activation =
                Some(cx.observe_window_activation(window, |view, window, cx| {
                    if window.is_window_active() {
                        view.refresh_sidebar_roots();
                        cx.notify();
                    }
                }));
        });
        *cached = Some(view.clone());
        view
    });
    let sidebar = create.clone();
    let breadcrumb = create.clone();
    let modal = create;
    vec![
        UiViewContribution::new(
            "navigation/sidebar-view",
            "workspace.sidebar",
            0,
            move |window, app| sidebar(window, app).into(),
        ),
        UiViewContribution::new(
            "navigation/breadcrumb-view",
            "command.breadcrumb",
            0,
            move |window, app| breadcrumb_view(breadcrumb(window, app), app).into(),
        ),
        UiViewContribution::new(
            "navigation/tag-management-modal",
            "workspace.modal",
            0,
            move |window, app| tag_modal_view(modal(window, app), app).into(),
        ),
    ]
}

struct SidebarView {
    _ui_ready_task: Option<Task<()>>,
    folder_picker_task: Option<Task<()>>,
    locale_revision: u64,
    service: Arc<NavigationService>,
    cordis: CordisContext,
    tagging: Option<Arc<photag_plugin_tagging::TaggingService>>,
    session: Arc<SessionService>,
    extension_views: Vec<AnyView>,
    launcher_shortcuts: Vec<AnyView>,
    active_page: NavigationPage,
    folder_path: Entity<InputState>,
    folder_search: Entity<InputState>,
    last_folder_search_surface: String,
    pending_folder_search: Option<String>,
    new_folder_name: Entity<InputState>,
    tag_name: Entity<InputState>,
    tag_parent: Entity<InputState>,
    parent_tag_id: Option<i64>,
    tag_edit_name: Entity<InputState>,
    tag_color: Entity<InputState>,
    tag_icon: Entity<InputState>,
    tag_edit_display: bool,
    tag_edit_name_only: bool,
    tag_move_target: Entity<InputState>,
    move_target_tag_id: Option<i64>,
    tag_search: Entity<InputState>,
    merge_search: Entity<InputState>,
    snapshot: Result<NavigationSnapshot, String>,
    active_scan: Option<std::sync::mpsc::Receiver<Option<ScanProgress>>>,
    active_scan_folder_id: Option<i64>,
    scan_failure: Option<(ScanRetryTarget, String)>,
    scan_progress: Option<ScanProgress>,
    expanded_directories: HashMap<String, Vec<SubdirectoryNode>>,
    pending_directories: std::collections::VecDeque<String>,
    directory_read: photag_ui_bridge::read_model::ReadModel<String, Vec<SubdirectoryNode>>,
    expanded_tags: HashSet<i64>,
    restored_folder_id: Option<i64>,
    restored_path: Option<String>,
    restore_selection_pending: bool,
    preferences: Arc<RwLock<TagInteractionPreferences>>,
    included_tags: Vec<i64>,
    excluded_tags: Vec<i64>,
    show_tag_search_controls: bool,
    new_folder_parent: Option<String>,
    exclude_mode: bool,
    tag_search_match: TagSearchMatch,
    apply_tag_appearance_to_children: bool,
    exclude_edited_tag_children: bool,
    associated_edit_tag: Option<i64>,
    associated_tag_ids: Vec<i64>,
    merge_source: Option<i64>,
    move_source_id: Option<i64>,
    merge_reparent_children: bool,
    confirm_delete: Option<(i64, String, usize)>,
    delete_clean_parents: bool,
    tag_create_open: bool,
    folders: Arc<crate::SidebarFoldersService>,
    sidebar_config_revision: u64,
    favorite_import_pending: bool,
    favorite_action_error: Option<String>,
    favorite_header_bounds: Arc<RwLock<std::collections::BTreeMap<String, [f32; 4]>>>,
    sidebar_io_revision: u64,
    sidebar_save_error: Option<String>,
    directory_revision: u64,
    system_folders_read:
        photag_ui_bridge::read_model::ReadModel<u64, Vec<(String, std::path::PathBuf)>>,
    _window_activation: Option<Subscription>,
    address: address_bar::AddressBarState,
    _address_subscription: Option<Subscription>,
    drives_read: photag_ui_bridge::read_model::ReadModel<(bool, u64), Vec<crate::DriveInfo>>,
    collapsed_sections: HashSet<&'static str>,
    sidebar_editor: Option<SidebarEditor>,
    sidebar_editor_error: Option<String>,
    sidebar_editor_input: Entity<InputState>,
    picker_editor: Option<SidebarEditor>,
    collapsed_groups: HashSet<String>,
    sidebar_state_signature_cache: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SidebarEditor {
    AddQuickAccess,
    RenameQuickAccess { path: String },
    CreateGroup,
    RenameGroup { group_id: String },
    AddFavorite { group_id: String },
    RenameFavorite { group_id: String, folder_id: String },
}

impl SidebarEditor {
    pub(super) fn placeholder(&self) -> &'static str {
        match self {
            Self::AddQuickAccess | Self::AddFavorite { .. } => "Folder path",
            Self::RenameQuickAccess { .. } | Self::RenameFavorite { .. } => "Display name",
            Self::CreateGroup | Self::RenameGroup { .. } => "Group name",
        }
    }
}

#[derive(Clone)]
enum ScanRetryTarget {
    Directory(String),
    Folder(i64),
    Path(String),
}

impl SidebarView {
    fn refresh(&mut self) {
        self.snapshot = self.service.snapshot().map_err(|error| error.to_string());
        self.favorite_import_pending = true;
    }

    fn set_navigation_page(&mut self, page: NavigationPage) {
        self.active_page = page;
        if let Ok(bridge) = self
            .cordis
            .try_service::<photag_ui_bridge::UiBridgeService>()
        {
            if let Err(error) = bridge.surfaces.set_value("navigation.tab", page.key()) {
                tracing::error!("navigation tab surface update failed: {error}");
            }
        }
        if let Ok(mut state) = self.session.snapshot() {
            state.sidebar_tab = page.key().to_owned();
            if let Err(error) = self.session.replace(state) {
                tracing::error!("navigation tab session update failed: {error}");
            }
        }
    }

    fn close_tag_modal(&mut self) {
        self.tag_create_open = false;
        self.associated_edit_tag = None;
        self.confirm_delete = None;
        self.merge_source = None;
        self.move_source_id = None;
        self.parent_tag_id = None;
        self.move_target_tag_id = None;
    }

    fn rescan(&mut self, folder_id: i64, cx: &mut Context<Self>) {
        if self.service.browser_enabled() {
            self.select_folder(Some(folder_id), cx);
            let cordis = self.cordis.clone();
            cx.spawn(async move |_, _| {
                let _ = cordis
                    .emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
                    .await;
            })
            .detach();
        }
        match self.service.rescan(folder_id) {
            Ok(progress) => {
                self.active_scan = Some(progress);
                self.active_scan_folder_id = Some(folder_id);
                self.scan_progress = None;
                self.scan_failure = None;
                self.refresh();
                self.monitor_scan(cx);
            }
            Err(error) => {
                let message = error.to_string();
                self.scan_failure = Some((ScanRetryTarget::Folder(folder_id), message.clone()));
                self.snapshot = Err(message);
            }
        }
    }

    fn add_folder_path(&mut self, path: String, cx: &mut Context<Self>) {
        let path = path.trim().to_owned();
        if path.is_empty() {
            return;
        }
        match self.service.add_folder(path.clone()) {
            Ok((folder, progress)) => {
                self.select_folder(Some(folder.id), cx);
                self.active_scan = Some(progress);
                self.active_scan_folder_id = Some(folder.id);
                self.scan_progress = None;
                self.scan_failure = None;
                self.refresh();
                self.monitor_scan(cx);
            }
            Err(error) => {
                let message = error.to_string();
                self.scan_failure = Some((ScanRetryTarget::Path(path), message.clone()));
                self.snapshot = Err(message);
            }
        }
    }

    fn monitor_scan(&mut self, cx: &mut Context<Self>) {
        cx.notify();
        photag_ui_bridge::refresh::poll(cx, Duration::from_millis(200), |view: &mut Self, cx| {
            let signature = |view: &Self| {
                view.scan_progress
                    .as_ref()
                    .map(|p| (p.scanned, p.total, p.phase.clone(), p.error.clone()))
            };
            let before = signature(view);
            let completed = view.poll_scan();
            if completed {
                view.notify_media_index(cx);
            }
            let changed = completed || before != signature(view);
            photag_ui_bridge::refresh::PollResult::until(changed, view.active_scan.is_some())
        });
    }

    fn poll_scan(&mut self) -> bool {
        let Some(receiver) = &self.active_scan else {
            return false;
        };
        let mut completed = false;
        let mut failure = None;
        loop {
            match receiver.try_recv() {
                Ok(Some(progress)) => {
                    if let Some(error) = progress.error.clone() {
                        failure = Some(error);
                    }
                    self.scan_progress = Some(progress);
                }
                Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    completed = true;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        if completed {
            self.active_scan = None;
            failure = failure.or_else(|| {
                self.scan_progress
                    .as_ref()
                    .and_then(|progress| progress.error.clone())
            });
            if let (Some(folder_id), Some(error)) = (self.active_scan_folder_id, failure) {
                self.scan_failure = Some((ScanRetryTarget::Folder(folder_id), error));
            }
            self.active_scan_folder_id = None;
            self.scan_progress = None;
            self.refresh();
        }
        completed
    }

    fn cancel_active_scan(&mut self, cx: &mut Context<Self>) {
        let Some(folder_id) = self.active_scan_folder_id.take() else {
            return;
        };
        if let Err(error) = self.service.cancel_scan(folder_id) {
            self.snapshot = Err(error.to_string());
        }
        self.active_scan = None;
        self.scan_progress = None;
        cx.notify();
    }

    fn retry_scan(&mut self, target: ScanRetryTarget, cx: &mut Context<Self>) {
        self.scan_failure = None;
        match target {
            ScanRetryTarget::Folder(folder_id) => self.rescan(folder_id, cx),
            ScanRetryTarget::Path(path) => self.add_folder_path(path, cx),
            ScanRetryTarget::Directory(path) => self.queue_directory(&path),
        }
        cx.notify();
    }

    fn remove_folder(&mut self, folder_id: i64, cx: &mut Context<Self>) {
        match self.service.remove_folder(folder_id) {
            Ok(_) => {
                self.refresh();
                self.notify_media_index(cx);
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
    }

    fn notify_media_index(&self, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<MediaIndexChanged>(Routing::Unscoped, ())
                .await
            {
                tracing::error!("media index change dispatch failed: {error}");
            }
        })
        .detach();
    }

    fn request_new_subfolder(
        &mut self,
        parent: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_folder_name
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.new_folder_parent = Some(parent);
        cx.notify();
    }

    fn create_subfolder(&mut self, cx: &mut Context<Self>) {
        let Some(parent) = self.new_folder_parent.clone() else {
            return;
        };
        let name = self.new_folder_name.read(cx).value().trim().to_owned();
        if name.is_empty() || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            self.snapshot = Err("Folder name is empty or contains an invalid character".into());
            cx.notify();
            return;
        }
        let target = Path::new(&parent).join(&name);
        if target.exists() {
            self.snapshot = Err(format!("Folder already exists: {}", target.display()));
            cx.notify();
            return;
        }
        if let Err(error) = std::fs::create_dir(&target) {
            self.snapshot = Err(format!("Could not create folder: {error}"));
            cx.notify();
            return;
        }
        if self.expanded_directories.contains_key(&parent) {
            let children = self.service.subdirectories(&parent);
            self.expanded_directories.insert(parent, children);
        }
        self.new_folder_parent = None;
        self.refresh();
        cx.notify();
    }

    fn open_folder_in_explorer(&self, path: &str) {
        #[cfg(target_os = "windows")]
        let result = std::process::Command::new("explorer").arg(path).spawn();
        #[cfg(target_os = "linux")]
        let result = std::process::Command::new("xdg-open").arg(path).spawn();
        #[cfg(target_os = "macos")]
        let result = std::process::Command::new("open").arg(path).spawn();
        if let Err(error) = result {
            tracing::error!("open folder in file manager failed: {error}");
        }
    }

    fn select_tag(&mut self, tag_id: Option<i64>, cx: &mut Context<Self>) {
        if let Some(tag_id) = tag_id {
            self.included_tags.clear();
            self.excluded_tags.clear();
            self.included_tags.push(tag_id);
        } else {
            self.included_tags.clear();
            self.excluded_tags.clear();
        }
        self.emit_tag_selection(cx);
    }

    fn toggle_tag_selection(&mut self, tag_id: i64, modifiers: Modifiers, cx: &mut Context<Self>) {
        let prefs = self
            .preferences
            .read()
            .map(|prefs| *prefs)
            .unwrap_or_default();
        let modifier_held = match prefs.exclude_modifier {
            TagExcludeModifier::None => false,
            TagExcludeModifier::Ctrl => modifiers.control,
            TagExcludeModifier::Shift => modifiers.shift,
            TagExcludeModifier::Alt => modifiers.alt,
        };
        let exclude = self.exclude_mode ^ modifier_held;
        let (target, other) = if exclude {
            (&mut self.excluded_tags, &mut self.included_tags)
        } else {
            (&mut self.included_tags, &mut self.excluded_tags)
        };
        other.retain(|id| *id != tag_id);
        if target.contains(&tag_id) {
            target.retain(|id| *id != tag_id);
        } else {
            target.push(tag_id);
        }
        self.emit_tag_selection(cx);
        cx.notify();
    }

    fn set_tag_click_mode(&mut self, exclude: bool, cx: &mut Context<Self>) {
        self.exclude_mode = exclude;
        if let Ok(mut session) = self.session.snapshot() {
            session.tag_click_mode = Some(if exclude {
                photag_plugin_session::TagClickMode::Exclude
            } else {
                photag_plugin_session::TagClickMode::Select
            });
            if let Err(error) = self.session.replace(session) {
                tracing::error!("could not save tag click mode: {error}");
            }
        }
        cx.notify();
    }

    fn select_all_tags(&mut self, tags: &[photag_plugin_database::Tag], cx: &mut Context<Self>) {
        fn collect(tags: &[photag_plugin_database::Tag], ids: &mut Vec<i64>) {
            for tag in tags {
                ids.push(tag.id);
                collect(&tag.children, ids);
            }
        }
        let mut all = Vec::new();
        collect(tags, &mut all);
        let target = if self.exclude_mode {
            &mut self.excluded_tags
        } else {
            &mut self.included_tags
        };
        if target.len() == all.len() {
            target.clear();
        } else {
            *target = all;
        }
        self.emit_tag_selection(cx);
        cx.notify();
    }

    fn toggle_tag(&mut self, tag_id: i64) {
        if !self.expanded_tags.remove(&tag_id) {
            self.expanded_tags.insert(tag_id);
        }
        if let Ok(mut saved) = self.session.snapshot() {
            saved.expanded_tag_ids = self.expanded_tags.clone();
            if let Err(error) = self.session.replace(saved) {
                tracing::error!("tag expansion session save failed: {error}");
            }
        }
    }
}
