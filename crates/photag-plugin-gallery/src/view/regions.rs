//! Projections of the single Gallery entity into the source window bands.
//! Source: main.rs:1007-1277 and ui/filterbar.rs:15-821.
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement as _;
use photag_ui_bridge::components::WorkbenchStyle as _;

#[derive(Clone, Copy)]
enum Region {
    Address,
    Search,
    Size,
    Filters,
    ViewModes,
    ViewOptions,
    Status,
    Tools,
    Modal,
    Inspector,
}

struct GalleryRegion {
    gallery: Entity<GalleryView>,
    region: Region,
    _subscription: Subscription,
}

impl Render for GalleryRegion {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.gallery.update(cx, |view, cx| {
            view.apply_pending_workspace_inputs(window, cx);
            match self.region {
                Region::Address => view.render_column_address(window, cx),
                Region::Search => view.render_search(window, cx),
                Region::Size => div()
                    .h_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new("gallery-open-folder")
                            .ghost()
                            .compact()
                            .icon(theme::icon(gpui_kit::assets::IconName::FolderOpen))
                            .tooltip(photag_ui_bridge::i18n::tr("Open folder"))
                            .on_click(cx.listener(|view, _, _, cx| view.pick_browse_location(cx))),
                    )
                    .child(directory_browser::refresh_button(cx))
                    .into_any_element(),
                Region::Filters => view.render_filters(cx),
                Region::ViewModes => view_controls::modes(view, cx).into_any_element(),
                Region::ViewOptions => view_controls::options(view, cx).into_any_element(),
                Region::Status => view.render_status(cx),
                Region::Tools => view.render_tools(cx),
                Region::Modal => view.render_modal(window, cx),
                Region::Inspector => view.render_inspector(window, cx),
            }
        })
    }
}

pub(super) fn column_contributions(
    create: Arc<dyn Fn(&mut Window, &mut App) -> Entity<GalleryView> + Send + Sync>,
    column: usize,
) -> Vec<UiViewContribution> {
    let grid = create.clone();
    let mut views = vec![UiViewContribution::new(
        format!("gallery/column-{column}-content"),
        format!("gallery.column.{column}.content"),
        0,
        move |window, cx| grid(window, cx).into(),
    )];
    let address = create.clone();
    views.push(UiViewContribution::new(
        format!("gallery/column-{column}-address"),
        format!("gallery.column.{column}.address"),
        0,
        move |window, cx| {
            let gallery = address(window, cx);
            cx.new(|cx| {
                let subscription = cx.observe(&gallery, |_, _, cx| cx.notify());
                GalleryRegion {
                    gallery,
                    region: Region::Address,
                    _subscription: subscription,
                }
            })
            .into()
        },
    ));
    for (region, id, slot) in [
        (Region::Search, "search", "search"),
        (Region::Size, "size", "size"),
        (Region::Filters, "filters", "filters"),
        (Region::ViewModes, "view-modes", "view-modes"),
        (Region::ViewOptions, "view-options", "view-options"),
        (Region::Status, "status", "status"),
        (Region::Tools, "tools", "tools"),
        (Region::Modal, "modal", "modal"),
        (Region::Inspector, "inspector", "inspector"),
    ] {
        let create = create.clone();
        views.push(UiViewContribution::new(
            format!("gallery/column-{column}-{id}"),
            format!("command.column.{column}.{slot}"),
            0,
            move |window, cx| {
                let gallery = create(window, cx);
                cx.new(|cx| {
                    let subscription = cx.observe(&gallery, |_, _, cx| cx.notify());
                    GalleryRegion {
                        gallery,
                        region,
                        _subscription: subscription,
                    }
                })
                .into()
            },
        ));
    }
    views
}

