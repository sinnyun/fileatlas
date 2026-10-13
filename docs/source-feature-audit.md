# PhoTag source audit and migration map

**Audit baseline:** `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` (2026-08-04),
PhoTag v0.2.0. Mirrored source: `vendor/photag-source/`. Source count:
18,434 Rust lines across 22 files. Migration status below was reconciled with
the latest implementation commits on 2026-10-13.
This audit catalogs source behavior and original locations, cross-checked
against the migrated implementation. It is not a claim that every internal
branch has been exhaustively behavior-tested.

This is the canonical current migration map. Its status prose describes code
reported as implemented; `unverified`, `validation pending`, and similar notes
are evidence gaps, not automatically missing implementations. Conversely,
compilation or a synthetic test is not proof of complete source parity. Older
summaries in `AGENTS.md`, `README.md`, and dated validation sections are
historical if they conflict with this map. Reconciliation did not establish a
blanket claim of full PhoTag equivalence.

## Feature-to-source mapping

The status column describes implemented behavior and remaining gaps. A
compiled crate or passing synthetic test does not by itself establish source
parity.

| Original feature / behavior | Original source | New destination | Status |
|---|---|---|---|
| App lifecycle, dialogs, pending action dispatch, undo/redo, selection state | `src/main.rs`, `src/app.rs` | shell + app-state/history plugins | Source-backed app actions, selection state, five history entry types and their producers are migrated. Gallery records ratings, tag assignment/removal/clear, confirmed rename and accepted AI tag suggestions; file actions repair selection/preview state after mutations. Shell stops Preview before its service providers and restores the session. On Windows MSVC, local startup validation now creates a native window for Gallery and dev-test profiles after raising the shell executable stack to 8 MiB. The source history has no copy/move/delete or tree-edit entries. The source only describes pick/reject glyphs and has no corresponding action handler, so there is no action behavior to migrate. Interactive lifecycle, dialogs and shortcut behavior remain unverified. |
| AI-assisted application observation and operation over MCP | No upstream MCP feature | `photag-plugin-mcp` + Shell window-capture handle | PhoTag-specific opt-in local stdio server (`--mcp`) exposes window PNG capture, library/media queries, preview/playback controls, scan controls, ratings/tags, metadata export, and file operations through Cordis services. All mutation tools require `PHOTAG_MCP_ALLOW_MUTATIONS=1`; selected higher-impact calls also require `confirm: true`, which is a call-site acknowledgement rather than proof of human confirmation. MCP client round trips and Windows `PrintWindow` fidelity remain unverified. This feature is outside upstream migration parity. |
| Folder and tag navigation, tree editing, folder actions | `src/ui/sidebar.rs`, `src/db/queries.rs` | `photag-plugin-navigation` + indexing/database/watcher plugins | Folder/tag trees, recursive counts, lazy nested folders, source-backed tag create/rename/delete/reparent/style/display/associations, folder add/rescan/remove, native folder picking and persisted selection/expansion are migrated. Tag rows expose merge-target search, media-impact delete confirmation, ancestor assignment choices, and before/into/after drag placement. Select/Exclude click mode restores from Session; merge child handling follows Settings. Dragging a tag onto a Gallery media card assigns it to that item or its current selection. The source has no folder-tree drag/drop behavior. Windows interaction and visual parity remain unverified. |
| Virtualized gallery data window, selection, grid, month groups | `src/ui/gallery.rs`, `src/app.rs::load_media/ensure_media_window` | `photag-plugin-gallery` + GPUI shell | Compact ordered skeletons and 200-row rich media pages remain. GPUI `uniform_list` renders visible rows with two-row overscan; column count follows available width and thumbnail size. Scroll near the loaded boundary loads the next page; keyboard targets load and scroll into view. Month grouping, persisted collapsed-month state, and 80–300 px card sizing are controllable. Gallery supports plain/Ctrl/Shift selection, keyboard selection/rating, selected-item details, named include/exclude tag pills, calendar date filters, and searchable hierarchical tag assignment. Cards paint cached image thumbnails with database 8×8 micro-thumbnails underneath. Card actions include Preview, open/reveal, hierarchical tag assignment, copy/move, confirmed delete, AI Suggest Tags and rating. Dragging a sidebar tag onto a card assigns it to the card or current selection. Rubber-band selection replaces selection on plain drag and adds to it with Ctrl+drag. Filtered bulk operations use native destination picking and ID-snapshot confirmation. Statistics displays totals, storage, rating distribution, top tags and recent months. Preview receives ordered path/media-ID pairs for its filmstrip. Source-sized square cards and month row spacing are being restored; Windows behavior and remaining visual parity are unverified. |
| Progressive image thumbnail decode, EXIF orientation, DCT-scaled JPEG decode, 8×8 micro-thumbnails, path-hash cache | `src/thumbs.rs` | `photag-plugin-thumbnails` | Original image thumbnail pipeline and cache are wrapped as a Cordis service; Gallery paints cached thumbnails over database micro-thumbnails and controls card size. Preview decodes low-resolution first and prewarms nearby images. Synthetic JPEG, PNG, WebP, TIFF and BMP thumbnail regression cases pass on Windows; representative user-media corpus, EXIF orientation corpus and UI behavior remain unverified. |
| Search, sort, date/rating/type/extension/folder filters | `src/ui/filterbar.rs`, `src/db/queries.rs`, `src/app.rs::load_media` | `photag-plugin-gallery::filter` + database plugin | Host-neutral state maps source query scopes, media type, include/exclude tags/extensions, inclusive dates, ratings and sort order into `MediaQuery`. GPUI exposes search/scope, media type, all eight sort orders, date pickers, rating bounds, persisted Select/Exclude extension mode, named include/exclude tag pills and clear actions. Query filters persist through `session.json`; search-scope defaults load from and save to Settings. The bulk-action panel uses independent calendar date pickers with clear controls. Windows behavior and visual parity remain unverified. |
| Image preview, progressive decoding, zoom/pan/rotate/flip, toolbar and filmstrip | `src/ui/preview.rs`, `src/thumbs.rs` | `photag-plugin-preview` + `photag-plugin-thumbnails` | Cordis preview service and lifecycle-owned GPUI surface support source-compatible image controls and decode behavior. Gallery supplies ordered path/media-ID pairs to a filmstrip. Transform/info state and toolbar offset persist through Session/Settings; F2 rename and confirmed Delete/Backspace use FileOperations. Audio preview has the source-style stage and actions; video items hand off to MediaPlayback. Video keyboard controls, FFprobe fallback to the default app, recent tags and collapsed picker-tree state are migrated. Windows visual parity and device behavior remain unverified. |
| Video/audio playback, synchronization, speed, rotation metadata, ffmpeg integration | `src/video.rs` | media-playback plugin | FFmpeg video frames and PCM audio play through a worker-owned Rodio sink. Audio-clock timing falls back to wall clock when the stream drains; seek and volume sliders, pause/frame-step, source speed presets, 0.05× adjustment, and 3×/4×/8× boosts are exposed. Video Preview routes Space/Left/Right/Escape to playback as in the source and opens unprobeable videos through the default application. Windows codecs/audio-device behavior and synchronization remain unverified. |
| EXIF and metadata display, dimensions, paths, open count | `src/ui/info.rs`, `src/ai.rs`, `src/db/queries.rs` | Gallery selected details + Preview info panel | Gallery selected-item details include filesystem read-only/modified/created values, formatted size, indexed dimensions/megapixels/aspect ratio, capture date, rating, hierarchical tags, and the original database open counter. The selected-image panel lazily parses source-compatible curated and complete EXIF fields with `kamadak-exif` 0.6.1. The preview info overlay/toggle parses filesystem and EXIF metadata and displays cached ffprobe video metadata. The pinned source exposes no generated-description feature; Windows visual parity is unverified. |
| Hierarchical tags, ancestor auto-apply/manual distinction, associations, merge, rename, reparent, colors/icons/order/display | `src/db/queries.rs`, `src/ui/sidebar.rs`, `src/ui/preview.rs` | navigation + `photag-plugin-tagging` | Database assignment, association expansion, automatic ancestor rows, manual/automatic distinction, removal/clear/merge and tree editing/style/display services are exposed through Cordis. Navigation supports association editing, source merge-target selection, media-aware delete confirmation/ancestor handling, and drag reorder. Gallery has searchable assignment/removal/clear across selected media with undo; Preview has a searchable tree picker with recent tags and persisted collapse state. AI suggestions enter source-compatible history. Background XMP write failures are surfaced in Gallery with retry/dismiss behavior. Windows behavior and writeback regression coverage remain unverified. |
| Ratings, pick/reject, keyboard shortcuts, tag hotkeys | `src/app.rs`, `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/settings.rs` | `photag-plugin-gallery` + command/keybinding contributions | Rating selection effects persist through the source bulk update query (0–5 clamp verified), with GPUI card selection, batch controls and 0–5 key handling. Arrow/Home/End/Ctrl+A/Escape selection keys are bound. Source Enter preview and the configurable open key are both handled unconditionally in `view/keyboard.rs::handle_key`; the action bar exposes Preview first selected for multi-selection, matching the source's selected-set action. Gallery and Preview delete actions use ID-bound Recycle Bin confirmation. Settings captures/persists the open key and F1–F9 tag targets; Preview supports F2 rename, fit/zoom keys, and image/video/audio filmstrip navigation. The source provides no pick/reject action handler; therefore those glyphs have no action to port. Windows shortcut/focus behavior and visual parity remain unverified. |
| Search tag assignment menus and context menus | `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/ui/sidebar.rs` | menu contributions owned by feature plugins | Gallery provides a searchable hierarchical tag picker for assign/remove/clear across selected media, with undo history. Card actions expose Preview, open/reveal, hierarchical tag assignment, copy/move, confirmed delete, AI Suggest Tags and rating; sidebar tags can be dragged onto cards. Preview's right-click menu exposes open/reveal, AI suggestions, rating and confirmed delete, and its toolbar has a searchable tag picker. Navigation tag rows provide edit, rename, show/hide chips, parent, save style, delete, and before/into/after drag placement. Windows interaction and visual parity remain unverified. |
| Recursive multi-thread scanning, hidden-file filtering, video/poster pairing, DB batches, image/video thumbnails, cancellation/progress, missing-thumbnail backfill, child directory listing | `src/scan.rs`, `src/thumbs.rs`, FFmpeg helpers from `src/video.rs` | `photag-plugin-indexing` + `photag-plugin-thumbnails` | Source scanning, video/poster pairing, database batching, cancellation/progress, thumbnail generation/backfill and child directory listing are implemented behind Cordis services. Synthetic regression coverage exists; full media corpus, FFmpeg deployment and Windows paths remain unverified. |
| Filesystem watcher and reconciliation | `src/watcher.rs`, `src/scan.rs`, `src/app.rs` watcher reconciliation | `photag-plugin-watcher` + indexing plugin | Original create/remove/rename filtering, hidden-file exclusion, 800ms debounce, 4,000-event drain and 200-path reconcile budget are retained. Cordis plugin watches registered folders, runs reconciliation in a bounded background poller, triggers missing-thumbnail generation for added rows, emits a Cordis media-index event after database changes, and stops the worker with the plugin. Native Windows tests passed for file create/remove reconciliation in the current second-round test run; Windows rename delivery and DB/file race cases remain unverified. |
| Recycle Bin delete, copy/move/rename/create folder, reveal/open, index follow-through | `src/app.rs`, `src/db/queries.rs`, `src/drag.rs` | `photag-plugin-file-operations` | Single/bulk copy, move, rename, folder creation and Recycle Bin delete preserve source operations and database follow-through. Windows temporary-directory tests now cover rename, in-library move, move outside the registered root, and destination collision; moved media outside all roots leaves the file on disk and removes its row. Gallery and Preview use native destination/confirmation dialogs, per-item outcomes, index refresh, selection repair, and filmstrip path/ID repair or advance. Recycle Bin, Shell dialogs, Explorer integration and cross-volume fallback remain unverified. |
| Drag-out to Explorer/drop targets via `CF_HDROP` | `src/drag.rs` | `photag-plugin-drag-drop` + Gallery | CF_HDROP/OLE service is wired to Gallery card drag-start; Windows delivery remains unverified. |
| Bulk action console and filter-driven delete/copy/move | `src/app.rs`, `src/main.rs`, `src/db/queries.rs` | Gallery + file-operations plugins | Filtered bulk panel supports scopes, AND filters, live counts, confirmation bound to a matching media-ID snapshot, native destination picking, and per-item copy/move/delete outcomes. It resets to source defaults on open, closes on Escape or canceled destination picking, and disables execution for zero matches or invalid filters. Date bounds use independent calendar pickers with per-bound clear controls. Windows behavior remains unverified. |
| JSON export of metadata | `src/db/queries.rs`, `src/main.rs` | `photag-plugin-metadata-export` | Source-compatible query, all-record ordering, rating default, hierarchical tag formatting, and pretty JSON export are available. A Cordis command and File menu item drive a lifecycle-owned GPUI panel; it accepts a destination path and reports progress, record count/path, or errors. The native Save File dialog uses a JSON filter and source default filename; overwrite behavior is delegated to the OS dialog. Repeated exports are single-flight and completion/failure feedback is visible in the panel. |
| Statistics modal | `src/main.rs`, `src/db/queries.rs` | `photag-plugin-gallery` + database plugin | Gallery exposes the source statistics query in a visible panel with media totals, storage, rating distribution, top ten tags, latest twelve media months, and visible query errors. Windows visual parity remains unverified. |
| AI MobileNetV2 suggestions, brightness/warmth/saturation analysis, mandatory groups, ONNX model download | `src/ai.rs`, `src/settings.rs` | `photag-plugin-ai` | The registered Cordis service provides MobileNetV2 inference, ImageNet mapping, image analysis, mandatory-group scoring and model download. Settings mandatory groups are passed into inference. Preview provides manual/automatic suggestion analysis, model download when manually requested, visible progress/errors, and source-group tag creation/application through Tagging; successful assignments enter undo history. Automatic analysis runs once per image per panel lifetime and only when the model is already available. Windows model/runtime validation remains unverified. |
| JPEG IPTC and XMP sidecar writing, restore original modified time | `src/xmp.rs`, `src/app.rs` | `photag-plugin-tagging::xmp` | Original XMP/APP13 implementation is reused; successful tagging service assignment/removal/clear/merge/reparent schedules best-effort metadata rewrites. A serialized worker coalesces same-path jobs and exposes path-keyed failures to Gallery, where users can retry or dismiss them. The workspace test pass validates tag assignment associations/ancestors/manual flags, but writeback byte compatibility, timestamps and failure/retry paths still need dedicated regression fixtures. |
| Settings persistence, accent presets, search scopes, tag search/merge behavior, toolbar config/position, update setting | `src/settings.rs`, `src/theme.rs` | `photag-plugin-settings` + `photag-ui-bridge::theme` | PhoTag-compatible settings schema and Cordis persistence are migrated. The floating six-section Settings modal has a toggle command/menu, drag, Close and Escape behavior. Controls expose appearance, toolbar lock/offset, shortcuts, mandatory AI groups, and update check/download/install status. A new-release offer presents notes and Update now/Not now; installation requires an explicit user action and saves session before shell replacement. Windows updater behavior and visual parity remain unverified. |
| Session restore: folder, selection, scroll, expanded nodes, sidebar width, filters | `src/session.rs`, `src/app.rs` | `photag-plugin-session` | Source-compatible session fields and Cordis persistence are migrated. Gallery restores selection/scroll, filters, extension click mode, thumbnail size, month grouping and collapsed months; Navigation restores selection, valid tag expansion and tag click mode; Shell restores sidebar width; Preview restores the last image, transforms/info and collapsed tag-picker nodes. Crash-time last-scroll durability remains limited. |
| Mica/Acrylic, dark title bar, system accent, fallback | `src/winfx.rs`, `src/theme.rs` | `photag-ui-bridge::winfx` + shared theme | Windows-gated registry accent/font probes and dark title bar/Mica/Acrylic APIs are implemented; the shell applies them after creating the GPUI HWND and Settings updates shared accent state. Source alpha levels are consumed through shared base (0xDE), modal (0xF4), and Gallery card (0xE8) RGBA helpers. Native material, registry and compositor behavior require Windows runtime verification. |
| GitHub update check, confirmation, download, SHA256 verification, staged swap/rollback | `src/update.rs`, `src/settings.rs` | `photag-plugin-updater` | The Cordis updater checks releases, presents an automatic version offer with notes and Update now/Not now actions, verifies and stages downloads with progress, and requires an explicit Restart & install action. Not now suppresses offers for the process; the Updates tab can reopen them. Install saves the latest session, schedules the staged binary, quits GPUI, then replaces/restarts from the shell; failures are logged. Windows replacement/restart remains unverified. |
| Theme, typography, custom fonts, icon availability, visual tokens | `src/theme.rs` | `photag-ui-bridge::theme` + GPUI Kit | Source palette, dimensions, font names, MDL2 glyphs, control states and accent presets are exposed. Premultiplied source colors convert to GPUI RGBA; shared surface, text, border, accent and transparency tokens style Gallery, Navigation, Settings, Preview, Metadata Export, Playback and Shell. Font resolution and Windows visual parity remain unverified. |
| SQLite schema, migrations, WAL, separate read connection, indexed queries | `src/db/mod.rs`, `src/db/queries.rs` | `photag-plugin-database` | Core schema/query source copied; Cordis service and separate connections implemented. Tests verify a pre-migration database preserves media/tag links and defaults after migration and reopen, and representative source filters/sorts (date/rating bounds, tags, search, extensions, folder/path and rated/date order). Full SQL edge cases, cascades and real-library compatibility still need broader fixtures. |

