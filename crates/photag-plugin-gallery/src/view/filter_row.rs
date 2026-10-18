//! v5 filter chip row for the gallery (slot `command.filters`). Every chip
//! opens a gpui-kit Popover with radio/checkbox lists; once a filter is
//! active its chip turns accent and its trailing control clears that filter.
//! Mockups: docs/reference-ui/final-v5/00-main-library.png and
//! 05-filters-menus.png.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::date_picker::DatePicker;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::radio::Radio;

use super::*;
use crate::filter::SearchScopes;
use gpui_kit::prelude::FluentBuilder as _;

mod helpers;
mod multi_select;
use helpers::*;
use multi_select::{extensions_chip, tags_chip};

pub(super) fn render(view: &mut GalleryView, cx: &mut Context<GalleryView>) -> AnyElement {
    let weak = cx.entity().downgrade();
    let mut row = div()
        .id("gallery-filter-row")
        .flex_1()
        .min_w_0()
        .h_flex()
        .w_full()
        .items_center()
        .gap(px(4.))
        .overflow_x_scroll();
    row = row.child(type_chip(view, weak.clone(), cx));
    row = row.child(tags_chip(view, weak.clone(), cx));
    row = row.child(extensions_chip(view, weak.clone(), cx));
    row = row.child(date_chip(view, weak.clone(), cx));
    row = row.child(rating_chip(view, weak.clone(), cx));
    row = row.child(sort_chip(view, weak.clone(), cx));
    row = row.child(grouping_chip(view, cx));
    div()
        .h_flex()
        .w_full()
        .min_w_0()
        .items_center()
        .gap(px(10.))
        .child(row)
        .child(view_controls::render(view, cx))
        .into_any_element()
}

/// Scoped-search control in the top bar, using the managed popover layer so
/// the panel is not clipped by the command-row layout.
pub(super) fn search_scope_popover(view: &GalleryView, cx: &mut Context<GalleryView>) -> Popover {
    let scopes = view.filter.search_scopes;
    let weak = cx.entity().downgrade();
    let open_weak = weak.clone();
    Popover::new("gallery-search-scopes-popover")
        .anchor(gpui_kit::Anchor::TopRight)
        .open(view.search_scopes_open)
        .on_open_change(move |open, _, cx| {
            let _ = open_weak.update(cx, |this, cx| {
                this.search_scopes_open = *open;
                cx.notify();
            });
        })
        .trigger(
            view_controls::button(
                "gallery-search-scopes",
                IconName::SlidersHorizontal,
                "Scope",
            )
            .toggled(view.search_scopes_open),
        )
        .content(move |_, _, _| search_scope_popover_panel(scopes, weak.clone()))
}

fn search_scope_popover_panel(scopes: SearchScopes, weak: WeakEntity<GalleryView>) -> Div {
    let mut panel = div()
        .v_flex()
        .w(px(260.))
        .gap(px(2.))
        .child(popover_title("Search scope"));
    for (id, scope, title, subtitle, checked) in [
        (
            "search-scope-filenames",
            SearchScope::Filenames,
            "Filenames",
            "Match file names",
            scopes.filenames,
        ),
        (
            "search-scope-paths",
            SearchScope::Paths,
            "Paths",
            "Match file paths",
            scopes.paths,
        ),
        (
            "search-scope-tags",
            SearchScope::Tags,
            "Tags",
            "Match file tags",
            scopes.tags,
        ),
        (
            "search-scope-time",
            SearchScope::Time,
            "Time range",
            "Apply the selected date range",
            scopes.time,
        ),
    ] {
        panel = panel.child(
            div()
                .id(format!("gallery-{id}"))
                .h_flex()
                .items_center()
                .gap(px(8.))
                .h(px(44.))
                .px(px(6.))
                .rounded(px(theme::ROUND_SM))
                .cursor_pointer()
                .hover(|style| style.bg(rgba(theme::FILL_HOVER)))
                .child(Checkbox::new(format!("gallery-{id}-box")).checked(checked))
                .child(
                    div()
                        .v_flex()
                        .child(
                            div()
                                .text_size(px(theme::FONT_SIZE_BUTTON))
                                .text_color(rgb(theme::TEXT_PRIMARY))
                                .child(photag_ui_bridge::i18n::tr(title)),
                        )
                        .child(
                            div()
                                .text_size(px(theme::FONT_SIZE_TINY))
                                .text_color(rgb(theme::TEXT_MUTED))
                                .child(photag_ui_bridge::i18n::tr(subtitle)),
                        ),
                )
                .on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        let _ = weak.update(cx, |this, cx| this.toggle_search_scope(scope, cx));
                    }
                }),
        );
    }
    panel
}

