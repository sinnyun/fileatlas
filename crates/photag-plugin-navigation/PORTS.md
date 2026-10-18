# Navigation plugin ports / 导航插件数据对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: the Cordis navigation service, folder tree actions and GPUI sidebar are present. The sidebar uses `rfd::AsyncFileDialog` for native directory selection and sends the selected path through the existing folder-registration, watcher and indexing flow. `library_refresh()` forwards the Watcher startup reconciliation state when that provider is available. Folder and subdirectory menus expose new-subfolder and file-manager actions. The tag UI exposes create, rename, delete, color/icon, display, one-way associated-tag assignment, reparent, merge, and confirmed deletion with optional ancestor-assignment cleanup. Appearance can be applied recursively while skipping descendants that already have an explicit color or icon. Typed GPUI drag/drop moves a dragged tag before/after/into an explicit row target. PhoTag has no folder-tree drag/drop behavior.

## Lifecycle and dependencies

- Cordis plugin name: `photag.navigation`; published service: `photag/navigation` (`NavigationService`).
- Requires `DatabaseService`, `IndexingService`, and `SessionService`; attaches `WatcherService` when available.
- GPUI view contribution: `navigation/sidebar-view` in `workspace.sidebar`.
- Sidebar rail extension slot: `navigation.launcher-shortcuts`; the program
  launcher contributes the Apps button here, and Navigation mounts it beside
  Folders/Tags/Add-ons.
- UI emits `FolderSelectionChanged(Option<i64>)`, `TagFilterSelectionChanged(TagFilterSelection)`, and `DirectoryPathSelected(Option<String>)`; `TagSelectionChanged(Option<i64>)` remains as a legacy single-tag contract. It emits `MediaIndexChanged(())` after operations that alter indexed membership.
- On sidebar startup, restores `sidebar_tab`, `selected_folder_id`, `selected_path`, `expanded_subdirs`, and `expanded_tag_ids` from `SessionService`. Folder IDs must still exist; selected paths must still resolve inside a registered folder; expanded paths are rebuilt through the live subdirectory listing. Tag expansion IDs missing from the current tag tree are pruned on startup. Startup emits folder and path selection events in order so Gallery receives the restored scope. Selecting a folder or All Media saves `selected_folder_id` and clears `selected_path`; selecting a path clears `selected_folder_id` and saves `selected_path`. Tab, directory-expansion and tag-expansion updates preserve fields owned by other consumers.
- Tag filtering restores and saves `SessionState.tag_click_mode` (Select/Exclude). The mode buttons control plain tag clicks and the source configured modifier inverts the mode. The All action applies the current mode to the full tag tree; emitted included/excluded IDs flow through `TagFilterSelectionChanged` to Gallery.

## Service input/output