## Crate port documents and status source

Every workspace crate has a `PORTS.md` describing its Cordis identity, public
inputs/outputs, dependencies, event relationships, limits, and debugging points.
That includes the AI, MCP, updater, history, playback, preview, and drag/drop
crates as well as the original database and UI feature crates. Keep those guides
aligned with their public Rust APIs.

The feature-to-source table above is the single source of truth for migration
status. This section previously repeated selected implementation slices and
fell behind later work; it has been removed so an older summary cannot override
the current feature map. Gallery implements rubber-band selection, card AI
Suggest Tags actions, and sidebar-tag drop assignment; these interactions still
need Windows validation. MCP mutation opt-in is an environment gate; per-call
`confirm: true` checks cover only selected tools and do not prove that a human
reviewed the operation.

## Source file inventory

Line counts are from the pinned commit. Original files over the requested 800-line
limit are retained unchanged in the audit mirror; they must be split along
behavioral seams during migration without dropping behavior.

| File | Lines | Migration area | Status |
|---|---:|---|---|
| `src/main.rs` | 2,735 | startup, main UI composition, modal/settings behavior | Startup, top-level GPUI composition, source Statistics modal behavior, and selected commands/dialogs are migrated into feature crates. Remaining original actions and dialogs are tracked in the feature map above. |
| `src/app.rs` | 1,975 | state, pending actions, undo/redo, operations | Gallery selection, source-supported history producers, and file operations are owned by feature crates; app-level action orchestration has been redistributed. File and tree edits have no source undo variants. |
| `src/db/queries.rs` | 1,317 | all query and domain persistence operations | Copied into database plugin and split across five modules, all below 500 lines; broad regression coverage pending |
| `src/ui/preview.rs` | 2,748 | preview and interactions | Image controls, smooth zoom/pan, progressive decode, persisted transforms, auto-hiding toolbar, filmstrip, video metadata, gallery image/video/audio handoff, F2 rename, confirmed Delete/Backspace, focused preview keys, EXIF toggle, recent tags and collapsed tag picker are implemented. Audio preview and full-sequence image/video/audio navigation are migrated; video playback is handed to MediaPlayback with source keyboard controls and default-app fallback. Windows visual parity and device behavior remain unverified. |
| `src/ui/gallery.rs` | 2,176 | gallery and interaction | Gallery uses visible-row rendering with overscan and loads further 200-row data pages as needed; it also supports keyboard scrolling, month grouping and collapse persistence, sizing, micro-thumbnails, filters including extension click mode, statistics, tag picker, file actions, image/video/audio handoff, AI Suggest Tags, sidebar-tag drop assignment, rubber-band selection, and source-supported history. Windows interaction and visual parity remain unverified. |
| `src/ui/sidebar.rs` | 838 | folder/tag sidebar | Navigation/tag services, native folder selection, tag row action menus and tag before/into/after drag/drop are migrated. Source folder rows have no drag/drop behavior. |
| `src/ui/filterbar.rs` | 822 | search/filter/sort controls | Search, sort, media type, extension Select/Exclude mode, date pickers, ratings, named tag pills, clear controls and Settings-backed search-scope defaults are migrated; Windows visual parity remains unverified. |
| `src/ui/info.rs` | 350 | metadata info panel | Selected gallery details migrate file/image/date/tags/engagement fields and lazily render curated plus all EXIF fields. Preview info includes cached ffprobe video metadata and the EXIF information toggle. |
| `src/db/mod.rs` | 190 | schema, migrations, connections | Core schema, migration, WAL, and connection behavior are ported to photag-plugin-database; broad regression coverage remains pending. |
| `src/scan.rs` | 489 | indexing and thumbnail work queue | Migrated to indexing plugin; split into scan and maintenance modules. Behavioral coverage is partial. |
| `src/thumbs.rs` | 420 | image/video thumbnail generation | Image thumbnail pipeline and progressive Preview decode/prewarm are ported; video poster behavior still needs representative media corpus verification. |
| `src/video.rs` | 1,037 | video/audio playback | FFmpeg video probing/frame decode, PCM/Rodio audio output, seek and volume sliders, source speed presets/boosts, and basic transport are migrated; Windows audio-device/codec behavior remains unverified. |
| `src/ai.rs` | 645 | AI inference and image analysis | Registered `photag-plugin-ai` provides the source-derived Cordis inference/download service. Preview suggestion polling/UI, tag application, automatic analysis and mandatory-group input are connected; model/runtime behavior requires Windows validation. |
| `src/update.rs` | 618 | updater | Registered updater ports release check, automatic offer, download verification, staged swap/rollback into Cordis and Settings, with session save and clean GPUI exit before shell install. Windows replacement/restart remains unverified. |
| `src/xmp.rs` | 278 | metadata write | Source XMP/IPTC writer is reused in the tagging crate; regressions cover JPEG IPTC insertion/removal and mtime restoration, non-JPEG sidecar escaping/removal and source-byte preservation, background write failure visibility/clearing, and same-path pending-job coalescing to the latest tags. Representative camera/editor interoperability remains unverified. |
| `src/watcher.rs` | 182 | filesystem watcher | Migrated to Cordis watcher plugin; Windows native behavior is unverified. |
| `src/drag.rs` | 332 | Windows drag-out | CF_HDROP/OLE source service is registered and invoked by Gallery card drag gestures; Windows OLE delivery remains unverified. |
| `src/winfx.rs` | 288 | Windows visual effects | Windows-gated appearance APIs are in `photag-ui-bridge`; shell integration is present but needs Windows runtime verification. |
| `src/settings.rs` | 306 | persisted settings and key mappings | Schema/defaults and Cordis persistence service migrated; Settings GPUI controls include source open-key capture, F1–F9 tag picker backed by the live navigation snapshot, search-scope defaults and Preview toolbar offsets; Gallery consumes these bindings. |
| `src/session.rs` | 73 | session persistence | Schema/defaults and Cordis persistence are migrated; active query filters, validated Gallery selection/scroll, Navigation selection/tag expansion, sidebar width, thumbnail size/month grouping, and last existing Preview image/transform/info state restore/save. |
| `src/theme.rs` | 610 | theme and drawing primitives | Palette, dimensions, typography names, control state colors, accent presets and source alpha levels are exposed and consumed through shared UI tokens; Windows font and visual parity remain unverified. |
| `src/ui/mod.rs` | 5 | UI module exports | Source module organization only; feature behavior is tracked in the rows above. |

