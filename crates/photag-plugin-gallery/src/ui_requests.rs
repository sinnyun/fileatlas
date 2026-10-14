use super::*;

/// Requests consumed by the live Gallery view, using its existing actions.
#[derive(Clone)]
pub enum GalleryUiRequest {
    Statistics,
    BulkActions,
    ConfigureBulk(crate::BulkActionKind, crate::BulkActionSource),
    OpenMediaActions(i64),
    CloseMediaActions,
    CloseDialogs,
    Select(i64),
    SelectMany(Vec<i64>),
    ScrollTo(usize),
    Deselect,
    Hover(Option<i64>),
    Search(String),
    MediaType(filter::MediaTypeFilter),
    Sort(queries::SortOrder),
    RatingRange(i32, i32),
    Extension(String, bool),
    TagFilter(i64, bool),
    TagFilterMode(bool),
    ExtensionMode(bool),
    DateBound(bool, Option<chrono::NaiveDate>),
    ClearFilters,
    InspectorSection(&'static str),
    TagPicker,
    MonthGrouping(bool),
}

/// Snapshot published by the rendered Gallery entity, not a separate query.
#[derive(Clone)]
pub struct GalleryUiSnapshot {
    pub filter: filter::GalleryFilter,
    pub selected_media_ids: Vec<i64>,
    pub matched_count: usize,
    pub window_offset: usize,
    pub loaded_count: usize,
    pub group_by_month: bool,
    pub inspector_sections: Vec<&'static str>,
    pub bulk_action: Option<&'static str>,
    pub bulk_source: &'static str,
    pub statistics_open: bool,
    pub tag_picker_open: bool,
    pub media_actions_open: bool,
    pub media_actions_id: Option<i64>,
}

impl GalleryService {
    pub fn ui_snapshot(&self) -> Option<GalleryUiSnapshot> {
        self.ui_snapshot
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.clone())
    }

    /// Queue the live media action surface for one indexed record.
    pub fn open_media_actions(&self, media_id: i64) -> Result<(), String> {
        if media_id <= 0 {
            return Err("media_id must be positive".into());
        }
        if self
            .media_by_id(media_id)
            .map_err(|error| error.to_string())?
            .is_none()
        {
            return Err(format!("media {media_id} is not indexed"));
        }
        self.request_ui(GalleryUiRequest::OpenMediaActions(media_id));
        Ok(())
    }
    pub(crate) fn publish_ui_snapshot(&self, snapshot: GalleryUiSnapshot) {
        if let Ok(mut current) = self.ui_snapshot.lock() {
            *current = Some(snapshot);
        }
    }
}
