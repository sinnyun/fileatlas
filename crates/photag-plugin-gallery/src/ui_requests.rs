use super::*;

/// Requests consumed by the live Gallery view, using its existing actions.
#[derive(Clone)]
pub enum GalleryUiRequest {
    Statistics,
    BulkActions,
    CloseDialogs,
    Select(i64),
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
    pub group_by_month: bool,
    pub inspector_sections: Vec<&'static str>,
}

impl GalleryService {
    pub fn ui_snapshot(&self) -> Option<GalleryUiSnapshot> {
        self.ui_snapshot
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.clone())
    }
    pub(crate) fn publish_ui_snapshot(&self, snapshot: GalleryUiSnapshot) {
        if let Ok(mut current) = self.ui_snapshot.lock() {
            *current = Some(snapshot);
        }
    }
}
