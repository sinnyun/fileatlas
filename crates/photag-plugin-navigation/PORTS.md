# Navigation plugin ports / 导航插件数据对接

Status: the Cordis navigation service, folder tree actions and GPUI sidebar are present. The sidebar uses `rfd::FileDialog::pick_folder` for native directory selection and sends the selected path through the existing folder-registration, watcher and indexing flow. Folder and subdirectory menus expose new-subfolder and file-manager actions. The tag UI exposes create, rename, delete, color/icon, display, one-way associated-tag assignment, reparent, merge, and confirmed deletion with optional ancestor-assignment cleanup. Appearance can be applied recursively while skipping descendants that already have an explicit color or icon. Typed GPUI drag/drop moves a dragged tag before/after/into an explicit row target. PhoTag has no folder-tree drag/drop behavior.

## Lifecycle and dependencies

- Cordis plugin name: `photag.navigation`; published service: `photag/navigation` (`NavigationService`).
- Requires `DatabaseService`, `IndexingService`, and `SessionService`; attaches `WatcherService` when available.
- GPUI view contribution: `navigation/sidebar-view` in `workspace.sidebar`.
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
