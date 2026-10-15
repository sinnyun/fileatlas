# Tagging plugin ports / 标签数据对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: database-backed assignment/association/merge/reparent service and
source-compatible JPEG IPTC/non-JPEG XMP metadata writeback are available.
The gallery now provides a searchable hierarchical tag picker for assigning or
removing tags from the current selection, plus clear-all. Temporary-file tests
cover XMP sidecar escaping/removal, source-byte preservation, JPEG IPTC
insertion/removal, best-effort mtime restoration, visibility/clearing of a
background write failure, and same-path pending-job coalescing to the latest
tags. Windows interoperability with real photo editors remains unverified.

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
- Successful `assign`, `remove`, `clear`, `merge`, and `move_tag` calls schedule best-effort metadata rewrites for affected media. JPEG/JPEG files receive IPTC keywords embedded in APP13; other extensions receive an `.xmp` sidecar with Dublin Core subjects and Lightroom hierarchical subjects. Empty tag lists remove prior sidecars or IPTC keywords. A single worker serializes writes, merges pending jobs for the same path, and ensures an already-running older job is followed by the newest tags. Completed file-write failures are retained in `writeback::failures()` (newest 100 distinct paths) for user-visible reporting; `writeback::retry_failure(path)` requeues the exact failed tag payload through the shared worker, while `writeback::clear_failure(path)` dismisses it.
- `xmp::write_metadata(path, tag_paths)` is also public for explicit integration. The original access/modified times are restored best-effort after writing.

## Consumers and debugging

Use stable integer media/tag IDs from the database service. The Gallery and Preview pickers search tag name and full path and apply source assignment/removal operations; Gallery/Navigation expose association editing and merge/delete flows. After mutation, consumers reload their views and record source-supported operations in HistoryService. Verify ancestor and association flags when debugging unexpected tags. Metadata writes remain detached and best-effort, matching the source; file-write failures are available through `writeback::failures()` for UI display. A database lookup or worker-start failure is logged by `schedule_xmp_sync`; inspect that diagnostic if no per-file write result appears. Compare `src/xmp.rs` byte behavior with `vendor/photag-source/src/xmp.rs` if local Windows testing exposes IPTC/XMP differences. F1–F9 hotkeys call the service through Settings mappings. Popup placement and visual behavior need Windows validation.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.


## Browser runtime update — 2026-10-15

Browser assignment captures current local size/time and image thumbnail before persisting the association. Identification images are copied to **tag-previews/** beside photag-tags.db and referenced by tag_previews, independent of rebuildable browse thumbnails. Existing hierarchy, association and XMP/IPTC writeback/failure queue semantics remain.

`missing_records()->Result<Vec<MediaItem>,String>` lists tagged paths for which metadata returns NotFound, substituting saved identification previews. Permission failures are not missing. `relink_record(i64,&str)` requires a currently missing old path and an explicit regular target; it rejects a target with existing tags or rating, preserves original tags, refreshes path/size/time/thumbnail and schedules writeback. `delete_missing_records(&[i64])->Result<usize,String>` checks again, deletes records only, never a local source file. Errors propagate to Gallery operation feedback/MCP. Camera/editor interoperability remains outside this change.
