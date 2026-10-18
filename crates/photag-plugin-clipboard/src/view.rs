use crate::{
    ClipboardService, PANEL,
    content::{ContentKind, classify},
};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;
use photag_plugin_database::{ClipboardCategoryRecord, ClipboardHistoryRecord};
use photag_ui_bridge::{UiSurfaceState, UiViewContribution, theme};
use std::{collections::HashMap, sync::Arc};
mod history;
mod inspector;
mod sidebar;
mod style;
use style::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    All,
    Favorites,
    Unclassified,
    Kind(ContentKind),
    Group(i64),
}
impl Filter {
    fn key(self) -> String {
        match self {
            Self::All => "all".into(),
            Self::Favorites => "favorites".into(),
            Self::Unclassified => "unclassified".into(),
            Self::Kind(kind) => kind.key().into(),
            Self::Group(id) => format!("group:{id}"),
        }
    }
}

pub(super) fn sidebar_button(
    panels: Arc<photag_ui_bridge::panel_catalog::PanelCatalog>,
) -> UiViewContribution {
    UiViewContribution::new(
        "clipboard-workbench/sidebar-button-view",
        "navigation.launcher-shortcuts",
        5,
        move |_, app| {
            let panels = Arc::clone(&panels);
            app.new(|_| ClipboardButton { panels }).into()
        },
    )
}
struct ClipboardButton {
    panels: Arc<photag_ui_bridge::panel_catalog::PanelCatalog>,
}
impl Render for ClipboardButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("clipboard-open-sidebar")
            .v_flex()
            .w(px(42.))
            .h(px(54.))
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(6.))
            .text_xs()
            .text_color(rgb(theme::TEXT_SECONDARY))
            .hover(|s| s.bg(rgba(theme::FILL_HOVER)))
            .on_click(cx.listener(|this, _, _, _| {
                if let Err(error) = this.panels.open(PANEL) {
                    tracing::error!(%error, "could not open clipboard workbench");
                }
            }))
            .child(icon(IconName::Clipboard, 18.))
            .child("剪贴板")
    }
}

pub(super) fn panel(
    service: Arc<ClipboardService>,
    surfaces: Arc<UiSurfaceState>,
    window: &mut Window,
    app: &mut App,
) -> AnyView {
    let items = service.snapshot().unwrap_or_default();
    let categories = service.categories().unwrap_or_default();
    let selected = items.first().map(|item| item.id);
    let initial = items
        .first()
        .filter(|item| item.image_path.is_none())
        .map(|item| item.content.clone())
        .unwrap_or_default();
    let search = app.new(|cx| InputState::new(window, cx).placeholder("搜索内容"));
    let editor = app.new(|cx| {
        let mut state = TextareaState::new(window, cx).placeholder("输入或编辑文本内容…");
        state.set_value(initial, window, cx);
        state
    });
    let category_name = app.new(|cx| InputState::new(window, cx).placeholder("分组名称"));
    app.new(|cx| {
        let wake_task = photag_ui_bridge::refresh::listen(
            cx,
            service.wake(),
            |view: &mut ClipboardPanel, _| {
                if view.service.revision() == view.revision {
                    return false;
                }
                view.sync();
                true
            },
        );
        let search_sub = cx.subscribe_in(
            &search,
            window,
            |this: &mut ClipboardPanel, _, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.set_filter(this.filter, window, cx);
                }
            },
        );
        let category_sub = cx.subscribe(&category_name, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let editor_sub = cx.subscribe(&editor, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let kinds = items
            .iter()
            .map(|item| (item.id, item_kind(item)))
            .collect();
        ClipboardPanel {
            revision: service.revision(),
            capture_paused: service.is_paused(),
            service,
            surfaces,
            items,
            categories,
            kinds,
            search,
            editor,
            category_name,
            selected,
            new_group: None,
            filter: Filter::All,
            category_form: false,
            rename_group: None,
            confirm_group_delete: false,
            confirm_clear: false,
            confirm_favorite_delete: None,
            message: None,
            state_gate: Default::default(),
            _wake_task: wake_task,
            _subscriptions: vec![search_sub, category_sub, editor_sub],
        }
    })
    .into()
}

