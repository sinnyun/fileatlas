# Gallery plugin ports / 图库数据对接

## Everything filename/path search (2026-10-17)

Gallery optionally reads `EverythingSearchService` (`photag/everything-search`)
for filename/path text searches. The adapter scopes current-directory queries
to that directory, collection searches to registered roots, and searches the
whole Everything index when there are no roots. `GalleryService::prepare_everything_search`
is called in snapshot and page reads, which already run off the GPUI render
path. Candidate paths are a prefilter; SQLite reapplies the checked filename,
path and tag scopes and retains library, type, extension, date, rating,
grouping, ordering and pagination behavior. Browser mode materializes real SDK
hits through `cache_search_files`; an unvisited nested file can therefore appear
without recursive scanning. Search inventory is session-only. Normal/extended
Windows paths reuse existing row identities and preserve annotations.
Missing adapter, unavailable IPC, unsupported platform, or oversized results use
SQLite fallback. Search can prepare an installed compatible background client.
After index loading, one readiness revision causes the existing monitor to retry
the current search; idle checks do not notify. The search bar uses a ReadModel
for EXE installation/version reads, shows install/compatible-version prompts,
opens only the official download page, and offers explicit rechecking. See `../photag-plugin-everything-search/PORTS.md` and
`docs/validation-ui/2026-10-17-everything-search/README.md`.

## Original-file viewer replacement — 2026-10-16

This section supersedes historical Preview/MediaPlayback references below.
Gallery requires `QuickLookService` (`photag/quicklook`) instead of the removed
PreviewService and MediaPlaybackService. Space, Enter, the retained custom open
key, file double-click, context-menu Preview, and inspector-thumbnail click send
all ordinary file kinds to the installed QuickLook. Calls run on a blocking
worker, with one pending Gallery launch at a time; held/modified Space does not
toggle repeatedly. A failure opens the shared GPUI alert, including a localized
instruction to install QuickLook when absent. There is no embedded viewer,
filmstrip, decoder, playback transport, install workflow or default-player fallback.

`GalleryUiRequest::Preview(i64)` retains its public name and now invokes that
same handler. `preview_items_by_ids` was removed with its filmstrip consumer;
`media_paths_by_ids` and `media_path_pairs_by_ids` remain for other operations.
Gallery file mutations retain selection repair, without advancing/reopening a
deleted viewer. Thumbnail generation, tag preview snapshots, inspector previews,
ratings, tags, EXIF, file history, drag/drop and file operations retain ownership.

AI suggestions moved from the removed Preview crate to Gallery's independent
`AiTarget` and `gallery/ai-suggestions-view` contribution. Explicit Suggest Tags
selects a target and requests inference without opening QuickLook. Automatic
analysis on file viewing follows the existing `settings.ai.auto_analyze` flag.
Polling still uses shared refresh and only notifies for actual changes.
Passive model-file availability uses a revision-keyed ReadModel, refreshed on
target changes and download completion; render and timer ticks do not stat the
model file repeatedly. Explicit Analyze retains fresh action-time validation.

Runtime limits and commands: `../photag-plugin-quicklook/PORTS.md` and
`../../docs/validation-ui/2026-10-16-quicklook/README.md`.

## Context inspector and isolated visual fixtures — 2026-10-16

The inspector projects Program / Tag / Folder / File from the actual selection
and current Gallery snapshot. `GalleryUiSnapshot::inspector_context` publishes
`program|tag|folder|file`. Clearing selection shows Program while preserving the
active path and counts. An internal Navigation action revision restores folder
or tag context even when the same target is selected; background data changes
preserve explicit deselection. No parallel navigation model is introduced.

Tag rename/appearance/display actions use NavigationService and preserve its
existing descendant behavior. `GalleryUiRequest::InspectorTagName(String)` fills
the live name InputState for MCP verification, without saving. The expanded
editor orders name, color, icon, display and one Confirm button. Color/icon/display
controls update a view-owned draft; Confirm revalidates the target and invokes
the existing rename/appearance/display services only for changed fields. These
are sequential existing commands, not a new atomic DB transaction; failures use
operation status and reload the model. An unchanged display flag does not
reapply descendant visibility when changing only name/color/icon. Icon selection
uses the 14 existing theme identifiers; unknown persisted icons/colors are
preserved until that field is changed. Folder statistics come from the current browse
projection; unopened child folders show unknown counts instead of fabricated 0.

History is separate from global undo/redo. The earlier visual fixture shown in
`docs/validation-ui/2026-10-16-inspector-complete/README.md` was a bounded
in-memory demonstration and is now historical evidence only.

## Lore-backed file history — 2026-10-17

