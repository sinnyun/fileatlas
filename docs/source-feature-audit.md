# PhoTag source audit and migration map

## User-directed viewer replacement (2026-10-16)

| Source behavior inspected | Current mapping | Compatibility boundary |
|---|---|---|
| `src/app.rs` preview_media and preview/file action routing; `src/ui/preview.rs`; `src/video.rs` | Removed independent image/audio/video viewers; Gallery → `photag-plugin-quicklook::QuickLookService::open` | User explicitly replaces original-file viewing with installed QuickLook; embedded transforms/filmstrip/transport intentionally removed |
| `src/thumbs.rs`, Gallery card/inspector preview, tag preview snapshots | Existing Thumbnails/Indexing/Gallery implementation retained | No SQLite or on-disk compatibility migration |
| AI suggestion/assignment integration formerly attached to Preview | Gallery-owned `ai_suggestions`, existing AI/Tagging/History services | Explicit inference and automatic-analysis preference retained without a file-viewer dependency |

The new adapter does not copy QuickLook code or binaries and does not install,
download, update or remove QuickLook. See `crates/photag-plugin-quicklook/PORTS.md`.

**Audit baseline:** `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` (2026-08-04),
PhoTag v0.2.0. Mirrored source: `vendor/photag-source/`. Source count:
18,434 Rust lines across 22 files. Migration status below was reconciled with
the latest implementation commits on 2026-10-13.

**Phase status:** migration-phase record. The project is now in the interface
redesign phase, where [`docs/reference-ui/final-v5/`](reference-ui/final-v5/README.md)
is the design authority. Use this audit to locate existing backend behavior and
its original source, not to require the new UI to match the egui interface.

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

## v5 S2 backend reuse note (2026-10-13)

The new Gallery inspector removes an existing assignment through the same
`TaggingService::remove` path as the tag picker. Source anchors:
`src/ui/gallery.rs:1612-1617` dispatches `PendingAction::RemoveTags` for the
clicked media/tag; `src/db/queries.rs:1006-1011` deletes those assignments.
The current port continues metadata writeback and `RemovedTags` history;
SQLite schema, assignment semantics and persisted files are unchanged. The
new empty-library command enters Navigation's existing native folder picker
and production add/scan/watch path. v5 visuals are validated separately.

## Feature-to-source mapping

The status column describes implemented behavior and remaining gaps. A
compiled crate or passing synthetic test does not by itself establish source
parity.

