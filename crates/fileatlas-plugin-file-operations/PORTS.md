# File operations plugin ports / 文件操作对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: source-compatible single-item and bulk copy/move/delete, rename,
create-folder, Recycle Bin delete, open, reveal, and database follow-through
are available. FileManager owns destination dialogs, Delete confirmation, operation
feedback, and preview/selection repair after mutations. The shell dependency
graph compiled in the migration pass; file-system behavior still requires
Windows runtime validation. Tests remain deferred to the concentrated debugging
pass.

## Lifecycle and dependencies

- Cordis plugin: `fileatlas.file-operations`; service: `fileatlas/file-operations`
  (`FileOperationsService`).
- Code map: service and filesystem flow in `src/lib.rs`; public per-operation
  result/error records in `src/model.rs`; service publication and plugin
  lifecycle errors in `src/plugin.rs`.
- Requires `fileatlas/database` and starts after the database plugin.
- Database-row mutations emit `MediaIndexChanged`; the file_manager consumes that
  event and reloads. Copy and folder creation do not mutate indexed rows; the
  existing watcher handles files created under watched roots.

## Inputs and outputs

- `copy_to(media_id, destination)` copies to the caller-selected full path and
  leaves the source row untouched.
- `bulk_copy_to_folder(media_ids, destination_dir)` copies each indexed file
  with its current filename, continues after per-item errors, and returns one
  `BulkCopyOutcome` per input ID. It does not alter rows; the folder watcher
  discovers successful copies under registered roots.
- `copy_paths_to_folder(paths, destination_dir)` and
  `move_paths_to_folder(paths, destination_dir)` accept arbitrary staged file
  paths as well as indexed paths, revalidate each source at execution, and
  return one `PathOperationOutcome` (`source`, `completed`, `error`) per input.
  Existing destinations are skipped/reported and the path-copy helper uses
  `create_new` so a concurrent collision cannot overwrite destination bytes;
  cross-volume move fallback uses the same no-overwrite copy path before source
  removal. Windows verbatim/ordinary paths are matched before deciding whether
  a staged source is already indexed.
  Indexed moves retain the existing database/root reconciliation and emit one
  `MediaIndexChanged`; unindexed moves use copy-then-remove and registered-root
  Watcher discovers resulting content. Staging basket calls these methods.
- `move_to(media_id, destination)` uses rename then copy/delete fallback for
  cross-volume moves. It updates the row when the destination string begins
  with a registered root, otherwise removes the row, matching FileAtlas's logic.
  `move_to_with_outcome` additionally returns whether the move occurred;
  `false` represents a source-compatible collision or filesystem move failure.
- `move_into_folder(media_id, destination_dir)` retains the filename, refuses
  to overwrite an existing file, and updates the indexed path/root when the
  destination is under a registered library root. Moving outside all roots
  leaves the file on disk and removes its media row, matching FileAtlas.
  `move_into_folder_with_outcome` reports whether it moved the file.
- `bulk_move_into_folder(media_ids, destination_dir)` applies the same behavior
  per item, continues after an individual failure, and returns one
  `BulkMoveOutcome` per input ID. A collision or missing item is `moved: false`;
  database errors are attached to that outcome.
- `rename(media_id, new_filename)` trims the name, refuses empty/unchanged or
  colliding names, renames on disk, then updates the existing DB row.
  `rename_with_outcome` returns `false` for an unchanged, empty, or colliding
  name so the UI can distinguish a no-op from a completed rename.
- `create_folder(parent, name)` preserves FileAtlas's invalid-character rules and
  creates exactly one directory level.
- `delete_to_recycle_bin(media_id)` uses the `trash` crate. It removes the
  thumbnail best-effort and deletes the DB row even if recycle-bin submission
  fails. `DeleteResult` reports whether recycling succeeded and its error.
- `media_by_id(media_id)` reads the indexed row for post-operation repair. Use
  it after an operation error when the error may be from event delivery after
  the filesystem/database mutation already committed.
- `bulk_delete_to_recycle_bin(media_ids)` resolves rows first, best-effort
  trashes files and thumbnails, removes each row even when trash submission
  fails, and returns one `BulkDeleteOutcome` per input. Database/query failures
  are per-item; successful batches emit one index event.
- `open_external(media_id)` opens with the OS default application;
  `reveal_in_file_manager(media_id)` selects/reveals the path in Explorer (or
  the platform file manager on non-Windows builds).
- Filesystem/database work runs on Tokio's blocking pool. Methods return typed
  I/O, database, worker, and event errors. A post-mutation event error means
  the operation may already have completed; refresh the file_manager before retrying.

## Debugging and remaining integration

Compare move fallback, path-prefix root matching, collision handling, thumbnail
cleanup, and delete-on-recycle-failure against `vendor/photag-source/src/app.rs`
(`do_copy_media`, `do_move_media`, `do_move_media_to`, `do_rename_media`,
`do_create_folder`, and `do_delete_media`). Database changes use the original
query helpers in `fileatlas-plugin-database`. Do not call these synchronous file
operations from GPUI render; the service already offloads filesystem/database
work. The FileManager view requests native save/folder dialogs and calls these
methods. Review `fileatlas-plugin-file-manager/PORTS.md` and
`src/view/file_actions.rs` for the UI flow, confirmation, feedback, and
post-operation preview/selection handling. This service leaves dialog and
presentation policy to its caller.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by fileatlas.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See fileatlas-plugin-debug/PORTS.md for capture scope and limits.


## Browser runtime update — 2026-10-15

Runtime file actions validate local metadata at action time. Known program-initiated move/rename updates the annotation path even outside favorites. Browser rows use owner 0 rather than bookmark ownership. Successful recycling removes live membership and retains annotated records for missing/relink maintenance; recycle failure does not erase records. External moves are not automatically matched. Legacy tests retain source semantics via open_at. Real recycle-bin/native-dialog paths remain explicitly unverified in browser mode.

## MCP feature contributions

`FileOperationsService` contributes `file-operations.rename_media`,
`file-operations.move_media`, `file-operations.copy_media`, and
`file-operations.delete_media` through `McpToolRegistry`. Definitions and
schemas live in this crate's `src/mcp.rs`; all four operations require explicit
confirmation, and batch operations validate 1–100 positive media IDs. Results
preserve per-item success/error outcomes. Former unnamespaced MCP names remain
as compatibility aliases during the migration.

## Feature-owned source document links

`open_linked_file(&self, path: &Path) -> Result<(), FileOperationError>` canonicalizes
and validates an existing file, then uses the same `open::that` platform handoff
as indexed `open_external`. It does not create media rows or modify the file.
WorkCrate calls it on a worker for the separately linked PSD/source document.
Missing files and Windows association errors remain visible to the feature UI.
Actual Photoshop editing/export is owned by the external editor and user.