The File > History page now reads and writes actual Lore revisions through
`VersionsService`; version titles, parent revisions, timestamps, size, preview
path and active revision state are held by the separate `VersionInfoService`.
Create asks for a title. Switching refuses to discard uncommitted source or
Lore worktree changes. On success, Gallery refreshes filesystem-derived media
metadata and image thumbnails while retaining ratings/tags/capture metadata.
Global undo/redo remains a separate action history.

MCP exposes `list_file_versions`, `create_file_version` and
`activate_file_version`. Writes remain disabled unless explicitly enabled in
the MCP process. See `../photag-plugin-versions/PORTS.md` and
`../photag-plugin-version-info/PORTS.md` for service identities, storage and
limits. Windows runtime/MCP exercise remains unverified until performed against
the same-process PhoTag MCP server.

Failures use the existing operation status, Navigation errors, thumbnail
fallback and read-model errors. Implementation is under `src/view/inspector/`.
Design and evidence: `docs/reference-ui/concepts/right-inspector-complete-2026-10-16-v2.md`
and `docs/validation-ui/2026-10-16-inspector-complete/README.md`.

User follow-up supersedes the compact history identity: all File tabs now share
the same `inspector_preview` (180 logical-pixel image, filename, rating), header,
and fixed tab row. Only the scrollable body changes. Latest design/evidence:
`docs/reference-ui/concepts/right-inspector-unified-edit-2026-10-16.md` and
`docs/validation-ui/2026-10-16-inspector-unified-edit/README.md`.

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: the query/window service and source-backed Gallery interactions are implemented, including virtualized rendering, filters, selection, preview routing, file actions, and bulk actions. C0–C4 coding continuation changes are written but unverified; Windows behavior and full visual parity remain unverified.

## Lifecycle and dependencies

- Cordis plugin name: `photag.gallery`; service: `photag/gallery` (`GalleryService`).
- Requires `DatabaseService`, `FileOperationsService`, `DragDropService`, `MediaPlaybackService`, `PreviewService`, and `SessionService`; registers its view/contributions in the Cordis-owned UI bridge.
- Optionally consumes `HistoryService`, `SettingsService`, and `TaggingService`. Gallery starts without these providers; history shortcuts/tag hotkeys/configurable opening are inactive when their provider is absent.
- Observes `FolderSelectionChanged`, `TagSelectionChanged`, `DirectoryPathSelected`, and `MediaIndexChanged`.

## Input / output