| Original feature / behavior | Original source | New destination | Status |
|---|---|---|---|
| English/Chinese UI localization | No upstream localization feature | `photag-plugin-i18n` + UI Bridge translator; Settings language slot | Fluent catalogs, English default/fallback, explicit zh-CN selection, independent preference persistence, live window/placeholder refresh and lifecycle cleanup are implemented. Real MCP screenshots cover both languages across Gallery, six Settings tabs, Statistics, Bulk actions, selection and Preview Info. Backend/native dialogs and some generated status/error strings retain their original language. This is an added feature, not source parity. |
| App lifecycle, dialogs, pending action dispatch, undo/redo, selection state | `src/main.rs`, `src/app.rs` | shell + app-state/history plugins | Source-backed app actions, selection state, five history entry types and their producers are migrated. Gallery records ratings, tag assignment/removal/clear, confirmed rename and accepted AI tag suggestions; file actions repair selection/preview state after mutations. Shell stops Preview before its service providers and restores the session. On Windows MSVC, local startup validation now creates a native window for Gallery and dev-test profiles after raising the shell executable stack to 8 MiB. The source history has no copy/move/delete or tree-edit entries. The source only describes pick/reject glyphs and has no corresponding action handler, so there is no action behavior to migrate. Interactive lifecycle, dialogs and shortcut behavior remain unverified. |
| AI-assisted application observation and operation over MCP | No upstream MCP feature | `photag-plugin-mcp` + Shell window-capture handle | PhoTag-specific opt-in local stdio server (`--mcp`) exposes window PNG capture, library/media queries, preview/playback controls, scan controls, ratings/tags, metadata export, and file operations through Cordis services. All mutation tools require `PHOTAG_MCP_ALLOW_MUTATIONS=1`; selected higher-impact calls also require `confirm: true`, which is a call-site acknowledgement rather than proof of human confirmation. MCP child round trips and PrintWindow PNGs were inspected for the isolated fixture. get_ui_state/control_ui add Settings/language/Gallery surface requests and Preview fade readings; pointer/native interaction remains unverified. This feature is outside upstream migration parity. |
| Folder and tag navigation, tree editing, folder actions | `src/ui/sidebar.rs`, `src/db/queries.rs` | `photag-plugin-navigation` + indexing/database/watcher plugins | Folder/tag trees, recursive counts, lazy nested folders, source-backed tag create/rename/delete/reparent/style/display/associations, folder add/rescan/remove, native folder picking and persisted selection/expansion are migrated. Tag rows expose merge-target search, media-impact delete confirmation, ancestor assignment choices, and before/into/after drag placement. Select/Exclude click mode restores from Session; merge child handling follows Settings. Dragging a tag onto a Gallery media card assigns it to that item or its current selection. The source has no folder-tree drag/drop behavior. Windows interaction and visual parity remain unverified. |
| Virtualized gallery data window, selection, grid, month groups | `src/ui/gallery.rs`, `src/app.rs::load_media/ensure_media_window` | `photag-plugin-gallery` + GPUI shell | Compact ordered skeletons and 200-row rich media pages remain. GPUI `uniform_list` renders visible rows with two-row overscan; column count follows available width and thumbnail size. Scroll near the loaded boundary loads the next page; keyboard targets load and scroll into view. Month grouping, persisted collapsed-month state, and 80–300 px card sizing are controllable. Gallery supports plain/Ctrl/Shift selection, keyboard selection/rating, selected-item details, named include/exclude tag pills, calendar date filters, and searchable hierarchical tag assignment. Cards paint cached image thumbnails with database 8×8 micro-thumbnails underneath. Card actions include Preview, open/reveal, hierarchical tag assignment, copy/move, confirmed delete, AI Suggest Tags and rating. Dragging a sidebar tag onto a card assigns it to the card or current selection. Rubber-band selection replaces selection on plain drag and adds to it with Ctrl+drag. Filtered bulk operations use native destination picking and ID-snapshot confirmation. Statistics displays totals, storage, rating distribution, top tags and recent months. Preview receives ordered path/media-ID pairs for its filmstrip. Full-width command/search/filter and bottom status projections now share one Gallery entity; Statistics/Bulk are floating surfaces and selection actions no longer consume the Gallery viewport. Source-sized square cards and month row spacing are being restored; Windows behavior and remaining visual parity are unverified. |
| Progressive image thumbnail decode, EXIF orientation, DCT-scaled JPEG decode, 8×8 micro-thumbnails, path-hash cache | `src/thumbs.rs` | `photag-plugin-thumbnails` | Original image thumbnail pipeline and cache are wrapped as a Cordis service; Gallery paints cached thumbnails over database micro-thumbnails and controls card size. Preview decodes low-resolution first and prewarms nearby images. Synthetic JPEG, PNG, WebP, TIFF and BMP thumbnail regression cases pass on Windows; representative user-media corpus, EXIF orientation corpus and UI behavior remain unverified. |
| Search, sort, date/rating/type/extension/folder filters | `src/ui/filterbar.rs`, `src/db/queries.rs`, `src/app.rs::load_media` | `photag-plugin-gallery::filter` + database plugin | Host-neutral state maps source query scopes, media type, include/exclude tags/extensions, inclusive dates, ratings and sort order into `MediaQuery`. GPUI exposes search/scope, media type, all eight sort orders, date pickers, rating bounds, persisted Select/Exclude extension mode, named include/exclude tag pills and clear actions. Query filters persist through `session.json`; search-scope defaults load from and save to Settings. The bulk-action panel uses independent calendar date pickers with clear controls. Windows behavior and visual parity remain unverified. |
| Image preview, progressive decoding, zoom/pan/rotate/flip, toolbar and filmstrip | `src/ui/preview.rs`, `src/thumbs.rs` | `photag-plugin-preview` + `photag-plugin-thumbnails` | Cordis preview service and lifecycle-owned GPUI surface support source-compatible image controls and decode behavior. Gallery supplies ordered path/media-ID pairs to a filmstrip. Transform/info state and toolbar offset persist through Session/Settings; F2 rename and confirmed Delete/Backspace use FileOperations. Audio preview has the source-style stage and actions; video items hand off to MediaPlayback. Video keyboard controls, FFprobe fallback to the default app, recent tags and collapsed picker-tree state are migrated. Windows visual parity and device behavior remain unverified. |
| Video/audio playback, synchronization, speed, rotation metadata, ffmpeg integration | `src/video.rs` | media-playback plugin | FFmpeg video frames and PCM audio play through a worker-owned Rodio sink. Audio-clock timing falls back to wall clock when the stream drains; seek and volume sliders, pause/frame-step, source speed presets, 0.05× adjustment, and 3×/4×/8× boosts are exposed. Video Preview routes Space/Left/Right/Escape to playback as in the source and opens unprobeable videos through the default application. Windows codecs/audio-device behavior and synchronization remain unverified. |
| EXIF and metadata display, dimensions, paths, open count | `src/ui/info.rs`, `src/ai.rs`, `src/db/queries.rs` | Gallery selected details + Preview info panel | Gallery selected-item details include filesystem read-only/modified/created values, formatted size, indexed dimensions/megapixels/aspect ratio, capture date, rating, hierarchical tags, and the original database open counter. The selected-image panel lazily parses source-compatible curated and complete EXIF fields with `kamadak-exif` 0.6.1. The preview info overlay/toggle parses filesystem and EXIF metadata and displays cached ffprobe video metadata. The pinned source exposes no generated-description feature; Windows visual parity is unverified. |
| Hierarchical tags, ancestor auto-apply/manual distinction, associations, merge, rename, reparent, colors/icons/order/display | `src/db/queries.rs`, `src/ui/sidebar.rs`, `src/ui/preview.rs` | navigation + `photag-plugin-tagging` | Database assignment, association expansion, automatic ancestor rows, manual/automatic distinction, removal/clear/merge and tree editing/style/display services are exposed through Cordis. Navigation supports association editing, source merge-target selection, media-aware delete confirmation/ancestor handling, and drag reorder. Gallery has searchable assignment/removal/clear across selected media with undo; the right Inspector adds tags through an inline hierarchical multi-select control. Preview retains its searchable picker. AI suggestions enter source-compatible history. Normal tag mutations no longer write metadata files, by explicit user decision on 2026-10-17. |
| Ratings, pick/reject, keyboard shortcuts, tag hotkeys | `src/app.rs`, `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/settings.rs` | `photag-plugin-gallery` + command/keybinding contributions | Rating selection effects persist through the source bulk update query (0–5 clamp verified), with GPUI card selection, batch controls and 0–5 key handling. Arrow/Home/End/Ctrl+A/Escape selection keys are bound. Source Enter preview and the configurable open key are both handled unconditionally in `view/keyboard.rs::handle_key`; the action bar exposes Preview first selected for multi-selection, matching the source's selected-set action. Gallery and Preview delete actions use ID-bound Recycle Bin confirmation. Settings captures/persists the open key and F1–F9 tag targets; Preview supports F2 rename, fit/zoom keys, and image/video/audio filmstrip navigation. The source provides no pick/reject action handler; therefore those glyphs have no action to port. Windows shortcut/focus behavior and visual parity remain unverified. |
| Search tag assignment menus and context menus | `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/ui/sidebar.rs` | menu contributions owned by feature plugins | Gallery provides a searchable hierarchical tag picker for assign/remove/clear across selected media, with undo history. Card actions expose Preview, open/reveal, hierarchical tag assignment, copy/move, confirmed delete, AI Suggest Tags and rating; sidebar tags can be dragged onto cards. Preview's right-click menu exposes open/reveal, AI suggestions, rating and confirmed delete, and its toolbar has a searchable tag picker. Navigation tag rows provide edit, rename, show/hide chips, parent, save style, delete, and before/into/after drag placement. Windows interaction and visual parity remain unverified. |
| Recursive multi-thread scanning, hidden-file filtering, video/poster pairing, DB batches, image/video thumbnails, cancellation/progress, missing-thumbnail backfill, child directory listing | `src/scan.rs`, `src/thumbs.rs`, FFmpeg helpers from `src/video.rs` | `photag-plugin-indexing` + `photag-plugin-thumbnails` | Source scanning, video/poster pairing, database batching, cancellation/progress, thumbnail generation/backfill and child directory listing are implemented behind Cordis services. Synthetic regression coverage exists; full media corpus, FFmpeg deployment and Windows paths remain unverified. |
| Filesystem watcher and reconciliation | `src/watcher.rs`, `src/scan.rs`, `src/app.rs` watcher reconciliation | `photag-plugin-watcher` + indexing plugin | Original create/remove/rename filtering, hidden-file exclusion, 800ms debounce, 4,000-event drain and 200-path reconcile budget are retained. Cordis plugin watches registered folders, runs reconciliation in a bounded background poller, triggers missing-thumbnail generation for added rows, emits a Cordis media-index event after database changes, and stops the worker with the plugin. Native Windows tests passed for file create/remove reconciliation in the current second-round test run; Windows rename delivery and DB/file race cases remain unverified. |
| Recycle Bin delete, copy/move/rename/create folder, reveal/open, index follow-through | `src/app.rs`, `src/db/queries.rs`, `src/drag.rs` | `photag-plugin-file-operations` | Single/bulk copy, move, rename, folder creation and Recycle Bin delete preserve source operations and database follow-through. Windows temporary-directory tests now cover rename, in-library move, move outside the registered root, and destination collision; moved media outside all roots leaves the file on disk and removes its row. Gallery and Preview use native destination/confirmation dialogs, per-item outcomes, index refresh, selection repair, and filmstrip path/ID repair or advance. Recycle Bin, Shell dialogs, Explorer integration and cross-volume fallback remain unverified. |
| Drag-out to Explorer/drop targets via `CF_HDROP` | `src/drag.rs` | `photag-plugin-drag-drop` + Gallery | CF_HDROP/OLE service is wired to Gallery card drag-start; Windows delivery remains unverified. |
| Bulk action console and filter-driven delete/copy/move | `src/app.rs`, `src/main.rs`, `src/db/queries.rs` | Gallery + file-operations plugins | Filtered bulk panel supports scopes, AND filters, live counts, confirmation bound to a matching media-ID snapshot, native destination picking, and per-item copy/move/delete outcomes. It resets to source defaults on open, closes on Escape or canceled destination picking, and disables execution for zero matches or invalid filters. Date bounds use independent calendar pickers with per-bound clear controls. Windows behavior remains unverified. |
| JSON export of metadata | `src/db/queries.rs`, `src/main.rs` | `photag-plugin-metadata-export` | Source-compatible query, all-record ordering, rating default, hierarchical tag formatting, and pretty JSON export are available. A Cordis command and File menu item open a lifecycle-owned GPUI preparation modal; choosing a native save destination invokes the existing worker and reports active state, record count/path, or errors. The native Save File dialog uses a JSON filter and source default filename; overwrite behavior is delegated to the OS dialog. Repeated exports are single-flight and completion/failure feedback is visible in the panel. |
| Statistics modal | `src/main.rs`, `src/db/queries.rs` | `photag-plugin-gallery` + database plugin | Gallery exposes the source statistics query in a visible panel with media totals, storage, rating distribution, top ten tags, latest twelve media months, and visible query errors. Windows visual parity remains unverified. |
| AI MobileNetV2 suggestions, brightness/warmth/saturation analysis, mandatory groups, ONNX model download | `src/ai.rs`, `src/settings.rs` | `photag-plugin-ai` | The registered Cordis service provides MobileNetV2 inference, ImageNet mapping, image analysis, mandatory-group scoring and model download. Settings mandatory groups are passed into inference. Preview provides manual/automatic suggestion analysis, model download when manually requested, visible progress/errors, and source-group tag creation/application through Tagging; successful assignments enter undo history. Automatic analysis runs once per image per panel lifetime and only when the model is already available. Windows model/runtime validation remains unverified. |
| JPEG IPTC and XMP sidecar writing, restore original modified time | `src/xmp.rs`, `src/app.rs` | `photag-plugin-tagging::xmp` | Source reference automatically wrote JPEG IPTC and XMP sidecars for other formats. User decision 2026-10-17 supersedes that default: assignment/removal/clear/merge/reparent now update only SQLite, and no normal tag operation writes or creates sidecar files. The existing writer is retained only for explicit integration requests. |
| Settings persistence, accent presets, search scopes, tag search/merge behavior, toolbar config/position, update setting | `src/settings.rs`, `src/theme.rs` | `photag-plugin-settings` + `photag-ui-bridge::theme` | PhoTag-compatible settings schema and Cordis persistence are migrated. The floating six-section Settings modal has a toggle command/menu, drag, Close and Escape behavior. Controls expose appearance, toolbar lock/offset, shortcuts, mandatory AI groups, and update check/download/install status. A new-release offer presents notes and Update now/Not now; installation requires an explicit user action and saves session before shell replacement. Windows updater behavior and visual parity remain unverified. |
| Session restore: folder, selection, scroll, expanded nodes, sidebar width, filters | `src/session.rs`, `src/app.rs` | `photag-plugin-session` | Source-compatible session fields and Cordis persistence are migrated. Gallery restores selection/scroll, filters, extension click mode, thumbnail size, month grouping and collapsed months; Navigation restores selection, valid tag expansion and tag click mode; Shell restores sidebar width; Preview restores the last image, transforms/info and collapsed tag-picker nodes. Crash-time last-scroll durability remains limited. |
| Mica/Acrylic, dark title bar, system accent, fallback | `src/winfx.rs`, `src/theme.rs` | `photag-ui-bridge::winfx` + shared theme | Windows-gated registry accent/font probes and dark title bar/Mica/Acrylic APIs are implemented; the shell applies them after creating the GPUI HWND and Settings updates shared accent state. Source alpha levels are consumed through shared base (0xDE), modal (0xF4), and Gallery card (0xE8) RGBA helpers. Native material, registry and compositor behavior require Windows runtime verification. |
| GitHub update check, confirmation, download, SHA256 verification, staged swap/rollback | `src/update.rs`, `src/settings.rs` | `photag-plugin-updater` | The Cordis updater checks releases, presents an automatic version offer with notes and Update now/Not now actions, verifies and stages downloads with progress, and requires an explicit Restart & install action. Not now suppresses offers for the process; the Updates tab can reopen them. Install saves the latest session, schedules the staged binary, quits GPUI, then replaces/restarts from the shell; failures are logged. Windows replacement/restart remains unverified. |
| Theme, typography, custom fonts, icon availability, visual tokens | `src/theme.rs` | `photag-ui-bridge::theme` + GPUI Kit | Source palette, dimensions, font names, MDL2 glyphs, control states and accent presets are exposed. Premultiplied source colors convert to GPUI RGBA; shared surface, text, border, accent and transparency tokens style Gallery, Navigation, Settings, Preview, Metadata Export, Playback and Shell. Font resolution and Windows visual parity remain unverified. |
| SQLite schema, migrations, WAL, separate read connection, indexed queries | `src/db/mod.rs`, `src/db/queries.rs` | `photag-plugin-database` | Core schema/query source copied; Cordis service and separate connections implemented. Tests verify a pre-migration database preserves media/tag links and defaults after migration and reopen, and representative source filters/sorts (date/rating bounds, tags, search, extensions, folder/path and rated/date order). Full SQL edge cases, cascades and real-library compatibility still need broader fixtures. |

