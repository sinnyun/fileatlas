//! Query control rows for the gallery view.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::date_picker::{DatePicker, DatePickerState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_database::db::queries::SortOrder;
use photag_plugin_database::db::queries::Tag;

use super::GalleryView;
use crate::filter::{GalleryFilter, MediaTypeFilter, SearchScope};

pub(super) fn search_scope_row(filter: &GalleryFilter, cx: &mut Context<GalleryView>) -> Div {
    div().h_flex().gap_1().children([
        scope_button("filenames", "Filenames", SearchScope::Filenames, filter, cx),
        scope_button("paths", "Paths", SearchScope::Paths, filter, cx),
        scope_button("tags", "Tags", SearchScope::Tags, filter, cx),
    ])
}

pub(super) fn clear_filters_button(cx: &mut Context<GalleryView>) -> Button {
    Button::new("gallery-clear-filters")
        .label("Clear filters")
        .on_click(cx.listener(|this, _, window, cx| this.clear_filters(window, cx)))
}

pub(super) fn active_filter_row(
    filter: &GalleryFilter,
    tags: &[Tag],
    cx: &mut Context<GalleryView>,
) -> impl IntoElement {
    let mut row = div().h_flex().items_center().gap_1().overflow_x_scrollbar();
    if !filter.search.trim().is_empty() {
        row = row.child(
            Button::new("gallery-clear-search-filter")
                .label(format!("Search: {} ×", filter.search))
                .on_click(cx.listener(|this, _, window, cx| this.clear_search_filter(window, cx))),
        );
    }
    if filter.media_type != MediaTypeFilter::All {
        let label = match filter.media_type {
            MediaTypeFilter::All => "All",
            MediaTypeFilter::Photos => "Photos",
            MediaTypeFilter::Videos => "Videos",
            MediaTypeFilter::Audio => "Audio",
        };
        row = row.child(
            Button::new("gallery-clear-type-filter")
                .label(format!("Type: {label} ×"))
                .on_click(cx.listener(|this, _, _, cx| this.clear_type_filter(cx))),
        );
    }
    if filter.rating_min > 0 || filter.rating_max < 5 {
        row = row.child(
            Button::new("gallery-clear-rating-filter")
                .label(format!(
                    "Rating: {}–{}★ ×",
                    filter.rating_min, filter.rating_max
                ))
                .on_click(cx.listener(|this, _, _, cx| this.clear_rating_filter(cx))),
        );
    }
    if filter.date_from.is_some() {
        let from = filter.date_from.expect("checked above").format("%Y-%m-%d");
        row = row.child(
            Button::new("gallery-clear-date-from-filter")
                .label(format!("From: {from} ×"))
                .on_click(
                    cx.listener(|this, _, window, cx| this.clear_date_filter(true, window, cx)),
                ),
        );
    }
    if filter.date_to.is_some() {
        let to = filter.date_to.expect("checked above").format("%Y-%m-%d");
        row = row.child(
            Button::new("gallery-clear-date-to-filter")
                .label(format!("To: {to} ×"))
                .on_click(
                    cx.listener(|this, _, window, cx| this.clear_date_filter(false, window, cx)),
                ),
        );
    }
    for extension in filter
        .extensions
        .iter()
        .chain(filter.excluded_extensions.iter())
    {
        let extension = extension.clone();
        let included = filter.extensions.contains(&extension);
        let label = format!(
            "{} .{} ×",
            if included { "Include" } else { "Exclude" },
            extension
        );
        row = row.child(
            Button::new(format!("gallery-clear-extension-{extension}"))
                .label(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.clear_extension_filter(extension.clone(), cx)
                })),
        );
    }
    for (ids, included) in [
        (&filter.selected_tag_ids, true),
        (&filter.excluded_tag_ids, false),
    ] {
        for tag_id in ids {
            let tag_id = *tag_id;
            let name =
                find_tag_name(tags, tag_id).unwrap_or_else(|| format!("Deleted tag {tag_id}"));
            row = row.child(
                Button::new(format!(
                    "gallery-clear-tag-filter-{}-{tag_id}",
                    if included { "include" } else { "exclude" }
                ))
                .label(format!(
                    "{} {name} ×",
                    if included { "Include" } else { "Exclude" }
                ))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.clear_tag_filter(tag_id, included, cx)),
                ),
            );
        }
    }
    row
}

fn find_tag_name(tags: &[Tag], id: i64) -> Option<String> {
    for tag in tags {
        if tag.id == id {
            return Some(tag.name.clone());
        }
        if let Some(name) = find_tag_name(&tag.children, id) {
            return Some(name);
        }
    }
    None
}

fn scope_button(
    id: &str,
    label: &str,
    scope: SearchScope,
    filter: &GalleryFilter,
    cx: &mut Context<GalleryView>,
) -> Button {
    let active = match scope {
        SearchScope::Filenames => filter.search_scopes.filenames,
        SearchScope::Paths => filter.search_scopes.paths,
        SearchScope::Tags => filter.search_scopes.tags,
    };
    Button::new(format!("gallery-search-scope-{id}"))
        .label(format!("{} {label}", if active { "✓" } else { "○" }))
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_search_scope(scope, cx)))
}

pub(super) fn date_row(from: &Entity<DatePickerState>, to: &Entity<DatePickerState>) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap_1()
        .child(div().text_sm().child("Date"))
        .child(DatePicker::new(from).placeholder("From date"))
        .child(div().text_sm().child("to"))
        .child(DatePicker::new(to).placeholder("To date"))
}