struct ClipboardPanel {
    service: Arc<ClipboardService>,
    surfaces: Arc<UiSurfaceState>,
    items: Vec<ClipboardHistoryRecord>,
    categories: Vec<ClipboardCategoryRecord>,
    kinds: HashMap<i64, ContentKind>,
    search: Entity<InputState>,
    editor: Entity<TextareaState>,
    category_name: Entity<InputState>,
    selected: Option<i64>,
    new_group: Option<i64>,
    filter: Filter,
    category_form: bool,
    rename_group: Option<i64>,
    confirm_group_delete: bool,
    capture_paused: bool,
    confirm_clear: bool,
    confirm_favorite_delete: Option<i64>,
    message: Option<String>,
    revision: u64,
    state_gate: photag_ui_bridge::refresh::ChangeGate<String>,
    _wake_task: gpui::Task<()>,
    _subscriptions: Vec<Subscription>,
}
impl ClipboardPanel {
    fn sync(&mut self) {
        if let Ok(items) = self.service.snapshot() {
            self.items = items;
        }
        if let Ok(categories) = self.service.categories() {
            self.categories = categories;
        }
        self.kinds = self
            .items
            .iter()
            .map(|item| (item.id, item_kind(item)))
            .collect();
        self.capture_paused = self.service.is_paused();
        self.revision = self.service.revision();
        if !self.items.iter().any(|item| Some(item.id) == self.selected) {
            self.selected = None;
        }
        if let Filter::Group(id) = self.filter {
            if !self.categories.iter().any(|group| group.id == id) {
                self.filter = Filter::Unclassified;
            }
        }
    }
    fn matches(&self, item: &ClipboardHistoryRecord, query: &str) -> bool {
        let filter_matches = match self.filter {
            Filter::All => true,
            Filter::Favorites => item.pinned,
            Filter::Unclassified => item.category_id.is_none(),
            Filter::Group(id) => item.category_id == Some(id),
            Filter::Kind(kind) => self.kinds.get(&item.id) == Some(&kind),
        };
        filter_matches
            && (query.is_empty()
                || item.content.to_lowercase().contains(query)
                || item
                    .image_path
                    .as_deref()
                    .is_some_and(|path| path.to_lowercase().contains(query)))
    }
    fn select(
        &mut self,
        item: &ClipboardHistoryRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirm_favorite_delete = None;
        self.selected = Some(item.id);
        self.editor.update(cx, |state, cx| {
            state.set_value(
                if item.image_path.is_some() {
                    ""
                } else {
                    &item.content
                },
                window,
                cx,
            )
        });
        self.message = None;
        cx.notify();
    }
    fn request_favorite_removal(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self
            .items
            .iter()
            .find(|item| item.id == id && item.pinned)
            .cloned()
        {
            self.select(&item, window, cx);
            self.confirm_favorite_delete = Some(id);
            cx.notify();
        }
    }
    fn set_filter(&mut self, filter: Filter, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_favorite_delete = None;
        self.filter = filter;
        self.confirm_group_delete = false;
        let query = self.search.read(cx).value().trim().to_lowercase();
        let first = self
            .items
            .iter()
            .find(|item| self.matches(item, &query))
            .cloned();
        if let Some(item) = first {
            self.select(&item, window, cx);
        } else {
            self.selected = None;
            self.editor
                .update(cx, |state, cx| state.set_value("", window, cx));
            cx.notify();
        }
    }
    fn finish(&mut self, result: Result<(), String>, message: &str, cx: &mut Context<Self>) {
        self.message = Some(match result {
            Ok(()) => {
                self.sync();
                message.to_owned()
            }
            Err(error) => error,
        });
        cx.notify();
    }
    fn group_name(&self, id: Option<i64>) -> String {
        self.categories
            .iter()
            .find(|group| Some(group.id) == id)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "未分类".into())
    }
    fn heading(&self) -> String {
        match self.filter {
            Filter::All => "全部历史".into(),
            Filter::Favorites => "本地收藏".into(),
            Filter::Unclassified => "未分类".into(),
            Filter::Kind(kind) => kind.label().into(),
            Filter::Group(id) => self.group_name(Some(id)),
        }
    }
    fn publish(&mut self, ids: &[i64], cx: &Context<Self>) {
        let state = serde_json::json!({
            "confirm_favorite_delete": self.confirm_favorite_delete,
            "selected": self.selected, "filter": self.filter.key(), "visible_ids": ids,
            "category_form": self.category_form, "rename_group": self.rename_group,
            "category_name_length": self.category_name.read(cx).value().chars().count(),
            "editor_length": self.editor.read(cx).value().chars().count(), "message": self.message,
        })
        .to_string();
        if self.state_gate.changed(state.clone()) {
            let _ = self.surfaces.set_value("clipboard.ui", &state);
        }
    }
}
fn item_kind(item: &ClipboardHistoryRecord) -> ContentKind {
    if item.image_path.is_some() || item.content_type == "image" {
        ContentKind::Image
    } else {
        classify(&item.content)
    }
}
impl Drop for ClipboardPanel {
    fn drop(&mut self) {
        let _ = self.surfaces.set_value("clipboard.ui", "null");
    }
}
impl Render for ClipboardPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value().trim().to_lowercase();
        let ids = self
            .items
            .iter()
            .filter(|item| self.matches(item, &query))
            .map(|item| item.id)
            .collect::<Vec<_>>();
        self.publish(&ids, cx);
        div()
            .h_flex()
            .size_full()
            .min_h_0()
            .gap(px(theme::PANEL_GAP))
            .p(px(theme::PANEL_GAP))
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .text_size(px(theme::FONT_SIZE_BODY))
            .child(self.sidebar(cx))
            .child(self.history(&ids, cx))
            .child(self.inspector(cx))
    }
}