## Crate port documents and status source

2026-10-13 GPUI Kit integration follow-up: Shell registers the bundled component
Assets at startup; Settings synchronizes the runtime accent through UI Bridge
`apply_accent_components(&mut gpui_kit::App)` and Kit `Theme::update`.
Existing source dimensions/palette rules and plugin ownership remain in place.
This repairs resource/theme plumbing; native icon rendering and interactive
accent-switch parity still require Windows workflow validation.

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
| `src/db/queries.rs`: folder/media/tag CRUD, scanner batches, watcher reconciliation and grouped Gallery reads | Typed `DatabaseService` methods in `photag-plugin-database` | Production feature crates use named service methods; multi-query operations retain their single reader/writer lock or transaction scope in Database. Raw callback compatibility methods remain available, while `check-layering.cjs` prevents production consumers from using them. SQL semantics and the SQLite schema are unchanged by this boundary migration. |
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

### User-directed startup update policy (2026-10-13)

Source `src/main.rs:111-113` starts automatic checks and
`src/main.rs:791` renders its offer. The redesign deliberately changes only
notification behavior: startup checks populate Settings silently; manual
checks may open the offer. `auto_check_updates` and persisted JSON models,
updater download/SHA256/install service semantics are preserved. Plugin MCP
verified available-release startup suppression and manual offer opening.

