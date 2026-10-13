# File operations plugin ports / 文件操作对接

Status: source-compatible single-item and bulk copy/move/delete, rename,
create-folder, Recycle Bin delete, open, reveal, and database follow-through
are available. Gallery owns destination dialogs, Delete confirmation, operation
feedback, and preview/selection repair after mutations. This migration slice
has not been compiled or tested in this cloud pass.

## Lifecycle and dependencies

- Cordis plugin: `photag.file-operations`; service: `photag/file-operations`
  (`FileOperationsService`).
- Code map: service and filesystem flow in `src/lib.rs`; public per-operation
  result/error records in `src/model.rs`; service publication and plugin
  lifecycle errors in `src/plugin.rs`.
- Requires `photag/database` and starts after the database plugin.
- Database-row mutations emit `MediaIndexChanged`; the gallery consumes that
  event and reloads. Copy and folder creation do not mutate indexed rows; the
  existing watcher handles files created under watched roots.

## Inputs and outputs

- `copy_to(media_id, destination)` copies to the caller-selected full path and
  leaves the source row untouched.
- `bulk_copy_to_folder(media_ids, destination_dir)` copies each indexed file
  with its current filename, continues after per-item errors, and returns one
  `BulkCopyOutcome` per input ID. It does not alter rows; the folder watcher
  discovers successful copies under registered roots.
- `move_to(media_id, destination)` uses rename then copy/delete fallback for
  cross-volume moves. It updates the row when the destination string begins
  with a registered root, otherwise removes the row, matching PhoTag's logic.
  `move_to_with_outcome` additionally returns whether the move occurred;
  `false` represents a source-compatible collision or filesystem move failure.
- `move_into_folder(media_id, destination_dir)` retains the filename, refuses
  to overwrite an existing file, and updates the indexed path/root.
  `move_into_folder_with_outcome` reports whether it moved the file.
- `bulk_move_into_folder(media_ids, destination_dir)` applies the same behavior
  per item, continues after an individual failure, and returns one
  `BulkMoveOutcome` per input ID. A collision or missing item is `moved: false`;
  database errors are attached to that outcome.
- `rename(media_id, new_filename)` trims the name, refuses empty/unchanged or
  colliding names, renames on disk, then updates the existing DB row.
  `rename_with_outcome` returns `false` for an unchanged, empty, or colliding
  name so the UI can distinguish a no-op from a completed rename.
- `create_folder(parent, name)` preserves PhoTag's invalid-character rules and
  creates exactly one directory level.
- `delete_to_recycle_bin(media_id)` uses the `trash` crate. It removes the
  thumbnail best-effort and deletes the DB row even if recycle-bin submission
  fails. `DeleteResult` reports whether recycling succeeded and its error.
- `bulk_delete_to_recycle_bin(media_ids)` resolves rows first, best-effort
  trashes files and thumbnails, removes each row even when trash submission
  fails, and returns one `BulkDeleteOutcome` per input. Database/query failures
  are per-item; successful batches emit one index event.
- `open_external(media_id)` opens with the OS default application;
  `reveal_in_file_manager(media_id)` selects/reveals the path in Explorer (or
  the platform file manager on non-Windows builds).
- Filesystem/database work runs on Tokio's blocking pool. Methods return typed
  I/O, database, worker, and event errors. A post-mutation event error means
  the operation may already have completed; refresh the gallery before retrying.

## Debugging and remaining integration

Compare move fallback, path-prefix root matching, collision handling, thumbnail
cleanup, and delete-on-recycle-failure against `vendor/photag-source/src/app.rs`
(`do_copy_media`, `do_move_media`, `do_move_media_to`, `do_rename_media`,
`do_create_folder`, and `do_delete_media`). Database changes use the original
query helpers in `photag-plugin-database`. Do not call these synchronous file
operations from GPUI render; the service already offloads filesystem/database
work. The Gallery view requests native save/folder dialogs and calls these
methods. Review `photag-plugin-gallery/PORTS.md` and
`src/view/file_actions.rs` for the UI flow, confirmation, feedback, and
post-operation preview/selection handling. This service leaves dialog and
presentation policy to its caller.
