//! Bottom drawer and status button. The user-requested addition preserves the v2 main layout.
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
#[path = "view/selection.rs"]
mod selection;
use crate::DebugService;
use fileatlas_ui_bridge::{UiViewContribution, components::WorkbenchStyle as _, i18n::tr, theme};
use gpui_kit::{
    assets::IconName,
    base::{Disableable, StyledExt},
    component::{
        button::ButtonVariants,
        input::{InputEvent, InputState},
        menu::{DropdownMenu, PopupMenuItem},
        scroll::ScrollableElement,
    },
    *,
};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};

pub fn contributions(service: &Arc<DebugService>) -> Vec<UiViewContribution> {
    [false, true]
        .into_iter()
        .map(|panel| {
            let service = Arc::downgrade(service);
            UiViewContribution::new(
                if panel {
                    "debug/drawer"
                } else {
                    "debug/button"
                },
                if panel {
                    "workspace.bottom"
                } else {
                    "workspace.debug-status"
                },
                100,
                move |window, cx| {
                    cx.new(|cx| DebugView::new(service.clone(), panel, window, cx))
                        .into()
                },
            )
        })
        .collect()
}
struct DebugView {
    service: Weak<DebugService>,
    panel: bool,
    search: Entity<InputState>,
    _subscription: Subscription,
    focus_handle: FocusHandle,
    exporting: bool,
    drag: Option<selection::Drag>,
    page_ids: Vec<u64>,
    page_latest: u64,
    row_bounds: Arc<std::sync::Mutex<std::collections::HashMap<u64, Bounds<Pixels>>>>,
    list_bounds: Arc<std::sync::Mutex<Option<Bounds<Pixels>>>>,
}
impl DebugView {
    fn new(
        service: Weak<DebugService>,
        panel: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Search logs")));
        let weak = service.clone();
        let subscription = cx.subscribe(&search, move |_, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(s) = weak.upgrade() {
                    let _ = s.control("search", &input.read(cx).value().to_string());
                }
            }
        });
        let weak = service.clone();
        let mut revision = fileatlas_ui_bridge::refresh::ChangeGate::default();
        fileatlas_ui_bridge::refresh::poll(cx, Duration::from_millis(250), move |view, cx| {
            let Some(service) = weak.upgrade() else {
                return fileatlas_ui_bridge::refresh::PollResult::until(false, false);
            };
            if panel && let Some(action) = service.take_action() {
                view.perform_action(&action, cx);
            }
            revision.changed(service.view_revision(panel)).into()
        });
        Self {
            service,
            panel,
            search,
            _subscription: subscription,
            focus_handle: cx.focus_handle(),
            exporting: false,
            drag: None,
            page_ids: Vec::new(),
            page_latest: 0,
            row_bounds: Default::default(),
            list_bounds: Default::default(),
        }
    }
}
fn control(
    service: &Arc<DebugService>,
    action: &'static str,
    value: String,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let weak = Arc::downgrade(service);
    move |_, _, cx| {
        if let Some(s) = weak.upgrade() {
            let _ = s.control(action, &value);
            cx.refresh_windows();
        }
    }
}
impl Render for DebugView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(s) = self.service.upgrade() else {
            return div();
        };
        let ui = s.ui_state();
        if !self.panel {
            return div().flex_shrink_0().child(
                fileatlas_ui_bridge::components::button("debug-toggle")
                    .ghost()
                    .toggled(ui.open)
                    .label(tr("Debug"))
                    .on_click(control(&s, "toggle", String::new())),
            );
        }
        if !ui.open {
            return div();
        }
        if self.search.read(cx).value().as_str() != ui.filter.search {
            self.search.update(cx, |input, cx| {
                input.set_value(ui.filter.search.clone(), window, cx)
            });
        }
        let page = s.query(&ui.filter);
        let category_s = s.clone();
        let level_s = s.clone();
        let size_s = s.clone();
        let categories = page.categories.clone();
        let category = fileatlas_ui_bridge::components::button("debug-category")
            .ghost()
            .label(format!(
                "{}: {}",
                tr("Category"),
                if ui.filter.category.is_empty() {
                    tr("All")
                } else {
                    ui.filter.category.clone()
                }
            ))
            .dropdown_caret(true)
            .dropdown_menu(move |mut menu, _, _| {
                menu = menu.max_h(px(320.)).scrollable(true);
                for value in std::iter::once(String::new()).chain(categories.clone()) {
                    let label = if value.is_empty() {
                        tr("All")
                    } else {
                        value.clone()
                    };
                    menu = menu.item(PopupMenuItem::new(label).on_click(control(
                        &category_s,
                        "category",
                        value,
                    )));
                }
                menu
            });
        let level = fileatlas_ui_bridge::components::button("debug-level")
            .ghost()
            .label(if ui.filter.level.is_empty() {
                tr("All levels")
            } else {
                ui.filter.level.clone()
            })
            .dropdown_caret(true)
            .dropdown_menu(move |mut menu, _, _| {
                for value in ["", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"] {
                    menu = menu.item(
                        PopupMenuItem::new(if value.is_empty() {
                            tr("All levels")
                        } else {
                            value.into()
                        })
                        .on_click(control(
                            &level_s,
                            "level",
                            value.into(),
                        )),
                    );
                }
                menu
            });
        let sizes = fileatlas_ui_bridge::components::button("debug-page-size")
            .ghost()
            .label(format!("{} / {}", page.page_size, tr("page")))
            .dropdown_caret(true)
            .dropdown_menu(move |mut menu, _, _| {
                for value in [25, 50, 100, 200] {
                    menu = menu.item(PopupMenuItem::new(value.to_string()).on_click(control(
                        &size_s,
                        "page_size",
                        value.to_string(),
                    )));
                }
                menu
            });
        let toolbar = div()
            .id("debug-toolbar")
            .h_flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(theme::space_standard()))
            .overflow_x_scroll()
            .child(div().workbench_title().child(tr("Runtime diagnostics")))
            .child(category)
            .child(level)
            .child(
                div()
                    .w(px(theme::CONTROL_WIDTH_STANDARD))
                    .flex_shrink_0()
                    .child(fileatlas_ui_bridge::components::text_input(&self.search)),
            )
            .child(
                fileatlas_ui_bridge::components::button("debug-pause")
                    .ghost()
                    .label(tr(if ui.filter.through_id.is_some() {
                        "Resume live"
                    } else {
                        "Pause view"
                    }))
                    .on_click(control(
                        &s,
                        if ui.filter.through_id.is_some() {
                            "follow"
                        } else {
                            "pause"
                        },
                        String::new(),
                    )),
            )
            .child(
                fileatlas_ui_bridge::components::button("debug-clear")
                    .ghost()
                    .label(tr("Clear logs"))
                    .on_click(control(&s, "clear", String::new())),
            )
            .child(self.selection_controls(&s, cx))
            .child(
                fileatlas_ui_bridge::components::button("debug-close")
                    .ghost()
                    .label(tr("Close"))
                    .on_click(control(&s, "close", String::new())),
            );
        let mut rows = div().v_flex().min_w_0().w_full();
        self.page_ids = page.records.iter().map(|r| r.id).collect();
        self.row_bounds
            .lock()
            .unwrap()
            .retain(|id, _| self.page_ids.contains(id));
        for r in &page.records {
            let row_bounds = Arc::clone(&self.row_bounds);
            let id = r.id;
            let color = match r.level.as_str() {
                "ERROR" => theme::danger_text(),
                "WARN" => theme::warning_text(),
                _ => theme::text_secondary(),
            };
            rows = rows.child(
                div()
                    .id(ElementId::Name(format!("debug-record-{}", r.id).into()))
                    .relative()
                    .h_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .py(px(fileatlas_ui_bridge::theme::space_compact()))
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Bottom)
                    .cursor_pointer()
                    .workbench_selectable_surface(ui.selected_ids.contains(&r.id))
                    .child(
                        div()
                            .w(px(91.))
                            .flex_shrink_0()
                            .child(r.time.get(11..23).unwrap_or(&r.time).to_owned()),
                    )
                    .child(
                        div()
                            .w(px(47.))
                            .flex_shrink_0()
                            .workbench_foreground(
                                fileatlas_ui_bridge::components::ForegroundRole::Custom(color),
                            )
                            .child(r.level.clone()),
                    )
                    .child(
                        div()
                            .w(px(110.))
                            .flex_shrink_0()
                            .truncate()
                            .child(r.category.clone()),
                    )
                    .child(
                        div()
                            .w(px(180.))
                            .flex_shrink_0()
                            .truncate()
                            .child(r.action.clone()),
                    )
                    .child(
                        div().w(px(85.)).flex_shrink_0().child(
                            r.duration_ms
                                .map(|v| format!("{v:.3} ms"))
                                .unwrap_or_else(|| "—".into()),
                        ),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(r.message.clone()))
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                row_bounds.lock().unwrap().insert(id, bounds);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            );
        }
        if page.records.is_empty() {
            rows = rows.child(
                div()
                    .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                    .workbench_assistant()
                    .child(tr("No matching logs")),
            );
        }
        let selected = ui.selected.and_then(|id| s.record_by_id(id));
        let mut panel = div()
            .v_flex()
            .w_full()
            .h(px(
                (f32::from(window.bounds().size.height) * 0.46).clamp(220., 380.)
            ))
            .flex_shrink_0()
            .min_h_0()
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Panel)
            .workbench_assistant()
            .px(px(fileatlas_ui_bridge::theme::space_standard()))
            .py(px(fileatlas_ui_bridge::theme::space_compact()))
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(toolbar)
            .child(
                div()
                    .h_flex()
                    .flex_shrink_0()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .px(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_body()
                    .children(
                        [
                            (91., "Time"),
                            (47., "Level"),
                            (110., "Category"),
                            (180., "Action"),
                            (85., "Duration"),
                        ]
                        .map(|(width, label)| {
                            div()
                                .w(px(width))
                                .flex_shrink_0()
                                .workbench_body()
                                .child(tr(label))
                        }),
                    )
                    .child(div().flex_1().workbench_body().child(tr("Details"))),
            )
            .child(self.selectable_list(rows, page.latest_id, cx));
        if let Some(message) = ui.feedback {
            panel = panel.child(div().workbench_assistant().child(message));
        }
        if let Some(r) = selected {
            panel = panel.child(
                div()
                    .id("debug-detail")
                    .max_h(px(88.))
                    .flex_shrink_0()
                    .overflow_y_scrollbar()
                    .p(px(fileatlas_ui_bridge::theme::space_standard()))
                    .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::Card)
                    .child(format!(
                        "#{}  {}  {} {}  {}  elapsed={:.3} ms duration={:?} ms  span={:?} parent={:?}  {}\n{}",
                        r.id, r.time, r.level, r.action, r.target, r.elapsed_ms, r.duration_ms, r.span_id, r.parent_span_id, r.thread, r.message
                    )),
            );
        }
        panel.child(
            div()
                .h_flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(div().flex_1().child(format!(
                    "{} {} · {} {} · {} {}",
                    page.total,
                    tr("matched"),
                    page.retained,
                    tr("retained"),
                    page.evicted,
                    tr("evicted")
                )))
                .child(sizes)
                .child(
                    fileatlas_ui_bridge::components::button("debug-prev")
                        .ghost()
                        .icon(theme::small_icon(IconName::ChevronLeft))
                        .disabled(page.page == 0)
                        .on_click(control(&s, "page", page.page.saturating_sub(1).to_string())),
                )
                .child(div().workbench_assistant().child(format!(
                    "{} / {}",
                    page.page + 1,
                    page.pages
                )))
                .child(
                    fileatlas_ui_bridge::components::button("debug-next")
                        .ghost()
                        .icon(theme::small_icon(IconName::ChevronRight))
                        .disabled(page.page + 1 >= page.pages)
                        .on_click(control(&s, "page", (page.page + 1).to_string())),
                ),
        )
    }
}