### v5 unverified audio-only extension (2026-10-14 handoff)

Pinned source `src/video.rs::get_video_info` selects v:0 and its
start_playback decodes video with an optional separate PCM audio stream.
The current media-playback worktree adds audio-only ffprobe classification,
container-duration parsing and PCM playback plus paused-seek preservation.
This is a redesign extension, not source parity. Parser/service-state tests
were added but not run in the latest pass; Windows audio output is unverified.
MCP still rejects audio in tools_write::open_media, so the audio screenshot
attempt showed the Gallery rather than playback. Planned indexed media_id
routing and Tags/More service integration are not implemented.

2026-10-14 redesign mapping update: metadata-export command/preparation modal/native JSON save maps to vendor/photag-source/src/app.rs:1535–1553 and db/queries.rs export_all_metadata. Query fields/order/writer unchanged. MCP all-library export12 success and os5 directory failure verified; native Save File clicks/overwrite remain unverified. Current UI follows final-v5, not historical source UI.

2026-10-14 v2 UI-only additions: SessionState.inspector_width is serde(default) and source fields are unchanged; sidebar_tab accepts existing free-form Folders/Tags plus the new extensions page. Sidebar folder/path selection continues existing Cordis events and database query prefixes; MCP path normalization follows canonical tree paths, not a schema/query change. History page uses existing global undo_description/redo_description, not invented per-file history. Reference sidebar-pages-preview-2026-10-14-v2 now governs main panels.

## 2026-10-14 functional additions mapping

| Feature | Pinned behavior inspected | Current implementation / compatibility |
|---|---|---|
| Grid/list preference | vendor/photag-source/src/session.rs:14–53 (serde-defaulted session), src/ui/gallery.rs:696/990 (grid layout and windowed rows) | User-requested list is additive; same Gallery queries/cache/selection/actions, one item per virtual row. New serde-defaulted gallery_list_layout=false leaves old sessions on grid; no SQLite change. |
| Stable card menus | vendor/photag-source/src/ui/gallery.rs:1673 onward (selected IDs and context actions) | Action semantics/target selection remain the shared builder; dropdown trigger now survives hover exit. |
| Draggable/resizable popup frames | Existing Settings, Preview and Playback service-driven GPUI views; source settings/preview panels as feature references | User-requested common frame in UI Bridge, with Cordis-owned PanelCatalog for new plugin Single/Multiple definitions. No parallel plugin manager; existing media services remain single-session. |

2026-10-14 AddFolder native-dialog mapping: inspected vendor/photag-source/src/app.rs:1174 PendingAction::AddFolder and its register→watch→scan flow. Navigation preserves optional-path/cancel semantics and existing add_folder_path service route, replacing synchronous UI-thread FileDialog with AsyncFileDialog to avoid GPUI RefCell reentry. No schema/query change. Native open/cancel/reopen is verified; actual native accept was not exercised by this run.

## User-directed all-file behavior (2026-10-14)

| Source anchor | Current mapping / intentional difference |
|---|---|
| `vendor/photag-source/src/scan.rs`: extension classifier, hidden filtering, consumed image posters | Indexing `scan.rs` keeps recognized image/video/audio semantics and video poster reuse, but now indexes every ordinary file as requested, including hidden/extensionless/unknown files and the poster image itself. Unknown extensions use `file`; the SQLite schema is preserved. |
| `vendor/photag-source/src/watcher.rs`: `MEDIA_EXTENSIONS`, `drain_events` | Watcher retains debounce and bounded processing; removes suffix/dot-prefix gates, guards against directory rows, and consumes Indexing's public classifier. |
| Gallery card action menu / accepted sidebar-preview v2 | The user's current instruction replaces the top-right ellipsis with a noninteractive colored format badge and adds category icons for unavailable thumbnails. Context-menu commands retain their existing handlers. This targeted update overrides the concept's earlier More-button treatment. |

Previously excluded files are added by scanning the registered folder again. Re-adding the same path is the incremental path and preserves existing rows; the historical Rebuild/Rescan action clears them. This changes visibility/index eligibility intentionally and is not a claim of new thumbnail/preview codec support.

All-file follow-up: the source scan calls `queries::insert_media` (`vendor/photag-source/src/db/queries.rs`, `INSERT OR REPLACE`), which replaces IDs and cascades away assignments. Scanner now uses Database's dedicated `scan_upsert_media` conflict update to retain existing user metadata while extending eligibility. Other consumers of `insert_media` are unchanged. The explicit Navigation Rebuild/Rescan action still clears rows before scanning; incremental upgrading uses Add folder with the existing path.

## Startup data refresh correction (2026-10-14)

Tracing the launcher's persisted profile exposed the stale-data path: source `AppState::new` loads folder rows and registers watches (`vendor/photag-source/src/app.rs`, `AppState::new`), but does not scan those folders. The source scans after AddFolder and explicit Rescan in `process_pending`; its watcher only reconciles later notifications. Therefore a database created before the all-file change stays incomplete after reopening unless the user manually rescans, and file changes while PhoTag is closed are not reconciled.

Current mapping: Watcher now runs the production Indexing scan once per persisted root at startup, reports per-root errors, and emits `MediaIndexChanged` after index chunks. The scanner preserves IDs, ratings, date and tags while refreshing file size and modification time; unchanged image metadata retains cached dimensions, while changed content invalidates and regenerates its path-keyed thumbnail and dimensions. After a successful traversal it removes missing files. If a registered root is unavailable, its cached rows are retained. Gallery's event reload preserves still-valid selection and scroll position. This is an intentional startup-reconciliation extension to source behavior, with no schema change; explicit Rebuild/Rescan still clears rows per its existing behavior. Validation is recorded in `docs/validation-ui/2026-10-14-startup-refresh/README.md`.