fn type_chip(
    view: &GalleryView,
    weak: WeakEntity<GalleryView>,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let active = view.filter.media_type != MediaTypeFilter::All;
    let label = media_type_label(view.filter.media_type).to_string();
    let selected = view.filter.media_type;
    let clear = active.then(|| {
        clear_control(
            "gallery-filter-type-clear",
            cx.listener(|this, _, _, cx| this.clear_type_filter(cx)),
        )
    });
    controlled_popover(view, weak.clone(), "gallery-filter-type-popover", "type")
        .trigger(chip(
            "gallery-filter-type",
            IconName::LayoutGrid,
            label,
            active,
            clear,
        ))
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let mut panel = div()
                .v_flex()
                .w(px(220.))
                .gap(px(2.))
                .child(popover_title("Media type"));
            for (value, label, icon) in [
                (MediaTypeFilter::All, "All files", IconName::LayoutGrid),
                (MediaTypeFilter::Photos, "Photos", IconName::Image),
                (MediaTypeFilter::Videos, "Videos", IconName::Video),
                (MediaTypeFilter::Audio, "Audio", IconName::Music),
            ] {
                let weak = weak.clone();
                let popover = popover.clone();
                panel = panel.child(
                    menu_row(format!("gallery-filter-type-{}", value as u8))
                        .child(
                            Radio::new(format!("gallery-filter-type-radio-{}", value as u8))
                                .checked(value == selected),
                        )
                        .child(theme::small_icon(icon).text_color(rgb(theme::TEXT_MUTED)))
                        .child(row_label(photag_ui_bridge::i18n::tr(label).to_string()))
                        .on_click(move |_, window, cx| {
                            let _ = weak.update(cx, |this, cx| this.set_media_type(value, cx));
                            popover.update(cx, |state, cx| state.dismiss(window, cx));
                        }),
                );
            }
            panel
        })
        .into_any_element()
}

fn date_chip(
    view: &GalleryView,
    weak: WeakEntity<GalleryView>,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let active = view.filter.date_from.is_some() || view.filter.date_to.is_some();
    let date_from = view.date_from.clone();
    let date_to = view.date_to.clone();
    let calendar = view.date_calendar.clone();
    let clear = active.then(|| {
        clear_control(
            "gallery-filter-date-clear",
            cx.listener(|this, _, window, cx| {
                if this.filter.date_from.is_some() {
                    this.clear_date_filter(true, window, cx);
                }
                if this.filter.date_to.is_some() {
                    this.clear_date_filter(false, window, cx);
                }
            }),
        )
    });
    controlled_popover(view, weak.clone(), "gallery-filter-date-popover", "date")
        .trigger(chip(
            "gallery-filter-date",
            IconName::CalendarDays,
            "Date".into(),
            active,
            clear,
        ))
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let weak = weak.clone();
            div()
                .v_flex()
                .w(px(304.))
                .gap(px(6.))
                .child(popover_title("Date range"))
                .child(
                    div()
                        .h_flex()
                        .w_full()
                        .gap(px(8.))
                        .child(
                            div()
                                .v_flex()
                                .flex_1()
                                .gap(px(4.))
                                .child(row_label(photag_ui_bridge::i18n::tr("Date from")))
                                .child(
                                    DatePicker::new(&date_from)
                                        .placeholder(photag_ui_bridge::i18n::tr("Select date"))
                                        .w_full(),
                                ),
                        )
                        .child(
                            div()
                                .v_flex()
                                .flex_1()
                                .gap(px(4.))
                                .child(row_label(photag_ui_bridge::i18n::tr("Date to")))
                                .child(
                                    DatePicker::new(&date_to)
                                        .placeholder(photag_ui_bridge::i18n::tr("Select date"))
                                        .w_full(),
                                ),
                        ),
                )
                .child(gpui_kit::component::calendar::Calendar::new(&calendar).w_full())
                .child(
                    Button::new("gallery-date-clear-all")
                        .ghost()
                        .compact()
                        .label(photag_ui_bridge::i18n::tr("Clear dates"))
                        .on_click(move |_, window, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                if this.filter.date_from.is_some() {
                                    this.clear_date_filter(true, window, cx);
                                }
                                if this.filter.date_to.is_some() {
                                    this.clear_date_filter(false, window, cx);
                                }
                            });
                            popover.update(cx, |state, cx| state.dismiss(window, cx));
                        }),
                )
        })
        .into_any_element()
}

