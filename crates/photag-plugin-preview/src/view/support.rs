use gpui_kit::component::button::Button;

use crate::PreviewService;

pub(super) fn button(label: &'static str) -> Button {
    Button::new(format!("preview-{label}")).label(photag_ui_bridge::i18n::tr(&(label).to_string()))
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
