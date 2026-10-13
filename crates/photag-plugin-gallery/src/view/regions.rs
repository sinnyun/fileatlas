//! Projections of the single Gallery entity into the source window bands.
//! Source: main.rs:1007-1277 and ui/filterbar.rs:15-821.
use super::*;
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
            .rounded(px(16.))
            .bg(rgb(theme::SURFACE_PANEL))
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
                    .child(
                        div()
                            .text_size(px(12.))
                            .child(photag_ui_bridge::i18n::tr("Search in")),
                    )
                    .child(controls::search_scope_row(&self.filter, cx)),
            );
        }
        div()
            .h_flex()
            .flex_1()
            .min_w_0()
            .justify_center()
            .child(div().w_full().min_w(px(220.)).max_w(px(540.)).child(field))
            .into_any_element()
    }

    fn render_filters(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();
        let sort = chip(
            "gallery-sort",
            format!("⇅ {}", self.filter.sort.label()),
            false,
        )
        .dropdown_menu(move |mut menu, _, _| {
            for order in [
                SortOrder::NewestFirst,
                SortOrder::OldestFirst,
                SortOrder::BiggestFirst,
                SortOrder::SmallestFirst,
                SortOrder::NameAsc,
                SortOrder::NameDesc,
                SortOrder::HighestRated,
                SortOrder::LowestRated,
            ] {
                let view = weak.clone();
                menu = menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr(order.label())).on_click(
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| this.set_sort(order, cx));
                        },
                    ),
                );
            }
            menu
        });
        let mut row = div()
            .h_flex()
            .w_full()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .child(sort);
        if matches!(
            self.filter.sort,
            SortOrder::NewestFirst | SortOrder::OldestFirst
        ) {
            row = row.child(
                chip("gallery-month-groups", "▦ Months", self.group_by_month).on_click(
                    cx.listener(|this, _, _, cx| this.set_month_grouping(!this.group_by_month, cx)),
                ),
            );
        }
        row = row.child(controls::media_type_row(&self.filter, cx));
        let weak = cx.entity().downgrade();
        let extensions = self
            .snapshot
            .as_ref()
            .map(|s| s.available_extensions.clone())
            .unwrap_or_default();
        let filter = self.filter.clone();
        let exclude_mode = self.extension_exclude_mode;
        row = row.child(
            chip(
                "gallery-extensions",
                "Extensions",
                !filter.extensions.is_empty() || !filter.excluded_extensions.is_empty(),
            )
            .dropdown_menu(move |mut menu, _, _| {
                for (exclude, label) in [(false, "✓ Select"), (true, "⊘ Exclude")] {
                    let view = weak.clone();
                    menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                        let _ =
                            view.update(cx, |this, cx| this.set_extension_click_mode(exclude, cx));
                    }));
                }
                menu = menu.item(PopupMenuItem::separator());
                for ext in &extensions {
                    let view = weak.clone();
                    let ext = ext.clone();
                    let selected = if exclude_mode {
                        filter.excluded_extensions.contains(&ext)
                    } else {
                        filter.extensions.contains(&ext)
                    };
                    menu = menu.item(
                        PopupMenuItem::new(format!("{} .{ext}", if selected { "✓" } else { "" }))
                            .on_click(move |event, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    let modifier = this
                                        .settings
                                        .as_ref()
                                        .and_then(|s| s.snapshot().ok())
                                        .map(|s| match s.exclude_modifier {
                                            photag_plugin_settings::ExcludeModifier::None => false,
                                            photag_plugin_settings::ExcludeModifier::Ctrl => {
                                                event.modifiers().control
                                            }
                                            photag_plugin_settings::ExcludeModifier::Shift => {
                                                event.modifiers().shift
                                            }
                                            photag_plugin_settings::ExcludeModifier::Alt => {
                                                event.modifiers().alt
                                            }
                                        })
                                        .unwrap_or(event.modifiers().control);
                                    let mode = this.extension_exclude_mode;
                                    this.extension_exclude_mode ^= modifier;
                                    this.cycle_extension(ext.clone(), cx);
                                    this.extension_exclude_mode = mode;
                                    this.persist_filter_session();
                                });
                            }),
                    );
                }
                let view = weak.clone();
                menu.item(
                    PopupMenuItem::new(photag_ui_bridge::i18n::tr("Clear all")).on_click(
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| this.clear_extensions(cx));
                        },
                    ),
                )
            }),
        );
        row = row
            .child(self.date_control(true, cx))
            .child(div().text_color(rgb(theme::TEXT_MUTED)).child("–"))
            .child(self.date_control(false, cx))
            .child(
                div()
                    .font_family(theme::ICON_FONT_FAMILY)
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(theme::icons::STAR.to_string()),
            )
            .child(sliders::rating(self, cx))
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(format!(
                        "{}–{}",
                        self.filter.rating_min, self.filter.rating_max
                    )),
            );
        let active = !self.filter.search.is_empty()
            || self.filter.media_type != MediaTypeFilter::All
            || self.filter.date_from.is_some()
            || self.filter.date_to.is_some()
            || self.filter.rating_min != 0
            || self.filter.rating_max != 5
            || !self.filter.extensions.is_empty()
            || !self.filter.excluded_extensions.is_empty()
            || !self.filter.selected_tag_ids.is_empty()
            || !self.filter.excluded_tag_ids.is_empty();
        if active {
            row = row.child(
                controls::clear_filters_button(cx)
                    .ghost()
                    .compact()
                    .h(px(26.)),
            );
        }
        let mut root = div().v_flex().gap(px(4.)).w_full().child(row);
        if active {
            let tags = self
                .snapshot
                .as_ref()
                .map(|s| s.tag_tree.as_slice())
                .unwrap_or_default();
            root = root.child(controls::active_filter_row(&self.filter, tags, cx));
        }
        root.into_any_element()
    }

    fn date_control(&self, from: bool, cx: &mut Context<Self>) -> AnyElement {
        let date = if from {
            self.filter.date_from
        } else {
            self.filter.date_to
        };
        if date.is_some() {
            let picker = if from { &self.date_from } else { &self.date_to };
            div()
                .h_flex()
                .items_center()
                .gap(px(2.))
                .child(gpui_kit::component::date_picker::DatePicker::new(picker).h(px(26.)))
                .child(
                    chip(
                        if from {
                            "date-from-clear"
                        } else {
                            "date-to-clear"
                        },
                        "×",
                        false,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.clear_date_filter(from, window, cx)
                    })),
                )
                .into_any_element()
        } else {
            chip(
                if from { "date-from" } else { "date-to" },
                if from { "From" } else { "To" },
                false,
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                let today = chrono::Local::now().date_naive();
                let picker = if from { &this.date_from } else { &this.date_to };
                picker.update(cx, |picker, cx| picker.set_date(today, window, cx));
                if from {
                    this.filter.date_from = Some(today);
                } else {
                    this.filter.date_to = Some(today);
                }
                this.reload();
                cx.notify();
            }))
            .into_any_element()
        }
    }

    fn render_status(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let n = self.selection.selected_media_ids.len();
        let mut row = div()
            .h_flex()
            .w_full()
            .items_center()
            .gap(px(8.))
            .text_size(px(12.));
        if n == 0 {
            row = row.child(
                div()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(photag_ui_bridge::i18n::tr_args(
                        "{count} items in library",
                        &[(
                            "count",
                            self.snapshot
                                .as_ref()
                                .map(|s| s.media_count)
                                .unwrap_or(0)
                                .to_string(),
                        )],
                    )),
            );
        } else {
            row = row
                .child(div().text_color(rgb(theme::accent_hover())).child(
                    photag_ui_bridge::i18n::tr_args(
                        "{count} selected",
                        &[("count", n.to_string())],
                    ),
                ))
                .child(
                    chip("gallery-preview-selected", "Open", false)
                        .on_click(cx.listener(|this, _, _, cx| this.preview_selected(cx))),
                )
                .child(
                    chip("gallery-selection-tags", "Tags", self.show_tag_picker).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.show_tag_picker = !this.show_tag_picker;
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    chip("gallery-clear-tags", "Clear tags", false)
                        .on_click(cx.listener(|this, _, _, cx| this.clear_picker_tags(cx))),
                )
                .child(chip("gallery-delete-selected", "Delete", false).on_click(
                    cx.listener(|this, _, window, cx| this.request_delete_confirmation(window, cx)),
                ))
                .child(
                    chip(
                        "gallery-selection-details",
                        "⋯",
                        self.show_selection_details,
                    )
                    .tooltip(photag_ui_bridge::i18n::tr(
                        "Details, rename, copy, move and ratings",
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_selection_details = !this.show_selection_details;
                        cx.notify();
                    })),
                )
                .child(div().flex_1())
                .child(
                    chip("gallery-clear-selection", "Deselect", false).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.selection = GallerySelection::default();
                            this.persist_filter_session();
                            cx.notify();
                        },
                    )),
                );
        }
        row.into_any_element()
    }

    fn render_tools(&mut self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .v_flex()
            .gap(px(6.))
            .child(
                chip("gallery-bulk-open", "Bulk actions… (Ctrl+Shift+D)", false).on_click(
                    cx.listener(|this, _, window, cx| {
                        bulk_actions::reset_inputs(this, window, cx);
                        this.bulk_source = BulkActionSource::WholeLibrary;
                        this.bulk_action = Some(BulkActionKind::Delete);
                        this.confirm_bulk_action = false;
                        this.confirm_bulk_action_ids = None;
                        cx.notify();
                    }),
                ),
            )
            .child(
                chip("gallery-statistics", "Statistics", false).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.show_statistics = true;
                        cx.notify();
                    },
                )),
            )
            .into_any_element()
    }

    fn render_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.refresh_locale(window, cx);
        let mut root = div().absolute().inset_0();
        if self.show_statistics || self.bulk_action.is_some() {
            let mut card = div()
                .v_flex()
                .w(px(if self.show_statistics { 480. } else { 440. }))
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
                    .left(px(120.))
                    .bottom(px(48.))
                    .w(px(320.))
                    .max_h(px(480.))
                    .overflow_y_scrollbar()
                    .child(tagging_picker::render(self, cx)),
            );
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