- `load(MediaQuery)` returns `GallerySnapshot`: compact ordered skeletons, total count, initial rich media window (up to 200), batched media tags, the full tag tree for naming active tag filters, available extensions, folder counts, and retained query.
- `ensure_window(snapshot, range)` returns `None` when current window covers range, otherwise a `GalleryWindow` with offset, media rows, and batched tags. `load_page(query, offset)` loads the next 200 rows.
- `media_by_id(id)` resolves an indexed row; `open_count(id)` reads the original informational counter; `apply_rating(ids, rating)` persists source-compatible clamped ratings. Filter mapping lives in `filter.rs`; selection behavior in `selection.rs`.
- The `Preview` button, Enter, and card double-click open the selected item through `PreviewService`. Its filmstrip follows the complete filtered media sequence: images render in Preview, audio shows the source-style dark stage with filename/rating/tags/default-player action, and video hands off to `MediaPlaybackService`. Left/Right navigation crosses all three media kinds in query order. Escape and `Return to gallery` close video playback. `Open` continues to launch the system default application.
- Starting a card drag calls `DragDropService::start_file_drag` on GPUI's drag-start callback. A selected card supplies the selected paths in selection order; an unselected card supplies only its own path. Ctrl/Shift-modified drags are suppressed, matching the source gallery. The native service starts a copy-only Windows CF_HDROP drag.
- The selected-item details area reads filesystem read-only/modified/created values, formats size and image dimensions/aspect ratio, displays the persisted open count, rating, capture date and hierarchical tags. Filesystem metadata failures are treated as unavailable fields.
- For selected images, `Show EXIF fields` lazily reads the source `kamadak-exif` 0.6.1 parser output, shows the original curated camera/exposure/GPS fields followed by the complete EXIF field list, and hides those fields again on request. Selection changes close the EXIF section so a new file is not read during each ordinary gallery render.
- The GPUI view restores and persists the source session's search, sort, media type, extension, date, rating, and tag filters through `SessionService`. Filter persistence preserves unrelated navigation/session fields. Search uses an input; date bounds use GPUI Kit calendar date pickers initialized from saved dates. Active include/exclude tag filters show individually removable pills with tag names (or a deleted-tag marker).
- Extension filter clicks follow the persisted source Select/Exclude mode in `SessionState.ext_click_mode`; the controls let users switch modes and each click toggles membership in that mode. Include/exclude filter pills remove their individual values, and Clear removes both sets.
- Search-scope defaults load from optional `SettingsService`; toggling filename/path/tag scopes in Gallery persists the complete scope set through that service. Settings write failures are shown in the Gallery status line.
- The GPUI view restores saved gallery selection IDs only when they remain in the active query and restores the saved pixel scroll offset. It persists selection/filter changes, changed scroll offsets (sampled every 200 ms), and the current state when the view is dropped. Changing the query clears selection and returns the scroll to the top. A process terminated before the next save interval may lose the latest scroll movement.
- Thumbnail size (100–300 px), date-sorted month grouping, and collapsed month sections restore from and save to `SessionService`; month grouping also reads and updates `SettingsService.show_date_dividers`, including changes made from the Settings panel. The default thumbnail size is 160 px and month grouping defaults on.
- The GPUI `uniform_list` renders visible media rows plus two rows of overscan. Its column count is derived from the current window width and thumbnail size; the list requests `ensure_window` for the visible global row range and keeps only the service's 200-row rich-media cache. Compact skeletons continue to drive keyboard navigation and month boundaries. Gallery cards display cached 8×8 RGB micro-thumbnails behind the asynchronous cached image. Their action menu provides AI Suggest Tags, Preview, open in the system default application, reveal in File Explorer, hierarchical tag assignment, copy, move, Recycle Bin deletion confirmation, and 0–5 ratings. Open/reveal and AI suggestions target the clicked card; tag, copy, move, delete, and rating target the current selection when the clicked card belongs to it, otherwise only that card. Tag changes use `TaggingService` and record source-shaped history entries when available. Preview and AI suggestion requests include the indexed media ID; the AI action opens the clicked image through `PreviewService`, whose AI suggestions surface observes the active image and starts inference.
- The `open_media_menu` Gallery UI request renders an application-owned action surface for an indexed ID, with Preview, open/reveal, rating, tags, AI suggestions, rename, copy, move, and delete. It reuses Gallery handlers and reports its live ID/open state; the native GPUI context menu remains pointer-triggered.
- `GalleryUiRequest::Preview(media_id)` exercises the same selected-card preview handler as double-click. It reuses the loaded `MediaItem` for immediate opening, sets the current rich-media window as the first filmstrip, and resolves the complete ordered result in the background. Late path results are ignored if another media item has become active.
- The optional selection-details popup reads open count and filesystem permissions/timestamps once on a background worker for its selected media ID; redraws use the cached result.
- Statistics opens the source database query and displays image/video totals, total storage, rating distribution, top ten tags, and the latest twelve media months. Query errors are shown in the panel.
- When TaggingService is available, Gallery samples the writeback-failure registry and surfaces each metadata error with Retry and Dismiss actions; the 200 ms service monitor notifies the view when asynchronous writes complete or fail. Retry calls `writeback::retry_failure(path)` to enqueue the exact failed payload; dismissal calls `writeback::clear_failure(path)`.
- Session scroll offsets are stored in pixels through the uniform list's underlying scroll handle. Rubber-band selection starts on empty gallery space, replaces selection on plain drag, adds to selection on Ctrl+drag, and clears selection on a plain empty-space click. It maps the drag rectangle through the uniform list's measured row height, viewport bounds, and scroll offset, considering only rows that intersect the visible rectangle. AI inference and suggestion assignment run through the Preview AI surface; non-image card requests report that suggestions require an image.
- Preview receives the active query's complete ordered `(path, media_id)` sequence asynchronously, resolved from compact gallery skeleton IDs, so image/audio/video navigation retains media identity beyond the currently loaded rich-media page. The loaded window provides immediate first paint; rows deleted during resolution are skipped without shifting path/ID pairing.
- Configured `Settings.open_key` opens the selected item when the Gallery root has focus and no bulk/delete confirmation is active (default `Space`; absent settings provider retains the current `Enter` behavior). F1–F9 settings mappings toggle their tag for the selected media, testing membership on the first selected item as in the source app; assignment/removal goes through `TaggingService`, preserving association/ancestor and metadata-write semantics. Successful toggles record source-shaped `AssignedTags`/`RemovedTags` entries when history is available.
- With media selected, the Gallery exposes a searchable hierarchical tag picker. Search matches tag names and full paths; selected choices can be assigned or removed from all selected media, or all assignments can be cleared. Successful operations reload results and record `AssignedTags`, `RemovedTags`, or pre-clear `ClearedTags` history entries when history is available.
- Successful Gallery rating changes record `RatingChanged` with the source's representative previous rating and clamped target rating; successful single-item rename records `Renamed` only after the indexed filename confirms the change. Ctrl+Z/Ctrl+Y call the optional `HistoryService`; completion status is visible in the Gallery and the query is reloaded after undo/redo.
- Current Gallery history producers are tag hotkeys and the tag picker, rating, and single-item rename. Delete, copy, move, and bulk file actions are not variants of the source `UndoEntry` and remain non-undoable. Settings open-key support depends on GPUI key names exposed by the adapter; Windows focus/shortcut behavior still needs native validation.