## Grouping extraction and extensions (2026-10-14)

| Source behavior / caller | Current implementation | Compatibility / extension |
|---|---|---|
| src/ui/gallery.rs build_gallery_layout (698–769), format_month_label; render_gallery -> layout | photag-plugin-grouping GroupingService::arrange/rows; Gallery render adapter | Keeps effective date COALESCE(date_taken,modified_at), collapse/count/virtual rows. YYYY-MM labels and old English collapse-key conversion. Grouping works with every sort, preserving within-group sort. |
| src/db/queries.rs get_media_skeletons/get_media; source session.rs collapsed_months and settings.show_date_dividers | Database compact filtered grouping inputs, Gallery arranged-ID rich window fetches, additive Session grouping_mode | Schema, legacy fields and query filters retained. Modified is indexed mtime; Created is real filesystem birth time or Unknown date; no substitution. |
| No upstream file-type/folder/creation grouping | photag-plugin-grouping FileType/Modified/Created/Folder/Subfolder | User-requested extension; extension groups are case insensitive, Folder is registered root, Subfolder is actual immediate parent path. |

See grouping PORTS and isolated Windows MCP validation for evidence and scale/filesystem limits.

## 子文件夹分组根目录优先（2026-10-14）

上游仅有日期分组（src/ui/gallery.rs build_gallery_layout/其1047–1063调用），本轮细化新增Subfolder：GroupingService::arrange_in_scope以当前query.path_prefix的直属文件组优先，随后子目录；全库优先各登记根目录。旧arrange签名保留，无SQLite或原筛选语义修改。调试请求SelectPath/ExpandPath仍用canonical路径做归属验证，消费端保留已索引输入路径拼写，与原生点击一致，避免MCP额外添加扩展前缀导致空结果。

## None目录浏览扩展（2026-10-14）

| 原行为/调用 | 当前映射 | 本轮差异 |
|---|---|---|
| src/db/queries.rs build_filters:145、path_prefix递归条件 | Database MediaQuery/direct_directory、Gallery load_grouped None投影 | 新内存字段提供字面直属文件条件；原递归API和SQLite未改。 |
| src/scan.rs list_subdirectories:466、app.rs get_subdirs:819 | IndexingService原list_subdirectories及新增try_list_subdirectories | 复用上一轮全文件/隐藏目录策略；新增带错返回，原best-effort语义保留。 |
| 旧Sidebar目录点击/会话路径 | Gallery文件夹卡片→NavigationService请求→现有选择事件/Session | 新卡片双击、上级、空态、根列表；Nav揭示祖先和当前根path高亮。无目录假媒体ID。 |

本轮None由递归平铺改为当前目录直属文件＋子目录；全库搜索/标签视图及其他分组保持递归。验证见validation-ui/2026-10-14-folder-browser。

## Runtime diagnostics mapping (2026-10-14)

| Source / existing boundary | Current implementation | Semantics |
|---|---|---|
| vendor/photag-source/src/db/mod.rs::open_db/open_read_conn; existing DatabaseService::with_reader/with_writer | Same dual WAL connections and closure calls, with caller-tagged tracing spans and SQL error events | No schema/query/locking-order change; timing includes contention |
| Existing feature plugin apply and error stderr paths | tracing spans and ERROR records via photag.debug weak sink | Original return values and lifecycle remain Cordis-owned |
| Existing indexing ProgressRegistry stream proxy | phase/error/stream-finish logs plus monotonic elapsed and scan ID | Same input/output stream forwarding and cancellation behavior |
| User-requested new diagnostic drawer (no upstream counterpart) | photag-plugin-debug + two UI bridge slots + same-process MCP tools | Bounded memory, filtering/pagination, no persistent DB/profile changes |

Backend trace checked pinned DB mod.rs, migrated database wrappers and callers; operation/data semantics are unchanged. Full per-instruction/native/OS tracing and crash persistence are outside this feature; captured records represent actual emitted events and instrumented boundaries.

## Preview latency and idle redraw follow-up (2026-10-15)

| Pinned behavior / boundary | Current mapping | Compatibility |
|---|---|---|
| `vendor/photag-source/src/ui/preview.rs` Fast→Full priorities and immediate gallery texture, `gallery.rs::decode_jpeg_scaled/load_full_image_to` DCT-scaled JPEG and orientation | `PreviewService::open_indexed_media`, `view/image_stage.rs`, `scaled_decode.rs`, prioritized two-worker queue | Gallery thumbnail appears while progressive/native full data load; fast images stay until full is ready. JPEG color-space fallback and EXIF orientation remain. GPUI applies EXIF on original assets, avoiding a redundant oriented PNG; user transforms still compose after EXIF. |
| Source animation stops requesting frames once settled; AI updates reflect worker transitions | `chrome::advance_wheel_animation`, `AiSuggestionsView::poll -> bool` | Settled/unchanged polls stop revision changes, writes and redraws; analysis requests/results remain functional. |
| New diagnostic drawer observes renderer events | `DebugService::view_revision` separates observation collection from redraw scheduling | Accessibility records remain complete; they cannot recursively drive the drawer's own paint. Application errors and domain actions still update live. |

First/full paint markers and MCP state measure displayed image readiness separately
from open-handler spans. Targeted development-profile optimization of image,
JPEG decoders and GPUI pixel conversion does not alter schema or media files.

## Full live idle regression follow-up (2026-10-15)

| Pinned behavior | Current mapping | Compatibility / intentional difference |
|---|---|---|
| `main.rs:2343` Shortcuts and `main.rs:2465` AI groups flatten the already-loaded `AppState.tag_tree` | Settings `tag_choices` background model shared by both pages; Navigation tag revision and MediaIndexChanged invalidation | Same tag IDs/names/hierarchy and F1–F9/group actions; avoid querying the complete navigation model during paint. |
| `main.rs:1698` Statistics invokes original `get_statistics`; `db/queries.rs::get_statistics` defines metrics | Gallery StatisticsRead on the model poller, keyed by index/writer revisions | Existing metrics/audio adaptation retained; load on open/change, not on paint. |
| `main.rs::show_bulk_action_modal`, `app.rs:1601::bulk_action_matching_ids` | Stable raw-input keyed bulk preview and existing fresh action-time revalidation | CurrentSelection still skips filters; same dates/extensions/rating/size predicates and target-change delete confirmation. Relative dates are computed for the request, not for every paint. |
| `db/mod.rs` separate WAL reader/writer connections | DatabaseService adds atomic writer revision after closure completion, also on errors | No SQL/schema/query/result/order change. Conservative memory-only invalidation covers direct mutations and possible partial writes. |
| `ui/preview.rs` mouse activity shows controls, then finite idle fade | Preview coalesces identical coordinates at stage/root and ignores zero-distance pan | Real input still wakes controls; stationary platform observations cannot perpetually reset fade. |
| `ui/gallery.rs` / existing Inspector EXIF fields for selected media | Gallery keyed EXIF read model resolves the path and reads fields off-thread, then projects cached results | Existing field parsing/error messages retained; cache checked before any work and invalidated by identity/data changes. |