impl GalleryView {
    pub(super) fn refresh_locale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let revision = photag_ui_bridge::i18n::revision();
        if self.locale_revision == revision {
            return;
        }
        self.locale_revision = revision;
        for (input, label) in [
            (&self.search, "Search files"),
            (&self.rename_input, "New filename"),
            (&self.tag_picker_search, "Find tags to assign"),
        ] {
            input.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr(label), window, cx)
            });
        }
        for (input, label) in self.bulk_inputs.iter().zip([
            "Older than days",
            "Newer than days",
            "Min rating",
            "Max rating",
            "Name contains",
            "Extension",
            "Size min MB",
            "Size max MB",
        ]) {
            input.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr(label), window, cx)
            });
        }
    }

    fn render_search(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_locale(window, cx);
        let search_field = div()
            .w_full()
            .min_w(px(220.))
            .max_w(px(280.))
            .h(px(32.))
            .child(photag_ui_bridge::components::search_input(&self.search));
        let search_bar = div()
            .h_flex()
            .items_center()
            .gap(px(6.))
            .child(search_field)
            .child(self.render_search_location(cx))
            .child(filter_row::search_scope_popover(self, cx));
        div()
            .h_flex()
            .flex_1()
            .min_w_0()
            .justify_center()
            .child(search_bar)
            .into_any_element()
    }

    fn render_filters(&mut self, cx: &mut Context<Self>) -> AnyElement {
        filter_row::render(self, cx)
    }

    fn render_status(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let n = self.selection.selected_media_ids.len();
        let count = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.skeletons.len())
            .unwrap_or(0);
        let count_label = self
            .snapshot
            .as_ref()
            .ok()
            .filter(|s| s.browser.active)
            .map(|s| {
                photag_ui_bridge::i18n::tr_args(
                    "{folders} folders · {files} files",
                    &[
                        ("folders", s.browser.directories.len().to_string()),
                        ("files", count.to_string()),
                    ],
                )
            })
            .unwrap_or_else(|| {
                photag_ui_bridge::i18n::tr_args("{count} items", &[("count", count.to_string())])
            });
        let mut row = div()
            .h_flex()
            .w_full()
            .items_center()
            .gap(px(8.))
            .text_size(px(theme::FONT_SIZE_SMALL))
            .child(
                div()
                    .text_color(rgb(theme::text_secondary()))
                    .child(count_label),
            )
            .child(div().text_color(rgb(theme::text_muted())).child("·"))
            .child(div().text_color(rgb(theme::text_secondary())).child(
                photag_ui_bridge::i18n::tr_args(
                    "{count} selected",
                    &[(
                        "count",
                        (n + usize::from(self.selected_directory.is_some())).to_string(),
                    )],
                ),
            ));
        if let Some(error) = self
            .snapshot
            .as_ref()
            .ok()
            .and_then(|s| s.browser.error.as_ref())
        {
            row = row.child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_color(rgb(theme::danger_text()))
                    .child(error.clone()),
            );
        }
        let directory_status = self.service.directory_status();
        if directory_status.2 {
            row = row
                .child(gpui_kit::component::spinner::Spinner::new())
                .child(div().child(format!(
                    "{}: {}",
                    photag_ui_bridge::i18n::tr("Loading folder…"),
                    directory_status.3
                )))
                .child(
                    Button::new("directory-cancel")
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Cancel"))
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.service.cancel_directory();
                            cx.notify();
                        })),
                );
        }
        if let Some(progress) = self.service.scan_progress() {
            let fraction = progress.scanned as f32 / progress.total.max(1) as f32;
            row = row
                .child(gpui_kit::component::spinner::Spinner::new())
                .child(
                    div()
                        .text_color(rgb(theme::text_secondary()))
                        .child(photag_ui_bridge::i18n::tr(&progress.phase)),
                )
                .child(
                    gpui_kit::component::progress::Progress::new("gallery-scan-progress")
                        .value(fraction.clamp(0., 1.) * 100.)
                        .w(px(180.)),
                );
        }
        row.into_any_element()
    }

    fn render_tools(&mut self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .v_flex()
            .gap(px(2.))
            .w_full()
            .child(
                tool_button(
                    "gallery-thumbnails",
                    if self.service.thumbnails_enabled() {
                        "Hide thumbnails"
                    } else {
                        "Show thumbnails"
                    },
                    gpui_kit::assets::IconName::Image,
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    view.service
                        .set_thumbnails_enabled(!view.service.thumbnails_enabled());
                    view.refresh_index();
                    view.persist_filter_session();
                    cx.notify();
                })),
            )
            .child(
                tool_button(
                    "gallery-missing-records",
                    "Relink missing files",
                    gpui_kit::assets::IconName::Folder,
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    view.show_records = true;
                    view.record_read_revision += 1;
                    view.records_page = 0;
                    cx.notify();
                })),
            )
            .child(
                tool_button(
                    "gallery-bulk-open",
                    "Bulk actions…",
                    gpui_kit::assets::IconName::ListChecks,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    bulk_actions::reset_inputs(this, window, cx);
                    this.bulk_source = BulkActionSource::WholeLibrary;
                    this.bulk_action = Some(BulkActionKind::Delete);
                    this.confirm_bulk_action = false;
                    this.confirm_bulk_action_ids = None;
                    cx.notify();
                })),
            )
            .child(
                tool_button(
                    "gallery-statistics",
                    "Statistics",
                    gpui_kit::assets::IconName::ChartColumn,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.show_statistics = true;
                    cx.notify();
                })),
            )
            .child(
                tool_button(
                    "gallery-export-metadata",
                    "Export metadata",
                    gpui_kit::assets::IconName::File,
                )
                .on_click(
                    cx.listener(|this, _, _, _| this.dispatch_command("metadata-export/export")),
                ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(rgba(theme::separator_light()))
                    .mt(px(6.))
                    .pt(px(6.))
                    .text_size(px(theme::FONT_SIZE_SMALL))
                    .text_color(rgb(theme::text_muted()))
                    .child(photag_ui_bridge::i18n::tr("View")),
            )
            .children(
                photag_plugin_grouping::GroupingMode::ALL
                    .into_iter()
                    .map(|mode| {
                        tool_button(
                            mode.key(),
                            mode.label(),
                            gpui_kit::assets::IconName::CalendarRange,
                        )
                        .toggled(self.grouping_mode == mode)
                        .on_click(cx.listener(move |this, _, _, cx| this.set_grouping(mode, cx)))
                    }),
            )
            .into_any_element()
    }

    fn render_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_locale(window, cx);
        let mut root = div()
            .absolute()
            .inset_0()
            .w(window.bounds().size.width)
            .h(window.bounds().size.height);
        if self.show_records || self.show_statistics || self.bulk_action.is_some() {
            let showing_statistics = self.show_statistics;
            let width = (f32::from(window.bounds().size.width) - 48.)
                .min(if self.show_statistics { 800. } else { 860. });
            let available_height = (f32::from(window.bounds().size.height) - 48.).max(200.);
            let height = if showing_statistics {
                available_height.min(552.)
            } else {
                available_height
            };
            let mut card = div()
                .v_flex()
                .w(px(width))
                .h(px(height))
                .max_h(px(height))
                .min_h_0()
                .overflow_hidden()
                .workbench_section()
                .bg(rgb(theme::surface_modal()))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .h(px(44.))
                        .flex_shrink_0()
                        .px(px(14.))
                        .border_b_1()
                        .border_color(rgba(theme::separator_light()))
                        .child(
                            div()
                                .text_size(px(theme::FONT_SIZE_BODY))
                                .font_semibold()
                                .text_color(rgb(theme::text_primary()))
                                .child(photag_ui_bridge::i18n::tr(if self.show_records {
                                    "Relink missing files"
                                } else if showing_statistics {
                                    "Media Statistics"
                                } else {
                                    "Bulk actions…"
                                })),
                        )
                        .child(
                            Button::new("gallery-modal-close")
                                .ghost()
                                .compact()
                                .icon(theme::icon(gpui_kit::assets::IconName::X))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_records = false;
                                    this.show_statistics = false;
                                    this.bulk_action = None;
                                    this.confirm_bulk_action = false;
                                    this.confirm_bulk_action_ids = None;
                                    cx.notify();
                                })),
                        ),
                )
                .px(px(14.))
                .pb(px(14.));
            if self.show_records {
                card = card.child(records::render(self, cx));
            } else if self.show_statistics {
                let stats_height = (height - 102.).max(250.);
                card = card
                    .child(statistics::render(self, stats_height, cx))
                    .child(
                        div()
                            .h_flex()
                            .justify_end()
                            .flex_shrink_0()
                            .h(px(44.))
                            .px(px(14.))
                            .border_t_1()
                            .border_color(rgba(theme::separator_light()))
                            .child(
                                Button::new("gallery-statistics-close")
                                    .ghost()
                                    .compact()
                                    .label(photag_ui_bridge::i18n::tr("Close"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_statistics = false;
                                        cx.notify();
                                    })),
                            ),
                    );
            }
            if self.bulk_action.is_some() {
                let bulk_height = (height - 58.).max(160.);
                card = card.child(bulk_actions::render(self, width - 28., bulk_height, cx));
            }
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(theme::shadow_overlay()))
                    .child(card),
            );
        }
        if self.show_tag_picker && !self.selection.selected_media_ids.is_empty() {
            let picker_width = (f32::from(window.bounds().size.width) - 32.).min(690.);
            let picker_height = (f32::from(window.bounds().size.height) - 96.).clamp(360., 430.);
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(theme::scrim()))
                    .child(
                        div()
                            .w(px(picker_width))
                            .max_w(px(690.))
                            .h(px(picker_height))
                            .max_h(px((f32::from(window.bounds().size.height) - 48.).max(360.)))
                            .overflow_hidden()
                            .child(tagging_picker::render(
                                self,
                                picker_width,
                                picker_height,
                                cx,
                            )),
                    ),
            );
        }
        if let Some(media_id) = self.media_actions_id {
            match self
                .media_actions_item
                .as_ref()
                .filter(|(id, _)| *id == media_id)
                .map(|(_, media)| media)
            {
                Some(media) => {
                    root = root.child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgba(theme::scrim()))
                            .child(card_menu::render_requested(&media, cx)),
                    );
                }
                None => self.media_actions_id = None,
            }
        }
        root = self.selection_details(root, cx);
        if let Some(status) = &self.operation_status {
            root = root.child(
                div()
                    .absolute()
                    .right(px(12.))
                    .bottom(px(48.))
                    .p_2()
                    .rounded_md()
                    .bg(rgb(theme::surface_panel()))
                    .child(status.clone()),
            );
        }
        let failures = self
            .tagging
            .as_ref()
            .map(|tagging| tagging.writeback_failures())
            .unwrap_or_default();
        if !failures.is_empty() {
            root = root.child(
                div()
                    .absolute()
                    .right(px(12.))
                    .bottom(px(56.))
                    .w(px(320.))
                    .child(
                        div()
                            .w_full()
                            .max_h(px((f32::from(window.bounds().size.height) - 80.).max(160.)))
                            .overflow_y_scrollbar()
                            .v_flex()
                            .gap_2()
                            .children(failures.into_iter().map(|failure| {
                                super::writeback_failure::render(self, failure, cx)
                            })),
                    ),
            );
        }
        root.into_any_element()
    }
}

fn tool_button(id: &'static str, label: &str, icon: gpui_kit::assets::IconName) -> Button {
    Button::new(id)
        .ghost()
        .compact()
        .w_full()
        .h(px(30.))
        .justify_start()
        .icon(theme::icon(icon))
        .label(photag_ui_bridge::i18n::tr(label))
}
