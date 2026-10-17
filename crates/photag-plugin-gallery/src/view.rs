//! Native GPUI rendering for the windowed media gallery.

mod behavior;
mod branch_actions;
pub(crate) mod bulk_actions;
mod card;
mod card_menu;
mod card_motion;
mod directory_browser;
mod empty_state;
mod factory;
mod file_actions;
mod file_format;
mod filter_actions;
mod filter_row;
mod inspector;
mod keyboard;
mod monitor;
mod month_header;
mod read_models;
mod records;
mod regions;
mod render;
mod rows;
mod scope_actions;
mod selection_panel;
mod sliders;
mod statistics;
mod tagging_picker;
mod version_badges;
mod view_state;
mod writeback_failure;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{future::Future, path::PathBuf};

use ::gpui::InteractiveElement as _;
use ::gpui::{
    Bounds, ListAlignment, ListOffset, ListState, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, list, point, px, size,
};
use cordis::{Context as CordisContext, Routing};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::calendar::CalendarState;
use gpui_kit::component::date_picker::{DatePickerEvent, DatePickerState, DateTime};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_database::SortOrder;
use photag_plugin_drag_drop::DragDropService;
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_history::{HistoryService, UndoEntry};
use photag_plugin_quicklook::QuickLookService;
use photag_plugin_session::{MediaTypeFilter as SavedMediaType, SessionService};
use photag_plugin_settings::SettingsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::theme;
use photag_ui_bridge::{UiCommandInvoked, UiViewContribution};

use bulk_actions::{BulkActionKind, BulkActionSource};
use card::{render_media_card, render_media_details};
use rows::{GalleryRubberband, GalleryVisualRow, build_gallery_rows};

use crate::{
    GalleryService, GallerySnapshot, IndexRevision, SelectedFolder, SelectedPath, SelectedTag,
    filter::{GalleryFilter, MediaTypeFilter, SearchScope, parse_date_input},
    selection::{GallerySelection, Modifiers as SelectionModifiers},
};

pub(crate) use factory::gallery;