Source audit alone missed the Settings/Statistics/Bulk render-time reads; the
same-process MCP matrix reproduced their logging feedback loops before fixing
them. See the full idle validation evidence for actual observed coverage and
remaining OS/codec/update-install boundaries.

### Shared refresh implementation (2026-10-15)

Pinned `src/ui/preview.rs`/`src/ui/info.rs` metadata and tag presentation, Gallery
selection/drag paths, scan progress and transport state keep their existing
service/query semantics. The GPUI implementations now reuse UI Bridge
`refresh::{poll,ChangeGate,PollResult}` and `read_model::ReadModel`, rather than
per-feature timer/receiver/async-completion templates. Rendering can declare a
nonblocking demand; work executes off-thread and model pollers deliver results.
All existing GPUI timers share notification/lifetime handling. SQL schema,
command-time revalidation, Cordis events and file mutation order are unchanged.
See docs/ui-refresh-contract.md for keys, errors and future caller requirements.

### Measured preview decode stages (2026-10-15)

Pinned `ui/gallery.rs:183::decode_jpeg_scaled` reads encoded bytes before DCT
scaling; the current path retains that decoder, fallback, orientation and aspect
semantics while recording each stage. Generic decode now uses the same
extension-selected ImageReader decoder/orientation against a compressed-memory
buffer to separate file read from decoding. PNG/BMP/TIFF/WebP pixel-equivalence
regressions compare the old open_oriented path; JPEG scaling and missing/corrupt
failures are tested. The two-worker queue, urgent ordering, cache freshness and
publication semantics remain; only failed temporary artifacts gain cleanup.
Per-file job/paint timing contracts and cache-hit/skipped-stage limits are in
Preview PORTS. No SQL or normal user media mutation is introduced.
### Preview decode optimization mapping (2026-10-15)

Pinned `src/ui/preview.rs:809-896` explicitly sources filmstrip images from
Gallery's `TextureManager::get_or_load`; the GPUI equivalent now carries
database thumb_path through public `FilmstripMedia` to Preview and validates
freshness in its existing workers. Missing/stale paths retain 96px fallback.
Source's immediate thumbnail/full-image behavior is retained; the GPUI-only
unconditional 1600px intermediate is intentionally limited to pending JPEGs
after 150ms, whose existing DCT-scaled decoder can avoid a complete decode.
PNG full decode is no longer duplicated for an intermediate image. Request
generations own cancellation, and failure is terminal for that generation's
progressive attempt. Cargo dev overrides optimize png0.18.1 and its DEFLATE/
checksum dependencies without dependency upgrades or release-profile changes.
No DB schema, source bytes or metadata semantics change. See Preview PORTS and
the measured optimization report for tests and remaining codec boundaries.


## Browser-mode replacement mapping — 2026-10-15

The user explicitly replaced indexed-library product semantics and excluded old data migration. The legacy source module remains the reference for retained tag hierarchy, queries, metadata writers and file operation behavior. Browser runtime uses a new photag-tags.db and leaves photag.db untouched.

| Original behavior / source entry | Current public boundary | Intentional difference |
|---|---|---|
| vendor/photag-source/src/db/queries.rs + scan.rs, registered-folder indexed media | DatabaseService::open_browser_at; GalleryService::load_grouped; gallery/live.rs | Real local read_dir + versioned session SQLite cache; ordinary browse never persists inventory. |
| source folder add/rescan/remove in state/mod.rs, ui/sidebar.rs | NavigationService::{add_folder,rescan,remove_folder,rename_favorite,move_favorite} | Favorites only; refresh navigates/rereads; removal preserves file associations. |
| source watcher.rs + startup registered-root scan | WatcherService::{set_browse_directory,retry_browse_watch,poll_and_reconcile} | One current directory subscription; direct by default; no favorite startup scan. File events stat/patch; external disappearance preserves tags. |
| source thumbnail pipeline thumbs.rs; preview source decoding | IndexingService::visible_thumbnail; ThumbnailService; PreviewService | Keep source decoding/orientation/embedded thumbnail and existing prioritized Preview; demand only visible browse rows, versioned invalidation and independent tag snapshots. |
| source assign/remove tag queries, xmp.rs | TaggingService::{assign,remove,relink_record,delete_missing_records} | Keep hierarchy/association/IPTC/XMP semantics; assignment captures local snapshot, missing/relink are explicit new operations. |
| source single/bulk file commands and undo actions | FileOperationsService; HistoryService | Known local moves update associations outside favorites; recycling retains missing tagged records; cache pins protect session history. |
| source session selected_media_ids | SessionState::selected_file_paths | Persist file identity by canonical path, resolve a fresh cache row after restart. |
| source global indexed statistics/export/bulk | get_statistics_scoped / export_metadata_scoped / annotated_only | Persistent annotation universe is separate from current visible directory. |
| Yazi VfsEntries, Refresher, local watcher | vendor/yazi-reference + gallery/live.rs + watcher/{watcher,browser}.rs | MIT-attributed adaptation: bounded batches, latest request, generation, cooperative cancel, fallback and stat patches integrated with Cordis/GPUI; not a replacement runtime. |

Validation evidence and exact limits: docs/validation-ui/2026-10-15-browser/README.md. No automatic external move matching or old library migration is introduced.


## 2026-10-16 浏览/Explorer 审查映射修正

| 行为来源 | 当前实现 | 用户需求下的变化 |
| --- | --- | --- |
| vendor/photag-source/src/session.rs 的 selected_path 与直接 save | NavigationPlugin 恢复；SessionService::replace；GalleryView::refresh_index | 浏览任意存在目录，不限制资料库；无变化不写盘、失败不提交内存、刷新不保存瞬时空选择。 |
| 原 sidebar.rs 子目录展开与本项目旧 indexed count | Navigation view/directories.rs 与 sections/render 子模块 | 单一懒加载树；未知数量不显示为 0；当前本地事件使展开树刷新。 |
| 本轮其他 AI 新增 quick_access/favorite_groups JSON | SidebarFoldersService；SidebarFoldersUpdate；SettingsService | 保留新设计，修复重复添加丢别名、异步旧版本覆盖、失败无反馈、分组恢复。 |
| 本轮其他 AI 驱动器/系统已知目录 | navigation/drives.rs；ReadModel | render 不探测本地路径；盘符校验不读取容量；后台独立刷新、容量失败明确显示。 |
| Yazi 分块结果流与现有 ReadModel 最新请求规则 | Gallery monitor snapshot_directory_dirty | 查询在途合并目录块，完成后先交付部分结果再刷新；单 worker 和显式导航过期拒绝规则保持。 |

