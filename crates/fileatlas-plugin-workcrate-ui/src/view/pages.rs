use super::preview::{page_preview, physical_label, pixel_label};
use super::*;
impl WorkCratePanel {
    pub(super) fn render_pages(&mut self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let mut grid = div()
            .id("workcrate-page-grid")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .h_flex()
            .flex_wrap()
            .content_start()
            .items_start()
            .gap(px(24.))
            .p(px(24.));
        if self.boards_view.is_empty() {
            grid = grid.child(
                div()
                    .text_color(rgb(theme::text_muted()))
                    .child("关联已有 VectorCraft 工程，或新增 A3 页面。"),
            );
        }
        let width = ((self.panel_content_size().0 - 96.) / 3.).max(210.) as f32;
        for card in self.boards_view.clone() {
            let id = card.board.id.clone();
            let selected = self.selected_pages.contains(&id);
            let select_id = id.clone();
            grid = grid.child(
                div()
                    .id(format!("page-card-{id}"))
                    .v_flex()
                    .w(px(width))
                    .workbench_card(selected)
                    .p_2()
                    .gap_2()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .when(self.multi_select, |el| {
                                el.child(
                                    Button::new(format!("select-page-{id}"))
                                        .when(selected, |b| b.primary())
                                        .ghost()
                                        .compact()
                                        .icon(theme::small_icon(if selected {
                                            gpui_kit::assets::IconName::SquareCheck
                                        } else {
                                            gpui_kit::assets::IconName::Square
                                        }))
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            if !view.selected_pages.remove(&select_id) {
                                                view.selected_pages.insert(select_id.clone());
                                            }
                                            cx.notify();
                                        })),
                                )
                            })
                            .child(div().flex_1().truncate().child(card.board.title.clone()))
                            .child(div().text_xs().text_color(rgb(theme::text_muted())).child(
                                format!(
                                    "{} · {}",
                                    physical_label(&card.board),
                                    pixel_label(&card.board)
                                ),
                            ))
                            .child(
                                Button::new(format!("open-page-{id}"))
                                    .ghost()
                                    .compact()
                                    .label("编辑")
                                    .on_click(
                                        cx.listener(move |view, _, _, cx| view.open_page(&id, cx)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id(format!("page-open-preview-{}", card.board.id))
                            .border_1()
                            .border_color(rgb(if selected {
                                theme::accent()
                            } else {
                                theme::separator()
                            }))
                            .child(super::tracking::control(
                                &format!("page-preview-{}", card.board.id),
                                page_preview(&card, width - 18.),
                                &self.surfaces,
                            ))
                            .on_click(cx.listener({
                                let id = card.board.id.clone();
                                move |view, _, _, cx| {
                                    if view.multi_select {
                                        if !view.selected_pages.remove(&id) {
                                            view.selected_pages.insert(id.clone());
                                        }
                                        cx.notify();
                                    } else {
                                        view.open_page(&id, cx);
                                    }
                                }
                            })),
                    ),
            );
        }
        div()
            .flex_1()
            .min_h_0()
            .v_flex()
            .child(grid)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .border_t_1()
                    .border_color(rgb(theme::separator()))
                    .flex_wrap()
                    .child(super::tracking::control(
                        "pages-multi-select",
                        Button::new("pages-multi-select")
                            .ghost()
                            .compact()
                            .label("多选")
                            .disabled(self.multi_select)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.multi_select = true;
                                cx.notify();
                            })),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "pages-select-all",
                        Button::new("pages-select-all")
                            .ghost()
                            .compact()
                            .label("全选")
                            .disabled(self.boards_view.is_empty())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.multi_select = true;
                                view.selected_pages = view
                                    .boards_view
                                    .iter()
                                    .map(|c| c.board.id.clone())
                                    .collect();
                                cx.notify();
                            })),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "pages-clear-selection",
                        Button::new("pages-clear-selection")
                            .ghost()
                            .compact()
                            .label("取消全选")
                            .disabled(self.selected_pages.is_empty())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.selected_pages.clear();
                                cx.notify();
                            })),
                        &self.surfaces,
                    ))
                    .child(super::tracking::control(
                        "pages-exit-multi-select",
                        Button::new("pages-exit-multi-select")
                            .ghost()
                            .compact()
                            .label("取消多选")
                            .disabled(!self.multi_select)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.multi_select = false;
                                view.selected_pages.clear();
                                cx.notify();
                            })),
                        &self.surfaces,
                    ))
                    .child(format!("已选 {} 页", self.selected_pages.len()))
                    .child(div().flex_1())
                    .child(
                        Button::new("export-image-format")
                            .ghost()
                            .label(if self.image_export_jpeg {
                                "JPEG"
                            } else {
                                "PNG"
                            })
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.image_export_jpeg = !view.image_export_jpeg;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("export-selected-images")
                            .outline()
                            .label("导出选中页为图片")
                            .disabled(self.busy || self.selected_pages.is_empty())
                            .on_click(cx.listener(|view, _, _, cx| view.export_catalog(true, cx))),
                    )
                    .child(
                        Button::new("export-all-pages")
                            .primary()
                            .label("导出全部为 PDF")
                            .disabled(self.busy || self.boards_view.is_empty())
                            .on_click(cx.listener(|view, _, _, cx| view.export_catalog(false, cx))),
                    ),
            )
            .into_any_element()
    }
}