## Consumers and debugging

UI requests should use visible ranges and retain the query paired with the snapshot. Filter changes reset selection/window state; `MediaIndexChanged` reloads the snapshot while preserving still-valid selected IDs, the anchor, and the pixel scroll offset. Startup folder reconciliation emits this event as rows are committed, so files added to older indexes appear in the open Gallery. Inspect query arguments, offsets, and media IDs when rows or tags appear stale. Micro-thumbnail placeholders accept exactly 192 RGB bytes from the database row; malformed or missing blobs fall back to the neutral media placeholder. The Preview plugin provides the source-style show/hide information overlay and cached ffprobe video details. EXIF values and Windows file attributes/timestamps require local real-file verification; Windows visual parity remains unverified.

## Filter-driven bulk actions

The gallery's “Bulk actions…” panel ports the source `BulkActionState` workflow: choose Delete (Recycle Bin), Copy to, or Move to; scope matches to the whole library, the selected folder/path, selected tag(s), or current selection; then apply AND-combined date range, older/newer-than day count, inclusive rating range, filename substring, extension, and file-size bounds. It shows a live match count from compact gallery skeletons and uses those same IDs at execution. Current-selection mode deliberately skips the extra filters. Each open resets action, scope, and filter inputs to the source defaults. Ctrl+Shift+D opens the panel only while the Gallery owns focus, no preview/delete confirmation is active, and the panel is closed; it cannot reset an in-progress bulk configuration. Escape closes the panel. Bulk delete requires a second explicit confirmation click tied to the first click's matched ID snapshot; if the target IDs change, the new set must be confirmed again. The Gallery selection is cleared when its query reloads, and a selection change between confirmation clicks is caught by the ID snapshot check. Bulk copy/move use native folder pickers and `FileOperationsService` per-item outcomes; cancelling a picker closes the panel. Moving outside registered roots leaves the file on disk and removes its row from the library, as in PhoTag. Move/delete refresh the gallery after completion. Single-item copy/move use source-compatible save-file pickers.

Bulk date bounds use independent GPUI Kit calendar pickers with the source's optional-date behavior: each bound begins unset, selecting a calendar day applies an inclusive local-calendar bound, and the adjacent clear button removes that bound. Changing either date clears a pending delete confirmation so the live match set must be reviewed again. Current-selection bulk previews resolve every selected ID through `GalleryService::media_by_id`, so they cover selections outside the active 200-row media window and report deleted rows instead of showing a partial set. Native file-operation behavior, folder picker cancellation, Recycle Bin behavior, keyboard focus, and bulk matching remain unverified on Windows until the concentrated Windows validation pass.

## Shared visual tokens

The GPUI gallery, cards, statistics and bulk-action surfaces consume photag-ui-bridge::theme for source palette surfaces, text hierarchy, danger messaging, selection accent, and backdrop-aware base/modal/card alpha. Toolkit text-size presets and spacing helpers still do not map to every source token; Windows visual parity is unverified.

## Region projections and MCP UI requests (2026-10-13)

One Gallery entity is projected into command.search/size/filters/tools,
gallery.content, workspace.status and workspace.modal. Projections observe the
same entity, sharing filters, selection, paging and session writes. Main content
no longer contains duplicate heading/search/options/action rows. Statistics and
bulk actions are floating surfaces. Selected details/rename/copy/move/ratings
remain available from the bottom status bar's extra-actions button.
Thumbnail size is continuous 80–300; rating uses a nearest-thumb range slider.
Source placement: main.rs:1007-1277 and filterbar.rs:600-821.

`GalleryService::request_ui(GalleryUiRequest)` queues Statistics, BulkActions,
CloseDialogs, Select(media_id) or Deselect. The live GPUI view consumes the request using
existing UI actions; this is asynchronous and does not inject OS input. MCP
validates media IDs before queuing selection. Requests replace an unconsumed
request, so clients must wait between operations. These UI requests do not run
bulk file operations. Cordis retains service ownership and all projections are
removed by the original Gallery registration cleanup.

The continuation adds typed `ConfigureBulk(action, source)` requests. The live
view uses the same reset and state transitions as its bulk buttons; MCP can open,
close, and configure the existing bulk surface and reads its current action/source
from `get_ui_state`. Card context menus remain pointer-triggered because the
GPUI Kit `ContextMenu` owns its state and only exposes right-click activation.
For deterministic access, `open_media_actions(media_id)` queues the visible
Gallery-owned media action surface for an indexed row; `control_gallery` accepts
`open_media_menu`/`close_media_menu`, and `get_ui_state` reports its live open
state and media ID. Its Preview, open/reveal, rating, tag, copy, move and delete
buttons invoke the existing Gallery actions.