见 validation-ui/2026-10-16-sidebar-review/README.md。新需求与旧源不同的地方是明确浏览语义变化，不要求旧索引 UI 对等。


## 2026-10-16 重复刷新修复映射

对照 vendor/photag-source/src/watcher.rs 的注册/800ms 防抖流程，保留本项目 notify/PollWatcher 与有界事件处理。浏览改造初期额外添加的注册后根提示与 Gallery activation 无条件 invalidate 是二次重扫来源。现由 FolderWatcher 注册完成版本、WatcherService::browse_registration_pending 和 LiveBrowser 的后台等待统一先订阅后枚举，不再发送该提示；切换焦点只由现有侧栏刷新盘信息，不启动 Gallery 扫描。Gallery 的背景 refresh_index 不再借用会清空列表的完整 reload 流程；新快照读取保留原 rich window。详见 validation-ui/2026-10-16-refresh/README.md。


## 2026-10-16 地址栏/导航历史映射

2026-10-16 延迟优化补充：原 PhoTag sidebar/main 的路径选择和 session 范围仍由
Navigation 事件复用；跳过浏览时无关收藏快照查询。Yazi 固定版本的
`yazi-actor/src/mgr/cd.rs`（新保存的原样 MIT 参考）先切换范围再 refresh/render，
以及已存档 VfsEntries/Refresher 的流、取消、Part/ticket 对应 Gallery live.rs。
共享 UI Bridge Wake/listen 将结果就绪交给原有变化采样器，保留 bounded4、
ReadModel 身份和过期拒绝，不照搬无界队列。真实 D 盘与输入计时、回归边界见
validation-ui/2026-10-16-navigation-latency/README.md。

对照 vendor/photag-source/src/main.rs:41-53 的 session selected_path/selected_folder_id 恢复与 ui/sidebar.rs:292-411 的文件夹选择语义。旧 Navigation render_breadcrumb 只显示“收藏集合 / 末级名称”；现按用户参考图圈定位置替换为盘符至当前目录完整面包屑和路径编辑，Navigation 的目录事件与 Gallery 浏览查询继续复用。新的 location.rs 是窗口内最多 200 项的纯导航历史，不复用文件操作 undo/redo、不新增 DB 表。Session 范围恢复改为只发对应路径/文件夹/标签事件，避免后续空路径事件清空前一个范围。外部目录/标签导航也进入同一历史，明确的新能力以本轮用户要求为准。验证：validation-ui/2026-10-16-address/README.md。

## 2026-10-16 标签内嵌统一确认映射

本轮只组合既有公开命令，不改SQL/schema或新增Tag服务。追踪 `vendor/photag-source/src/app.rs` 的 `PendingAction::UpdateTagAppearance`：名称、appearance、display统一确认；display向所有后代传播，appearance只有明确apply_to_children时传播。Gallery `view/inspector/tag_editor.rs` 先保留名称/颜色/图标/显示草稿，确认时对照Navigation新快照验证目标并调用既有 `rename_tag`、`set_tag_appearance`、`set_tag_display`。本内嵌入口只修改自身appearance，display发生变化时沿用既有后代传播。不同服务调用依次执行，未宣称跨命令原子提交；错误保留反馈。现有完整标签管理器的关联/合并/父级等结构操作不在本轮四字段编辑范围。运行证据见 `validation-ui/2026-10-16-inspector-unified-edit/README.md`。

## 2026-10-17 Lore 历史回退与显示保留映射

| 参考原始代码 | 当前实现 | 修复行为 |
| --- | --- | --- |
| newworkspace/src-tauri/src/plugins/version_info/graph.rs::graph_view；InspectorPanel.vue::versionGraph 的 metadata fallback | VersionsService::list、VersionInfoService::list | 以完整持久化版本目录作为列表主体；回退后的较新版本仍可显示和切回，顺序稳定。 |
| newworkspace/src/plugins/versions/service.ts::call；InspectorPanel.vue::activateVersion | Gallery history_generation、ReadModel；activate_history_version | 写后失效并重新读取版本状态，保留原有版本身份。 |
| newworkspace/src/composables/useWorkspace.ts::refreshEntry | BrowserSnapshot.loading；Gallery monitor::tick | 目录重扫的空/部分结果不再清除文件选择，完成结果仍修复真实消失的选择。 |

逐文件对照、参考项目完整投影协议与当前差异见 validation-ui/2026-10-17-lore-version-history/reference-source-trace.md。

## 2026-10-17 历史 checkout 再提交映射

| 官方/固定来源 | 当前实现 | 行为 |
| --- | --- | --- |
| Lore shared-store 官方恢复顺序；lore-vm storage_shared_store_sync.rs::concurrent_branch_advance_is_rejected_until_sync | versions/commits.rs::prepare_snapshot_commit | 比较本地分支 tip；仅取消失败的私有 content 暂存，按明确本地 revision sync/reset，再复制当前源文件 stage/commit。 |
| Lore GUI RevisionSyncArgs / BranchListEntry.latest | VersionsService::create_version | 离线不使用空 revision，忽略初始全零 tip；03按顺序追加，保留01/02，源文件不被同步覆盖。 |

官方资料、参考项目缺少的预处理及回归证据见 validation-ui/2026-10-17-lore-version-history/official-api-rewind-save.md。

## 2026-10-17 Everything 搜索映射

源项目 `src/db/queries.rs::MediaQuery/build_filters` 将搜索定义为大小写不敏感的文件名、路径和标签子串 OR，图库 `src/app.rs::load_media` 通过同一查询构造器执行筛选、排序和窗口加载。本项目沿用这些PhoTag可见语义：Everything SDK IPC仅提供注册目录内的候选文件路径，数据库再次执行启用的文件名/路径子串和标签条件；媒体类型、扩展名、日期、评分、目录、分组、排序与分页仍由原SQLite查询完成。适配器不可用时走原SQL查询，不改变数据库schema。实现和验证边界见 `validation-ui/2026-10-17-everything-search/README.md`。

顶部搜索范围控件提供文件名、路径、标签和时间范围开关；新增时间项启用或忽略现有日期起止条件，不改写日期筛选语义，默认启用以兼容旧设置。设置通过已有 `SearchScopes` JSON 字段持久化，旧配置缺失 `time` 时按启用处理。

