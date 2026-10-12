# PhoTag source audit and migration map

**Audit baseline:** `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` (2026-08-04),
PhoTag v0.2.0. Mirrored source: `vendor/photag-source/`. Source count:
18,434 Rust lines across 22 files. Migration status below was updated 2026-10-12.
This audit catalogs documented behavior and original source locations; it is
not a claim that every internal branch has been exhaustively behavior-tested.
Detailed code-level migration requires working through the listed modules and
capturing behavioral regression cases before each port.

## Feature-to-source mapping

| Original feature / behavior | Original source | New destination | Status |
|---|---|---|---|
| App lifecycle, dialogs, pending action dispatch, undo/redo, selection state | `src/main.rs`, `src/app.rs` | shell + app-state/history plugins | Not migrated |
| Folder and tag navigation, tree editing, folder actions | `src/ui/sidebar.rs`, `src/db/queries.rs` | `photag-plugin-navigation` + indexing/database/watcher plugins | Folder and tag tree snapshots, recursive folder counts, lazy subdirectory listings, tag create/rename/delete/reparent/appearance/display commands, and source hex-color helpers are migrated behind Cordis. The GPUI sidebar accepts a folder path and invokes the original add service, which validates the directory, stores it, registers monitoring, and starts scanning; the view exposes refresh/rescan/remove and displays rescan progress; rescan cancels any prior scan and clears indexed rows before restarting; remove cancels scanning, unregisters monitoring, and removes indexed rows through the source query. Root and nested folders expand through the lazy listing service, display recursive counts, and emit Cordis path-selection events consumed by gallery filtering. Folder and tag selection events clear conflicting filters. The native folder picker, tag context menus and drag/drop editing remain pending. |
| Virtualized gallery data window, selection, grid, month groups | `src/ui/gallery.rs`, `src/app.rs::load_media/ensure_media_window` | `photag-plugin-gallery` + GPUI shell | Data layer preserves compact ordered skeletons, 200-item rich-row window, tag batching, available extensions and folder counts. Viewport shift and page-loading regression tests pass. Keyboard selection rules for arrows/Home/End, shift ranges, Ctrl+A, ratings 0–5, Escape, Enter and Delete are migrated to a host-neutral state machine with tests; GPUI binds arrows/Home/End/Ctrl+A/Escape and rating digits 0–5, and selection writes use the original bulk update query. GPUI cards paint indexed cached thumbnails and support plain, Ctrl and Shift click selection plus batch rating buttons. Automatic viewport-driven paging and scrolling to keyboard targets, Enter/preview, Delete confirmation, progressive thumbnail sizing, neighbor prewarm, micro-thumbnail placeholders, date grouping, advanced filters, rubber-band selection, drag/context interactions and dispatching host actions remain incomplete. |
| Progressive image thumbnail decode, EXIF orientation, DCT-scaled JPEG decode, 8×8 micro-thumbnails, path-hash cache | `src/thumbs.rs` | `photag-plugin-thumbnails` | Original image pipeline copied without algorithm changes, wrapped as a Cordis service and tested for generation/cache reuse. Windows/UI integration is pending. |
| Search, sort, date/rating/type/extension/folder filters | `src/ui/filterbar.rs`, `src/db/queries.rs`, `src/app.rs::load_media` | `photag-plugin-gallery::filter` + database plugin | Host-neutral filter state maps all source query scopes, tab-specific folder/path behavior, media type, include/exclude tags/extensions, inclusive calendar-day bounds, ratings and sort order into existing `MediaQuery`. Mapping tests pass. GPUI search input updates and reloads the source query, All/Photos/Videos/Audio controls update the media-type query, and all eight source sort orders can be selected. Search scope popup, date/rating/extension controls, chips, clear/reset interactions and settings persistence remain unported. |
| Image preview, progressive decoding, zoom/pan/rotate/flip, toolbar and filmstrip | `src/ui/preview.rs`, `src/thumbs.rs` | preview plugin | Not migrated |
| Video/audio playback, synchronization, speed, rotation metadata, ffmpeg integration | `src/video.rs` | media-playback plugin | Not migrated |
| EXIF and metadata display, dimensions, paths, open count | `src/ui/info.rs`, `src/ai.rs`, `src/db/queries.rs` | metadata plugin | Not migrated |
| Hierarchical tags, ancestor auto-apply/manual distinction, associations, merge, rename, reparent, colors/icons/order/display | `src/db/queries.rs`, `src/ui/sidebar.rs`, `src/ui/preview.rs` | navigation + `photag-plugin-tagging` | Database-backed assignment, one-way association expansion, automatic ancestor rows, manual/auto distinction, removal/clear, merge, and navigation-side rename/reparent/appearance/display are exposed through Cordis services. Synthetic assignment regression passes. Tag picker/context-menu UI, hotkeys, XMP updates after tag mutations, and visual/interaction parity remain pending. |
| Ratings, pick/reject, keyboard shortcuts, tag hotkeys | `src/app.rs`, `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/settings.rs` | `photag-plugin-gallery` + command/keybinding contributions | Rating selection effects persist through the source bulk update query (0–5 clamp verified), with GPUI card selection, batch controls and 0–5 key handling. Arrow/Home/End/Ctrl+A/Escape selection keys are bound; pick/reject, configurable shortcut settings, Enter/Delete actions and F1–F9 tag hotkeys remain unported. |
| Search tag assignment menus and context menus | `src/ui/gallery.rs`, `src/ui/preview.rs`, `src/ui/sidebar.rs` | menu contributions owned by feature plugins | Not migrated |
| Recursive multi-thread scanning, hidden-file filtering, video/poster pairing, DB batches, image/video thumbnails, cancellation/progress, missing-thumbnail backfill, child directory listing | `src/scan.rs`, `src/thumbs.rs`, FFmpeg helpers from `src/video.rs` | `photag-plugin-indexing` + `photag-plugin-thumbnails` | Scanner workflow copied and adapted behind a Cordis service; synthetic folder scan regression test passes. Real media corpus, FFmpeg, Windows paths, rescan policy and watcher integration remain unverified. |
| Filesystem watcher and reconciliation | `src/watcher.rs`, `src/scan.rs`, `src/app.rs` watcher reconciliation | `photag-plugin-watcher` + indexing plugin | Original create/remove/rename filtering, hidden-file exclusion, 800ms debounce, 4,000-event drain and 200-path reconcile budget are retained. Cordis plugin watches registered folders, runs reconciliation in a bounded background poller, triggers missing-thumbnail generation for added rows, emits a Cordis media-index event after database changes, and stops the worker with the plugin. Linux integration tests cover file create/remove and dispatch of both change notifications; native Windows delivery and DB/file race cases remain unverified. |
| Recycle Bin delete, copy/move/rename/create folder, reveal/open, index follow-through | `src/app.rs`, `src/db/queries.rs`, `src/drag.rs` | file-operations plugin | Not migrated |
| Drag-out to Explorer/drop targets via `CF_HDROP` | `src/drag.rs` | Windows shell integration plugin | Not migrated |
| Bulk action console and filter-driven delete/copy/move | `src/app.rs`, `src/main.rs`, `src/db/queries.rs` | bulk-operations plugin | Not migrated |
| JSON export of metadata | `src/db/queries.rs`, `src/main.rs` | metadata-export plugin | Not migrated |
| AI MobileNetV2 suggestions, brightness/warmth/saturation analysis, mandatory groups, ONNX model download | `src/ai.rs`, `src/settings.rs` | ai-tagging plugin | Not migrated |
| JPEG IPTC and XMP sidecar writing, restore original modified time | `src/xmp.rs`, `src/app.rs` | metadata-write plugin | Not migrated |
| Settings persistence, accent presets, search scopes, tag search/merge behavior, toolbar config/position, update setting | `src/settings.rs`, `src/theme.rs` | settings + theme plugins | Not migrated |
| Session restore: folder, selection, scroll, expanded nodes, sidebar width, filters | `src/session.rs`, `src/app.rs` | session plugin | Not migrated |
| Mica/Acrylic, dark title bar, system accent, fallback | `src/winfx.rs`, `src/theme.rs` | Windows appearance plugin | Not migrated |
| GitHub update check, confirmation, download, SHA256 verification, staged swap/rollback | `src/update.rs`, `src/settings.rs` | updater plugin | Not migrated |
| Theme, typography, custom fonts, icon availability, visual tokens | `src/theme.rs` | GPUI Kit theme layer + theme plugin | Not migrated |
| SQLite schema, migrations, WAL, separate read connection, indexed queries | `src/db/mod.rs`, `src/db/queries.rs` | `photag-plugin-database` | Core schema/query source copied; Cordis service and separate connections implemented. A pre-migration database fixture verifies media rows and tag links survive opening. Broad SQL regression coverage remains incomplete. |