## Source-code and graph cross-check (2026-10-13)

The codebase-memory graph indexed the pinned reference (`vendor/photag-source`)
as 1,228 nodes / 5,195 edges across all 22 Rust files, with no reported parse
gaps. The post-test migrated workspace index contained 2,807 nodes / 10,539 edges; vendor,
docs and build output are intentionally excluded. Graph traces were followed
from the source's `AppState::process_pending`, `show_gallery`, `show_tag_node`,
and `show_preview` entry points, then matched to destination crate/service and
view symbols. The source and destination use different architectural boundaries,
so this is a behavior mapping, not a requirement for one-to-one function names.

| Reference source path / traced behavior | Migrated code counterpart | Finding |
|---|---|---|
| `src/main.rs`, `src/app.rs`: startup, pending actions, gallery, history, navigation, file actions, export, update | `photag-shell`; app-state, Gallery, Navigation, History, File Operations, Metadata Export, Updater and Settings plugins | Every source-backed action family inspected has a destination implementation. Concrete misses found in Gallery Enter handling and multi-selection Preview affordance were repaired in Gallery keyboard/render code; see the keyboard row above. |
| `src/ui/gallery.rs`, `src/ui/filterbar.rs`, `src/db/queries.rs`: windowed media, selection, filters, bulk actions, stats | Gallery view/filter/bulk/statistics modules and Database service | Corresponding code paths exist for paging/rendering, selection, filter-to-query mapping, snapshot-bound bulk actions and statistics. This structural check does not prove every SQL edge case or Windows interaction. |
| `src/ui/sidebar.rs`, `src/db/queries.rs`: folder/tag trees, lazy children, edits, associations, merge | Navigation and Database services; Tagging commands | Corresponding folder/tag operations and UI flows exist, including lazy loading and tag placement/association controls. Native picker, drag and confirmation behavior still needs Windows validation. |
| `src/ui/preview.rs`, `src/ui/info.rs`, `src/thumbs.rs`, `src/video.rs`: preview, metadata, image transforms, filmstrip, playback | Preview, Thumbnails, Media Playback, File Operations and Tagging services | Corresponding source feature areas are present. Live Windows validation corrected the preview to a full-window modal overlay and centered generated PNG rendering. Remaining controls, DPI sizes, Windows devices, codecs and metadata corpus still need validation. |
| `src/scan.rs`, `src/watcher.rs`, `src/app.rs`: indexing, progress/cancel, reconciliation | Indexing, Thumbnails, Watcher and Database services | Corresponding scan, backfill, notification and reconciliation paths exist; native event delivery and filesystem races remain unverified. |
| `src/ai.rs`, `src/xmp.rs`: inference, suggestions, model download, metadata writeback | AI and Tagging services plus Preview suggestion view | Corresponding analysis, assignment, history and writeback paths exist. One explicit semantic difference remains: source auto-analysis leaves failed image IDs eligible for retry, while migrated `AiSuggestionsView::poll` marks failed/disconnected inference as analyzed to prevent automatic retry loops. Manual Analyze remains available. |
| `src/settings.rs`, `src/session.rs`, `src/theme.rs`, `src/update.rs`, `src/drag.rs`, `src/winfx.rs` | Settings, Session, UI Bridge, Updater, Drag/Drop and Shell | Corresponding persistence, theme, updater and OLE feature areas have code counterparts. OS-specific effects remain unverified on Windows. |

