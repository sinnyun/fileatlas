# Navigation plugin ports / 导航数据对接

Status: folder/tag snapshots, lazy subdirectories, folder operations, and initial GPUI tree are present. Native folder picker, editing menus, drag/drop, and persistence parity are incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.navigation`; service: `photag/navigation` (`NavigationService`).
- Requires `DatabaseService` and `IndexingService`; optionally connects `WatcherService` when available.
- Emits `FolderSelectionChanged(Option<i64>)`, `TagSelectionChanged(Option<i64>)`, and `DirectoryPathSelected(Option<String>)` through Cordis.

## Input / output

- `snapshot()` returns folder nodes with recursive media counts and tag records. `list_subdirectories(path)` returns lazy child nodes including whether children exist and their indexed count.
- `add_folder(path)` validates a directory, stores it, registers watcher, starts scan, and returns `(Folder, progress receiver)`. `rescan(folder_id)` cancels active work, removes indexed rows and restarts scanning. `remove_folder(folder_id)` cancels, removes DB rows and unregisters watcher.
- Tag create/rename/delete/reparent/appearance/display operations preserve source query semantics.

## Consumers and debugging

The UI sends user intent to Navigation; Gallery listens to selection events. For empty counts inspect folder identity, recursive count query, active scan progress, and gallery selection filters. Do not treat a display-only path selection as a folder ID.