Labels and core parameterized counts use the UI Bridge translator with English
fallback. Month collapse keys stay in source English; only display labels are
localized. MCP captures confirmed 9 fixture cards, bottom selected actions,
statistics and bulk surfaces in both languages. Hover filename/tag overlays,
120ms hover/140ms selection animation, exact month typography and scan-status
priority remain unverified or different from source; this is not full parity.

## v5 S2 region wiring (2026-10-13)

`view::gallery` now receives `Arc<UiSurfaceState>` from `UiBridgeService`.
The single Gallery entity also contributes `gallery/inspector` to
`gallery.inspector`, through the existing lifecycle-owned view registry.
The inspector closes `surfaces["inspector"]`; Shell observes that state every
100 ms. Basic info and tags start expanded; EXIF/path start collapsed. EXIF is
cached by selected media ID. Off-page media/tags use the existing Gallery and
Tagging service lookups. Removing a tag uses `TaggingService::remove` and
records `UndoEntry::RemovedTags`, then restores selection after reload.

`command.filters` uses GPUI Kit popovers for type, tags, extensions, dates,
ratings and sort. Single-choice popovers dismiss through their own entity;
multi-choice popovers stay open and read the current include/exclude state.
Tag selection emits `TagFilterSelectionChanged` (shared navigation scope);
its include/exclude click mode is transient. Extension mode retains the
source `ext_click_mode` persistence. `command.tools` dispatches the existing
`metadata-export/export` command through `UiCommandInvoked`. Status actions
reuse preview, tagging and file-operation services. Runtime evidence and
remaining designed states are recorded in `docs/redesign-validation.md`.

## Current handoff / v5 control contracts (2026-10-14)

`src/ui_requests.rs` exports `GalleryUiRequest` and `GalleryUiSnapshot`.
Requests now include search, media type/sort, rating range, extension/tag
include/exclude, date bounds, clear filters, inspector section, tag picker,
and month grouping, in addition to existing selection/dialog/hover requests.
`GalleryService::ui_snapshot()` reads the state published by the live Gallery
render; it includes filter, selected IDs, matched count, grouping and expanded
inspector sections. It is not an independent database query.

GPUI `ListState/list` now accommodates differing month-header/card-row
heights; pixel scroll persistence, visible-range service caching, keyboard
reveal and measured-row rubber-band mapping were adapted. The service still
keeps the 200-row rich-media cache. Real nine-item MCP screenshots show the
spacing correction; cross-page selection, restore and scale behavior remain
to be verified. The obsolete `uniform_list` descriptions above are historical.

Filter popovers share `surfaces.value("gallery.popover")` with native trigger
open/dismiss callbacks and plugin MCP controls. Date Calendar range selection
and 04/06 bulk/tag picker layout are the latest unverified edits. Bulk state
adds MediaTypeFilter, tag IDs and extension exclusion; original scopes and
confirmation-bound file-operation execution are retained. Current Gallery
compilation fails on 19 missing type/trait/visibility errors in these new
panels. See docs/development-handoff-2026-10-14.md for exact diagnostics and
pending public-interface/runtime validation; do not claim this work complete.

The right Inspector keeps the selected item's thumbnail preview, filename and
editable rating fixed above independently scrollable Information, History and
Extensions tabs. Clicking the thumbnail calls the existing `PreviewService`
route. The Extensions tab reads executable contributions from
`UiViewCatalog::snapshot()` whose slot begins `gallery.inspector`; an empty
catalog shows an explicit unavailable state. History only displays actual
`HistoryService` undo/redo descriptions and labels them global, since the
service does not expose per-media history. The active tab is stored in
`UiSurfaceState` as `gallery.inspector.tab` with values `information`,
`history`, and `extensions`; MCP can set this key through the shared surface
state. The inspector layout and new copy still require the final PhoTag MCP
visual pass.

## 2026-10-14 current implementation and MCP evidence

UI requests SelectMany(Vec<i64>) and ScrollTo(usize) support explicit indexed selections and zero-based current-result scrolling. GalleryUiSnapshot publishes window_offset/loaded_count. Unmeasured virtual rows use logical ListOffset, measured visible rows retain reveal behavior. Scroll page changes notify publication; production page size remains200. Reference00/01/04/06/07/08 layouts and real 100012-row last-page200, cross-page bulk2, compact writeback os32/retry/dismiss were checked through PhoTag MCP. Native pointer/drag/window restore remain unverified. Earlier19error notes are historical; current Shell build passes.

Finalv2: Inspector pages controlled by gallery.inspector.tab(information/history/extensions), watched every200ms. Preview fixed220pt and basic/tag/exif/path content independently scrolls; preview/open uses existing service. New Add-ons labels preserve Extensions meaning for filenamefilter. Thumbnail selected check overlay removed; blue border now packs realRGBA alpha. Runtime matched3/selected7, preview retained during page switches and minwindow; history is existing globalundo/redo summary, right extension slot presently empty.