This pass directly read both source and destination implementations for the
identified regressions; graph presence alone is not treated as proof of parity.
No whole source feature family in this mapping lacked a destination code area.
The table below records specific remaining behavior and validation gaps; this
result does not certify every branch as equivalent.

## Interaction inventory from upstream README

Gallery keys include configurable open (default Space), arrows/Home/End, Ctrl+A,
0–5 ratings, F1–F9 tags, Delete/Backspace, Ctrl+Z/Ctrl+Y and Ctrl+Shift+D.
The open-key setting, history commands, source-supported history producers,
filter-driven bulk panel, paging, tag picker, month grouping, thumbnail size
controls, rubber-band selection, Gallery AI Suggest Tags, sidebar-tag drop
assignment, and card/Preview action menus are present in the migrated code.
Bulk delete binds confirmation to the exact media-ID set. Source Enter preview
and multi-selection Preview are present after code-review repairs. Preview
supports filmstrip navigation,
low-resolution-first decode, arrows, Escape, I info toggle, F2 rename,
delete-and-advance, persisted transforms and toolbar offsets. Audio preview and
image/video/audio filmstrip navigation are migrated; video hands off to Playback.
Windows visual/media/file-operation behavior remains unverified. The
source history does not record file-system or tree edits, and the source does
not implement pick/reject actions.

