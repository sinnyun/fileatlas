# Gallery plugin ports / 图库数据对接

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
- Statistics opens the source database query and displays image/video totals, total storage, rating distribution, top ten tags, and the latest twelve media months. Query errors are shown in the panel.
- When TaggingService is available, Gallery samples the writeback-failure registry and surfaces each metadata error with Retry and Dismiss actions; the 200 ms service monitor notifies the view when asynchronous writes complete or fail. Retry calls `writeback::retry_failure(path)` to enqueue the exact failed payload; dismissal calls `writeback::clear_failure(path)`.
- Session scroll offsets are stored in pixels through the uniform list's underlying scroll handle. Rubber-band selection starts on empty gallery space, replaces selection on plain drag, adds to selection on Ctrl+drag, and clears selection on a plain empty-space click. It maps the drag rectangle through the uniform list's measured row height, viewport bounds, and scroll offset, considering only rows that intersect the visible rectangle. AI inference and suggestion assignment run through the Preview AI surface; non-image card requests report that suggestions require an image.
- Preview receives the active query's complete ordered `(path, media_id)` sequence, resolved from compact gallery skeleton IDs, so image/audio/video navigation retains media identity beyond the currently loaded rich-media page. Rows deleted during resolution are skipped without shifting path/ID pairing.
- Configured `Settings.open_key` opens the selected item when the Gallery root has focus and no bulk/delete confirmation is active (default `Space`; absent settings provider retains the current `Enter` behavior). F1–F9 settings mappings toggle their tag for the selected media, testing membership on the first selected item as in the source app; assignment/removal goes through `TaggingService`, preserving association/ancestor and metadata-write semantics. Successful toggles record source-shaped `AssignedTags`/`RemovedTags` entries when history is available.
- With media selected, the Gallery exposes a searchable hierarchical tag picker. Search matches tag names and full paths; selected choices can be assigned or removed from all selected media, or all assignments can be cleared. Successful operations reload results and record `AssignedTags`, `RemovedTags`, or pre-clear `ClearedTags` history entries when history is available.
- Successful Gallery rating changes record `RatingChanged` with the source's representative previous rating and clamped target rating; successful single-item rename records `Renamed` only after the indexed filename confirms the change. Ctrl+Z/Ctrl+Y call the optional `HistoryService`; completion status is visible in the Gallery and the query is reloaded after undo/redo.
- Current Gallery history producers are tag hotkeys and the tag picker, rating, and single-item rename. Delete, copy, move, and bulk file actions are not variants of the source `UndoEntry` and remain non-undoable. Settings open-key support depends on GPUI key names exposed by the adapter; Windows focus/shortcut behavior still needs native validation.

## Consumers and debugging

UI requests should use visible ranges and retain the query paired with the snapshot. After a filter or index event, reload the snapshot and reset selection/window state consistently. Inspect query arguments, offsets, and media IDs when rows or tags appear stale. Micro-thumbnail placeholders accept exactly 192 RGB bytes from the database row; malformed or missing blobs fall back to the neutral media placeholder. The Preview plugin provides the source-style show/hide information overlay and cached ffprobe video details. EXIF values and Windows file attributes/timestamps require local real-file verification; Windows visual parity remains unverified.

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