## Grid/list and stable card menus (2026-10-14)

`GalleryUiRequest::Layout(bool)` selects list(true)/grid(false); `GalleryUiSnapshot.list_layout` publishes the live choice. Top command.size projection supplies Grid/List buttons beside the existing size control. List uses one media per virtual row with56pt thumbnail, filename, localized type/dimensions/file size/rating; it shares query/order, 200-row rich cache, selection, drag/tag drop, context menu and preview handlers. Grid/list switching invalidates row layout without changing filters or selected IDs. `SessionState.gallery_list_layout` saves/restores the choice and serde-defaults false for existing sessions. Rubber-band bounds use the entire list row.

Card action buttons remain mounted when hover fades (minimum opacity0.08) so GPUI Kit does not drop the dropdown owner when the pointer enters its menu. Direct right-click still uses ContextMenu with the same card_menu builder. Actual menu movement, Ratings submenu and Preview execution were exercised through same-process MCP pointer events.

## All-file cards (2026-10-14)

The All filter includes generic `file` rows without special exclusions. Grid and list thumbnails use a shared extension/type icon when no usable thumbnail exists (including image-load failure); the inspector uses the same icon resolver. Image/video/audio icons and document/archive/code/font/generic icons are UI categories, not claims of decoder support. Every card has a noninteractive uppercase extension badge, or `FILE` for no extension; stable extension hashing supplies its background color. The upper-right ellipsis button is removed. Pointer-triggered context menus and their existing handlers remain available. Unsupported inline preview continues to report the existing error; default-application Open is available in the context menu. Existing type/extension/tag/search filters continue to apply normally.

## Grouping provider (2026-10-14)

Gallery now requires `GroupingService` from `photag.grouping` in the production Cordis graph. Group rules, stable ordering and virtual section calculation live in photag-plugin-grouping. Existing filter/tools month toggle is a seven-choice grouping popover. `load_grouped(query, mode)` returns GallerySnapshot.group_labels aligned with skeletons plus groups `[label,count]`; initial/visible rich windows resolve these arranged IDs (200-row normal cache), rather than applying the original SQL offset to a reordered view. `load(query)`/`load_page(query, offset)` remain ungrouped public query APIs. Select/keyboard/Preview/filmstrip use arranged skeleton order. Mode changes preserve surviving selected IDs and reset scroll; switching modes clears collapse keys. Session saves the additive mode string plus current collapsed labels; old month keys migrate in memory. Source date grouping's capture/modification fallback remains; explicit Modified/Created use different documented timestamps. Legacy Settings show_date_dividers controls Month/None; specialized grouping modes are not overridden by the old boolean. `GalleryUiRequest::Grouping(GroupingMode)`/`ToggleGroup(String)` and snapshot.grouping_mode/groups/collapsed_groups extend MCP access. Narrow filter rows scroll horizontally and Tools also lists grouping choices.

## Group title bands (2026-10-14)

The shared group header now paints an opaque blue-grey fill, fine outline and hover fill for every mode and grid/list layout. The40pt height/count/collapse public behavior is unchanged. Same-process MCP pointer collapse and rendered states are recorded in docs/validation-ui/2026-10-14-group-header/README.md.

Subfolder ordering now calls GroupingService::arrange_in_scope with query.path_prefix. Current directory direct files precede descendants, including when the selected directory is itself nested; unscoped queries place registered root direct groups first. No group labels, snapshot/window/selection contract or section geometry changed. See docs/validation-ui/2026-10-14-root-first.

## Ungrouped directory browser (2026-10-14)

`GallerySnapshot.browser: BrowserSnapshot` describes active/current directory, bounded parent, immediate GalleryDirectory{name,path} cards and an optional listing error. `load_grouped(query, GroupingMode::None)` now projects the selected path/root's immediate files through MediaQuery.direct_directory and reads actual child folders through IndexingService::try_list_subdirectories. With no directory it shows registered roots; global text search and tag views retain recursive file results. Other grouping modes and public load/load_page remain recursive. Gallery rows have separate directory_range/media_range and account for directory columns in rubber-band geometry; folders do not get MediaSkeleton/MediaItem IDs. The200-row rich cache and filmstrip remain file-only.

Folder selection is ephemeral; actual location/layout use existing Session fields. Double-click/Enter queues public NavigationService::request_ui(SelectPath), Up/Alt+Up selects parent or AllFiles, Refresh/F5 re-reads. Shell enables Navigation/Indexing alongside Gallery; Gallery-only service tests can still use recursive load without UI providers. SelectedFolder bulk scope is immediate in browser mode and rejects unselected virtual roots; WholeLibrary stays global. New UI requests OpenDirectory(String), DirectoryUp, Refresh and snapshot browsing/directories/current_directory/parent_directory/directory_error/selected_directory support same-process MCP. See docs/validation-ui/2026-10-14-folder-browser for precise runtime/test limits.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.

