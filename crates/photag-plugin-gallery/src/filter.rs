//! Host-neutral PhoTag gallery search and filter state.

use chrono::NaiveDate;
use photag_plugin_database::{MediaQuery, SortOrder};

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
    pub time: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Filenames,
    Paths,
    Tags,
    Time,
}

/// Parse a date edit; `None` means temporarily invalid, while `Some(None)` clears it.
pub fn parse_date_input(value: &str) -> Option<Option<NaiveDate>> {
    if value.trim().is_empty() {
        Some(None)
    } else {
        NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
            .ok()
            .map(Some)
    }
}

impl Default for SearchScopes {
    fn default() -> Self {
        Self {
            filenames: true,
            paths: true,
            tags: true,
            time: true,
        }
    }
}

impl From<photag_plugin_settings::SearchScopes> for SearchScopes {
    fn from(scopes: photag_plugin_settings::SearchScopes) -> Self {
        Self {
            filenames: scopes.filenames,
            paths: scopes.paths,
            tags: scopes.tags,
            time: scopes.time,
        }
    }
}

impl From<SearchScopes> for photag_plugin_settings::SearchScopes {
    fn from(scopes: SearchScopes) -> Self {
        Self {
            filenames: scopes.filenames,
            paths: scopes.paths,
            tags: scopes.tags,
            time: scopes.time,
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
    pub search_entire_computer: bool,
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
            search_entire_computer: false,
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
    pub fn toggle_search_scope(&mut self, scope: SearchScope) {
        match scope {
            SearchScope::Filenames => self.search_scopes.filenames = !self.search_scopes.filenames,
            SearchScope::Paths => self.search_scopes.paths = !self.search_scopes.paths,
            SearchScope::Tags => self.search_scopes.tags = !self.search_scopes.tags,
            SearchScope::Time => self.search_scopes.time = !self.search_scopes.time,
        }
    }

    /// Set one end of the inclusive rating range while preserving a valid interval.
    pub fn set_rating_bound(&mut self, minimum: bool, value: i32) {
        let value = value.clamp(0, 5);
        if minimum {
            self.rating_min = value;
            if self.rating_min > self.rating_max {
                self.rating_max = value;
            }
        } else {
            self.rating_max = value;
            if self.rating_max < self.rating_min {
                self.rating_min = value;
            }
        }
    }

    /// Cycle one extension through included, excluded, and neutral query states.
    pub fn cycle_extension(&mut self, extension: &str) {
        if let Some(index) = self.extensions.iter().position(|item| item == extension) {
            self.extensions.remove(index);
            self.excluded_extensions.push(extension.to_owned());
        } else if let Some(index) = self
            .excluded_extensions
            .iter()
            .position(|item| item == extension)
        {
            self.excluded_extensions.remove(index);
        } else {
            self.extensions.push(extension.to_owned());
        }
    }

    pub fn clear_extensions(&mut self) {
        self.extensions.clear();
        self.excluded_extensions.clear();
    }

    /// Remove an extension from either inclusion state.
    pub fn clear_extension(&mut self, extension: &str) {
        self.extensions.retain(|item| item != extension);
        self.excluded_extensions.retain(|item| item != extension);
    }

    /// Convert UI state into the exact query semantics used by PhoTag's source AppState.
    pub fn to_query(&self) -> MediaQuery {
        let tags_tab = self.tab == GalleryTab::Tags
            && (self.search.trim().is_empty() || self.search_entire_computer);
        MediaQuery {
            local_only: false,
            annotated_only: false,
            directory_tree: None,
            direct_directory: None,
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
            search_entire_computer: self.search_entire_computer,
            search_in_filenames: self.search_scopes.filenames,
            search_in_paths: self.search_scopes.paths,
            search_in_tags: self.search_scopes.tags,
            media_type: self.media_type.database_value().map(str::to_owned),
            extensions: non_empty(self.extensions.clone()),
            excluded_extensions: non_empty(self.excluded_extensions.clone()),
            date_from: self
                .search_scopes
                .time
                .then(|| self.date_from)
                .flatten()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|datetime| datetime.and_utc().timestamp()),
            date_to: self
                .search_scopes
                .time
                .then(|| self.date_to)
                .flatten()
                .and_then(|date| date.and_hms_opt(23, 59, 59))
                .map(|datetime| datetime.and_utc().timestamp()),
            rating_min: (self.rating_min > 0).then_some(self.rating_min),
            rating_max: (self.rating_max < 5).then_some(self.rating_max),
            sort: Some(self.sort),
            offset: None,
            limit: None,
            size_min: None,
            size_max: None,
            external_search_paths: None,
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
            time: true,
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

    #[test]
    fn rating_range_bounds_clamp_and_remain_ordered() {
        let mut filter = GalleryFilter::default();
        filter.set_rating_bound(true, 9);
        assert_eq!((filter.rating_min, filter.rating_max), (5, 5));
        filter.set_rating_bound(false, -1);
        assert_eq!((filter.rating_min, filter.rating_max), (0, 0));
        filter.set_rating_bound(true, 3);
        filter.set_rating_bound(false, 4);
        assert_eq!((filter.rating_min, filter.rating_max), (3, 4));
    }

    #[test]
    fn extension_click_cycles_include_exclude_and_neutral() {
        let mut filter = GalleryFilter::default();
        filter.cycle_extension("jpg");
        assert_eq!(filter.extensions, ["jpg"]);
        filter.cycle_extension("jpg");
        assert!(filter.extensions.is_empty());
        assert_eq!(filter.excluded_extensions, ["jpg"]);
        filter.cycle_extension("jpg");
        assert!(filter.extensions.is_empty());
        assert!(filter.excluded_extensions.is_empty());
        filter.cycle_extension("png");
        filter.cycle_extension("gif");
        filter.clear_extensions();
        assert!(filter.extensions.is_empty() && filter.excluded_extensions.is_empty());
    }

    #[test]
    fn clearing_one_extension_removes_only_that_filter_state() {
        let mut filter = GalleryFilter::default();
        filter.extensions = vec!["jpg".into(), "png".into()];
        filter.excluded_extensions = vec!["tmp".into()];

        filter.clear_extension("jpg");
        assert_eq!(filter.extensions, ["png"]);
        assert_eq!(filter.excluded_extensions, ["tmp"]);

        filter.clear_extension("tmp");
        assert_eq!(filter.extensions, ["png"]);
        assert!(filter.excluded_extensions.is_empty());
    }

    #[test]
    fn date_input_distinguishes_valid_empty_and_partial_values() {
        assert_eq!(
            parse_date_input("2025-03-04"),
            Some(Some(NaiveDate::from_ymd_opt(2025, 3, 4).unwrap()))
        );
        assert_eq!(parse_date_input(""), Some(None));
        assert_eq!(parse_date_input("2025-03-"), None);
        assert_eq!(parse_date_input("not a date"), None);
    }

    #[test]
    fn search_scope_toggles_map_to_query_flags() {
        let mut filter = GalleryFilter::default();
        filter.toggle_search_scope(SearchScope::Paths);
        assert_eq!(filter.to_query().search_in_paths, false);
        filter.toggle_search_scope(SearchScope::Filenames);
        filter.toggle_search_scope(SearchScope::Tags);
        filter.toggle_search_scope(SearchScope::Time);
        let query = filter.to_query();
        assert!(!query.search_in_filenames && !query.search_in_paths && !query.search_in_tags);
        assert!(!filter.search_scopes.time);
    }

    #[test]
    fn disabling_time_scope_omits_date_bounds_from_search_query() {
        let mut filter = GalleryFilter::default();
        filter.date_from = Some(NaiveDate::MIN);
        filter.date_to = Some(NaiveDate::MAX);
        assert!(filter.to_query().date_from.is_some());
        assert!(filter.to_query().date_to.is_some());

        filter.toggle_search_scope(SearchScope::Time);
        let query = filter.to_query();
        assert_eq!(query.date_from, None);
        assert_eq!(query.date_to, None);
    }
}