fn rating_chip(
    view: &GalleryView,
    weak: WeakEntity<GalleryView>,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let rating_min = view.filter.rating_min;
    let rating_max = view.filter.rating_max;
    let active = rating_min != 0 || rating_max != 5;
    let clear = active.then(|| {
        clear_control(
            "gallery-filter-rating-clear",
            cx.listener(|this, _, _, cx| {
                this.clear_rating_filter(cx);
            }),
        )
    });
    controlled_popover(
        view,
        weak.clone(),
        "gallery-filter-rating-popover",
        "rating",
    )
    .trigger(chip(
        "gallery-filter-rating",
        IconName::Star,
        "Rating".into(),
        active,
        clear,
    ))
    .content(move |_, _, cx| {
        let popup = cx.entity().downgrade();
        let weak = weak.clone();
        let (rating_min, rating_max) = weak
            .upgrade()
            .map(|view| {
                let view = view.read(cx);
                (view.filter.rating_min, view.filter.rating_max)
            })
            .unwrap_or((rating_min, rating_max));
        let mut stars = div().h_flex().items_center().gap(px(4.));
        for star in 1..=5u8 {
            let weak = weak.clone();
            stars = stars.child(
                div()
                    .id(format!("gallery-filter-rating-star-{star}"))
                    .cursor_pointer()
                    .text_color(rgb(if i32::from(star) <= rating_min {
                        theme::STAR
                    } else {
                        theme::TEXT_MUTED
                    }))
                    .hover(|style| style.text_color(rgb(theme::STAR)))
                    .child(theme::icon(IconName::Star).text_color(rgb(
                        if i32::from(star) <= rating_min {
                            theme::STAR
                        } else {
                            theme::TEXT_MUTED
                        },
                    )))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            let next = i32::from(star);
                            if this.filter.rating_min == next && this.filter.rating_max == 5 {
                                this.set_rating_bound(true, 0, cx);
                            } else {
                                this.set_rating_bound(true, next, cx);
                                this.set_rating_bound(false, 5, cx);
                            }
                        });
                        cx.notify();
                    })),
            );
        }
        div()
            .v_flex()
            .w(px(270.))
            .gap(px(8.))
            .child(popover_title("Rating range"))
            .child(stars)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_TINY))
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child("0"),
                    )
                    .child(rating_slider(weak, popup, rating_min, rating_max))
                    .child(
                        div()
                            .text_size(px(theme::FONT_SIZE_TINY))
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child("5"),
                    ),
            )
    })
    .into_any_element()
}

