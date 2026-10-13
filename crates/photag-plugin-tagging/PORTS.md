# Tagging plugin ports / 标签数据对接

Status: database-backed assignment/association/merge/reparent service and
source-compatible JPEG IPTC/non-JPEG XMP metadata writeback are available.
The gallery now provides a searchable hierarchical tag picker for assigning or
removing tags from the current selection, plus clear-all. This slice has not
been checked by crate-specific tests; the shell dependency graph compiled in the
migration pass. Windows writeback behavior remains unverified.

## Lifecycle and dependencies

- Cordis plugin name: `photag.tagging`; service: `photag/tagging` (`TaggingService`).
- Requires `DatabaseService`; contributes a tagging widget at `gallery.selection-actions`.

## Input / output

- `assign(media_ids, tag_ids)` applies requested tags, one-way associations, and ancestor tags while preserving `auto_applied` distinction.
- Successful user-facing assignments update `recent_tags()` in source order (newest first, at most eight IDs); `restore_assignment` is used by History replay and does not alter recency.
- `apply_ai_suggestions(media_id, labels)` resolves existing names case-insensitively, creates missing labels under `AI Suggested`, then delegates assignment and metadata synchronization through the normal service path. It returns the direct label IDs only after the database assignment succeeds so Preview can record source-compatible History; failed assignments return an error and do not generate history entries.
- `remove(media_ids, tag_ids)`, `clear(media_ids)`, `for_media(media_id)`, and `associated_with(tag_id)` expose the corresponding source queries.
- `set_associations(tag_id, associated_ids)` updates association edges; `merge(source_id, target_id, reparent_children)` retains source merge behavior. Operations return `Result<_, TaggingError>`; database failures propagate.
- Navigation's source “Merge into…” flow now uses this API after the user selects a filtered target outside the source subtree. It lets the user choose whether source children are re-parented under the target (default) or deleted. Successful merges refresh the Navigation and Gallery views and schedule writeback for affected media.
- `move_tag(source_id, drop_position)` returns the media IDs whose implicit ancestor assignments changed and schedules metadata synchronization.
- `tag_tree()` returns the current recursive database tag tree for UI pickers.
- Successful `assign`, `remove`, `clear`, `merge`, and `move_tag` calls schedule best-effort metadata rewrites for affected media. JPEG/JPEG files receive IPTC keywords embedded in APP13; other extensions receive an `.xmp` sidecar with Dublin Core subjects and Lightroom hierarchical subjects. Empty tag lists remove prior sidecars or IPTC keywords. A single worker serializes writes, merges pending jobs for the same path, and ensures an already-running older job is followed by the newest tags. Completed file-write failures are retained in `writeback::failures()` (newest 100 distinct paths) for user-visible reporting; a new write attempt clears the prior failure for that path, and `writeback::clear_failure(path)` acknowledges it.
- `xmp::write_metadata(path, tag_paths)` is also public for explicit integration. The original access/modified times are restored best-effort after writing.

## Consumers and debugging

Use stable integer media/tag IDs from the database service. The Gallery and Preview pickers search tag name and full path and apply source assignment/removal operations; Gallery/Navigation expose association editing and merge/delete flows. After mutation, consumers reload their views and record source-supported operations in HistoryService. Verify ancestor and association flags when debugging unexpected tags. Metadata writes remain detached and best-effort, matching the source; file-write failures are available through `writeback::failures()` for UI display. A database lookup or worker-start failure is logged by `schedule_xmp_sync`; inspect that diagnostic if no per-file write result appears. Compare `src/xmp.rs` byte behavior with `vendor/photag-source/src/xmp.rs` if local Windows testing exposes IPTC/XMP differences. F1–F9 hotkeys call the service through Settings mappings. Popup placement and visual behavior need Windows validation.