## Original persistent data contract

- `%LOCALAPPDATA%\\PhoTag\\photag.db`: SQLite database.
- `settings.json`, `session.json`, `thumbs\\`, `models\\`, `updates\\`.
- Tables: `folders`, `media`, `tags`, `media_tags`, `tag_associations`.
- Columns added by in-place migrations: media `micro_thumb`, `open_count`,
  `rating`, `pick_flag`; tags `color`, `icon`, `display`, `sort_order`;
  media_tags `auto_applied`.
- Indexes: media folder and modified time; media_tags tag/media; tags parent.
- WAL mode, `synchronous=NORMAL`, 1000-page checkpoint, FK constraints, and a
  separate read connection are observable performance/data integrity choices.
- The original README documents 8x8 database micro-thumbnails, configurable
  gallery thumbnails (80–300 px), progressive decode with neighbor prewarm, and
  scanner workers capped below the available core count.

Compatibility requirement: keep the database path/schema, existing JSON serde
fields/defaults, and cached media identity behavior. Do not migrate formats
until fixture-based forward-and-reopen tests prove existing libraries survive.

## Audit gaps / next audit pass

The README and file inventory establish the feature baseline, but the following
must be extracted into executable regression specifications before parity can
be accepted: every menu item and confirmation, all settings controls/defaults,
all SQL query semantics and cascades, edge cases in file-operation rollback,
watcher event reconciliation, viewer toolbar configuration, and per-control
visual behavior. These are specification/validation gaps; they do not imply
that the corresponding UI or service code is absent. The source mirror remains
available for deeper branch-by-branch comparison. Windows interaction, native
system integration, representative full-format media coverage, update restart,
and real-library compatibility also remain unverified until exercised in the
appropriate environment.
