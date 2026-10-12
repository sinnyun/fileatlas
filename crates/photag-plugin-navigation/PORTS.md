# Navigation plugin ports / 导航插件数据对接

Status: the Cordis navigation service, folder tree actions and GPUI sidebar are present. The tag UI now exposes create, rename, delete, color/icon, display and explicit reparent actions. Original egui interactions are only partially migrated: no native folder picker, tag drag/drop zones, multi-select/exclude mode, modal confirmation/edit flow, recursive tag search/collapse state, merge workflow, or undo integration.

## Lifecycle and dependencies

- Cordis plugin name: `photag.navigation`; published service: `photag/navigation` (`NavigationService`).
- Requires `DatabaseService` and `IndexingService`; attaches `WatcherService` when available.
- GPUI view contribution: `navigation/sidebar-view` in `workspace.sidebar`.
- UI emits `FolderSelectionChanged(Option<i64>)`, `TagSelectionChanged(Option<i64>)`, `DirectoryPathSelected(Option<String>)`; emits `MediaIndexChanged(())` after operations that alter indexed membership.

## Service input/output

- `snapshot()` -> registered folders with media counts and the recursive PhoTag tag tree.
- `subdirectories(path)` -> lazy direct child nodes: name, path, child-presence and indexed count.
- `add_folder(path)` -> validates an existing directory; returns `(Folder, Receiver<Option<ScanProgress>>)` after registering it and scheduling indexing. Watcher registration follows when available.
- `rescan(folder_id)` -> cancels prior scan, clears folder media rows and starts scanning; returns a progress receiver.
- `remove_folder(folder_id)` -> cancels scanning, removes indexed rows and unregisters watcher; returns the removed folder.
- `create_tag(name, parent_id)` -> new `Tag`; parent is optional. The sidebar parent field takes a tag database ID; each tag row's **Use as parent** fills it.
- `rename_tag(id, name)` -> `()`; **Edit fields** loads name and **Rename** persists it.
- `delete_tag(id)` -> `()`; **Delete** invokes database cascade semantics. Unlike original PhoTag, no affected-media/ancestor-cleanup confirmation is shown yet.
- `set_tag_appearance(id, color, icon)` -> `()`; color accepts `#RRGGBB` or empty to clear; icon accepts text/emoji or empty to clear. **Edit fields** loads existing values; **Save style** applies them. Appearance inheritance UI is pending.
- `set_tag_display(id, display)` -> `()`; applies gallery chip visibility to the selected tag and all descendants, matching original edit behavior.
- `move_tag(id, TagDropPos)` -> IDs of media whose implicit tag set changed. The UI offers **Move into ID** using the parent ID field. Before/after sibling ordering and drag hit-testing are pending.

## GPUI controls and debugging

The tag panel uses GPUI Kit `Input` and `Button`. `TagSelectionChanged` filters one tag. Original multi-select, include/exclude modifier, select-all, clear controls, and tag query mode remain pending. A service/input error currently replaces the snapshot panel; use Refresh after correcting input or inspect the displayed error and database query. For failed reparent operations, verify that both IDs exist and the move does not create a cycle. For stale Gallery results, inspect `MediaIndexChanged` delivery and the Gallery subscriber.

## Original source mapping / 未完成

Reference: pinned `vendor/photag-source/src/ui/sidebar.rs` (`show_sidebar`, `show_tag_tree`, `show_tag_node`) and tag actions in `vendor/photag-source/src/app.rs`.

- Migrated: folder registration/rescan/removal, lazy directory listing, recursive tag tree, single-tag filter, root/child creation, rename, delete, color/icon editing, display propagation, reparent into a selected tag.
- Pending: native directory picker; original tabs and exact visuals; drag/drop before/after/into with feedback; tag search and ancestor expansion; include/exclude multi-tag interaction and modifier settings; merge; edit/delete confirmation and delete-and-clean-parents policy; apply appearance to descendants with original override rules; tag action undo/redo; recent-tag and XMP feedback after tag-tree mutations.
- Local integration: verify event refresh and destructive delete behavior against representative PhoTag databases; run Windows GUI/filesystem flows; then finish visual and interaction parity. Plugin tests remain deferred to the local integration pass.