struct GalleryView {
    version_states: photag_ui_bridge::read_model::ReadModel<
        version_badges::StateKey,
        Vec<crate::GalleryVersionState>,
    >,
    _ready_task: Option<Task<()>>,
    month_motion: std::collections::HashMap<String, card_motion::CardMotion>,
    hovered_cards: HashSet<i64>,
    card_motion: std::collections::HashMap<i64, card_motion::CardMotion>,
    locale_revision: u64,
    service: Arc<GalleryService>,
    file_operations: Arc<FileOperationsService>,
    drag_drop: Arc<DragDropService>,
    quicklook: Arc<QuickLookService>,
    versions: Arc<photag_plugin_versions::VersionsService>,
    runtime: tokio::runtime::Handle,
    cordis: CordisContext,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
    history: Option<Arc<HistoryService>>,
    settings: Option<Arc<SettingsService>>,
    tagging: Option<Arc<TaggingService>>,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    ui_views: Arc<photag_ui_bridge::UiViewCatalog>,
    tag_picker_search: Entity<InputState>,
    tag_picker_selected: std::collections::HashSet<i64>,
    search: Entity<InputState>,
    date_from: Entity<DatePickerState>,
    date_to: Entity<DatePickerState>,
    date_calendar: Entity<CalendarState>,
    bulk_date_from: Entity<DatePickerState>,
    bulk_date_to: Entity<DatePickerState>,
    bulk_date_from_value: Option<chrono::NaiveDate>,
    bulk_date_to_value: Option<chrono::NaiveDate>,
    rename_input: Entity<InputState>,
    inspector_tag_name: Entity<InputState>,
    inspector_tag_loaded: Option<(i64, String)>,
    inspector_tag_draft: Option<inspector::tag_editor::TagDraft>,
    history_title: Entity<InputState>,
    history_branch_name: Entity<InputState>,
    history_read: photag_ui_bridge::read_model::ReadModel<
        (i64, String, u64),
        Vec<photag_plugin_versions::VersionEntry>,
    >,
    history_branches: photag_ui_bridge::read_model::ReadModel<
        (i64, String, u64),
        photag_plugin_versions::VersionGraph,
    >,
    history_generation: u64,
    history_selected: Option<(i64, String)>,
    history_create_open: bool,
    history_busy: bool,
    history_error: Option<String>,
    bulk_inputs: Vec<Entity<InputState>>,
    bulk_action: Option<BulkActionKind>,
    bulk_source: BulkActionSource,
    bulk_media_type: MediaTypeFilter,
    bulk_tag_ids: Vec<i64>,
    bulk_extension_exclude: bool,
    confirm_bulk_action: bool,
    confirm_bulk_action_ids: Option<Vec<i64>>,
    focus_handle: FocusHandle,
    filter: GalleryFilter,
    selection: GallerySelection,
    selected_directory: Option<String>,
    inspector_deselected: bool,
    observed_navigation_revision: u64,
    rubberband: Option<GalleryRubberband>,
    scroll: ListState,
    pending_scroll_restore: Option<f32>,
    columns: usize,
    row_layout: Arc<Vec<GalleryVisualRow>>,
    row_layout_key: Option<(usize, bool, SortOrder, HashSet<String>)>,
    thumb_size: f32,
    list_layout: bool,
    grouping_mode: photag_plugin_grouping::GroupingMode,
    before_descendants_grouping: Option<photag_plugin_grouping::GroupingMode>,
    restored_file_paths: Vec<String>,
    restore_in_directory: bool,
    selection_restore: photag_ui_bridge::read_model::ReadModel<(u64, Vec<String>), Vec<i64>>,
    show_selection_details: bool,
    show_tag_picker: bool,
    media_actions_id: Option<i64>,
    media_actions_item: Option<(i64, photag_plugin_database::MediaItem)>,
    slider_drag: Option<sliders::SliderDrag>,
    show_statistics: bool,
    show_records: bool,
    records_page: usize,
    record_read_revision: u64,
    location_picker_pending: bool,
    confirm_delete_missing: bool,
    record_candidates: std::collections::HashMap<i64, String>,
    records_read:
        photag_ui_bridge::read_model::ReadModel<(u64, u64), Vec<photag_plugin_database::MediaItem>>,
    statistics_read:
        photag_ui_bridge::read_model::ReadModel<(u64, u64), read_models::StatisticsRead>,
    bulk_read: photag_ui_bridge::read_model::ReadModel<String, bulk_actions::MatchResults>,
    collapsed_months: HashSet<String>,
    inspector_expanded: HashSet<&'static str>,
    tag_filter_exclude_mode: bool,
    extension_exclude_mode: bool,
    drag_allowed: Arc<AtomicBool>,
    quicklook_notice: Option<String>,
    quicklook_busy: bool,
    show_exif: bool,
    exif_cache: Option<(i64, Result<Vec<(String, String)>, String>)>,
    exif_read: photag_ui_bridge::read_model::ReadModel<(i64, u64, u64), Vec<(String, String)>>,
    operation_status: Option<String>,
    writeback_failure_signature: Vec<(String, String)>,
    previous_inspector_tab: photag_ui_bridge::refresh::ChangeGate<Option<String>>,
    confirm_delete: bool,
    snapshot: Result<GallerySnapshot, String>,
    snapshot_read: photag_ui_bridge::read_model::ReadModel<u64, GallerySnapshot>,
    snapshot_generation: u64,
    snapshot_directory_dirty: bool,
    directory_snapshot_scope: u64,
    directory_snapshot_started: std::time::Instant,
    visible_demand: Option<std::ops::Range<usize>>,
    viewport_read: photag_ui_bridge::read_model::ReadModel<
        (u64, u64, Vec<i64>, Vec<i64>),
        crate::GalleryWindow,
    >,
    inspector_fallback: Option<InspectorFallback>,
    inspector_read: photag_ui_bridge::read_model::ReadModel<i64, InspectorFallback>,
    selection_details_cache: Option<(i64, i64, Option<card::FileMetadataSnapshot>)>,
    selection_details_read:
        photag_ui_bridge::read_model::ReadModel<i64, (i64, Option<card::FileMetadataSnapshot>)>,
    selected_drag_paths_cache: Option<(Vec<i64>, Vec<PathBuf>)>,
    selected_drag_read: photag_ui_bridge::read_model::ReadModel<Vec<i64>, Vec<PathBuf>>,
    observed_index_revision: u64,
    last_saved_scroll: f32,
    previous_scan_progress: photag_ui_bridge::refresh::ChangeGate<Option<(usize, usize, String)>>,
    previous_popover: photag_ui_bridge::refresh::ChangeGate<Option<String>>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct InspectorFallback {
    media_id: i64,
    media: Option<photag_plugin_database::MediaItem>,
    tags: Vec<photag_plugin_database::Tag>,
}

impl Drop for GalleryView {
    fn drop(&mut self) {
        self.persist_filter_session();
    }
}

pub(super) fn writeback_failure_signature(tagging: &TaggingService) -> Vec<(String, String)> {
    tagging
        .writeback_failures()
        .into_iter()
        .map(|failure| (failure.path.display().to_string(), failure.error))
        .collect()
}

pub(super) fn format_bulk_outcome<'a>(
    verb: &str,
    total: usize,
    succeeded: usize,
    failures: impl Iterator<Item = (i64, &'a String)>,
) -> String {
    let failures = failures
        .map(|(media_id, error)| format!("{media_id}: {error}"))
        .collect::<Vec<_>>();
    let mut result = format!("{verb} {succeeded} of {total} selected items.");
    if succeeded < total && failures.is_empty() {
        result.push_str(" Some items were missing or had destination collisions.");
    }
    if !failures.is_empty() {
        result.push_str(" Errors: ");
        result.push_str(&failures.join("; "));
    }
    result
}