- `snapshot()` -> registered folders with media counts and the recursive PhoTag tag tree.
- `request_ui(NavigationUiRequest)` -> queues one typed action for the mounted sidebar. Tag modal requests (`OpenCreate`, `OpenEdit(tag_id)`, `OpenDelete(tag_id)`, `OpenMerge(tag_id)`, `OpenMove(tag_id)`, and `CloseModal`) retain their existing validation and behavior. `ExpandFolder(folder_id)` loads that registered root's direct children idempotently; `SelectPath(path)` validates the canonical directory is inside a registered root before selecting it through existing folder/path events. `AllFiles` clears scope. Requests are consumed on the GPUI thread by the same cached `SidebarView` used by sidebar, breadcrumb, and tag modal. Invalid IDs/paths, a poisoned queue, and a second pending request return `NavigationUiRequestError`. The sidebar polls this queue with the existing native folder-picker request.
- `subdirectories(path)` -> lazy direct child nodes: name, path, child-presence and indexed count.
- `add_folder(path)` -> validates an existing directory; returns `(Folder, Receiver<Option<ScanProgress>>)` after registering it and scheduling indexing. Watcher registration follows when available.
- `rescan(folder_id)` -> cancels prior scan, clears folder media rows and starts scanning; returns a progress receiver.
- `cancel_scan(folder_id)` -> requests cancellation of that folder's active indexing work; succeeds when none is active.
- The live sidebar shows the active folder scan's phase/count/progress and Cancel control; Cancel calls `cancel_scan` with the registered active folder identity and drops the view's receiver. Immediate `add_folder`/`rescan` errors and terminal `ScanProgress.error` events both render a retry card; folder failures retry through `rescan`, path registration failures retry through `add_folder`.
- `remove_folder(folder_id)` -> cancels scanning, removes indexed rows and unregisters watcher; returns the removed folder.
- `create_tag(name, parent_id)` -> new `Tag`; parent is optional. The create modal provides a searchable hierarchical parent picker and a root option; tag IDs remain internal. Each tag row's **Use as parent** selects that row in the picker.
- `rename_tag(id, name)` -> `()`; **Edit fields** loads name and **Rename** persists it.
- `delete_tag(id, clean_parents)` -> affected media IDs; deleting an unassigned tag or a root tag proceeds immediately. Deleting an assigned child tag asks whether strict ancestor tag assignments should also be removed from those affected media. Both confirmed choices delete the target; cleanup removes ancestor links first. The returned IDs are sent to `TaggingService` for metadata refresh.
- `set_tag_appearance(id, color, icon)` -> `()`; color accepts `#RRGGBB` or empty to clear; icon accepts text/emoji or empty to clear. **Edit fields** loads existing values; **Save style** applies them to the selected tag. **Apply to children** propagates both style fields depth-first; **Skip edited children** (enabled by default) preserves descendants with an explicit color or icon, matching the source override rule.
- `TaggingService::associated_with(id)` / `set_associations(id, ids)` -> source-compatible one-way association list. **Edit fields** loads the selected tag's associated IDs; its checkbox list excludes the tag itself, and **Save style** replaces its outgoing associations. Assigning the tag expands its associated tags and their required ancestors through Tagging.
- `set_tag_display(id, display)` -> `()`; applies gallery chip visibility to the selected tag and all descendants, matching original edit behavior.
- `move_tag(id, TagDropPos)` -> IDs of media whose implicit tag set changed. The tag name is the drag source; highlighted targets above, on, and below another tag invoke `Before`, `Into`, and `After` respectively. The move modal provides a searchable hierarchy picker excluding the source subtree; target IDs remain internal. Database cycle checks and subtree path recomputation are authoritative; self-drops and cycle-forming moves return an empty list as safe no-ops. Only emit `MediaIndexChanged` when the returned affected-media list is nonempty.

## GPUI controls and debugging

The v5 sidebar has a 48-logical-pixel page rail with Folders, Tags and Add-ons. The host shell owns Settings separately. The selected page is shared through UiBridge surface value `navigation.tab` (`folders`, `tags`, or `extensions`) and persisted in `SessionState.sidebar_tab`; MCP can set that same surface value. Folders provides a real `navigation.folder-search` surface value bound bidirectionally to its Input. Search filters registered roots and already expanded/lazy-loaded descendants, so it does not scan the filesystem on each keystroke. Root folder expansion and directory selection continue through NavigationService and the existing selection events. The folder tree uses Lucide folder glyphs, right-aligned counts, an accent selection row and connector lines. Add-ons mounts executable view contributions registered to the dedicated `navigation.extensions` UiBridge view slot. With no such contribution, it shows an explicit empty state; core command and menu metadata is not presented as product content.

The tag page keeps the existing GPUI Kit `Input`, `Button`, dropdown actions, and typed drag payloads. It supports included and excluded tag sets, Select/Exclude click modes, the configured modifier to apply the opposite mode for one click, selecting or clearing all tags in the active mode, per-mode clear actions, and source-compatible Contains/Starts with search matching that reveals matching descendants and their ancestors. `TagInteractionPreferencesChanged` supplies the current settings without adding a Settings → Navigation crate dependency; defaults are Ctrl, Contains, persistent search, and reparent children when merging. Tag modal surfaces dismiss on Escape. Drag a tag name onto the highlighted target above/on/below another row; database errors appear in the snapshot panel. Self and cycle drops are silently rejected by the source-compatible database operation. For stale Gallery results after inherited assignments change, inspect `MediaIndexChanged` delivery and the Gallery subscriber.

## Original source mapping / 源行为映射

Reference: pinned `vendor/photag-source/src/ui/sidebar.rs` (`show_sidebar`, `show_tag_tree`, `show_tag_node`) and tag actions in `vendor/photag-source/src/app.rs`.

