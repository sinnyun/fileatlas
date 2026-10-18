# Tagging plugin ports / 标签数据对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: database-backed assignment/association/merge/reparent service is
available. Normal tag actions are database-only; automatic IPTC/XMP file
writes are disabled by the 2026-10-17 user decision. The explicit metadata
writer API remains available to integrations that request file writeback. The
Gallery Inspector provides an inline searchable, multi-select add control.

## Lifecycle and dependencies

- Cordis plugin name: `photag.tagging`; service: `photag/tagging` (`TaggingService`).
- Requires `DatabaseService`; contributes a tagging widget at `gallery.selection-actions`.

## Input / output

- `assign(media_ids, tag_ids)` applies requested tags, one-way associations, and ancestor tags while preserving `auto_applied` distinction. Normal assignment does not modify media files or create XMP sidecars.
- Successful user-facing assignments update `recent_tags()` in source order (newest first, at most eight IDs); `restore_assignment` is used by History replay and does not alter recency.
- `apply_ai_suggestions(media_id, labels)` resolves existing names case-insensitively, creates missing labels under `AI Suggested`, then delegates assignment through the normal service path. It returns the direct label IDs only after the database assignment succeeds so Preview can record source-compatible History; failed assignments return an error and do not generate history entries.
- `remove(media_ids, tag_ids)`, `clear(media_ids)`, `for_media(media_id)`, and `associated_with(tag_id)` expose the corresponding source queries.
- `set_associations(tag_id, associated_ids)` updates association edges; `merge(source_id, target_id, reparent_children)` retains source merge behavior. Operations return `Result<_, TaggingError>`; database failures propagate.
- Navigation's source “Merge into…” flow now uses this API after the user selects a filtered target outside the source subtree. It lets the user choose whether source children are re-parented under the target (default) or deleted. Successful merges refresh the Navigation and Gallery views.
- `move_tag(source_id, drop_position)` returns the media IDs whose implicit ancestor assignments changed.
- `tag_tree()` returns the current recursive database tag tree for UI pickers.
- `writeback_failures()`, `retry_writeback_failure(path)`, and
  `dismiss_writeback_failure(path)` expose retained metadata-write results
  through `TaggingService`; consumers must not reach into the private worker
  module.
- `schedule_xmp_sync(media_ids)` is an explicit integration entry point; normal user actions do not call it. If explicitly requested, JPEG/JPEG files receive IPTC keywords embedded in APP13 and other extensions receive an `.xmp` sidecar. Failure snapshots and retry/dismiss methods remain scoped to those explicit requests.
- `xmp::write_metadata(path, tag_paths)` is also public for explicit integration. The original access/modified times are restored best-effort after writing.

## Consumers and debugging

Use stable integer media/tag IDs from the database service. The Gallery Inspector's Tags section searches the tag hierarchy and adds multiple selected tags inline; Gallery bulk actions and Preview retain their broader selection pickers. Gallery/Navigation expose association editing and merge/delete flows. After mutation, consumers reload their views and record operations in HistoryService. Verify ancestor and association flags when debugging unexpected tags. Source reference: `vendor/photag-source/src/xmp.rs` automatically writes JPEG IPTC and non-JPEG sidecars; PhoTag now intentionally keeps normal tag management in SQLite and only writes file metadata when an integration explicitly requests it.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.


## Browser runtime update — 2026-10-15

Browser assignment captures current local size/time and image thumbnail before persisting the association. Identification images are copied to **tag-previews/** beside photag-tags.db and referenced by tag_previews, independent of rebuildable browse thumbnails. Existing hierarchy and association semantics remain; automatic XMP/IPTC writeback is disabled as described above.

`missing_records()->Result<Vec<MediaItem>,String>` lists tagged paths for which metadata returns NotFound, substituting saved identification previews. Permission failures are not missing. `relink_record(i64,&str)` requires a currently missing old path and an explicit regular target; it rejects a target with existing tags or rating and preserves original tags while refreshing path/size/time/thumbnail. `delete_missing_records(&[i64])->Result<usize,String>` checks again, deletes records only, never a local source file. Errors propagate to Gallery operation feedback/MCP. Camera/editor interoperability remains outside this change.

## MCP contributions

When the shared `McpToolRegistry` service is available, this plugin registers `tagging.list_tags`, `tagging.assign`, `tagging.remove`, `tagging.list_missing_records`, `tagging.relink_missing_record`, and `tagging.delete_missing_records`. Schemas, validation, mutation classification and handlers live in this crate and call `TaggingService`; the registry guard is owned by Cordis and removes the tools when the plugin stops. Assignment/removal accept 1–100 positive media IDs and 1–50 positive tag IDs. Deleting missing rows requires explicit confirmation and rechecks the source paths at action time. Mutations are subject to the host MCP mutation gate.
