//! Projections of the single Gallery entity into the source window bands.
//! Source: main.rs:1007-1277 and ui/filterbar.rs:15-821.
use super::*;
use gpui_kit::base::Disableable;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

#[derive(Clone, Copy)]
enum Region {
    Search,
    Size,
    Filters,
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
        self.gallery.update(cx, |view, cx| match self.region {
            Region::Search => view.render_search(window, cx),
            Region::Size => sliders::thumbnail(view, cx).into_any_element(),
            Region::Filters => view.render_filters(cx),
            Region::Status => view.render_status(cx),
            Region::Tools => view.render_tools(cx),
            Region::Modal => view.render_modal(window, cx),
            Region::Inspector => view.render_inspector(cx),
        })
    }
}

pub(super) fn contributions(
    create: Arc<dyn Fn(&mut Window, &mut App) -> Entity<GalleryView> + Send + Sync>,
) -> Vec<UiViewContribution> {
    let grid = create.clone();
    let mut views = vec![UiViewContribution::new(
        "gallery/media-grid-view",
        "gallery.content",
        0,
        move |window, cx| grid(window, cx).into(),
    )];
    for (region, id, slot) in [
        (Region::Search, "search", "command.search"),
        (Region::Size, "size", "command.size"),
        (Region::Filters, "filters", "command.filters"),
        (Region::Status, "status", "workspace.status"),
        (Region::Tools, "tools", "command.tools"),
        (Region::Modal, "modal", "workspace.modal"),
        (Region::Inspector, "inspector", "gallery.inspector"),
    ] {
        let create = create.clone();
        views.push(UiViewContribution::new(
            format!("gallery/{id}"),
            slot,
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

pub(super) fn chip(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
) -> Button {
    let label = label.into();
    Button::new(id)
        .ghost()
        .compact()
        .label(photag_ui_bridge::i18n::tr(&(label).to_string()))
        .h(px(26.))
        .px(px(12.))
        .rounded(px(13.))
        .text_size(px(12.5))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .bg(if active {
            rgba((theme::accent() << 8) | 64)
        } else {
            rgba(0)
        })
}

impl GalleryView {
    pub(super) fn refresh_locale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let revision = photag_ui_bridge::i18n::revision();
        if self.locale_revision == revision {
            return;
        }
        self.locale_revision = revision;
        for (input, label) in [
            (&self.search, "Search your library"),
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
            "Rating min 0-5",
            "Rating max 0-5",
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
        let show_scopes = !self.filter.search.is_empty()
            && self.search.read(cx).focus_handle(cx).is_focused(window);
        let mut field = div()
            .relative()
            .w_full()
            .h(px(32.))
            .rounded(px(theme::ROUND_MD))
            .bg(rgb(theme::SURFACE_FAINT))
            .child(
                Input::new(&self.search)
                    .appearance(false)
                    .bordered(false)
                    .cleanable(true)
                    .prefix(
                        div()
                            .font_family(theme::ICON_FONT_FAMILY)
                            .text_size(px(13.))
                            .child(theme::icons::SEARCH.to_string()),
                    )
                    .h(px(32.))
                    .w_full()
                    .text_size(px(13.5)),
            );
        if show_scopes {
            field = field.child(
                div()
                    .absolute()
                    .left(px(8.))
                    .top(px(38.))
                    .min_w(px(150.))
                    .v_flex()
                    .gap(px(2.))
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(8.))
                    .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                    .child(filter_row::search_scope_panel(self, cx)),
            );
        }
        div()
            .h_flex()
            .flex_1()
            .min_w_0()
            .justify_center()
            .child(div().w_full().min_w(px(220.)).max_w(px(280.)).child(field))
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
        let weak = cx.entity().downgrade();
        let more = Button::new("gallery-selection-more")
            .ghost()
            .compact()
            .label(photag_ui_bridge::i18n::tr("More"))
            .icon(gpui_kit::assets::IconName::Ellipsis)
            .disabled(n == 0)
            .dropdown_menu(move |menu, _, _| {
                let preview = weak.clone();
                let copy = weak.clone();
                let move_view = weak.clone();
                let delete = weak.clone();
                let details = weak.clone();
                let clear = weak.clone();
                menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Preview")).on_click(
                        move |_, _, cx| {
                            let _ = preview.update(cx, |this, cx| this.preview_selected(cx));
                        },
                    ),
                )
                .item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(
                        "Details, rename, copy, move and ratings",
                    ))
                    .on_click(move |_, _, cx| {
                        let _ = details.update(cx, |this, cx| {
                            this.show_selection_details = true;
                            cx.notify();
                        });
                    }),
                )
                .item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Copy to…")).on_click(
                        move |_, _, cx| {
                            let _ = copy.update(cx, |this, cx| this.copy_selected_with_dialog(cx));
                        },
                    ),
                )
                .item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Move to…")).on_click(
                        move |_, _, cx| {
                            let _ =
                                move_view.update(cx, |this, cx| this.move_selected_with_dialog(cx));
                        },
                    ),
                )
                .item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Delete…")).on_click(
                        move |_, window, cx| {
                            let _ = delete.update(cx, |this, cx| {
                                this.request_delete_confirmation(window, cx)
                            });
                        },
                    ),
                )
                .item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Deselect")).on_click(
                        move |_, _, cx| {
                            let _ = clear.update(cx, |this, cx| {
                                this.selection = GallerySelection::default();
                                this.persist_filter_session();
                                cx.notify();
                            });
                        },
                    ),
                )
            });
        let mut row = div()
            .h_flex()
            .w_full()
            .items_center()
            .gap(px(8.))
            .text_size(px(theme::FONT_SIZE_SMALL))
            .child(div().text_color(rgb(theme::TEXT_SECONDARY)).child(
                photag_ui_bridge::i18n::tr_args("{count} items", &[("count", count.to_string())]),
            ))
            .child(div().text_color(rgb(theme::TEXT_MUTED)).child("·"))
            .child(div().text_color(rgb(theme::TEXT_SECONDARY)).child(
                photag_ui_bridge::i18n::tr_args("{count} selected", &[("count", n.to_string())]),
            ));
        if let Some(progress) = self.service.scan_progress() {
            let fraction = progress.scanned as f32 / progress.total.max(1) as f32;
            row = row
                .child(gpui_kit::component::spinner::Spinner::new())
                .child(
                    div()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr(&progress.phase)),
                )
                .child(
                    gpui_kit::component::progress::Progress::new("gallery-scan-progress")
                        .value(fraction.clamp(0., 1.) * 100.)
                        .w(px(180.)),
                );
        }
        row.child(div().flex_1())
            .child(
                Button::new("gallery-selection-tags")
                    .ghost()
                    .compact()
                    .icon(gpui_kit::assets::IconName::Tag)
                    .label(photag_ui_bridge::i18n::tr("Tags"))
                    .disabled(n == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_tag_picker = !this.show_tag_picker;
                        cx.notify();
                    })),
            )
            .child(more)
            .into_any_element()
    }

    fn render_tools(&mut self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .v_flex()
            .gap(px(2.))
            .w_full()
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
                    .border_color(rgba(theme::SEPARATOR_LIGHT))
                    .mt(px(6.))
                    .pt(px(6.))
                    .text_size(px(theme::FONT_SIZE_SMALL))
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(photag_ui_bridge::i18n::tr("View")),
            )
            .child(
                tool_button(
                    "gallery-tools-months",
                    "Group by month",
                    gpui_kit::assets::IconName::Calendar,
                )
                .toggled(self.group_by_month)
                .on_click(
                    cx.listener(|this, _, _, cx| this.set_month_grouping(!this.group_by_month, cx)),
                ),
            )
            .into_any_element()
    }

    fn render_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_locale(window, cx);
        let mut root = div().absolute().inset_0();
        if self.show_statistics || self.bulk_action.is_some() {
            let mut card = div()
                .v_flex()
                .w(px(if self.show_statistics { 800. } else { 860. }))
                .max_h(px(600.))
                .overflow_y_scrollbar()
                .rounded(px(8.))
                .p(px(14.))
                .bg(rgba(theme::modal_rgba(theme::SURFACE_MODAL)))
                .child(
                    chip("gallery-modal-close", "Close", false).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.show_statistics = false;
                            this.bulk_action = None;
                            this.confirm_bulk_action = false;
                            this.confirm_bulk_action_ids = None;
                            cx.notify();
                        },
                    )),
                );
            if self.show_statistics {
                card = card.child(statistics::render(self, cx));
            }
            if self.bulk_action.is_some() {
                card = card.child(bulk_actions::render(self, cx));
            }
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000060))
                    .child(card),
            );
        }
        if self.show_tag_picker && !self.selection.selected_media_ids.is_empty() {
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(theme::SCRIM))
                    .child(
                        div()
                            .w(px(690.))
                            .max_h(px((f32::from(window.bounds().size.height) - 70.).max(300.)))
                            .overflow_y_scrollbar()
                            .child(tagging_picker::render(self, cx)),
                    ),
            );
        }
        if let Some(media_id) = self.media_actions_id {
            match self.service.media_by_id(media_id) {
                Ok(Some(media)) => {
                    root = root.child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgba(theme::SCRIM))
                            .child(card_menu::render_requested(&media, cx)),
                    );
                }
                Ok(None) => self.media_actions_id = None,
                Err(error) => {
                    self.operation_status = Some(error.to_string());
                    self.media_actions_id = None;
                }
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
                    .bg(rgb(theme::SURFACE_PANEL))
                    .child(status.clone()),
            );
        }
        for failure in photag_plugin_tagging::writeback::failures() {
            let path = failure.path.clone();
            let retry_path = path.clone();
            root = root.child(
                div()
                    .absolute()
                    .left(px(240.))
                    .bottom(px(48.))
                    .p_2()
                    .bg(rgb(theme::SURFACE_PANEL))
                    .child(format!(
                        "Tag writeback failed: {}: {}",
                        failure.path.display(),
                        failure.error
                    ))
                    .child(
                        Button::new(format!("writeback-retry-{}", path.display()))
                            .primary()
                            .compact()
                            .label(photag_ui_bridge::i18n::tr("Retry"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.operation_status = match photag_plugin_tagging::writeback::retry_failure(&retry_path) {
                                    Ok(true) => Some("Metadata writeback retry queued.".into()),
                                    Ok(false) => Some("This writeback failure is no longer available.".into()),
                                    Err(error) => Some(photag_ui_bridge::i18n::tr_args(
                                        "Could not retry metadata writeback: {error}",
                                        &[("error", error.to_string())],
                                    )),
                                };
                                cx.notify();
                            })),
                    )
                    .child(
                        chip(
                            format!("writeback-dismiss-{}", path.display()),
                            "Dismiss",
                            false,
                        )
                        .on_click(cx.listener(move |_, _, _, cx| {
                            photag_plugin_tagging::writeback::clear_failure(&path);
                            cx.notify();
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
        .icon(icon)
        .label(photag_ui_bridge::i18n::tr(label))
}