- Migrated: folder registration/rescan/removal, lazy directory listing, recursive tag tree, single-tag filter, root/child creation, rename, delete, color/icon editing, display propagation, reparent into a selected tag, Folder/Tags tabs, tag-search clearing preference, physical subfolder creation, open-in-file-manager action, and recursive tag appearance propagation with explicit-child override preservation.
- Migrated: native directory picker via the same `rfd` API used by PhoTag's `PendingAction::AddFolder`. Cancel/backend `None` leaves the sidebar unchanged; folder validation and registration errors surface in the navigation panel as before. Tag drag/drop reparenting and sibling ordering use the source `TagDropPos` database commands. PhoTag does not implement folder drag/drop.
- Source history has no tag-tree edit undo entries. Tag assignment recency is shared by Gallery and Preview; tree edits do not alter recency. Expanded tag nodes are persisted; Windows visuals remain unverified.
- Source behavior audit (2026-10-13): `vendor/photag-source/src/ui/sidebar.rs`'s folder/tag pivot, lazy folder expansion, tag search match modes with ancestor visibility, include/exclude selection, modifier override, bulk mode selection/clear, expansion persistence, and row drag zones have corresponding implementations here. Source context-menu operations create child tags, edit appearance/display, select a parent, merge into a filtered target list, and confirm assigned-child deletion with either parent cleanup or preservation. Merge excludes the source subtree and lets users choose whether its children move under the target or are deleted. Windows rendering and interaction remain unverified.
- Local integration: verify event refresh and destructive delete behavior against representative PhoTag databases; run Windows GUI/filesystem flows; then finish visual and interaction parity. Plugin tests remain deferred to the local integration pass.

## Shared visual tokens

The sidebar uses photag-ui-bridge::theme for the source backdrop-aware base surface and secondary, muted, and danger text colors. Layout spacing and typography still use GPUI Kit presets; Windows visual parity is unverified.

## Code map

- `src/view.rs` owns the sidebar view state, folder scanning/picking, selection, and view lifecycle.
- `src/view/tag_actions.rs` implements tag creation/editing, appearance/association updates, delete/merge/reparent actions, and tag display propagation.
- `src/view/render.rs` builds the folder/tag sidebar; `src/view/render/tag_render.rs` renders tag rows and subtrees; `src/view/tag_modal.rs` renders create/edit/rename/delete/merge/move surfaces; `src/view/support.rs` holds tree traversal and session restoration helpers.


## Localization integration (2026-10-13)

Visible control labels consume the public UI Bridge translator supplied by
photag.i18n. Stable commands, source settings/schema values, media paths and
user-created tag data are unchanged. Missing translations fall back to English;
OS/backend errors and some generated diagnostics are not fully localized.
The real MCP-only validation coverage and remaining source layout differences
are recorded in docs/validation-report.md; screenshots do not prove pointer
interaction or animation parity.

## Empty-library folder command (2026-10-13)

The Cordis-owned `UiCommandInvoked` listener accepts
`"navigation/add-folder"`. A coalesced atomic pending flag is consumed by the
mounted Sidebar entity every 100 ms on the GPUI thread and calls its existing
native `pick_folder` flow. Cancellation is a no-op. A valid directory uses
`NavigationService::add_folder`, production Indexing, watcher registration
and existing progress/error handling. The listener is removed on plugin stop;
the poller ends when its view entity disappears. No new public service payload
or database schema is introduced.

## v5 navigation projection update (2026-10-13)

The sidebar now renders All files, the registered-folder tree and the tag tree
at the same time; the old Folders/Tags pivot is removed. All files is the first
row, followed by the Folders group header and tree. Folder/subdirectory actions
remain in hover `⋯` menus; subdirectory names select their path, and chevrons
expand descendants. The Tags header keeps Search and Create; its searchable
filter/mode controls expand only when Search is activated. Tag names toggle
their selection state and selected rows use an accent bar and background;
create-child, merge, edit, rename, display, appearance, parent, move and delete
remain in each row's hover `⋯` menu. All files clears both folder/path selection
and included/excluded tag filters.

`navigation/sidebar-view` in `workspace.sidebar` and
`navigation/breadcrumb-view` in `command.breadcrumb` are projections of one
cached `SidebarView` entity. The breadcrumb observes that entity and derives
its current location from its existing session and navigation snapshot; its
All files action clears the complete navigation scope. The two views remain
Cordis-owned and are removed when the navigation plugin stops. The initial
PhoTag MCP screenshot was checked against `00-main-library.png`; the observed
sidebar density and ordering mismatches were corrected in the latest source.
The revised state still needs a new MCP capture and interaction check.

