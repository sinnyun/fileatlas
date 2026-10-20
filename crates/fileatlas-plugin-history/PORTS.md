# History plugin ports / 历史操作对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: Cordis service and executable undo/redo handlers are implemented for the five undoable operation types present in FileAtlas's source. FileManager records rating, tag-hotkey, tag-picker assignment/removal/clear, and confirmed single-item rename mutations. FileManager handles Ctrl+Z/Ctrl+Y, and the history plugin exposes lifecycle-owned Edit menu command contributions for undo and redo.

## Lifecycle and dependencies

- Cordis plugin: `fileatlas.history`; service: `fileatlas/history` (`HistoryService`).
- Requires `DatabaseService`, `TaggingService`, and `FileOperationsService` at startup. Cordis owns service publication lifetime.
- The history stacks are in-memory and scoped to this service instance; they are not persisted across launches.
- Source: `vendor/photag-source/src/app.rs` (`UndoEntry`, `push_undo`, `execute_undo`, `execute_redo`, pending action handlers) and `vendor/photag-source/src/main.rs` keyboard dispatch.

## Public operations

- `push(UndoEntry)` clears redo and caps undo at 100 entries.
- `cleared_tags_entry(media_ids)` snapshots each media row's assigned tag IDs before clear.
- `undo().await` and `redo().await` return source-compatible toast text, or `None` for an empty stack.
- `undo_description()` / `redo_description()` return the current top action description.
- `history/undo` and `history/redo` are registered as commands with Edit menu entries. The `UiCommandInvoked` listener runs the matching stack action and emits `MediaIndexChanged` after a non-empty action so FileManager refreshes.
- `UndoEntry` matches the source data model: `AssignedTags`, `RemovedTags`, `ClearedTags`, `RatingChanged`, `Renamed`.
- Tag changes use `TaggingService` (including source XMP metadata synchronization); ratings use the original database bulk query; rename uses `FileOperationsService`, which emits `MediaIndexChanged`.
- Undo/redo restores tag assignments through `TaggingService::restore_assignment`, so history replay does not change the source-compatible recent-tag list.

## Source action map and semantics

- The pinned source's complete `UndoEntry` enum has exactly five variants:
  `AssignedTags`, `RemovedTags`, `ClearedTags`, `RatingChanged`, and `Renamed`.
  Its producers are pending-action handling in `vendor/photag-source/src/app.rs`
  (tag assignment/removal/clear, rating, rename, and AI suggestion acceptance).
- FileManager produces tag-hotkey/picker assignment and removal, clear, rating,
  and confirmed rename entries. Preview's AI suggestion Apply actions also
  record `AssignedTags` after a successful assignment. File copy/move/delete
  and folder/tag-tree edits are not undoable in the source model and must not
  be represented as history entries.

- As in FileAtlas, operation failures are best-effort and do not stop the entry from moving to the opposite stack. A new push always clears redo.
- Stack transfer performed by undo/redo does not trim the destination stack, matching the source handlers.
- File operation success/failure is currently not exposed to the caller as a result; FileManager shows the returned history status and refreshes its query after undo/redo.
- Folder/tag tree edits, delete/copy/move/bulk operations remain outside history,
  matching the source `UndoEntry` model.

## Debugging

- Check service startup ordering: database, tagging and file operations must be published before history; Preview must stop before the History, AI, Settings, Tagging, and MediaPlayback providers it consumes.
- Inspect `undo_description` / `redo_description` when keyboard actions appear inert; `None` means the respective stack is empty.
- Inspect the `history/undo` and `history/redo` command IDs and `UiCommandInvoked` delivery when Edit commands appear inert. Command actions refresh subscribers through `MediaIndexChanged`.
- A rename that cannot be performed still transfers its entry, as the original caller did. Check the media row, target filename collision, filesystem permissions, and `MediaIndexChanged` delivery.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by fileatlas.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See fileatlas-plugin-debug/PORTS.md for capture scope and limits.


## Browser runtime update — 2026-10-15

`push` pins session cache handles used by the at-most-100 undo/redo operations through `DatabaseService::pin_history`, so directory eviction cannot discard an untagged file needed for undo. New operations refresh the union of pins; history remains session-only and does not create persistent browse inventory.
