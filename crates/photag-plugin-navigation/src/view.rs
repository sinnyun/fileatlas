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
use gpui_kit::component::input::{Input, InputState};
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
use photag_plugin_indexing::scan::ScanProgress;
use photag_plugin_session::SessionService;
use photag_ui_bridge::{UiViewContribution, theme};

use crate::{NavigationService, NavigationSnapshot, SubdirectoryNode};

mod render;
mod support;
mod tag_actions;
mod tag_modal;
mod tag_picker;

use support::{
    collect_tag_appearance_descendants, collect_tag_ids, find_tag, restore_expanded_directories,
    tag_matches,
};

pub(super) fn sidebar(
    service: Arc<NavigationService>,
    cordis: CordisContext,
    session: Arc<SessionService>,
    selected_folder_id: Option<i64>,
    selected_path: Option<String>,
    expanded_subdirs: HashSet<String>,
    preferences: Arc<RwLock<TagInteractionPreferences>>,
    add_folder_requested: Arc<std::sync::atomic::AtomicBool>,
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
        let view_selected_folder_id = selected_folder_id;
        let view_selected_path = selected_path.clone();
        let view_expanded_subdirs = expanded_subdirs.clone();
        let view_preferences = Arc::clone(&preferences);
        let folder_path = app.new(|cx| {
            InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Folder path"))
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
        let pending = Arc::clone(&add_folder_requested);
        let view = app.new(|cx| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(100))
                        .await;
                    if this
                        .update(cx, |view: &mut SidebarView, cx| {
                            let requested =
                                pending.swap(false, std::sync::atomic::Ordering::AcqRel);
                            let ui_requested = view.service.has_ui_request();
                            if requested || ui_requested {
                                if requested {
                                    view.pick_folder(cx);
                                }
                                cx.notify();
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
            SidebarView::new(
                view_service,
                view_cordis,
                tagging,
                view_session,
                view_selected_folder_id,
                view_selected_path,
                view_expanded_subdirs,
                saved.expanded_tag_ids,
                saved.selected_tag_ids,
                saved.excluded_tag_ids,
                saved.tag_click_mode,
                folder_path,
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
            )
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
            move |window, app| {
                let sidebar = breadcrumb(window, app);
                app.new(|cx| {
                    let subscription = cx.observe(&sidebar, |_, _, cx| cx.notify());
                    SidebarBreadcrumb {
                        sidebar,
                        _subscription: subscription,
                    }
                })
                .into()
            },
        ),
        UiViewContribution::new(
            "navigation/tag-management-modal",
            "workspace.modal",
            0,
            move |window, app| {
                let sidebar = modal(window, app);
                app.new(|cx| {
                    let subscription = cx.observe(&sidebar, |_, _, cx| cx.notify());
                    SidebarTagModal {
                        sidebar,
                        _subscription: subscription,
                    }
                })
                .into()
            },
        ),
    ]
}

struct SidebarBreadcrumb {
    sidebar: Entity<SidebarView>,
    _subscription: Subscription,
}

impl Render for SidebarBreadcrumb {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.render_breadcrumb(cx))
    }
}

struct SidebarTagModal {
    sidebar: Entity<SidebarView>,
    _subscription: Subscription,
}

impl Render for SidebarTagModal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.render_tag_modal(cx))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.eq_ignore_ascii_case("escape") {
                    let sidebar = this.sidebar.clone();
                    let _ = sidebar.update(cx, |sidebar, cx| {
                        sidebar.close_tag_modal();
                        cx.notify();
                    });
                    cx.notify();
                }
            }))
    }
}

struct SidebarView {
    locale_revision: u64,
    service: Arc<NavigationService>,
    cordis: CordisContext,
    tagging: Option<Arc<photag_plugin_tagging::TaggingService>>,
    session: Arc<SessionService>,
    folder_path: Entity<InputState>,
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
    expanded_tags: HashSet<i64>,
    restored_folder_id: Option<i64>,
    restored_path: Option<String>,
    restore_selection_pending: bool,
    preferences: Arc<RwLock<TagInteractionPreferences>>,
    included_tags: Vec<i64>,
    excluded_tags: Vec<i64>,
    show_folder_path_entry: bool,
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
    tag_create_open: bool,
}

#[derive(Clone)]
enum ScanRetryTarget {
    Folder(i64),
    Path(String),
}