The `workspace.modal` projection in the same view set handles tag creation,
rename/edit, assignment-aware deletion confirmation, merge target selection,
and reparenting. `NavigationService::request_ui` exposes typed create/edit/delete/
merge/move/close-modal requests plus `ExpandFolder(folder_id)` and validated
`SelectPath(path)` for the mounted view; requests invoke existing actions and reject invalid IDs/paths or an occupied single-request slot. Parent and
move modals use searchable hierarchy paths, and Escape dismisses an active tag
modal. Merge still executes when a target is selected; the source-compatible
child reparent/delete option is retained. The edit modal includes name, color
swatches, icon presets, associated tags, child appearance propagation and
display-on-thumbnail state. Live appearance and interaction remain unverified
until checked through PhoTag MCP.

## 2026-10-14 current implementation and MCP evidence

Reference00/04 sidebar and five tag modal layouts were rendered via PhoTag MCP. New NavigationUiRequest::AllFiles dispatches existing folder/tag deselection. MediaIndexChanged listener/poller refreshes sidebar counts; modal render also consumes queued requests so close/switch is reachable under an open modal. Terminal scan errors preserve the last progress error for retry. Real tag tree/production data semantics are unchanged. Native tag submission/drag/scan-failure interaction remain unverified.

Finalv2 corrections: ExpandPath(String) validates the same registered-root directory boundary as SelectPath. Request paths are canonicalized before selection so SQLite path_prefix matches real indexed Windows paths. Tree render receives selected_directory directly; it cannot read its currently-updated entity. Headers/menus now compact overlay, rgba white0x38 tree branches are visible; blue selected fill is explicit RGBA. MCP expanded root and03-nested, selected02-mixed-formats matched3 real media, searched mixed/noresults, clicked Folders/Tags through GPUI events.

## Native folder picker crash fix (2026-10-14)

Sidebar +, Browse and command requests now start rfd::AsyncFileDialog::pick_folder in a GPUI task. The native modal loop runs on rfd's separate Windows STA thread, outside the Sidebar/App borrow. Returned FileHandle paths still call add_folder_path→NavigationService::add_folder→watcher/indexing; None remains a no-op. FolderPickerState and PickerLease enforce one in-flight dialog, disable the picker buttons while pending and reset pending on completion/drop. Sidebar owns the Task so view disposal cancels it; lease drop requests closing its same-process native window when already created.

Public NavigationService::folder_picker_pending()->bool, folder_picker_window()->Option<isize>, cancel_folder_picker()->Result<(),String> support direct plugin MCP validation. Native lookup is limited to this process, #32770 class and the exact current picker title. Cancellation posts WM_CLOSE to that identified dialog, never generates mouse/key input and never chooses/registers a directory. Late window creation during task disposal is not separately exercised. Real Windows button→native dialog→cancel→reopen was verified twice; native folder-selection acceptance remains unverified in this run. See docs/validation-ui/2026-10-14-folder-picker.

## Indexed path spelling in debug requests (2026-10-14)

SelectPath/ExpandPath are still canonicalized for registered-root containment validation by NavigationService::request_ui. Their GPUI consumer preserves the supplied indexed path spelling when selecting/expanding; it no longer adds Windows extended prefixes a second time. This matches native tree clicks and avoids empty Gallery queries for normally spelled indexed paths. Real isolated MCP root/nested selections now return4/2 matches. The folder/tag snapshot regression now expects .hidden as well as nested under the already-established all-file policy.

## Gallery folder-card navigation (2026-10-14)

Gallery consumes public NavigationService::request_ui SelectPath/AllFiles; no private UI imports. SelectPath now reveals and refreshes ancestors under registered roots before applying selection, so folder-card navigation updates the existing tree/breadcrumb/session path. A root selected by path is highlighted like the same root selected by ID. Existing canonical containment validation and indexed input-path spelling remain unchanged. Grid/list actual double-click and multi-level navigation were exercised through PhoTag MCP.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.

## Tag-choice invalidation (2026-10-15)