pub(super) fn media_type_row(filter: &GalleryFilter, cx: &mut Context<GalleryView>) -> Div {
    div().h_flex().gap_1().children([
        media_type_button("all", "All", MediaTypeFilter::All, filter.media_type, cx),
        media_type_button(
            "photos",
            "Photos",
            MediaTypeFilter::Photos,
            filter.media_type,
            cx,
        ),
        media_type_button(
            "videos",
            "Videos",
            MediaTypeFilter::Videos,
            filter.media_type,
            cx,
        ),
        media_type_button(
            "audio",
            "Audio",
            MediaTypeFilter::Audio,
            filter.media_type,
            cx,
        ),
    ])
}

pub(super) fn sort_row(filter: &GalleryFilter, cx: &mut Context<GalleryView>) -> impl IntoElement {
    let mut row = div().h_flex().gap_1().overflow_x_scrollbar();
    for (id, order) in [
        ("newest", SortOrder::NewestFirst),
        ("oldest", SortOrder::OldestFirst),
        ("largest", SortOrder::BiggestFirst),
        ("smallest", SortOrder::SmallestFirst),
        ("name-asc", SortOrder::NameAsc),
        ("name-desc", SortOrder::NameDesc),
        ("highest-rated", SortOrder::HighestRated),
        ("lowest-rated", SortOrder::LowestRated),
    ] {
        row = row.child(sort_button(id, order, filter.sort, cx));
    }
    row
}

pub(super) fn rating_row(filter: &GalleryFilter, cx: &mut Context<GalleryView>) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap_1()
        .child(div().text_sm().child("Rating"))
        .child(
            Button::new("rating-min-down")
                .label("−")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_rating_bound(true, this.filter.rating_min - 1, cx)
                })),
        )
        .child(
            div()
                .text_sm()
                .child(format!("{}–{}★", filter.rating_min, filter.rating_max)),
        )
        .child(
            Button::new("rating-min-up")
                .label("+")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_rating_bound(true, this.filter.rating_min + 1, cx)
                })),
        )
        .child(div().text_sm().child("through"))
        .child(
            Button::new("rating-max-down")
                .label("−")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_rating_bound(false, this.filter.rating_max - 1, cx)
                })),
        )
        .child(
            Button::new("rating-max-up")
                .label("+")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_rating_bound(false, this.filter.rating_max + 1, cx)
                })),
        )
        .child(
            Button::new("rating-reset")
                .label("Reset")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.filter.rating_min = 0;
                    this.filter.rating_max = 5;
                    this.reload();
                    cx.notify();
                })),
        )
}

pub(super) fn extension_row(
    filter: &GalleryFilter,
    extensions: &[String],
    exclude_mode: bool,
    cx: &mut Context<GalleryView>,
) -> impl IntoElement {
    let mut row = div()
        .h_flex()
        .items_center()
        .gap_1()
        .overflow_x_scrollbar()
        .child(div().text_sm().child("Extensions"))
        .child(
            Button::new("gallery-extension-mode-select")
                .label(if !exclude_mode {
                    "✓ Select"
                } else {
                    "Select"
                })
                .on_click(cx.listener(|this, _, _, cx| this.set_extension_click_mode(false, cx))),
        )
        .child(
            Button::new("gallery-extension-mode-exclude")
                .label(if exclude_mode {
                    "⊘ Exclude"
                } else {
                    "Exclude"
                })
                .on_click(cx.listener(|this, _, _, cx| this.set_extension_click_mode(true, cx))),
        );
    for extension in extensions {
        let selected = filter.extensions.contains(extension);
        let excluded = filter.excluded_extensions.contains(extension);
        let label = if selected {
            format!("• .{extension}")
        } else if excluded {
            format!("− .{extension}")
        } else {
            format!(".{extension}")
        };
        let extension = extension.clone();
        row = row.child(
            Button::new(format!("gallery-extension-{}", extension))
                .label(label)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.cycle_extension(extension.clone(), cx)),
                ),
        );
    }
    if !filter.extensions.is_empty() || !filter.excluded_extensions.is_empty() {
        row = row.child(
            Button::new("gallery-extension-clear")
                .label("Clear")
                .on_click(cx.listener(|this, _, _, cx| this.clear_extensions(cx))),
        );
    }
    row
}

fn media_type_button(
    id: &str,
    label: &str,
    media_type: MediaTypeFilter,
    active: MediaTypeFilter,
    cx: &mut Context<GalleryView>,
) -> Button {
    let selected_label = if media_type == active {
        format!("• {label}")
    } else {
        label.to_owned()
    };
    Button::new(format!("gallery-media-type-{id}"))
        .label(selected_label)
        .on_click(cx.listener(move |this, _, _, cx| this.set_media_type(media_type, cx)))
}

fn sort_button(
    id: &str,
    sort: SortOrder,
    active: SortOrder,
    cx: &mut Context<GalleryView>,
) -> Button {
    let label = if sort == active {
        format!("• {}", sort.label())
    } else {
        sort.label().to_owned()
    };
    Button::new(format!("gallery-sort-{id}"))
        .label(label)
        .on_click(cx.listener(move |this, _, _, cx| this.set_sort(sort, cx)))
}
