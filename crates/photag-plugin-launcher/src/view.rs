use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

mod cards;
mod inspector;
mod layout;
mod menu;
mod sidebar;
mod sorting;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;
use photag_ui_bridge::panel_catalog::PanelCatalog;
use photag_ui_bridge::{UiSurfaceState, read_model::ReadModel};
use photag_ui_bridge::{UiViewContribution, theme};
use sorting::SortBy;

use crate::{LauncherCategory, LauncherEntry, LauncherService, LauncherSnapshot, PANEL_ID};

fn tr(source: &str) -> String {
    photag_ui_bridge::i18n::tr(source)
}

fn category_icon(name: &str) -> IconName {
    match name {
        "常用" | "General" => IconName::Star,
        "开发工具" => IconName::Code,
        "设计创作" => IconName::Image,
        "办公软件" => IconName::FileText,
        "通讯社交" => IconName::Link,
        "网络浏览" => IconName::Globe,
        "影音播放" => IconName::Music,
        "游戏" => IconName::Puzzle,
        "系统工具" => IconName::Settings,
        _ => IconName::LayoutGrid,
    }
}

pub(super) fn sidebar_button(panels: Arc<PanelCatalog>) -> UiViewContribution {
    UiViewContribution::new(
        "program-launcher/sidebar-button-view",
        "navigation.launcher-shortcuts",
        0,
        move |_, app| {
            let panels = Arc::clone(&panels);
            app.new(|_| LauncherShortcut { panels }).into()
        },
    )
}

struct LauncherShortcut {
    panels: Arc<PanelCatalog>,
}

impl Render for LauncherShortcut {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("program-launcher-open-sidebar")
            .v_flex()
            .w(px(42.))
            .h(px(54.))
            .items_center()
            .justify_center()
            .gap(px(3.))
            .rounded(px(6.))
            .text_xs()
            .text_color(rgb(theme::TEXT_SECONDARY))
            .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
            .on_click(cx.listener(|this, _, _, _| {
                if let Err(error) = this.panels.open(PANEL_ID) {
                    tracing::error!(%error, "could not open program launchpad");
                }
            }))
            .child(theme::icon(IconName::LayoutGrid))
            .child(tr("Apps"))
            .into_any_element()
    }
}