`NavigationService::tag_revision() -> u64` is a memory-only revision. Successful
tag create/rename/delete/move/appearance/display operations advance it; the
Cordis-owned MediaIndexChanged observer also advances it for Tagging merges
and metadata reconciliation. It does not query SQLite or emit extra events.
Settings uses this identity to refresh background choices while remaining
idle between changes. SQL ordering/schema and tag command results are unchanged.

## Shared refresh ownership (2026-10-15)

Sidebar and scan monitors use UI Bridge refresh::poll. Scan sampling compares scanned/total/phase/error before and after draining events; remaining active without progress does not notify. Completion retains existing refresh and MediaIndexChanged emission, then terminates the finite poller.


## Browser runtime update — 2026-10-15

Runtime folder records are **favorites**. `add_folder` saves a bookmark and selects it without indexing; `rescan` requests a fresh browse of that directory, preserving annotations; remove only removes the bookmark. The legacy scan receiver signature remains for source-compatible callers and completes immediately in browser mode. `rename_favorite(i64,&str)` changes an alias; `move_favorite(i64,isize)` changes order. Neither changes local names.

Arbitrary `SelectPath(String)` is accepted for an existing directory. `browse_subdirectories(&str)->io::Result<Vec<SubdirectoryNode>>` reads one level without child probes/count scans; Sidebar queues expanded paths through one ReadModel. Rows optimistically expose an expansion arrow until expanded. Favorites missing from disk remain in navigation, with browse errors. UI contributions and picker ownership retain Cordis cleanup. No favorite watcher or startup scan is installed.

## Explorer-style sidebar sections (2026-10-15)

The Folders page stacks three new sections above/below the existing favorite
tree: **Drives** (top), **Quick access** (top), and grouped **Favorites**
(bottom, above nothing; the existing folder tree stays in the middle).

- `src/drives.rs` `enumerate_drives(include_network) -> Vec<DriveInfo>` uses
  windows-sys `GetLogicalDrives`/`GetDriveTypeW`/`GetVolumeInformationW`/
  `GetDiskFreeSpaceExW` (Windows only; other targets return empty). Network
  drives show letter+usage only (no UNC). Reads run through a
  `ReadModel<(show_network, roots_revision), Vec<DriveInfo>>`; render only
  requests, the refresh::poll timer delivers results (`drives_read.poll()`).
- `src/sidebar_folders.rs` `SidebarFoldersService` owns
  `photag_contracts::SidebarFoldersConfig` (quick_access entries, named
  favorite_groups with folders, `show_network_drives`, default true) behind a
  RwLock plus an AtomicU64 revision. `update()` bumps the revision and returns
  the new document; `restore()` seeds silently-but-bumped at startup. It is
  not a Cordis service; Settings reaches it via
  `NavigationService::sidebar_folders()`.
- Persistence: after each mutation the view emits
  `SidebarFoldersChanged(SidebarFoldersUpdate { revision, config })`; the Settings plugin stores it
  in its JSON (`settings.sidebar_folders`) and Navigation restores it on
  Settings startup. No SQLite change. The sidebar poll compares
  `folders.revision()` against the view snapshot to re-render after external
  restore/persist cycles.
- Interactions follow the confirmed decisions: browse mode clicks
  select the path without expanding it (`activate_sidebar_path`); library mode click/menu adds the
  folder to the library (`add_folder_path`); per-row menus add/open in file
  manager/rename/remove; the inline editor row (`SidebarEditor`) edits paths,
  group names and display names, with native picker routing through
  `picker_editor`. Folder rows share the lazy `expanded_directories` tree and
  session expansion persistence.

## Sidebar semantic requests and debug snapshot (2026-10-16)

The Explorer-style sections are drivable without pointer hit-tests, so MCP
verification no longer depends on screenshot pixel calibration.

- `src/ui_requests.rs` adds `NavigationUiRequest::Sidebar(SidebarUiRequest)` plus
  `SidebarEditorRequest`. `SidebarUiRequest` derives `serde::Deserialize` with an
  internal `"kind"` tag (snake_case), so the MCP client sends exactly the JSON
  shape it logs, e.g. `{"kind":"remove_quick_access","path":"F:\library"}` or
  `{"kind":"open_editor","text":"新分组","editor":{"target":"create_group"}}`.
