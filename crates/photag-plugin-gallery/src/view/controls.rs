//! Query control rows for the gallery view.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_database::db::queries::Tag;

use super::GalleryView;
use crate::filter::{GalleryFilter, MediaTypeFilter, SearchScope};
use gpui_kit::component::button::ButtonVariants;

pub(super) fn search_scope_row(filter: &GalleryFilter, cx: &mut Context<GalleryView>) -> Div {
    div().v_flex().gap_1().children([
        scope_button("filenames", "Filenames", SearchScope::Filenames, filter, cx),
        scope_button("paths", "Paths", SearchScope::Paths, filter, cx),
        scope_button("tags", "Tags", SearchScope::Tags, filter, cx),
    ])
}

pub(super) fn clear_filters_button(cx: &mut Context<GalleryView>) -> Button {
    Button::new("gallery-clear-filters")
        .label(photag_ui_bridge::i18n::tr("Clear filters"))
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
                .ghost()
                .compact()
                .h(px(22.))
                .rounded(px(11.))
                .label(photag_ui_bridge::i18n::tr(
                    &(format!("Search: {} ×", filter.search)).to_string(),
                ))
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
                .label(photag_ui_bridge::i18n::tr(
                    &(format!("Type: {label} ×")).to_string(),
                ))
                .on_click(cx.listener(|this, _, _, cx| this.clear_type_filter(cx))),
        );
    }
    if filter.rating_min > 0 || filter.rating_max < 5 {
        row = row.child(
            Button::new("gallery-clear-rating-filter")
                .label(photag_ui_bridge::i18n::tr(
                    &(format!("Rating: {}–{}★ ×", filter.rating_min, filter.rating_max))
                        .to_string(),
                ))
                .on_click(cx.listener(|this, _, _, cx| this.clear_rating_filter(cx))),
        );
    }
    if filter.date_from.is_some() {
        let from = filter.date_from.expect("checked above").format("%Y-%m-%d");
        row = row.child(
            Button::new("gallery-clear-date-from-filter")
                .label(photag_ui_bridge::i18n::tr(
                    &(format!("From: {from} ×")).to_string(),
                ))
                .on_click(
                    cx.listener(|this, _, window, cx| this.clear_date_filter(true, window, cx)),
                ),
        );
    }
    if filter.date_to.is_some() {
        let to = filter.date_to.expect("checked above").format("%Y-%m-%d");
        row = row.child(
            Button::new("gallery-clear-date-to-filter")
                .label(photag_ui_bridge::i18n::tr(
                    &(format!("To: {to} ×")).to_string(),
                ))
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
                .label(photag_ui_bridge::i18n::tr(&(label).to_string()))
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
                .label(photag_ui_bridge::i18n::tr(
                    &(format!("{} {name} ×", if included { "Include" } else { "Exclude" }))
                        .to_string(),
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
        .label(photag_ui_bridge::i18n::tr(
            &(format!(
                "{} {}",
                if active { "✓" } else { "○" },
                photag_ui_bridge::i18n::tr(label)
            ))
            .to_string(),
        ))
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_search_scope(scope, cx)))
}

pub(super) fn media_type_row(filter: &GalleryFilter, cx: &mut Context<GalleryView>) -> Div {
    div()
        .h_flex()
        .gap_0()
        .p(px(3.))
        .h(px(26.))
        .rounded(px(13.))
        .bg(rgba(0xffffff06))
        .children([
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

fn media_type_button(
    id: &str,
    label: &str,
    media_type: MediaTypeFilter,
    active: MediaTypeFilter,
    cx: &mut Context<GalleryView>,
) -> Button {
    super::regions::chip(
        format!("gallery-media-type-{id}"),
        label.to_owned(),
        media_type == active,
    )
    .px(px(13.))
    .h(px(20.))
    .rounded(px(10.))
    .on_click(cx.listener(move |this, _, _, cx| this.set_media_type(media_type, cx)))
}
