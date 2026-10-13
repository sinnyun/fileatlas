# PhoTag source audit and migration map

**Audit baseline:** `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` (2026-08-04),
PhoTag v0.2.0. Mirrored source: `vendor/photag-source/`. Source count:
18,434 Rust lines across 22 files. Migration status below was reconciled with
the latest implementation commits on 2026-10-13.
This audit catalogs documented behavior and original source locations; it is
not a claim that every internal branch has been exhaustively behavior-tested.
Detailed code-level migration requires working through the listed modules and
capturing behavioral regression cases before each port.

## Feature-to-source mapping

The status column describes implemented behavior and remaining gaps. A
compiled crate or passing synthetic test does not by itself establish source
parity.

| Original feature / behavior | Original source | New destination | Status |
|---|---|---|---|
| App lifecycle, dialogs, pending action dispatch, undo/redo, selection state | `src/main.rs`, `src/app.rs` | shell + app-state/history plugins | **Partial:** Shell exposes lifecycle-owned Undo/Redo menu commands. Gallery records ratings, tag assignments/removals/clear, and confirmed rename; accepted AI tag suggestions use the source `AssignedTags` history entry. File actions expose native copy/move/delete dialogs and post-operation selection/preview repair. Session restores folder/path, expanded tags, selection, scroll, sidebar width, preview path/transforms, thumbnail size, and month grouping. The source history has no copy/move/delete or tree-edit entries. Pick/reject remains incomplete: the source only describes its glyph row and has no corresponding action handler. |
| AI-assisted application observation and operation over MCP | No upstream MCP feature | `photag-plugin-mcp` + Shell window-capture handle | **Partial:** opt-in local stdio server (`--mcp`) exposes window PNG capture, library/media queries, preview/playback controls, scan controls, ratings/tags, metadata export, and file operations through Cordis services. All mutation tools require `PHOTAG_MCP_ALLOW_MUTATIONS=1`; selected higher-impact calls also require `confirm: true`, which is a call-site acknowledgement rather than proof of human confirmation. MCP client round trips and Windows `PrintWindow` fidelity remain unverified. MCP access does not complete any native UI migration. |
| Folder and tag navigation, tree editing, folder actions | `src/ui/sidebar.rs`, `src/db/queries.rs` | `photag-plugin-navigation` + indexing/database/watcher plugins | **Mostly migrated:** Folder/tag trees, recursive counts, lazy nested folders, source-backed tag create/rename/delete/reparent/style/display, folder add/rescan/remove, native folder picking and persisted reachable selection/expansion are migrated. Tag rows expose an action menu for edit, rename, chip display, parent assignment, style save and delete; tag drag/drop supports before/into/after placement. The source has no folder-tree drag/drop behavior. Broader context-menu and visual parity remain incomplete. |
| Virtualized gallery data window, selection, grid, month groups | `src/ui/gallery.rs`, `src/app.rs::load_media/ensure_media_window` | `photag-plugin-gallery` + GPUI shell | **Partial:** Compact ordered skeletons and 200-row rich media pages remain. GPUI `uniform_list` renders visible rows with two-row overscan; column count follows available width and thumbnail size. Scroll near the loaded boundary loads the next page; keyboard targets load and scroll into view. Month grouping and 100–300 px card sizing are controllable. Gallery supports plain/Ctrl/Shift selection, keyboard selection/rating, selected-item details, named include/exclude tag pills, calendar date filters, and a searchable hierarchical tag assignment picker. Cards paint cached image thumbnails with database 8×8 micro-thumbnails underneath and expose Preview/rating actions. Filtered bulk copy/move uses native destination picking; Delete/Backspace opens an ID-snapshot confirmation dialog. Preview receives ordered path/media-ID pairs for its filmstrip. Rubber-band selection and full source card/preview context menus remain incomplete. |
| Progressive image thumbnail decode, EXIF orientation, DCT-scaled JPEG decode, 8×8 micro-thumbnails, path-hash cache | `src/thumbs.rs` | `photag-plugin-thumbnails` | Original image thumbnail pipeline and cache are wrapped as a Cordis service; Gallery paints cached thumbnails over database micro-thumbnails and controls card size. Preview decodes low-resolution first and prewarms nearby images. Video-poster corpus parity and Windows/UI validation remain incomplete or unverified. |
| Search, sort, date/rating/type/extension/folder filters | `src/ui/filterbar.rs`, `src/db/queries.rs`, `src/app.rs::load_media` | `photag-plugin-gallery::filter` + database plugin | **Mostly migrated:** Host-neutral state maps source query scopes, media type, include/exclude tags/extensions, inclusive dates, ratings and sort order into `MediaQuery`. GPUI exposes search/scope, media type, all eight sort orders, date pickers, rating bounds, extension cycling, named include/exclude tag pills and clear actions. Query filters persist through `session.json`; search-scope defaults load from and save to the Settings service. The bulk-action panel still uses ISO date text inputs rather than the source date controls. |
| Image preview, progressive decoding, zoom/pan/rotate/flip, toolbar and filmstrip | `src/ui/preview.rs`, `src/thumbs.rs` | `photag-plugin-preview` + `photag-plugin-thumbnails` | Cordis preview service and lifecycle-owned GPUI surface support image open, rotate/flip, source-compatible fit/preset/continuous zoom, cursor anchoring, pan/clipping and EXIF info. Gallery supplies ordered path/media-ID pairs to a filmstrip. Preview decodes a low-resolution frame before the original image and prewarms nearby images through a bounded two-worker queue that stops with the service. Transform/info state and toolbar offset persist through Session/Settings; the toolbar can be dragged within the preview stage. F2 rename and confirmed Delete/Backspace use FileOperations and preserve or advance the current filmstrip item. Video handoff goes to MediaPlayback. Non-image preview and Windows visual parity remain incomplete or unverified. |
| Video/audio playback, synchronization, speed, rotation metadata, ffmpeg integration | `src/video.rs` | media-playback plugin | FFmpeg video frames and PCM audio play through a worker-owned Rodio sink. Audio-clock timing falls back to wall clock when the stream drains; seek and volume sliders, pause/frame-step, source speed presets, 0.05× adjustment, and 3×/4×/8× boosts are exposed. Windows codecs/audio-device behavior and full synchronization parity remain unverified. |
| EXIF and metadata display, dimensions, paths, open count | `src/ui/info.rs`, `src/ai.rs`, `src/db/queries.rs` | Gallery selected details + Preview info panel | Gallery selected-item details include filesystem read-only/modified/created values, formatted size, indexed dimensions/megapixels/aspect ratio, capture date, rating, hierarchical tags, and the original database open counter. The selected-image panel lazily parses source-compatible curated and complete EXIF fields with `kamadak-exif` 0.6.1. The preview info overlay/toggle parses filesystem and EXIF metadata and displays cached ffprobe video metadata. AI descriptions and visual parity remain incomplete. |
| Hierarchical tags, ancestor auto-apply/manual distinction, associations, merge, rename, reparent, colors/icons/order/display | `src/db/queries.rs`, `src/ui/sidebar.rs`, `src/ui/preview.rs` | navigation + `photag-plugin-tagging` | Database assignment, association expansion, automatic ancestor rows, manual/automatic distinction, removal/clear/merge and tree editing/style/display services are exposed through Cordis. Navigation has tag-row actions and before/into/after tag drag/drop; Gallery has searchable assignment/removal/clear across selected media with undo history. AI suggestion assignments also enter source-compatible history. Background XMP write failures are serialized by path and surfaced in Gallery with retry/dismiss behavior. Complete card/preview context menus and writeback regression coverage remain incomplete. |
| Ratings, pick/reject, keyboard shortcuts, tag hotkeys | `src/app.rs`, `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/settings.rs` | `photag-plugin-gallery` + command/keybinding contributions | Rating selection effects persist through the source bulk update query (0–5 clamp verified), with GPUI card selection, batch controls and 0–5 key handling. Arrow/Home/End/Ctrl+A/Escape selection keys are bound; Enter opens the selected image in the preview service; Gallery and Preview delete actions use ID-bound Recycle Bin confirmation. Settings captures/persists the open key and F1–F9 tag targets; Gallery consumes open and tag keys, with history entries for tag mutations. Preview supports F2 rename, fit/zoom keys, and filmstrip navigation. The source has no pick/reject action handler; remaining shortcuts and visual parity are unported. |
| Search tag assignment menus and context menus | `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/ui/sidebar.rs` | menu contributions owned by feature plugins | Gallery provides a searchable hierarchical tag picker for assign/remove/clear across selected media, with undo history. Cards expose Preview/rating actions; Navigation tag rows provide edit, rename, show/hide chips, parent, save style, delete, and drag/drop placement. Full source card/preview context menus and drag/drop tag assignment remain incomplete. |
| Recursive multi-thread scanning, hidden-file filtering, video/poster pairing, DB batches, image/video thumbnails, cancellation/progress, missing-thumbnail backfill, child directory listing | `src/scan.rs`, `src/thumbs.rs`, FFmpeg helpers from `src/video.rs` | `photag-plugin-indexing` + `photag-plugin-thumbnails` | **Partial:** Scanner workflow is adapted behind a Cordis service; synthetic folder scan regression test passes. Real media corpus, FFmpeg, Windows paths, rescan policy and watcher integration remain unverified. |
| Filesystem watcher and reconciliation | `src/watcher.rs`, `src/scan.rs`, `src/app.rs` watcher reconciliation | `photag-plugin-watcher` + indexing plugin | Original create/remove/rename filtering, hidden-file exclusion, 800ms debounce, 4,000-event drain and 200-path reconcile budget are retained. Cordis plugin watches registered folders, runs reconciliation in a bounded background poller, triggers missing-thumbnail generation for added rows, emits a Cordis media-index event after database changes, and stops the worker with the plugin. Linux integration tests cover file create/remove and dispatch of both change notifications; native Windows delivery and DB/file race cases remain unverified. |
| Recycle Bin delete, copy/move/rename/create folder, reveal/open, index follow-through | `src/app.rs`, `src/db/queries.rs`, `src/drag.rs` | `photag-plugin-file-operations` | Single/bulk copy, move, rename, folder creation and Recycle Bin delete preserve source operations and database follow-through. Moving media outside every registered library root removes its database row while keeping the file on disk. Gallery and Preview use native destination/confirmation dialogs, per-item outcomes, index refresh, selection repair, and filmstrip path/ID repair or advance. Windows Shell behavior remains unverified. |
| Drag-out to Explorer/drop targets via `CF_HDROP` | `src/drag.rs` | `photag-plugin-drag-drop` + Gallery | CF_HDROP/OLE service is wired to Gallery card drag-start; Windows delivery remains unverified. |
| Bulk action console and filter-driven delete/copy/move | `src/app.rs`, `src/main.rs`, `src/db/queries.rs` | Gallery + file-operations plugins | Filtered bulk panel supports scopes, AND filters, live counts, confirmation bound to a matching media-ID snapshot, native destination picking, and per-item copy/move/delete outcomes. It resets to source defaults on open, closes on Escape or canceled destination picking, and disables execution for zero matches or invalid filters. Bulk-panel ISO date text inputs remain different from the source date controls. |
| JSON export of metadata | `src/db/queries.rs`, `src/main.rs` | `photag-plugin-metadata-export` | Source-compatible query, all-record ordering, rating default, hierarchical tag formatting, and pretty JSON export are available. A Cordis command and File menu item drive a lifecycle-owned GPUI panel; it accepts a destination path and reports progress, record count/path, or errors. The native Save File dialog uses a JSON filter and source default filename; overwrite behavior is delegated to the OS dialog. Repeated exports are single-flight and completion/failure feedback is visible in the panel. |
| AI MobileNetV2 suggestions, brightness/warmth/saturation analysis, mandatory groups, ONNX model download | `src/ai.rs`, `src/settings.rs` | `photag-plugin-ai` | The registered Cordis service provides MobileNetV2 inference, ImageNet mapping, image analysis, mandatory-group scoring and model download. Settings mandatory groups are passed into inference. Preview provides manual/automatic suggestion analysis, model download when manually requested, visible progress/errors, and source-group tag creation/application through Tagging; successful assignments enter undo history. Automatic analysis runs once per image per panel lifetime and only when the model is already available. Windows model/runtime validation remains unverified. |
| JPEG IPTC and XMP sidecar writing, restore original modified time | `src/xmp.rs`, `src/app.rs` | `photag-plugin-tagging::xmp` | Original XMP/APP13 implementation is reused; successful tagging service assignment/removal/clear/merge/reparent schedules best-effort metadata rewrites. A serialized worker coalesces same-path jobs and exposes path-keyed failures to Gallery, where users can retry or dismiss them. Regression coverage remains incomplete. |
| Settings persistence, accent presets, search scopes, tag search/merge behavior, toolbar config/position, update setting | `src/settings.rs`, `src/theme.rs` | `photag-plugin-settings` + `photag-ui-bridge::theme` | PhoTag-compatible settings schema and Cordis persistence are migrated. GPUI controls expose appearance, toolbar lock and offset controls, shortcuts, editable mandatory AI groups, and update check/download/install status. The open key, F1–F9 tag targets, search scopes, accent state, and Preview toolbar drag offsets are connected and persisted. Updater install ordering is wired through session save, GPUI quit, then shell install. Windows updater/UI behavior and full modal parity remain unverified/incomplete. |
| Session restore: folder, selection, scroll, expanded nodes, sidebar width, filters | `src/session.rs`, `src/app.rs` | `photag-plugin-session` | Source-compatible session fields and Cordis persistence are migrated. Gallery restores selection and scroll against the active query, filters, thumbnail size, and month grouping; Navigation restores selection and valid tag expansion; Shell restores the resizable sidebar width; Preview restores an existing last image and its transform/info state. Crash-time last-scroll durability remains limited. |
| Mica/Acrylic, dark title bar, system accent, fallback | `src/winfx.rs`, `src/theme.rs` | `photag-ui-bridge::winfx` + shared theme | Windows-gated registry accent/font probes and dark title bar/Mica/Acrylic APIs are implemented; the shell applies them after creating the GPUI HWND and settings update shared accent state. Transparent GPUI surface alpha is exposed but existing surfaces do not yet consume it. Windows runtime behavior remains unverified. |
| GitHub update check, confirmation, download, SHA256 verification, staged swap/rollback | `src/update.rs`, `src/settings.rs` | `photag-plugin-updater` | **Mostly migrated:** The Cordis updater is registered and Settings exposes release check, notes, verified download progress, and a two-step install confirmation. Install saves the latest session, schedules the staged binary, quits GPUI, then replaces/restarts from the shell; failures are logged. Windows replacement/restart behavior remains unverified. |
| Theme, typography, custom fonts, icon availability, visual tokens | `src/theme.rs` | `photag-ui-bridge::theme` + GPUI Kit | Source palette, dimensions, font names, MDL2 glyphs, control states and accent presets are exposed. Premultiplied source colors convert to GPUI RGBA; shared surface, text, border, and accent tokens style Gallery, Navigation, Settings, Preview, Metadata Export and Playback. Some view literals remain; actual font resolution and Windows visual parity are unverified. |
| SQLite schema, migrations, WAL, separate read connection, indexed queries | `src/db/mod.rs`, `src/db/queries.rs` | `photag-plugin-database` | Core schema/query source copied; Cordis service and separate connections implemented. A pre-migration database fixture verifies media rows and tag links survive opening. Broad SQL regression coverage remains incomplete. |