- `ui_requests::validate_sidebar_request(folders, request)` runs inside
  `NavigationService::request_ui` before queueing: section keys are limited to
  `drives`/`quick_access`/`favorites`/`folders`, drive roots must be present from
  the cheap `GetLogicalDrives` bitmask (no capacity probe), paths must be existing directories, and favorite
  group/folder ids must exist in the current `SidebarFoldersConfig`. Rejections
  return `NavigationUiRequestError::InvalidSidebarRequest(reason)`.
- The mounted view applies a queued request in
  `view/sections.rs::apply_sidebar_request`, which only calls the existing row
  and menu handlers (`toggle_section`, `set_show_network_drives`, `toggle_drive`,
  `activate_sidebar_path`, `toggle_directory`, `add_folder_path`,
  `add_quick_access_path`, `remove_quick_access`, `add_favorite_folder_path`,
  `remove_favorite_folder`, `remove_favorite_group`, `open_sidebar_editor`,
  `confirm_sidebar_editor`). It is a trigger, not a parallel implementation: the
  pointer path and the request path share the same code, including the
  `SidebarFoldersChanged` persistence emit.
- `view/sidebar_state.rs::publish_sidebar_state` runs in the existing
  `refresh::poll` timer and writes a JSON snapshot into
  `NavigationService::sidebar_state()` (`publish_sidebar_state`/`sidebar_state`
  accessors in `src/lib.rs`). Publishing is change-gated by a signature over
  page, browse mode, collapsed sections, expanded drives, config revision, drive
  read state, expanded-directory paths/revision, group collapse state, editor target+text+error and the session
  selected path, so an idle sidebar costs nothing.
- Snapshot fields: `page`, `browse_mode`, `*_collapsed`, `show_network_drives`,
  `config_revision`, `drives_pending`, `drives[]` (root/letter/label/kind/usage
  bytes/expanded/selected), `system_quick_access[]`, `quick_access[]`,
  `favorite_groups[]` (with nested folders), `library_folders[]`, and
  `editor` (`target` debug name, current `text`, inline `error`). Additional fields: `roots_revision`, `system_folders_pending`, `save_error`, drive `usage_error` and row `child_count`.
- Inspect failures: the `navigation request` tracing line in the profile
  `stderr.log` records every queued request; validation errors come back as the
  MCP tool error payload instead.

## Sidebar review corrections — 2026-10-16

`SidebarFoldersService::update` returns None for a no-op and preserves aliases on duplicate adds. Quick access and drive chevrons share `expanded_directories`; section key is `quick_access`. `SessionState.sidebar_collapsed_sections/sidebar_collapsed_groups` persist section/group state. Existing directories outside bookmarks now restore in browser mode; legacy mode retains its root membership rule.

New `SidebarUiRequest::{RefreshRoots, RetrySave, ToggleGroup { group_id }}` routes to the same UI handlers. Window activation/manual RefreshRoots invalidate drive and known-folder ReadModels independently of favorite edits. System known-folder lookup/probes run off render. Expanded active directory children refresh on local index events. Unknown browser media counts are hidden. Extended Windows paths compare lexically for selection without render-time filesystem IO.

Save events carry a monotonic revision. The view displays a failed latest save with Retry; obsolete failures cannot overwrite newer status. Inline path/name validation stays in the editor rather than replacing the library snapshot. Capacity failures show Unavailable, not a fictitious 0/0. Slow OS/network calls cannot be forcibly interrupted; offline network and removable-drive dialogs are not verified by local-drive checks.

Reproducible checks and defects: docs/validation-ui/2026-10-16-sidebar-review/README.md. Rendering is split across sections/render.rs and its quick_access/favorites/editor modules.


## Explorer address bar — 2026-10-16

Latency update: request_ui triggers a coalesced shared Wake after releasing the
queue lock; Sidebar owns the listener task. SelectPath/ExpandPath/AddressBar skip
unrelated favorite/tag snapshot reloads. Browser path validation no longer loads
favorite roots for the legacy membership check. Target existence is still checked
at action time. Tag history validates against a fresh service snapshot, not a
potentially old rendered tree. The 100ms passive sampler remains unchanged.

The existing navigation/breadcrumb-view contribution in command.breadcrumb is now the full address bar. view/address_bar.rs owns its typed actions/input/history; address_bar/render.rs renders Back/Forward/Up, root-to-leaf clickable breadcrumbs and an editable path. location.rs owns pure path presentation and bounded window-local NavigationHistory (200 locations). No new database or persistent history table. Current scope still uses existing session path/folder/tag fields; restore emits exactly one matching scope event so an empty directory event cannot clear restored tags/favorites.

