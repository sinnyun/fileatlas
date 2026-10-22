use super::preview::{page_preview, physical_label, pixel_label};
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::Selectable as _;

impl WorkCratePanel {
    pub(super) fn render_pages(&mut self, cx: &mut ViewContext<Self>) -> gpui::AnyElement {
        let width = ((self.panel_content_size().0 - 96.) / 3.).max(210.) as f32;
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
            .px(px(28.))
            .py(px(16.));
        if self.boards_view.is_empty() {
            grid = grid.child(
                div()
                    .w_full()
                    .py(px(48.))
                    .text_color(rgb(theme::text_muted()))
                    .child(if self.document.is_some() {
                        "本地没有画板目录。点击右上角“同步工程与预览”读取，或新增画布。"
                    } else {
                        "项目尚未关联 VectorCraft 工程。可关联已有工程，或新增画布。"
                    }),
            );
        }
        for (index, card) in self.boards_view.clone().into_iter().enumerate() {
            grid = grid.child(self.page_card(card, index, width, cx));
        }
        let header = self.pages_header(cx);
        div()
            .flex_1()
            .min_h_0()
            .v_flex()
            .child(header)
            .child(grid)
            .child(self.pages_export_bar(cx))
            .into_any_element()
    }

    /// Hero header from the Tier 2 draft: project title, page/selection
    /// statistics and the unified action cluster on the same top line.
    fn pages_header(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        let project = self
            .selected_project
            .as_ref()
            .and_then(|id| self.items.iter().find(|p| &p.id == id))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        div()
            .h_flex()
            .items_start()
            .gap_3()
            .px(px(28.))
            .pt(px(20.))
            .pb(px(4.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_TINY))
                            .text_color(rgb(theme::text_muted()))
                            .child(format!("TIER 2 · PAGE MATRIX · {project}")),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_size(px(theme::SETTINGS_TITLE_SIZE))
                                    .truncate()
                                    .child(project.clone()),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .rounded(px(theme::ROUND_MD))
                                    .bg(rgb(theme::selection_color()))
                                    .text_color(rgb(theme::accent()))
                                    .text_size(px(theme::FONT_SIZE_TINY))
                                    .px_2()
                                    .py(px(2.))
                                    .child("页面管理 (Tier 2)"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_SMALL))
                            .text_color(rgb(theme::text_secondary()))
                            .child(format!(
                                "VectorCraft 内部多画布 · 共 {} 个独立物料 · 已选中 {} 个",
                                self.boards_view.len(),
                                self.selected_pages.len()
                            )),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    .pt(px(6.))
                    .child(super::tracking::control(
                        "pages-multi-select",
                        fileatlas_ui_bridge::components::icon_button(
                            "pages-multi-select",
                            IconName::ListChecks,
                        )
                        .tooltip(if self.multi_select {
                            "已开启多选，点击退出多选"
                        } else {
                            "开启多选模式"
                        })
                        .selected(self.multi_select)
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.multi_select = !view.multi_select;
                            if !view.multi_select {
                                view.selected_pages.clear();
                            }
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
                    .child(self.screen_action(
                        "attach-document",
                        Some(IconName::Link2),
                        "关联工程",
                        false,
                        |v, cx| v.pick_document(cx),
                        cx,
                    ))
                    .child(self.screen_action(
                        "refresh-native",
                        Some(IconName::RefreshCw),
                        "同步工程与预览",
                        false,
                        |v, cx| v.sync_project(cx),
                        cx,
                    ))
                    .child(self.screen_action(
                        "workcrate-new-page",
                        Some(IconName::Plus),
                        "新增画布",
                        true,
                        |v, cx| v.create_board(cx),
                        cx,
                    )),
            )
    }

