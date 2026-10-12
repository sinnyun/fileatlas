//! Host-neutral PhoTag gallery search and filter state.

use chrono::NaiveDate;
use photag_plugin_database::db::queries::{MediaQuery, SortOrder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryTab {
    Folders,
    Tags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaTypeFilter {
    All,
    Photos,
    Videos,
    Audio,
}

impl MediaTypeFilter {
    fn database_value(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Photos => Some("image"),
            Self::Videos => Some("video"),
            Self::Audio => Some("audio"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchScopes {
    pub filenames: bool,
    pub paths: bool,
    pub tags: bool,
}

impl Default for SearchScopes {
    fn default() -> Self {
        Self {
            filenames: true,
            paths: true,
            tags: true,
        }
    }
}

/// Filter values that the original top bar/sidebar combine into one database query.
#[derive(Debug, Clone)]
pub struct GalleryFilter {
    pub tab: GalleryTab,
    pub folder_id: Option<i64>,
    pub path_prefix: Option<String>,
    pub selected_tag_ids: Vec<i64>,
    pub excluded_tag_ids: Vec<i64>,
    pub search: String,
    pub search_scopes: SearchScopes,
    pub media_type: MediaTypeFilter,
    pub extensions: Vec<String>,
    pub excluded_extensions: Vec<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub rating_min: i32,
    pub rating_max: i32,
    pub sort: SortOrder,
}

impl Default for GalleryFilter {
    fn default() -> Self {
        Self {
            tab: GalleryTab::Folders,
            folder_id: None,
            path_prefix: None,
            selected_tag_ids: Vec::new(),
            excluded_tag_ids: Vec::new(),
            search: String::new(),
            search_scopes: SearchScopes::default(),
            media_type: MediaTypeFilter::All,
            extensions: Vec::new(),
            excluded_extensions: Vec::new(),
            date_from: None,
            date_to: None,
            rating_min: 0,
            rating_max: 5,
            sort: SortOrder::NewestFirst,
        }
    }
}

impl GalleryFilter {
    /// Convert UI state into the exact query semantics used by PhoTag's source AppState.
    pub fn to_query(&self) -> MediaQuery {
        let tags_tab = self.tab == GalleryTab::Tags;
        MediaQuery {
            folder_id: if tags_tab || self.path_prefix.is_some() {
                None
            } else {
                self.folder_id
            },
            tag_ids: non_empty(self.selected_tag_ids.clone()),
            excluded_tag_ids: non_empty(self.excluded_tag_ids.clone()),
            path_prefix: if tags_tab {
                None
            } else {
                self.path_prefix.clone()
            },
            search: (!self.search.trim().is_empty()).then(|| self.search.clone()),
            search_in_filenames: self.search_scopes.filenames,
            search_in_paths: self.search_scopes.paths,
            search_in_tags: self.search_scopes.tags,
            media_type: self.media_type.database_value().map(str::to_owned),
            extensions: non_empty(self.extensions.clone()),
            excluded_extensions: non_empty(self.excluded_extensions.clone()),
            date_from: self
                .date_from
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|datetime| datetime.and_utc().timestamp()),
            date_to: self
                .date_to
                .and_then(|date| date.and_hms_opt(23, 59, 59))
                .map(|datetime| datetime.and_utc().timestamp()),
            rating_min: (self.rating_min > 0).then_some(self.rating_min),
            rating_max: (self.rating_max < 5).then_some(self.rating_max),
            sort: Some(self.sort),
            offset: None,
            limit: None,
            size_min: None,
            size_max: None,
        }
    }
}

fn non_empty<T>(values: Vec<T>) -> Option<Vec<T>> {
    (!values.is_empty()).then_some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn maps_all_filters_and_inclusive_calendar_day_bounds() {
        let mut filter = GalleryFilter::default();
        filter.folder_id = Some(7);
        filter.selected_tag_ids = vec![11, 12];
        filter.excluded_tag_ids = vec![19];
        filter.search = "  Cat ".into();
        filter.search_scopes = SearchScopes {
            filenames: true,
            paths: false,
            tags: true,
        };
        filter.media_type = MediaTypeFilter::Videos;
        filter.extensions = vec!["mp4".into()];
        filter.excluded_extensions = vec!["tmp".into()];
        filter.date_from = Some(NaiveDate::from_ymd_opt(2025, 3, 4).unwrap());
        filter.date_to = Some(NaiveDate::from_ymd_opt(2025, 3, 6).unwrap());
        filter.rating_min = 2;
        filter.rating_max = 4;
        filter.sort = SortOrder::HighestRated;
        let query = filter.to_query();
        assert_eq!(query.folder_id, Some(7));
        assert_eq!(query.tag_ids, Some(vec![11, 12]));
        assert_eq!(query.excluded_tag_ids, Some(vec![19]));
        assert_eq!(query.search.as_deref(), Some("  Cat "));
        assert_eq!(query.media_type.as_deref(), Some("video"));
        assert_eq!(query.extensions, Some(vec!["mp4".into()]));
        assert_eq!(query.excluded_extensions, Some(vec!["tmp".into()]));
        assert_eq!(query.date_from, Some(1_741_046_400));
        assert_eq!(query.date_to, Some(1_741_305_599));
        assert_eq!(query.rating_min, Some(2));
        assert_eq!(query.rating_max, Some(4));
        assert_eq!(query.sort, Some(SortOrder::HighestRated));
        assert!(query.search_in_filenames && !query.search_in_paths && query.search_in_tags);
    }

    #[test]
    fn tags_tab_ignores_folder_path_and_empty_filters_are_disabled() {
        let filter = GalleryFilter {
            tab: GalleryTab::Tags,
            folder_id: Some(4),
            path_prefix: Some("C:/Photos/".into()),
            search: "  ".into(),
            ..GalleryFilter::default()
        };
        let query = filter.to_query();
        assert_eq!(query.folder_id, None);
        assert_eq!(query.path_prefix, None);
        assert_eq!(query.search, None);
        assert_eq!(query.tag_ids, None);
        assert_eq!(query.excluded_extensions, None);
        assert_eq!(query.rating_min, None);
        assert_eq!(query.rating_max, None);
    }
}