## Crate port documents and status source

Every workspace crate has a `PORTS.md` describing its Cordis identity, public
inputs/outputs, dependencies, event relationships, limits, and debugging points.
That includes the AI, MCP, updater, history, playback, preview, and drag/drop
crates as well as the original database and UI feature crates. Keep those guides
aligned with their public Rust APIs.

The feature-to-source table above is the single source of truth for migration
status. This section previously repeated selected implementation slices and
fell behind later work; it has been removed so an older summary cannot override
the current feature map. In particular, current Gallery paging and visible-row
rendering do not implement rubber-band selection. MCP mutation opt-in is an
environment gate; per-call `confirm: true` checks cover only selected tools and
do not prove that a human reviewed the operation.

## Source file inventory

Line counts are from the pinned commit. Original files over the requested 800-line
limit are retained unchanged in the audit mirror; they must be split along
behavioral seams during migration without dropping behavior.

| File | Lines | Migration area | Status |
|---|---:|---|---|
| `src/main.rs` | 2,735 | startup, main UI composition, modal/settings behavior | Startup and top-level GPUI composition are migrated, with selected commands and dialogs owned by feature crates. Remaining original actions, dialogs, and complete screen behavior are tracked in the feature map above. |
| `src/app.rs` | 1,975 | state, pending actions, undo/redo, operations | Gallery selection, file operations, and source-supported history producers are split across feature crates; the remaining pending-action orchestration is incomplete. File and tree edits have no source undo variants. |
| `src/db/queries.rs` | 1,317 | all query and domain persistence operations | Copied into database plugin and split across five modules, all below 500 lines; broad regression coverage pending |
| `src/ui/preview.rs` | 2,748 | preview and interactions | Image controls, zoom/pan, progressive decode, persisted transforms, filmstrip, video metadata, gallery image/video handoff, F2 rename, confirmed Delete/Backspace, focused preview keys and EXIF toggle are implemented. Non-image preview and visual parity remain incomplete. |
| `src/ui/gallery.rs` | 2,176 | gallery and interaction | Gallery uses visible-row rendering with overscan and loads further 200-row data pages as needed; it also supports keyboard scrolling, month grouping, sizing, micro-thumbnails, filters, tag picker, file actions, image/video handoff and source-supported history. Rubber-band selection and full card/preview context menus remain incomplete. |
| `src/ui/sidebar.rs` | 838 | folder/tag sidebar | Navigation/tag services, native folder selection, tag row action menus and tag before/into/after drag/drop are migrated. Broader context-menu parity remains incomplete; source folder rows have no drag/drop behavior. |
| `src/ui/filterbar.rs` | 822 | search/filter/sort controls | Search, sort, media type, extension, date pickers, ratings, named tag pills, clear controls and Settings-backed search-scope defaults are migrated; Windows visual parity remains unverified. |
| `src/ui/info.rs` | 350 | metadata info panel | Selected gallery details migrate file/image/date/tags/engagement fields and lazily render curated plus all EXIF fields. Preview info includes cached ffprobe video metadata and the EXIF information toggle. |
| `src/db/mod.rs` | 190 | schema, migrations, connections | Core schema, migration, WAL, and connection behavior are ported to photag-plugin-database; broad regression coverage remains pending. |
| `src/scan.rs` | 489 | indexing and thumbnail work queue | Migrated to indexing plugin; split into scan and maintenance modules. Behavioral coverage is partial. |
| `src/thumbs.rs` | 420 | image/video thumbnail generation | Image thumbnail pipeline is ported; video poster corpus parity and progressive UI sizing/prewarm remain incomplete. |
| `src/video.rs` | 1,037 | video/audio playback | FFmpeg video probing/frame decode, PCM/Rodio audio output, seek and volume sliders, source speed presets/boosts, and basic transport are migrated; Windows audio-device/codec behavior remains unverified. |
| `src/ai.rs` | 645 | AI inference and image analysis | Registered `photag-plugin-ai` provides the source-derived Cordis inference/download service. Preview suggestion polling/UI, tag application, automatic analysis and mandatory-group input are connected; model/runtime behavior requires Windows validation. |
| `src/update.rs` | 618 | updater | Registered updater ports release check/download verification/swap/rollback into Cordis and Settings, with session save and clean GPUI exit before shell install. Windows replacement/restart remains unverified. |
| `src/xmp.rs` | 278 | metadata write | Source XMP/IPTC writer is reused in the tagging crate; path-keyed writeback failures are surfaced in Gallery. Writeback regression validation remains incomplete. |
| `src/watcher.rs` | 182 | filesystem watcher | Migrated to Cordis watcher plugin; Windows native behavior is unverified. |
| `src/drag.rs` | 332 | Windows drag-out | CF_HDROP/OLE source service is registered and invoked by Gallery card drag gestures; Windows OLE delivery remains unverified. |
| `src/winfx.rs` | 288 | Windows visual effects | Windows-gated appearance APIs are in `photag-ui-bridge`; shell integration is present but needs Windows runtime verification. |
| `src/settings.rs` | 306 | persisted settings and key mappings | Schema/defaults and Cordis persistence service migrated; Settings GPUI controls include source open-key capture, F1–F9 tag picker backed by the live navigation snapshot, search-scope defaults and Preview toolbar offsets; Gallery consumes these bindings. |
| `src/session.rs` | 73 | session persistence | Schema/defaults and Cordis persistence are migrated; active query filters, validated Gallery selection/scroll, Navigation selection/tag expansion, sidebar width, thumbnail size/month grouping, and last existing Preview image/transform/info state restore/save. |
| `src/theme.rs` | 610 | theme and drawing primitives | Palette, dimensions, typography names, control state colors and accent presets are exposed; fixed colors remain in some views and Windows font/visual parity is incomplete. |
| `src/ui/mod.rs` | 5 | UI module exports | Source module organization only; feature behavior is tracked in the rows above. |

## Interaction inventory from upstream README

Gallery keys include configurable open (default Space), arrows/Home/End, Ctrl+A,
0–5 ratings, F1–F9 tags, Delete/Backspace, Ctrl+Z/Ctrl+Y and Ctrl+Shift+D.
The open-key setting, history commands, source-supported history producers,
filter-driven bulk panel, paging, tag picker, month grouping and thumbnail size
controls are migrated. Bulk delete binds confirmation to the exact media-ID set.
Preview supports filmstrip navigation, low-resolution-first decode, arrows,
Escape, I info toggle, F2 rename, delete-and-advance, persisted transforms and
toolbar offsets. Rubber-band selection, full source context menus,
non-image Preview, additional host actions and Windows visual/file-operation
behavior remain incomplete or unverified. The source history does not record
file-system or tree edits, and the source does not implement pick/reject
actions.

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
visual behavior. The baseline source remains available to inspect directly.
