use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;

use crate::PreviewService;

pub(super) fn clamp_toolbar_offset(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
    const TOOLBAR_CONTENT_WIDTH: f32 = 920.0;
    let max_x = ((width - TOOLBAR_CONTENT_WIDTH) / 2.0).max(0.0);
    let min_y = -(height - 72.0).max(0.0);
    (x.clamp(-max_x, max_x), y.clamp(min_y, 16.0))
}

pub(super) fn button(label: &'static str) -> Button {
    Button::new(format!("preview-{label}")).label(label)
}

pub(super) fn title(text: String, alpha: f32) -> Div {
    div().text_sm().font_medium().opacity(alpha).child(text)
}

pub(super) fn indexed_info(
    service: &PreviewService,
    media_id: i64,
) -> Option<crate::info::IndexedInfo> {
    service
        .database()?
        .with_reader(|connection| {
            let Some(media) =
                photag_plugin_database::db::queries::get_media_by_id(connection, media_id)?
            else {
                return Ok(None);
            };
            let tags = photag_plugin_database::db::queries::get_media_tags(connection, media_id)?;
            Ok(Some(crate::info::IndexedInfo {
                date_taken: media.date_taken,
                open_count: photag_plugin_database::db::get_open_count(connection, media_id),
                tags: tags.into_iter().map(|tag| tag.name).collect(),
            }))
        })
        .ok()
        .flatten()
}
