# File Staging Basket plugin ports

Status: the persistent path basket uses a responsive grid of image thumbnails
and title-first cards. Users can add multiple arbitrary files through the OS
picker, select one or more cards, remove one or clear all, refresh missing-file
status, remove unavailable entries, and copy or move paths to a selected
directory. Dragging a card onto a FileManager subfolder group is intended to
move it immediately using the existing default file-drop behavior. Transfers delegate
to `FileOperationsService`; the staging plugin owns no file-transfer engine.

## Cordis and UI lifecycle

- Plugin: `fileatlas.file-staging`; service: `fileatlas/file-staging`
  (`FileStagingService`).
- Requires `FileOperationsService`, `SettingsService`, and `UiBridgeService`.
- Registers a single native floating panel (`file-staging/panel`), an open
  command, and a lifecycle-owned launcher rail widget. Cordis/UI Bridge removes
  registrations and panel definition when the plugin stops.
- FileManager card context menus emit `FileStagingAddRequested` through the
  shared contracts crate for the current selected file IDs. The staging plugin
  resolves the path list on a blocking worker and wakes an open basket view only
  after its in-memory list changes. FileManager remains independent of this
  feature crate.
- Staging cards use the framework drag system with shared
  `FileStagingDragPayload`. The staging panel also accepts
  `FileManagerFileDragPayload`, adds the dragged paths to its manifest on drop,
  and preserves source files. This shared payload keeps feature crates
  independent. FileManager's subfolder group frame accepts the staging payload;
  dropping calls `move_paths_to_folder`, reports per-path outcomes in
  the FileManager operation status, and emits `FileStagingPathsMoved` so only
  successfully moved paths are removed from the basket. No copy/move prompt is
  shown; explicit copy remains available from the panel.
- Registers `file-staging.list`, `file-staging.add`, `file-staging.remove`,
  `file-staging.clear`, `file-staging.refresh_status`,
  `file-staging.remove_missing`, `file-staging.copy`, and `file-staging.move` through
  the shared lifecycle-owned MCP registry when it is present. Add/remove/clear
  are ordinary gated mutations; copy/move require `confirm=true` at Full access.
- Shell starts the feature by default in normal/core mode and lists it in
  Plugin settings for runtime enable/disable. Development can isolate it with
  `FILEATLAS_DEV_PLUGINS=file-staging` (the debug client also enables dev-test).

## Data and behavior

- Stores a version 1 JSON document at
  `%LOCALAPPDATA%\FileAtlas\file-staging\staging.json`; an isolated profile
  uses `<profile>\file-staging\staging.json`. It contains canonical file paths,
  added timestamps, and cached last-known availability. It never copies file
  bytes into the basket.
- JSON suits the bounded, whole-document list. Writes use a same-directory
  temporary file and backup replacement; startup restores `.json.bak` when the
  primary file is absent. Unknown versions stop plugin startup with a clear
  error. No earlier format exists to migrate.
- Duplicate paths are ignored with Windows case-insensitive path identity.
  Directories are not staged. Missing entries remain until removed or pruned.
- Image files render from their source paths in a cropped thumbnail viewport;
  other file types and unavailable files use the shared theme file icon. No
  staging-specific thumbnail cache or file decoding service was added.
- Destination conflicts are reported per path and never replace destination
  files for the path-oriented copy API. Successful entries are removed from the
  basket; failed or skipped entries remain for retry. UI result feedback shows
  counts and up to three failed paths/reasons.
- Path operations in File Operations validate the source again at execution.
  Indexed moves use the existing database path update/removal and index event;
  unindexed files are moved on disk and registered-folder Watcher can discover
  them. Copies are discovered by Watcher when copied under watched roots.

## Public operations and diagnostics

- `snapshot()` returns a cloned basket.
- `add_paths(paths)` canonicalizes existing files and deduplicates them.
- `remove(path)`, `clear()`, `refresh_status()`, and `remove_missing()` persist
  each successful change before publishing its in-memory snapshot.
- File Operations adds `copy_paths_to_folder` and `move_paths_to_folder` and
  returns one `PathOperationOutcome` per input path. These use the existing
  File Operations Cordis service; no second operation plugin or storage layer
  was added.
- Inspect `fileatlas/file-staging` startup diagnostics, `staging.json`, and
  File Operations per-path outcomes when a path is missing, locked, or collides.

## Limits

- Adding paths uses the native multi-file picker, the FileManager card menu
  (which stages the current selection), or the MCP tool. Directory staging and
  a keyboard shortcut are not provided.
- The intended internal drop target is a FileManager subfolder group; other
  grouping modes ignore the basket payload. Dragging a selected card should
  move all available selected paths immediately; otherwise it should move only
  the dragged card. Same-process MCP showed the drag ghost and target highlight,
  but release did not invoke the move, so this interaction is not runtime
  verified. The explicit “移动到…” picker remains available.
- Basket changes persist immediately. There is no multi-stage undo, queue,
  pause/cancel for an executing batch, or restart recovery for an interrupted
  transfer. A transfer is completed/reported by the existing service call.
- File Operations' existing indexed move semantics are retained. Recycle Bin
  delete is not offered by the basket.
- Windows dialogs, cross-volume moves, path permission failures and collision
  races still need same-process MCP/Windows runtime validation.

## Reference

The current implementation follows
`docs/reference-ui/concepts/file-staging-grid-drag-2026-10-20.png`: responsive
grid cards prioritize image previews and titles; the intended immediate
subfolder drop uses the established FileManager flow. The generated concept is
a layout guide; native Theme and UI Bridge controls own resolved colors and
component states.
