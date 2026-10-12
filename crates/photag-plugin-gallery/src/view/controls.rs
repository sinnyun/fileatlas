//! Query control rows for the gallery view.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_database::db::queries::SortOrder;

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

pub(super) fn date_row(from: &Entity<InputState>, to: &Entity<InputState>) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap_1()
        .child(div().text_sm().child("Date"))
        .child(Input::new(from).w(px(150.0)))
        .child(div().text_sm().child("to"))
        .child(Input::new(to).w(px(150.0)))
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
    div().h_flex().gap_1().overflow_x_scrollbar().children([
        sort_button("newest", SortOrder::NewestFirst, filter.sort, cx),
        sort_button("oldest", SortOrder::OldestFirst, filter.sort, cx),
        sort_button("largest", SortOrder::BiggestFirst, filter.sort, cx),
        sort_button("smallest", SortOrder::SmallestFirst, filter.sort, cx),
        sort_button("name-asc", SortOrder::NameAsc, filter.sort, cx),
        sort_button("name-desc", SortOrder::NameDesc, filter.sort, cx),
        sort_button("highest-rated", SortOrder::HighestRated, filter.sort, cx),
        sort_button("lowest-rated", SortOrder::LowestRated, filter.sort, cx),
    ])
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
    cx: &mut Context<GalleryView>,
) -> impl IntoElement {
    let mut row = div()
        .h_flex()
        .items_center()
        .gap_1()
        .overflow_x_scrollbar()
        .child(div().text_sm().child("Extensions"));
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