    /// One page card: checkbox + title + spec line, optional material band,
    /// preview and a footer with the page number and the edit action.
    fn page_card(
        &self,
        card: super::BoardCard,
        index: usize,
        width: f32,
        cx: &mut ViewContext<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = card.board.id.clone();
        let selected = self.selected_pages.contains(&id);
        let select_id = id.clone();
        let facts = [
            card.board.info.material.clone(),
            card.board.info.size.clone(),
            card.board.info.quantity.clone(),
        ]
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join("  ");
        div()
            .id(format!("page-card-{id}"))
            .v_flex()
            .w(px(width))
            .workbench_card(selected)
            .p(px(theme::SPACE_3))
            .gap_2()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_1()
                    .child(self.card_icon(
                        &format!("select-page-{id}"),
                        if selected {
                            IconName::SquareCheck
                        } else {
                            IconName::Square
                        },
                        if selected {
                            "取消选择本页"
                        } else {
                            "选择本页"
                        },
                        false,
                        cx.listener(move |view, _, _, cx| {
                            if !view.selected_pages.remove(&select_id) {
                                view.selected_pages.insert(select_id.clone());
                                view.multi_select = true;
                            }
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_semibold()
                            .text_size(px(theme::FONT_SIZE_TITLE))
                            .child(card.board.title.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(theme::FONT_SIZE_TINY))
                            .text_color(rgb(theme::text_muted()))
                            .child(format!(
                                "{} · {}",
                                physical_label(&card.board),
                                pixel_label(&card.board)
                            )),
                    ),
            )
            .child({
                let mut band = div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .rounded(px(theme::ROUND_MD))
                    .bg(rgb(theme::selection_color()))
                    .px_2()
                    .h(px(26.))
                    .text_size(px(theme::FONT_SIZE_TINY))
                    .child(
                        div()
                            .font_semibold()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(format!("{:02} {}", index + 1, card.board.title)),
                    );
                if !facts.is_empty() {
                    band = band.child(
                        div()
                            .flex_shrink_0()
                            .text_color(rgb(theme::text_secondary()))
                            .child(facts),
                    );
                }
                band
            })
            .child(
                div()
                    .id(format!("page-open-preview-{}", card.board.id))
                    .w_full()
                    .rounded(px(theme::ROUND_LG))
                    .overflow_hidden()
                    .border_1()
                    .border_color(rgb(if selected {
                        theme::accent()
                    } else {
                        theme::separator()
                    }))
                    .bg(rgb(theme::surface_faint()))
                    .cursor_pointer()
                    .child(super::tracking::control(
                        &format!("page-preview-{}", card.board.id),
                        page_preview(&card, width - 24.),
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
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .pt_1()
                    .border_t_1()
                    .border_color(rgb(theme::separator()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(theme::FONT_SIZE_TINY))
                            .text_color(rgb(theme::text_muted()))
                            .child(format!("第 {} 页", index + 1)),
                    )
                    .child(super::tracking::control(
                        &format!("open-page-{id}"),
                        Button::new(format!("open-page-{id}"))
                            .ghost()
                            .compact()
                            .label("编辑")
                            .icon(theme::small_icon(IconName::Pencil))
                            .on_click(cx.listener(move |view, _, _, cx| view.open_page(&id, cx))),
                        &self.surfaces,
                    )),
            )
    }

    /// Export bar from the draft: hint and selection count left, the format
    /// toggle and the two export actions right.
    fn pages_export_bar(&self, cx: &mut ViewContext<Self>) -> gpui::Div {
        div()
            .h_flex()
            .items_center()
            .gap_3()
            .p_3()
            .px(px(28.))
            .border_t_1()
            .border_color(rgb(theme::separator()))
            .flex_wrap()
            .child(
                div()
                    .text_size(px(theme::FONT_SIZE_TINY))
                    .text_color(rgb(theme::text_muted()))
                    .child("本地预览 · 外部编辑保存后请同步"),
            )
            .child(
                div()
                    .text_size(px(theme::FONT_SIZE_SMALL))
                    .text_color(rgb(theme::text_secondary()))
                    .child(format!("已选 {} 页", self.selected_pages.len())),
            )
            .child(div().flex_1())
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
            .child(super::tracking::control(
                "export-image-format",
                Button::new("export-image-format")
                    .ghost()
                    .compact()
                    .label(if self.image_export_jpeg {
                        "JPEG"
                    } else {
                        "PNG"
                    })
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.image_export_jpeg = !view.image_export_jpeg;
                        cx.notify();
                    })),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "export-selected-images",
                Button::new("export-selected-images")
                    .outline()
                    .compact()
                    .rounded(px(theme::ROUND_MD))
                    .label(format!("导出选中页为图片 ({})", self.selected_pages.len()))
                    .icon(theme::icon(IconName::FileImage))
                    .disabled(self.busy || self.selected_pages.is_empty())
                    .on_click(cx.listener(|view, _, _, cx| view.export_catalog(true, cx))),
                &self.surfaces,
            ))
            .child(super::tracking::control(
                "export-all-pages",
                Button::new("export-all-pages")
                    .primary()
                    .compact()
                    .rounded(px(theme::ROUND_MD))
                    .label(format!("导出全部为 PDF ({} 页)", self.boards_view.len()))
                    .disabled(self.busy || self.boards_view.is_empty())
                    .on_click(cx.listener(|view, _, _, cx| view.export_catalog(false, cx))),
                &self.surfaces,
            ))
    }
}