后续修正：SDK 3用转义正则保留原SQL LIKE子串语义，避免把`!`等合法文件名字符解释为搜索操作符。真实SDK命中通过数据库公开`cache_search_files`进入临时会话，不要求文件预先浏览/扫描；共用原浏览缓存upsert顺序，保留行ID/标签/评分，未注释清单不持久化。搜索可准备已安装后台客户端；被动安装/版本读取使用ReadModel，搜索栏在缺少或版本不兼容时提供安装链接。数据库兼容测试、真实IPC/中文路径/分页和提示状态截图见本轮实现记录。

## 2026-10-17 画廊未保存版本提示映射

用户新增提示能力，延续参考项目 VersionInfo 的稳定版本ID/当前标记协议；
不是 vendor/photag-source 中已有的版本提示。固定 Lore GUI 的
`ops/file/hash.rs::hash` 和 `ops/file/info.rs::info` 提供真实内容hash、size及
指定不可变revision查询；本项目 `versions/state.rs::file_state` 先比较当前
所选版本，再比较其余已存版本。Gallery `version_badges` 复用共享ReadModel、
原Watcher的800ms防抖及HistoryChanged失效，在可见窗口后台读取。
只对不存在于任何已存版本的内容显示琥珀点，点击打开History手动保存。
验证与边界见 validation-ui/2026-10-17-version-status/README.md。

### Search location mapping — 2026-10-17

| Source / requirement | Current mapping | Compatibility |
|---|---|---|
| vendor/photag-source/src/app.rs search_query and selected_path/selected_folder_id; src/db/queries.rs MediaQuery folder/path and filename/path/tag predicates | GalleryFilter carries explicit search_entire_computer; search.rs scope_text_search normalizes the current directory to a recursive directory_tree, or removes directory restrictions for all-computer SDK queries | New user-requested switch; no source UI equivalent. Existing field/date/tag filtering remains; empty search preserves original direct/recursive browser semantics. No database migration. |

Everything receives the selected directory as its query root in local mode;
global mode supplies no roots and follows its configured index. It does not
reconfigure indexed drives or exclusion rules. Real same-process 2→3→2 hits,
button click, quiet state and settings restart evidence are recorded in
validation-ui/2026-10-17-everything-search/README.md.

### Program launcher mapping — 2026-10-17

| Source / requirement | Current mapping | Compatibility |
|---|---|---|
| User-requested independent launcher, desktop-icon reference and installation sorting; no launcher equivalent in vendor/photag-source | LauncherService discovery reads Windows Start Menu/uninstall registry; IShellItemImageFactory supplies native icons; DatabaseService owns typed launcher entries/nullable installation metadata; shared GPUI panels/menus/ReadModel/Wake supply UI infrastructure | Additive app_launcher_metadata table, no changes to source media/tag schema. Previous launcher database upgrade and restart regression preserves custom names/categories/hidden state. Unknown installation date/size remain unknown. |

### Clipboard workbench mapping — 2026-10-18

| Source / requirement | Current mapping | Compatibility / evidence |
|---|---|---|
| User-requested clipboard history, organization and reference-style floating panel; no clipboard history/group capability in vendor/photag-source | ClipboardService uses clipboard-master/arboard; public content type classifier derives text/link/code. DatabaseService owns clipboard tables, assignment and category CRUD. UI Bridge owns the single floating panel/sidebar/command/shortcut contribution. | Additive clipboard tables; existing browser-store whitelist now persists them and reconciles deletions. Photo schema and source queries unchanged. Database11/type1 tests plus 91-step same-process UI and three-process restart evidence; OS capture/write/hotkey delivery remain unverified. |

### 2026-10-18 剪贴板收藏语义映射

已追踪 ClipboardService → DatabaseService::clipboard_* → browser_store::persist，以及 history/inspector/sidebar 的删除调用。vendor/photag-source 为照片软件，没有对应剪贴板收藏功能。复用现有共享 SQLite 保存收藏，兼容旧 pinned 列；普通删除、自动淘汰和清理不能删除收藏，只有 clipboard_remove_favorite 显式删除本地内容。
# 2026-10-18 独立收藏分组映射

## 2026-10-18 被动版本状态缓存

vendor/photag-source无此版本功能；沿用本项目固定Lore不可变版本比较。已追踪Gallery
flush_visible_window→request_version_states→VersionsService::file_state，以及共享Database
write_revision（也包含剪贴板写入）和Lore StoreInstruments→SDK meter复用调试日志。
仅去除版本状态缓存键的无关全库写入修订，文件监听/快照和显式历史修订保持；无数据迁移。
复现及真实变化证据见validation-ui/2026-10-18-version-idle/README.md。

| 源行为/模型 | 当前公共入口 | 本轮处理 |
|---|---|---|
| `vendor/photag-source/src/db/queries.rs` add_folder/list_folders/remove_folder；`db/mod.rs` folders.id 为 INTEGER PRIMARY KEY，可能重用 | Navigation snapshot 的登记根；DatabaseService 原有登记/删除方法与扫描/监听链路 | 不改 SQLite，不把收藏删除映射为源 remove_folder（它还会删除媒体索引）；登记操作保留为明确菜单动作 |
| 用户要求多个独立收藏组、＋收藏当前目录；源 sidebar.rs 没有对应分组模型 | SidebarFoldersService::import_library_folders、SidebarFoldersConfig、SidebarFoldersChanged→Settings；Session 已提交位置 | 将旧登记路径一次性复制为快捷收藏，规范化路径标记而非数据库 ID，保留别名/顺序，删除后不复活；新增 serde-default 字段兼容旧 JSON。无需修改源查询或媒体数据 |

# 2026-10-18 全局工作台主题映射补充

| 源行为/模型 | 当前公共入口 | 本轮处理 |
|---|---|---|
| `vendor/photag-source/src/settings.rs:52,141–142,225` 的强调色名默认/序列化 | Settings `accent_name` → UI Bridge `DEFAULT_ACCENT_NAME` / `set_accent_name` | 按用户新设计默认工作台蓝；保存的 System/旧预设继续解析，不改 JSON 字段、路径及用户文件 |
| 源图像/标签/程序数据与当前公共服务 | 原有 Database、Tagging、Launcher、Navigation、Gallery 服务 | 本轮只改渲染和共享组件，不替换数据查询、文件操作、媒体像素或用户标签色 |
| 旧 v5 / 剪贴板私有 palette | UI Bridge `theme`、`components`、`panels` | 深蓝公共主题覆盖历史配色，功能布局仍按活动参考；详见 `docs/ui-theme.md` |