Public NavigationUiRequest::AddressBar(AddressBarRequest) accepts serde kind back/forward/up/edit/cancel/submit/set_text {text}/crumb {path}. It is consumed by the mounted Sidebar, with the same handlers as pointer controls. Crumb is validated against current ancestors; typed and historical paths are checked at action time. Quoted absolute and relative paths are supported; relative paths use the current folder. Invalid/missing locations keep scope and history cursor, with visible error state. Current breadcrumb and unchanged submits do not reload. Drive and UNC share roots have no Up destination.

Session changes from Gallery tag filters also synchronize history without filesystem IO; selecting a folder clears tag scope. Home and tag views have context labels, not fictitious filesystem paths. Back/Forward restores directory/home/tag scope, truncates forward history on a new branch and deduplicates equivalent Windows paths. History resets at the restored location for a new window.

get_ui_state.sidebar_folders.address_bar is change-gated: path, editing, text, error, can_back/can_forward/can_up, breadcrumbs[{label,path}]. Input uses InputState and actual PressEnter; Escape cancels, Ctrl+L edits, Alt+Left/Right navigates when the bar is focused. Gallery forwards Ctrl+L and Alt+Left/Right from its own focus; existing Alt+Up is retained. Long breadcrumbs scroll horizontally; edit exposes the complete path. No render-time metadata/canonicalize probes are added. See docs/validation-ui/2026-10-16-address/README.md.

## Unified theme (2026-10-18)

View paints use UI Bridge semantic theme tokens instead of local color literals.
Gallery inspector/statistics cards and headings consume shared components; compact
navigation preserves its layout with common selection/hover paints. Status, branch
and media-overlay colors remain semantically distinct in the central theme.
No product service/data/event contract changes. See `docs/ui-theme.md`.
# Unified favorite group sidebar (2026-10-18)

Each favorite group is a sibling region, using the same UI Bridge
`components::sidebar_section/sidebar_header` as Drives/Quick Access. No outer
Folder favorites / Library folders / Favorites collection container remains.
The whole sidebar column scrolls within the host viewport. Group plus calls
`add_current_favorite`: resolves the committed Session directory (including nested
paths), checks that the group and directory still exist, adds a shortcut through
the existing revisioned Settings event, expands the target group, and never opens
a picker or starts indexing. Home/tag sets disable plus. Per-group path matching
uses normalized Windows identity, preserving aliases and revisions on duplicates.

`SidebarFoldersService::import_library_folders(&[(i64, String, String)], &str)`
returns `Option<SidebarFoldersConfig>` for an actual import. Existing library
roots are represented once in the default Favorites group (or an existing group
with that name); existing groups/aliases/order survive. The additive serde-default
`SidebarFoldersConfig.imported_library_paths: Vec<String>` stores normalized
paths, rather than SQLite IDs which can be reused. Removing a shortcut/group
does not remove database registrations or make it reappear on restart. Import
runs on the existing snapshot-change/startup poll, not per render; persistence
and retry still use `SidebarFoldersChanged` / SettingsService. There is no SQL
schema migration or parallel persistence facility.

Public `SidebarUiRequest::AddCurrentFavorite { group_id }` maps to the actual plus
handler. Existing `AddFavoriteFolder` remains the explicit-path variant. Legacy
`ToggleSection favorites/folders` now toggles all sibling groups; individual group
collapse uses `ToggleGroup`. Registered bookmark context menus retain rescan,
subfolder creation and explicit Remove from library; plain Remove from favorites
only removes the shortcut. Old library-root display reorder/rename UI is superseded
by group placement/alias editing; public backend compatibility methods remain.

`sidebar_state()`/MCP `get_ui_state.sidebar_folders` adds `favorite_current_path`,
`favorite_action_error`, `favorite_imported_paths`, and per-group `header_bounds`
([x,y,width,height] in logical GPUI client coordinates). Bounds come from the same
view's paint canvas, are change-gated for diagnostics and trigger no repaint/IO.
Inspect `save_error` for persistence failures. Reference: `docs/ui-theme.md` and
`docs/reference-ui/concepts/sidebar-favorite-groups-2026-10-18.md`.