pub(super) fn panel(
    service: Arc<LauncherService>,
    surfaces: Arc<UiSurfaceState>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    let name_input = app.new(|cx| InputState::new(window, cx).placeholder(tr("Name")));
    let entry_name = app.new(|cx| InputState::new(window, cx).placeholder(tr("Name")));
    let search_input = app.new(|cx| InputState::new(window, cx).placeholder(tr("Search programs")));
    app.new(|cx| {
        let service_task =
            photag_ui_bridge::refresh::listen(cx, service.wake(), |view: &mut LauncherPanel, _| {
                let latest = view.service.snapshot();
                if latest.revision == view.snapshot.revision {
                    return false;
                }
                view.snapshot = latest;
                if view
                    .selected_entry
                    .is_some_and(|id| !view.snapshot.entries.iter().any(|e| e.id == id))
                {
                    view.selected_entry = None;
                }
                true
            });
        let icon_wake = photag_ui_bridge::refresh::Wake::default();
        let mut icons = ReadModel::default();
        icons.set_wake(icon_wake.clone());
        let icon_task =
            photag_ui_bridge::refresh::listen(cx, icon_wake, |view: &mut LauncherPanel, _| {
                view.icons.poll()
            });
        let entry_subscription = cx.subscribe(&entry_name, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let search_subscription = cx.subscribe(&search_input, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        LauncherPanel {
            _search_subscription: search_subscription,
            _entry_subscription: entry_subscription,
            _icon_task: icon_task,
            _service_task: service_task,
            add_program_task: None,
            icons,
            surfaces,
            renders: 0,
            focus_handle: cx.focus_handle(),
            entry_name,
            selected_entry: None,
            draft_category: 0,
            draft_hidden: false,
            sort_by: SortBy::Name,
            descending: false,
            snapshot: service.snapshot(),
            service,
            name_input,
            search_input,
            selected_category: None,
            show_hidden: false,
            editing_category: None,
            message: None,
        }
    })
    .into()
}

struct LauncherPanel {
    _search_subscription: Subscription,
    _entry_subscription: Subscription,
    _icon_task: Task<()>,
    _service_task: Task<()>,
    add_program_task: Option<Task<()>>,
    icons: ReadModel<Vec<std::path::PathBuf>, crate::icons::IconSet>,
    surfaces: Arc<UiSurfaceState>,
    renders: u64,
    focus_handle: FocusHandle,
    entry_name: Entity<InputState>,
    selected_entry: Option<i64>,
    draft_category: i64,
    draft_hidden: bool,
    sort_by: SortBy,
    descending: bool,
    service: Arc<LauncherService>,
    snapshot: LauncherSnapshot,
    name_input: Entity<InputState>,
    search_input: Entity<InputState>,
    selected_category: Option<i64>,
    show_hidden: bool,
    editing_category: Option<i64>,
    message: Option<String>,
}

impl LauncherPanel {
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.snapshot = self.service.snapshot();
        if self
            .selected_entry
            .is_some_and(|id| !self.snapshot.entries.iter().any(|e| e.id == id))
        {
            self.selected_entry = None;
        }
        cx.notify();
    }

    fn report(&mut self, result: Result<(), crate::LauncherError>, cx: &mut Context<Self>) {
        self.message = result.err().map(|error| error.to_string());
        self.refresh(cx);
    }

    fn save_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value().to_string();
        let result = match self.editing_category {
            Some(id) => self.service.rename_category(id, &name),
            None => self.service.add_category(&name),
        };
        if result.is_ok() {
            self.editing_category = None;
            self.name_input
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.report(result, cx);
    }

    fn start_rename_category(
        &mut self,
        category: &LauncherCategory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editing_category = Some(category.id);
        self.name_input
            .update(cx, |input, cx| input.set_value(&category.name, window, cx));
        cx.notify();
    }

    fn select_entry(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
        if self.selected_entry == Some(id) {
            return;
        }
        if let Some(entry) = self
            .snapshot
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned()
        {
            self.selected_entry = Some(id);
            self.draft_category = entry.category_id;
            self.draft_hidden = entry.hidden;
            self.entry_name.update(cx, |input, cx| {
                input.set_value(&entry.display_name, window, cx)
            });
            cx.notify();
        }
    }

    fn save_entry(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.selected_entry {
            let name = self.entry_name.read(cx).value().to_string();
            let result =
                self.service
                    .update_entry(id, &name, self.draft_category, self.draft_hidden);
            self.report(result, cx);
        }
    }

    fn request_icons(&mut self) {
        let mut paths = self
            .snapshot
            .entries
            .iter()
            .map(|e| e.launch_path.clone())
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        if self.icons.key() == Some(&paths) {
            return;
        }
        let service = Arc::clone(&self.service);
        self.icons
            .request(paths.clone(), move || service.read_icons(paths));
    }

    fn add_program(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.add_program_task.is_some() {
            return;
        }
        let title = "添加程序或快捷方式".to_string();
        let service = Arc::clone(&self.service);
        self.add_program_task = Some(cx.spawn_in(window, async move |this, cx| {
            // Native Windows dialogs must run asynchronously so GPUI's main
            // thread remains available to process window and modal messages.
            let selected = rfd::AsyncFileDialog::new()
                .set_title(&title)
                .add_filter("程序和快捷方式", &["exe", "lnk", "bat", "cmd"])
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf());
            let _ = cx.update(|window, app| {
                let _ = this.update(app, |view, cx| {
                    view.add_program_task = None;
                    if let Some(path) = selected {
                        let result = service.add_manual(&path);
                        if result.is_ok() {
                            // Manual entries default to Favorites. Leave any
                            // category/hidden/search filter and select the new
                            // or already-discovered item so it is immediately
                            // visible after the picker closes.
                            view.selected_category = None;
                            view.show_hidden = false;
                            view.search_input
                                .update(cx, |input, cx| input.set_value("", window, cx));
                            let path = path.canonicalize().unwrap_or(path);
                            if let Some(id) = view
                                .service
                                .snapshot()
                                .entries
                                .iter()
                                .find(|entry| entry.launch_path == path)
                                .map(|entry| entry.id)
                            {
                                view.select_entry(id, window, cx);
                            }
                        }
                        view.report(result, cx);
                    } else {
                        cx.notify();
                    }
                });
            });
        }));
        cx.notify();
    }

    fn visible_entries(&self, search: &str) -> Vec<LauncherEntry> {
        let search = search.trim().to_lowercase();
        self.snapshot
            .entries
            .iter()
            .filter(|entry| {
                let category_match = self
                    .selected_category
                    .is_none_or(|id| entry.category_id == id);
                let search_match = search.is_empty()
                    || entry.display_name.to_lowercase().contains(&search)
                    || entry
                        .launch_path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&search);
                category_match && (self.show_hidden == entry.hidden) && search_match
            })
            .cloned()
            .collect()
    }
}

impl Drop for LauncherPanel {
    fn drop(&mut self) {
        let _ = self.surfaces.set_value("launcher.ui", "null");
    }
}