## Implemented migration slices

| Slice | Files | Status / limits |
|---|---|---|
| Cordis-owned UI contribution catalog, executable GPUI view factories, duplicate validation, command event, plugin-stop cleanup | `crates/photag-ui-bridge/src/lib.rs` | Metadata and typed GPUI view factories are registered against Cordis plugin owners and removed on disposal; lifecycle test covers both catalogs. Shell mounts factory views into slot regions. Most original feature rendering and interactions remain unported. |
| Original SQLite schema and query code behind a Cordis service | `crates/photag-plugin-database/src/db/` and `src/lib.rs` | Implemented with reader/writer connections, Cordis lifecycle test, fresh schema test, and existing-library compatibility fixture. Broad query behavioral coverage remains pending. |
| Original image thumbnail pipeline behind a Cordis service | `crates/photag-plugin-thumbnails/src/` | Implemented and tested on generated PNG input; shell starts and stops this plugin. Image-format and Windows integration parity remain to verify. |
| Original folder scanning and thumbnail backfill | `crates/photag-plugin-indexing/src/{scan,maintenance,ffmpeg}.rs` | Integrated as Cordis service depending on database and thumbnail services; shell starts/stops it. Scan test checks hidden-file exclusion, same-stem video poster pairing, DB rows, cache output and completion signal. Actual library parity and FFmpeg extraction still need Windows/media-corpus tests. |
| Original filesystem watcher and reconciliation | `crates/photag-plugin-watcher/src/` | Cordis plugin depends on database and indexing, watches persisted folders, debounces events, reconciles SQLite and triggers thumbnail backfill. Classification and local native event create/remove integration tests pass on Linux; Windows Shell behavior remains pending. |
| Gallery query/window service and initial GPUI card view | `crates/photag-plugin-gallery/src/` | Cordis service keeps compact skeletons, a 200-row media window, associated tags/extensions/counts, source rating updates and live search-query refresh. It consumes Cordis folder/tag/path selection and media-index events, clearing conflicting filters and reloading on selection or index changes. GPUI cards paint cached thumbnail files, load additional rows by page, support tested plain/Ctrl/Shift selection, 0–5 batch/key ratings, basic keyboard selection/navigation keys, media-type controls and all eight source sort orders. Automatic viewport paging, scrolling to keyboard targets, Delete/preview, remaining filter controls, actions and full visual parity remain outstanding. |
| Sidebar navigation/tag service and initial GPUI tree view | `crates/photag-plugin-navigation/src/` | Cordis plugin provides folder/tag snapshots, subdirectory counts, source tag mutations, and scan/watch-coordinated folder operations. Initial tree view is mounted at `workspace.sidebar`; path-based Add, lazy root/nested directory expansion, Refresh/Rescan/Remove, Open/All Media and tag Filter/Clear actions are wired to services/events. The native folder picker, tag editing menus, drag/drop and full visual parity remain unported. |
| Media tagging and association service | `crates/photag-plugin-tagging/src/lib.rs` | Cordis service wraps source assignment/association/merge queries and preserves auto-applied ancestor semantics; synthetic assignment/removal regression passes. Picker UI, hotkeys and XMP rewrite workflow are not yet ported. |
| GPUI Kit native shell and runtime composition | `crates/photag-shell/src/main.rs` | Compiles on Linux. Hosts the contribution catalog and emits command events; it does not yet implement PhoTag’s original screen or controls. Windows runtime remains unverified. |

