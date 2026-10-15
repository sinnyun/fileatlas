//! Bottom drawer and status button. The user-requested addition preserves the v2 main layout.
use crate::DebugService;
use gpui_kit::{
    assets::IconName,
    base::{Disableable, StyledExt},
    component::{
        button::{Button, ButtonVariants},
        input::{Input, InputEvent, InputState},
        menu::{DropdownMenu, PopupMenuItem},
        scroll::ScrollableElement,
    },
    *,
};
use photag_ui_bridge::{UiViewContribution, i18n::tr, theme};
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
        let mut revision = photag_ui_bridge::refresh::ChangeGate::default();
        photag_ui_bridge::refresh::poll(cx, Duration::from_millis(250), move |_, _| {
            let Some(service) = weak.upgrade() else {
                return photag_ui_bridge::refresh::PollResult::until(false, false);
            };
            revision.changed(service.view_revision(panel)).into()
        });
        Self {
            service,
            panel,
            search,
            _subscription: subscription,
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
                Button::new("debug-toggle")
                    .ghost()
                    .compact()
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
        let category = Button::new("debug-category")
            .ghost()
            .compact()
            .label(format!(
                "{}: {} ▾",
                tr("Category"),
                if ui.filter.category.is_empty() {
                    tr("All")
                } else {
                    ui.filter.category.clone()
                }
            ))
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
        let level = Button::new("debug-level")
            .ghost()
            .compact()
            .label(format!(
                "{} ▾",
                if ui.filter.level.is_empty() {
                    tr("All levels")
                } else {
                    ui.filter.level.clone()
                }
            ))
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
        let sizes = Button::new("debug-page-size")
            .ghost()
            .compact()
            .label(format!("{} / {} ▾", page.page_size, tr("page")))
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
            .gap_1()
            .overflow_x_scroll()
            .child(div().font_semibold().child(tr("Runtime diagnostics")))
            .child(category)
            .child(level)
            .child(
                div()
                    .w(px(210.))
                    .flex_shrink_0()
                    .child(Input::new(&self.search)),
            )
            .child(
                Button::new("debug-pause")
                    .ghost()
                    .compact()
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
                Button::new("debug-clear")
                    .ghost()
                    .compact()
                    .label(tr("Clear logs"))
                    .on_click(control(&s, "clear", String::new())),
            )
            .child(
                Button::new("debug-close")
                    .ghost()
                    .compact()
                    .label(tr("Close"))
                    .on_click(control(&s, "close", String::new())),
            );
        let mut rows = div().v_flex().min_w_0().w_full();
        for r in &page.records {
            let color = match r.level.as_str() {
                "ERROR" => theme::DANGER,
                "WARN" => 0xe6b85c,
                _ => theme::TEXT_SECONDARY,
            };
            rows = rows.child(
                div()
                    .id(ElementId::Name(format!("debug-record-{}", r.id).into()))
                    .h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .py_1()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(theme::SEPARATOR))
                    .cursor_pointer()
                    .bg(rgb(if ui.selected == Some(r.id) {
                        theme::SURFACE_ELEVATED
                    } else {
                        theme::SURFACE_PANEL
                    }))
                    .hover(|s| s.bg(rgb(theme::SURFACE_CARD)))
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
                            .text_color(rgb(color))
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
                    .on_click(control(&s, "select", r.id.to_string())),
            );
        }
        if page.records.is_empty() {
            rows = rows.child(div().p_4().child(tr("No matching logs")));
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
            .border_t_1()
            .border_color(rgba(0x799bc488))
            .bg(rgb(theme::SURFACE_PANEL))
            .text_size(px(11.))
            .px_2()
            .py_1()
            .gap_1()
            .child(toolbar)
            .child(
                div()
                    .h_flex()
                    .flex_shrink_0()
                    .gap_2()
                    .px_2()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .children(
                        [
                            (91., "Time"),
                            (47., "Level"),
                            (110., "Category"),
                            (180., "Action"),
                            (85., "Duration"),
                        ]
                        .map(|(width, label)| div().w(px(width)).flex_shrink_0().child(tr(label))),
                    )
                    .child(div().flex_1().child(tr("Details"))),
            )
            .child(
                div()
                    .id("debug-records")
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(rows),
            );
        if let Some(r) = selected {
            panel = panel.child(
                div()
                    .id("debug-detail")
                    .max_h(px(88.))
                    .flex_shrink_0()
                    .overflow_y_scrollbar()
                    .p_2()
                    .bg(rgb(theme::SURFACE_CARD))
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
                .gap_2()
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
                    Button::new("debug-prev")
                        .ghost()
                        .compact()
                        .icon(theme::small_icon(IconName::ChevronLeft))
                        .disabled(page.page == 0)
                        .on_click(control(&s, "page", page.page.saturating_sub(1).to_string())),
                )
                .child(div().child(format!("{} / {}", page.page + 1, page.pages)))
                .child(
                    Button::new("debug-next")
                        .ghost()
                        .compact()
                        .icon(theme::small_icon(IconName::ChevronRight))
                        .disabled(page.page + 1 >= page.pages)
                        .on_click(control(&s, "page", (page.page + 1).to_string())),
                ),
        )
    }
}