## Dialog read models (2026-10-15)

Statistics and bulk-match preview load through UI Bridge ReadModel on the
existing Gallery model poller, not during render. Statistics caches by index
and Database writer revision for the open dialog. Bulk preview caches by source, folder/path/tag
scope, selected IDs, all raw filter fields/dates, index and writer revision. Relative
dates are fixed for that preview request. CurrentSelection still bypasses
filters, and execute revalidates current targets before any file operation.
Errors are cached; editing a field retries with a new key. Dialog close drops
the read model; no database schema/file operation order changed.
`GalleryUiRequest::BulkField(usize,String)` updates the real input and clears
delete confirmation. `GalleryUiSnapshot::dialog_reads` exposes typed
GalleryDialogReads (statistics state/error/photo/video/audio counts; bulk
state/error/matched count/loaded preview IDs), with no extra queries. Statistics
also publishes rating_counts. `GalleryService::data_revision() -> u64` forwards
the Database writer clock, so direct service mutations also invalidate reads.

Inspector/selection-detail EXIF uses the same keyed background boundary, with
selected ID + index/writer revisions. A loaded/fallback media path is reused;
otherwise its lookup is done once by the worker. Missing source/EXIF/errors
are cached. Rendering and expansion actions never query a path or read EXIF;
deselect/multiple selection/closing EXIF abandons the read model. Late results
cannot populate the next selection. `exif_cache` is its paint-only projection.

## Unified passive UI models (2026-10-15)

Gallery monitor now returns one aggregated change flag to UI Bridge refresh::poll. Surface/progress signatures use shared ChangeGate. Statistics, bulk, EXIF, off-window Inspector fallback, single-selection metadata and selected drag paths all use shared ReadModel. Render declares only nonblocking demand; completion delivery checks current selection and publishes existing snapshot fields. Errors/empty results are cached, never re-read on each paint. Existing index refresh clears selection-derived caches.
## Preview thumbnail handoff (2026-10-15)

`preview_items_by_ids(&[i64]) -> Result<Vec<photag_plugin_preview::FilmstripMedia>, GalleryError>`
returns existing source path, ID and optional thumb_path in requested order,
skipping missing rows with the existing query semantics. The visible loaded
window is projected without IO; full filtered-result resolution remains on the
background executor and applies only while the opened identity is active.
Preview consumes the public contract; Gallery does not own preview decoding.
No schema, filters, events or source file mutation changes. See Preview PORTS
for thumbnail freshness, fallback, generation cancellation and diagnostics.


## Browser runtime update — 2026-10-15

Production browsing is filesystem driven for every grouping. `live.rs` adapts Yazi VfsEntries/Refresher (MIT, pinned reference under vendor/yazi-reference). One directory worker, one latest pending request, cooperative cancellation, generation rejection, a bounded four-message channel, first batch 32 entries then 2048 files. The UI drains at most four messages per shared poll. Filesystem reads/stat and session cache batches run off-thread. Directories include empty/hidden directories; descendant traversal is explicit and does not follow symlink directories. Child read errors produce partial results plus an error.

Public controls: `browser_enabled`, `show_descendants`, `set_show_descendants(bool)`, `thumbnails_enabled`, `set_thumbnails_enabled(bool)`, `invalidate_directory`, `cancel_directory`, `poll_directory`, `directory_worker_active`. `directory_status() -> (u64 generation,u64 revision,bool active,usize scanned,Option<String> error)`. Active means view loading; worker_active separately reports an OS call/producer still running after cancellation. A blocked OS read or an already running decoder is not forcibly interruptible.

Gallery consumes WatcherService and IndexingService. Native watcher changes patch the session cache or invalidate a directory; explicit refresh rereads it; activation uses the continuously running watcher. Snapshots use ReadModel; row renderers declare demand which is merged once per poll. A 200-row rich window and a small visible overscan request thumbnails; stale generation/request/file versions cannot publish decoded results. Preview is still the existing prioritized decoder. The tools menu includes thumbnail enable/disable and missing-record maintenance. Session restores selection by path, not cached row ID. Favorite home, tag results, locate-and-select, grid/list, groups and existing action surfaces share the accepted layout.

Missing-record UI chooses exact files or proposes same-name candidates from one explicitly picked folder, then applies each row or skips. Delete missing records requires two-step confirmation and revalidates local missing status. No automatic move inference. See browser validation evidence for exercised Windows paths; codec, inaccessible-drive and native picker boundary coverage remains limited.


## Review corrections — 2026-10-16

Directory chunk changes coalesce while a snapshot read is in flight. A finished partial snapshot is delivered before the next directory revision is requested, preventing continuous key invalidation from starving the first result. Explicit navigation/filter reloads still invalidate old results. Refresh preserves selection/scroll without persisting a transient empty selection; unchanged SessionService saves do not write. Browsing the isolated profile itself and external file creation are checked through same-process MCP.