fn sort_chip(
    view: &GalleryView,
    weak: WeakEntity<GalleryView>,
    _: &mut Context<GalleryView>,
) -> AnyElement {
    let selected = view.filter.sort;

    controlled_popover(view, weak.clone(), "gallery-filter-sort-popover", "sort")
        .trigger(chip(
            "gallery-filter-sort",
            IconName::ArrowUpDown,
            photag_ui_bridge::i18n::tr(selected.label()).to_string(),
            false,
            None,
        ))
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let mut panel = div()
                .v_flex()
                .w(px(250.))
                .gap(px(2.))
                .child(popover_title("Sort by"));
            for (order, icon) in [
                (SortOrder::NewestFirst, IconName::CalendarArrowDown),
                (SortOrder::OldestFirst, IconName::CalendarArrowUp),
                (SortOrder::BiggestFirst, IconName::ArrowDownWideNarrow),
                (SortOrder::SmallestFirst, IconName::ArrowUpNarrowWide),
                (SortOrder::NameAsc, IconName::ArrowDownAZ),
                (SortOrder::NameDesc, IconName::ArrowUpZA),
                (SortOrder::HighestRated, IconName::Star),
                (SortOrder::LowestRated, IconName::Star),
            ] {
                let weak = weak.clone();
                let popover = popover.clone();
                panel = panel.child(
                    menu_row(format!("gallery-filter-sort-{}", order as u8))
                        .child(
                            Radio::new(format!("gallery-filter-sort-radio-{}", order as u8))
                                .checked(order == selected),
                        )
                        .child(theme::small_icon(icon).text_color(rgb(theme::TEXT_MUTED)))
                        .child(row_label(
                            photag_ui_bridge::i18n::tr(order.label()).to_string(),
                        ))
                        .on_click(move |_, window, cx| {
                            let _ = weak.update(cx, |this, cx| this.set_sort(order, cx));
                            popover.update(cx, |state, cx| state.dismiss(window, cx));
                        }),
                );
            }
            panel
        })
        .into_any_element()
}

pub(super) fn grouping_chip(view: &GalleryView, cx: &mut Context<GalleryView>) -> AnyElement {
    let selected = view.grouping_mode;
    let weak = cx.entity().downgrade();
    controlled_popover(view, weak.clone(), "gallery-grouping-popover", "grouping")
        .trigger(chip(
            "gallery-filter-months",
            IconName::CalendarRange,
            photag_ui_bridge::i18n::tr(selected.label()).to_string(),
            selected != photag_plugin_grouping::GroupingMode::None,
            None,
        ))
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let mut panel = div()
                .v_flex()
                .w(px(280.))
                .gap(px(2.))
                .child(popover_title("Group by"));
            for mode in photag_plugin_grouping::GroupingMode::ALL {
                let weak = weak.clone();
                let popover = popover.clone();
                panel = panel.child(
                    menu_row(format!("grouping-{}", mode.key()))
                        .child(
                            Radio::new(format!("grouping-radio-{}", mode.key()))
                                .checked(mode == selected),
                        )
                        .child(row_label(
                            photag_ui_bridge::i18n::tr(mode.label()).to_string(),
                        ))
                        .on_click(move |_, window, cx| {
                            let _ = weak.update(cx, |this, cx| this.set_grouping(mode, cx));
                            popover.update(cx, |state, cx| state.dismiss(window, cx));
                        }),
                );
            }
            panel
        })
        .into_any_element()
}

fn media_type_label(media_type: MediaTypeFilter) -> &'static str {
    match media_type {
        MediaTypeFilter::All => "All types",
        MediaTypeFilter::Photos => "Photos",
        MediaTypeFilter::Videos => "Videos",
        MediaTypeFilter::Audio => "Audio",
    }
}

fn flatten_tag_tree(
    tags: &[photag_plugin_database::Tag],
) -> Vec<(photag_plugin_database::Tag, usize)> {
    fn visit(
        tags: &[photag_plugin_database::Tag],
        depth: usize,
        out: &mut Vec<(photag_plugin_database::Tag, usize)>,
    ) {
        for tag in tags {
            out.push((tag.clone(), depth));
            visit(&tag.children, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    visit(tags, 0, &mut out);
    out
}

pub(super) fn subscribe_calendar(view: &Entity<GalleryView>, window: &mut Window, app: &mut App) {
    view.update(app, |view, cx| {
        let calendar = view.date_calendar.clone();
        view._subscriptions.push(cx.subscribe_in(
            &calendar,
            window,
            |this, _, event: &gpui_kit::component::calendar::CalendarEvent, window, cx| {
                let gpui_kit::component::calendar::CalendarEvent::Selected(date) = event;
                let (from, to) = (date.start(), date.end());
                this.filter.date_from = from;
                this.filter.date_to = to;
                this.date_from.update(cx, |state, cx| {
                    state.set_date(
                        gpui_kit::component::calendar::Date::Single(from),
                        window,
                        cx,
                    )
                });
                this.date_to.update(cx, |state, cx| {
                    state.set_date(gpui_kit::component::calendar::Date::Single(to), window, cx)
                });
                this.reload();
                cx.notify();
            },
        ));
    });
}