impl SidebarView {
    fn new(
        service: Arc<NavigationService>,
        cordis: CordisContext,
        tagging: Option<Arc<photag_plugin_tagging::TaggingService>>,
        session: Arc<SessionService>,
        restored_folder_id: Option<i64>,
        restored_path: Option<String>,
        expanded_subdirs: HashSet<String>,
        expanded_tags: HashSet<i64>,
        included_tags: Vec<i64>,
        excluded_tags: Vec<i64>,
        saved_tag_click_mode: Option<photag_plugin_session::TagClickMode>,
        folder_path: Entity<InputState>,
        new_folder_name: Entity<InputState>,
        tag_name: Entity<InputState>,
        tag_parent: Entity<InputState>,
        tag_edit_name: Entity<InputState>,
        tag_color: Entity<InputState>,
        tag_icon: Entity<InputState>,
        tag_move_target: Entity<InputState>,
        tag_search: Entity<InputState>,
        merge_search: Entity<InputState>,
        preferences: Arc<RwLock<TagInteractionPreferences>>,
    ) -> Self {
        let snapshot = service.snapshot().map_err(|error| error.to_string());
        let mut expanded_tags = expanded_tags;
        if let Ok(snapshot) = &snapshot {
            let mut live_tag_ids = HashSet::new();
            for tag in &snapshot.tags {
                collect_tag_ids(tag, &mut live_tag_ids);
            }
            let saved_count = expanded_tags.len();
            expanded_tags.retain(|id| live_tag_ids.contains(id));
            if expanded_tags.len() != saved_count {
                if let Ok(mut saved) = session.snapshot() {
                    saved.expanded_tag_ids = expanded_tags.clone();
                    if let Err(error) = session.replace(saved) {
                        eprintln!("stale tag expansion cleanup failed: {error}");
                    }
                }
            }
        }
        let expanded_directories = match &snapshot {
            Ok(snapshot) => restore_expanded_directories(&service, snapshot, &expanded_subdirs),
            Err(_) => HashMap::new(),
        };
        Self {
            locale_revision: u64::MAX,
            service: Arc::clone(&service),
            cordis,
            tagging,
            session,
            folder_path,
            new_folder_name,
            tag_name,
            tag_parent,
            parent_tag_id: None,
            tag_edit_name,
            tag_color,
            tag_icon,
            tag_edit_display: false,
            tag_edit_name_only: false,
            tag_move_target,
            move_target_tag_id: None,
            tag_search,
            merge_search,
            snapshot,
            active_scan: None,
            active_scan_folder_id: None,
            scan_failure: None,
            scan_progress: None,
            expanded_directories,
            expanded_tags,
            restored_folder_id,
            restored_path,
            restore_selection_pending: true,
            preferences,
            included_tags,
            excluded_tags,
            show_folder_path_entry: false,
            show_tag_search_controls: false,
            new_folder_parent: None,
            exclude_mode: saved_tag_click_mode
                == Some(photag_plugin_session::TagClickMode::Exclude),
            tag_search_match: TagSearchMatch::Contains,
            apply_tag_appearance_to_children: false,
            exclude_edited_tag_children: true,
            associated_edit_tag: None,
            associated_tag_ids: Vec::new(),
            merge_source: None,
            move_source_id: None,
            merge_reparent_children: true,
            confirm_delete: None,
            tag_create_open: false,
        }
    }

    fn refresh(&mut self) {
        self.snapshot = self.service.snapshot().map_err(|error| error.to_string());
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

    fn pick_folder(&mut self, cx: &mut Context<Self>) {
        // Match PhoTag's PendingAction::AddFolder flow: a native directory
        // dialog returns an optional path, and cancellation is a no-op.
        let Some(path) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let path = path.to_string_lossy().into_owned();
        self.add_folder_path(path, cx);
    }

    fn add_folder(&mut self, cx: &mut Context<Self>) {
        let path = self.folder_path.read(cx).value().trim().to_owned();
        self.add_folder_path(path, cx);
    }

    fn monitor_scan(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let active = this
                    .update(cx, |view, cx| {
                        if view.poll_scan() {
                            view.notify_media_index(cx);
                        }
                        cx.notify();
                        view.active_scan.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
            }
        })
        .detach();
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
                eprintln!("media index change dispatch failed: {error}");
            }
        })
        .detach();
    }

    fn select_folder(&self, folder_id: Option<i64>, cx: &mut Context<Self>) {
        if let Ok(mut session) = self.session.snapshot() {
            session.selected_folder_id = folder_id;
            session.selected_path = None;
            if let Err(error) = self.session.replace(session) {
                eprintln!("folder selection session save failed: {error}");
            }
        }
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<FolderSelectionChanged>(Routing::Unscoped, folder_id)
                .await
            {
                eprintln!("folder selection dispatch failed: {error}");
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
            eprintln!("open folder in file manager failed: {error}");
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

    fn emit_tag_selection(&self, cx: &mut Context<Self>) {
        let cordis = self.cordis.clone();
        let selection = TagFilterSelection {
            included: self.included_tags.clone(),
            excluded: self.excluded_tags.clone(),
        };
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<TagFilterSelectionChanged>(Routing::Unscoped, selection)
                .await
            {
                eprintln!("tag selection dispatch failed: {error}");
            }
        })
        .detach();
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
                eprintln!("could not save tag click mode: {error}");
            }
        }
        cx.notify();
    }

    fn select_all_tags(
        &mut self,
        tags: &[photag_plugin_database::db::queries::Tag],
        cx: &mut Context<Self>,
    ) {
        fn collect(tags: &[photag_plugin_database::db::queries::Tag], ids: &mut Vec<i64>) {
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

    fn toggle_directory(&mut self, path: &str) {
        if self.expanded_directories.remove(path).is_none() {
            self.expanded_directories
                .insert(path.to_owned(), self.service.subdirectories(path));
        }
        if let Ok(mut session) = self.session.snapshot() {
            session.expanded_subdirs = self.expanded_directories.keys().cloned().collect();
            if let Err(error) = self.session.replace(session) {
                eprintln!("directory expansion session save failed: {error}");
            }
        }
    }

    fn toggle_tag(&mut self, tag_id: i64) {
        if !self.expanded_tags.remove(&tag_id) {
            self.expanded_tags.insert(tag_id);
        }
        if let Ok(mut saved) = self.session.snapshot() {
            saved.expanded_tag_ids = self.expanded_tags.clone();
            if let Err(error) = self.session.replace(saved) {
                eprintln!("tag expansion session save failed: {error}");
            }
        }
    }

    fn select_path(&self, path: String, cx: &mut Context<Self>) {
        if let Ok(mut session) = self.session.snapshot() {
            session.selected_folder_id = None;
            session.selected_path = Some(path.clone());
            if let Err(error) = self.session.replace(session) {
                eprintln!("path selection session save failed: {error}");
            }
        }
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<DirectoryPathSelected>(Routing::Unscoped, Some(path))
                .await
            {
                eprintln!("directory selection dispatch failed: {error}");
            }
        })
        .detach();
    }
}
