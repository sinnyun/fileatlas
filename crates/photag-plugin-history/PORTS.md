# History plugin ports / 历史操作对接

Status: Cordis service and executable undo/redo handlers are implemented for the five undoable operation types present in PhoTag's source. Gallery records rating, tag-hotkey, and confirmed single-item rename mutations and invokes undo/redo on Ctrl+Z/Ctrl+Y; clear-tags and file-operation history producers remain unwired.

## Lifecycle and dependencies

- Cordis plugin: `photag.history`; service: `photag/history` (`HistoryService`).
- Requires `DatabaseService`, `TaggingService`, and `FileOperationsService` at startup. Cordis owns service publication lifetime.
- The history stacks are in-memory and scoped to this service instance; they are not persisted across launches.
- Source: `vendor/photag-source/src/app.rs` (`UndoEntry`, `push_undo`, `execute_undo`, `execute_redo`, pending action handlers) and `vendor/photag-source/src/main.rs` keyboard dispatch.

## Public operations

- `push(UndoEntry)` clears redo and caps undo at 100 entries.
- `cleared_tags_entry(media_ids)` snapshots each media row's assigned tag IDs before clear.
- `undo().await` and `redo().await` return source-compatible toast text, or `None` for an empty stack.
- `undo_description()` / `redo_description()` return the current top action description.
- `UndoEntry` matches the source data model: `AssignedTags`, `RemovedTags`, `ClearedTags`, `RatingChanged`, `Renamed`.
- Tag changes use `TaggingService` (including source XMP metadata synchronization); ratings use the original database bulk query; rename uses `FileOperationsService`, which emits `MediaIndexChanged`.

## Semantics and limits

- As in PhoTag, operation failures are best-effort and do not stop the entry from moving to the opposite stack. A new push always clears redo.
- Stack transfer performed by undo/redo does not trim the destination stack, matching the source handlers.
- File operation success/failure is currently not exposed to the caller as a result; Gallery shows the returned history status and refreshes its query after undo/redo.
- Pending UI actions outside the five source `UndoEntry` variants are not undoable here. Folder/tag tree edits, delete/copy/move/bulk operations remain outside history.

## Debugging

- Check service startup ordering: database, tagging and file operations must be published before history.
- Inspect `undo_description` / `redo_description` when keyboard actions appear inert; `None` means the respective stack is empty.
- A rename that cannot be performed still transfers its entry, as the original caller did. Check the media row, target filename collision, filesystem permissions, and `MediaIndexChanged` delivery.