## Duplicate directory refresh fix — 2026-10-16

Window activation no longer invalidates the browser or retries its subscription. The current native/PollWatcher remains active while unfocused; actual file events and explicit refresh still update the view. LiveBrowser registers the selected watch before queuing enumeration; its worker waits cooperatively on WatcherService::browse_registration_pending (off the UI thread, cancellable). The old synthetic root change on registration is removed. Initial snapshots wait for the first current enumeration batch instead of briefly showing an earlier cached inventory.

Background refresh keeps existing rows, scroll, selection, dialogs and the current 200-row rich window until the new result arrives; load_grouped_window is a crate-private query variant preserving its offset. BrowserSnapshot.loading marks partial directory results; selection is validated only against complete delivered skeletons, so a version restore's temporary empty result cannot replace the file History inspector with folder information. Explicit navigation/filter reloads retain reset semantics. GalleryUiSnapshot.snapshot_revision:u64 is a read-only diagnostic for query scheduling, exposed as MCP gallery.snapshot_revision. Same-process lifecycle and external-change evidence: docs/validation-ui/2026-10-16-refresh/README.md.

## Tag assignment refresh scope — 2026-10-17

Inspector tag assign/remove/clear actions patch `GallerySnapshot.media_tags` and
the selected-file fallback cache for the affected media IDs. They do not reload
the gallery query or reset its list state. If an include/exclude tag filter is
active, assignment can change result membership, so the Gallery refreshes only
the current query window through `refresh_index`; that path retains the current
logical scroll anchor and selection while replacing the window. Explicit
filter/navigation reloads keep their existing reset behavior. Same-process
scroll-middle assignment evidence is in
`docs/validation-ui/2026-10-17-gallery-tag-scroll/README.md`.


## Address bar integration — 2026-10-16

Directory latency update: the view owns a shared Wake listener alongside its
200ms fallback. UI requests, all five scope/index observers, bounded producer
batches and snapshot/viewport ReadModel completions signal it after publishing
state. One sampler consumes both paths. First batch counts files plus directories
(32 entries); subsequent batches flush at 2048 entries or 32ms when nonempty.
Intermediate snapshot queries coalesce with a 50ms minimum spacing; first scope
and final completion bypass it. Logical row geometry preserves scroll even when
a second result arrives before GPUI measures reset rows. No extra persistence or
source-file writes. Evidence: docs/validation-ui/2026-10-16-navigation-latency.

The duplicate Up button is removed from command.size; Navigation now places Up before the full breadcrumb/address field. Gallery's folder cards, picker, locate and existing Alt+Up still route through NavigationUiRequest::SelectPath. Gallery-focused Ctrl+L and Alt+Left/Right enqueue public AddressBarRequest variants. Navigation owns history/input; Gallery does not implement a parallel history. Existing selection/file-operation undo history is separate and unchanged.

## Unsaved version indicator — 2026-10-17

Gallery requests `VersionsService::file_state` for up to 200 visible files through
`ReadModel`, keyed by snapshot/history/data revisions and file identity. IO runs
on a worker; rendering reads cached results and completion polling stops when idle.
`Modified` adds an amber dot at the thumbnail's top left in grid layout and before
the filename in list layout, avoiding overlap with small thumbnail format labels.
Its tooltip identifies the selected saved baseline. Clicking selects that file and
opens History; it never creates or switches versions. Unversioned files, an unchanged
old selected revision, and content matching another saved revision have no dot.
History explains the current baseline and any other matched revision. Read errors
are available in History and `GalleryUiSnapshot.version_states`; they are not
classified as modified. Snapshot fields also expose `version_states_loading`.
Real filesystem changes and `HistoryChanged` invalidate the cached state.
Validation: `docs/validation-ui/2026-10-17-version-status/README.md`.

## Search location switch — 2026-10-17

`GalleryFilter.search_entire_computer` maps to `MediaQuery.search_entire_computer`.
The top search button toggles current-folder/all-computer modes through
`GalleryUiRequest::SearchEntireComputer(bool)`, saving
`Settings.search_entire_computer` (serde default false). Active text search
normalizes the folder/path to a recursive `directory_tree`, independently of
the direct/recursive browsing setting. The all-computer mode clears folder,
path and live-cache restrictions and passes no roots to Everything; results
therefore follow Everything's configured indexed volumes, folders and exclusions.
It does not edit Everything settings. Other PhoTag filters remain active.
Without a selected folder the local mode retains registered collection roots;
its tooltip explains this fallback. Clearing text restores ordinary browsing
and the selected navigation location. Gallery changes do not replace Navigation
selection or history. Same-process evidence: Everything search validation README.
