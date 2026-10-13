# Navigation plugin ports / 导航插件数据对接

Status: the Cordis navigation service, folder tree actions and GPUI sidebar are present. The sidebar uses `rfd::FileDialog::pick_folder` for native directory selection and sends the selected path through the existing folder-registration, watcher and indexing flow. Folder/Tags pivot tabs restore and save `SessionState.sidebar_tab`; leaving Tags clears the query only when the settings preference `tag_search_persist` is false. Folder and subdirectory menus expose new-subfolder and file-manager actions. The tag UI exposes create, rename, delete, color/icon, display and reparent actions, with common tag operations also available from a per-tag dropdown menu. Appearance can be applied recursively while skipping descendants that already have an explicit color or icon. Typed GPUI drag/drop moves a dragged tag before/after/into an explicit row target. PhoTag has no folder-tree drag/drop behavior.

## Lifecycle and dependencies

- Cordis plugin name: `photag.navigation`; published service: `photag/navigation` (`NavigationService`).
- Requires `DatabaseService`, `IndexingService`, and `SessionService`; attaches `WatcherService` when available.
- GPUI view contribution: `navigation/sidebar-view` in `workspace.sidebar`.
- UI emits `FolderSelectionChanged(Option<i64>)`, `TagFilterSelectionChanged(TagFilterSelection)`, and `DirectoryPathSelected(Option<String>)`; `TagSelectionChanged(Option<i64>)` remains as a legacy single-tag contract. It emits `MediaIndexChanged(())` after operations that alter indexed membership.
- On sidebar startup, restores `sidebar_tab`, `selected_folder_id`, `selected_path`, `expanded_subdirs`, and `expanded_tag_ids` from `SessionService`. Folder IDs must still exist; selected paths must still resolve inside a registered folder; expanded paths are rebuilt through the live subdirectory listing. Tag expansion IDs missing from the current tag tree are pruned on startup. Startup emits folder and path selection events in order so Gallery receives the restored scope. Selecting a folder or All Media saves `selected_folder_id` and clears `selected_path`; selecting a path clears `selected_folder_id` and saves `selected_path`. Tab, directory-expansion and tag-expansion updates preserve fields owned by other consumers.

## Service input/output

- `snapshot()` -> registered folders with media counts and the recursive PhoTag tag tree.
- `subdirectories(path)` -> lazy direct child nodes: name, path, child-presence and indexed count.
- `add_folder(path)` -> validates an existing directory; returns `(Folder, Receiver<Option<ScanProgress>>)` after registering it and scheduling indexing. Watcher registration follows when available.
- `rescan(folder_id)` -> cancels prior scan, clears folder media rows and starts scanning; returns a progress receiver.
- `cancel_scan(folder_id)` -> requests cancellation of that folder's active indexing work; succeeds when none is active.
- `remove_folder(folder_id)` -> cancels scanning, removes indexed rows and unregisters watcher; returns the removed folder.
- `create_tag(name, parent_id)` -> new `Tag`; parent is optional. The sidebar parent field takes a tag database ID; each tag row's **Use as parent** fills it.
- `rename_tag(id, name)` -> `()`; **Edit fields** loads name and **Rename** persists it.
- `delete_tag(id)` -> `()`; **Delete** invokes database cascade semantics. Unlike original PhoTag, no affected-media/ancestor-cleanup confirmation is shown yet.
- `set_tag_appearance(id, color, icon)` -> `()`; color accepts `#RRGGBB` or empty to clear; icon accepts text/emoji or empty to clear. **Edit fields** loads existing values; **Save style** applies them to the selected tag. **Apply to children** propagates both style fields depth-first; **Skip edited children** (enabled by default) preserves descendants with an explicit color or icon, matching the source override rule.
- `set_tag_display(id, display)` -> `()`; applies gallery chip visibility to the selected tag and all descendants, matching original edit behavior.
- `move_tag(id, TagDropPos)` -> IDs of media whose implicit tag set changed. The tag name is the drag source; highlighted targets above, on, and below another tag invoke `Before`, `Into`, and `After` respectively. The existing **Move into ID** command remains available. Database cycle checks and subtree path recomputation are authoritative; self-drops and cycle-forming moves return an empty list as safe no-ops. Only emit `MediaIndexChanged` when the returned affected-media list is nonempty.

## GPUI controls and debugging

The sidebar uses Folder/Tags tabs. Tag search follows source `tag_search_persist`: its in-memory query survives tab switches by default and is cleared when leaving Tags if the preference is disabled. The tag panel uses GPUI Kit `Input`, `Button`, dropdown actions, and typed drag payloads. It supports included and excluded tag sets, Select/Exclude click modes, the configured modifier to apply the opposite mode for one click, selecting or clearing all tags in the active mode, per-mode clear actions, and source-compatible Contains/Starts with search matching that reveals matching descendants and their ancestors. `TagInteractionPreferencesChanged` supplies the current settings without adding a Settings → Navigation crate dependency; defaults are Ctrl, Contains, and persistent search. Drag a tag name onto the highlighted target above/on/below another row; database errors appear in the snapshot panel. Self and cycle drops are silently rejected by the source-compatible database operation. For stale Gallery results after inherited assignments change, inspect `MediaIndexChanged` delivery and the Gallery subscriber.

## Original source mapping / 未完成

Reference: pinned `vendor/photag-source/src/ui/sidebar.rs` (`show_sidebar`, `show_tag_tree`, `show_tag_node`) and tag actions in `vendor/photag-source/src/app.rs`.

- Migrated: folder registration/rescan/removal, lazy directory listing, recursive tag tree, single-tag filter, root/child creation, rename, delete, color/icon editing, display propagation, reparent into a selected tag, Folder/Tags tabs, tag-search clearing preference, physical subfolder creation, open-in-file-manager action, and recursive tag appearance propagation with explicit-child override preservation.
- Migrated: native directory picker via the same `rfd` API used by PhoTag's `PendingAction::AddFolder`. Cancel/backend `None` leaves the sidebar unchanged; folder validation and registration errors surface in the navigation panel as before. Tag drag/drop reparenting and sibling ordering use the source `TagDropPos` database commands. PhoTag does not implement folder drag/drop.
- Pending: exact Fluent tab/tree visuals; merge; edit/delete confirmation and delete-and-clean-parents policy; tag action undo/redo; recent-tag and XMP feedback after tag-tree mutations. Expanded tag nodes are persisted, but the current GPUI renderer only allows collapsing/expanding already-loaded tree nodes.
- Local integration: verify event refresh and destructive delete behavior against representative PhoTag databases; run Windows GUI/filesystem flows; then finish visual and interaction parity. Plugin tests remain deferred to the local integration pass.

## Shared visual tokens

The sidebar uses photag-ui-bridge::theme for the source base surface and secondary, muted, and danger text colors. Layout spacing and typography still use GPUI Kit presets; Windows visual parity is unverified.