## Source file inventory

Line counts are from the pinned commit. Original files over the requested 800-line
limit are retained unchanged in the audit mirror; they must be split along
behavioral seams during migration without dropping behavior.

| File | Lines | Migration area | Status |
|---|---:|---|---|
| `src/main.rs` | 2,735 | startup, main UI composition, modal/settings behavior | Audited inventory; not ported |
| `src/app.rs` | 1,975 | state, pending actions, undo/redo, operations | Audited inventory; not ported |
| `src/db/queries.rs` | 1,317 | all query and domain persistence operations | Copied into database plugin and split across five modules, all below 500 lines; broad regression coverage pending |
| `src/ui/preview.rs` | 2,748 | preview and interactions | Audited inventory; not ported |
| `src/ui/gallery.rs` | 2,176 | gallery and interaction | Gallery data window, keyboard selection rules and rating persistence partially migrated; native GPUI view and pointer interactions remain unported. |
| `src/ui/sidebar.rs` | 838 | folder/tag sidebar | Navigation/tag service partially migrated; GPUI layout, context menus and drag/drop remain unported. |
| `src/ui/filterbar.rs` | 822 | search/filter/sort controls | Audited inventory; not ported |
| `src/ui/info.rs` | 350 | metadata info panel | Audited inventory; not ported |
| `src/db/mod.rs` | 190 | schema, migrations, connections | Audited inventory; not ported |
| `src/scan.rs` | 489 | indexing and thumbnail work queue | Migrated to indexing plugin; split into scan and maintenance modules. Behavioral coverage is partial. |
| `src/thumbs.rs` | 420 | image/video thumbnail generation | Audited inventory; not ported |
| `src/video.rs` | 1,037 | video/audio playback | Audited inventory; not ported |
| `src/ai.rs` | 645 | AI inference and image analysis | Audited inventory; not ported |
| `src/update.rs` | 618 | updater | Audited inventory; not ported |
| `src/xmp.rs` | 278 | metadata write | Audited inventory; not ported |
| `src/watcher.rs` | 182 | filesystem watcher | Migrated to Cordis watcher plugin; Windows native behavior is unverified. |
| `src/drag.rs` | 332 | Windows drag-out | Audited inventory; not ported |
| `src/winfx.rs` | 288 | Windows visual effects | Audited inventory; not ported |
| `src/settings.rs` | 306 | persisted settings and key mappings | Audited inventory; not ported |
| `src/session.rs` | 73 | session persistence | Audited inventory; not ported |
| `src/theme.rs` | 610 | theme and drawing primitives | Audited inventory; not ported |
| `src/ui/mod.rs` | 5 | UI module exports | Audited inventory; not ported |

## Interaction inventory from upstream README

Gallery keys: configurable open key (default Space), arrows/Home/End navigation,
Ctrl+A select all, 0–5 ratings, F1–F9 tag hotkeys, Delete/Backspace, Ctrl+Z,
Ctrl+Y, Ctrl+Shift+D bulk console. Preview keys: arrows, I info toggle, F2 rename,
Delete/Backspace delete-and-advance, Escape return. Arrow/Home/End selection, Ctrl+A, ratings with persisted 0–5 bulk updates, Delete/Backspace and Escape/Enter selection handling are migrated as host-neutral behavior; GPUI key event binding and F1–F9 tag hotkeys remain pending. Filter UI visual/interactive behavior is also not migrated even though filter-to-query semantics are. Additional click/drag/context menu behaviors must be enumerated from the source during UI port; this list is not a substitute for that review.

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
