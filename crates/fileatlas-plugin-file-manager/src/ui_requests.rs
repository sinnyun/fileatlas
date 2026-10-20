use super::*;

/// Requests consumed by the live FileManager view, using its existing actions.
#[derive(Debug, Clone)]
pub enum FileManagerUiRequest {
    Layout(bool),
    OpenDirectory(String),
    LocateFile(i64),
    DirectoryUp,
    Refresh,
    ShowDescendants(bool),
    Thumbnails(bool),
    CancelDirectory,
    Statistics,
    MissingRecords,
    RecordCandidate(i64, String),
    RelinkRecord(i64),
    DeleteMissingRecords,
    BulkActions,
    ConfigureBulk(crate::BulkActionKind, crate::BulkActionSource),
    BulkField(usize, String),
    OpenMediaActions(i64),
    CloseMediaActions,
    CloseDialogs,
    Select(i64),
    Preview(i64),
    SelectMany(Vec<i64>),
    ScrollTo(usize),
    Deselect,
    Hover(Option<i64>),
    Search(String),
    SearchScopesOpen(bool),
    EverythingRecheck,
    SearchScope(filter::SearchScope),
    SearchEntireComputer(bool),
    MediaType(filter::MediaTypeFilter),
    Sort(fileatlas_plugin_database::SortOrder),
    RatingRange(i32, i32),
    Extension(String, bool),
    TagFilter(i64, bool),
    TagFilterMode(bool),
    ExtensionMode(bool),
    DateBound(bool, Option<chrono::NaiveDate>),
    ClearFilters,
    InspectorSection(&'static str),
    InspectorTagName(String),
    HistoryChanged(i64),
    HistorySelect(i64, String),
    TagPicker,
    MonthGrouping(bool),
    Grouping(fileatlas_plugin_grouping::GroupingMode),
    ToggleGroup(String),
    MoveFilesToSubfolder(Vec<i64>, String),
    /// Isolated dev-test pointer validation; never persisted as a product setting.
    DebugInternalFileDrag(bool),
}

/// Snapshot published by the rendered FileManager entity, not a separate query.
#[derive(Clone)]
pub struct FileManagerUiSnapshot {
    pub search_provider: &'static str,
    pub everything_state: &'static str,
    pub everything_version: Option<String>,
    pub everything_path: Option<String>,
    pub everything_loading: bool,
    pub version_states: Vec<FileManagerVersionState>,
    pub version_states_loading: bool,
    pub snapshot_revision: u64,
    pub records_open: bool,
    pub records_count: Option<usize>,
    pub record_candidates: Vec<(i64, String)>,
    pub filter: filter::FileManagerFilter,
    pub search_scopes_open: bool,
    pub browsing: bool,
    pub directories: Vec<crate::FileManagerDirectory>,
    pub current_directory: Option<String>,
    pub parent_directory: Option<String>,
    pub directory_error: Option<String>,
    pub selected_directory: Option<String>,
    pub selected_media_ids: Vec<i64>,
    pub file_copy_buffer_count: usize,
    pub matched_count: usize,
    pub window_offset: usize,
    pub loaded_count: usize,
    pub group_by_month: bool,
    pub grouping_mode: fileatlas_plugin_grouping::GroupingMode,
    pub groups: Vec<(String, usize)>,
    pub collapsed_groups: Vec<String>,
    pub list_layout: bool,
    pub inspector_sections: Vec<&'static str>,
    pub inspector_context: &'static str,
    pub bulk_action: Option<&'static str>,
    pub bulk_source: &'static str,
    pub statistics_open: bool,
    pub dialog_reads: FileManagerDialogReads,
    pub tag_picker_open: bool,
    pub media_actions_open: bool,
    pub media_actions_id: Option<i64>,
}

#[derive(Clone)]
pub struct FileManagerVersionState {
    pub media_id: i64,
    pub path: String,
    pub state: Option<fileatlas_plugin_versions::FileVersionState>,
    pub error: Option<String>,
}

#[derive(Clone, Default)]
pub struct FileManagerDialogReads {
    pub statistics_state: &'static str,
    pub statistics_error: Option<String>,
    pub photos: Option<i64>,
    pub videos: Option<i64>,
    pub audio: Option<i64>,
    pub rating_counts: Option<[i64; 6]>,
    pub bulk_state: &'static str,
    pub bulk_error: Option<String>,
    pub bulk_count: Option<usize>,
    pub bulk_preview_ids: Vec<i64>,
}

impl FileManagerService {
    pub fn ui_snapshot(&self) -> Option<FileManagerUiSnapshot> {
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
        self.request_ui(FileManagerUiRequest::OpenMediaActions(media_id));
        Ok(())
    }
    pub(crate) fn publish_ui_snapshot(&self, snapshot: FileManagerUiSnapshot) {
        if let Ok(mut current) = self.ui_snapshot.lock() {
            *current = Some(snapshot);
        }
    }
}
