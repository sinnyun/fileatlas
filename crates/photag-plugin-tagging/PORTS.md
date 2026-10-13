# Tagging plugin ports / 标签数据对接

Status: database-backed assignment/association/merge/reparent service and
source-compatible JPEG IPTC/non-JPEG XMP metadata writeback are available.
Picker UI and hotkeys are incomplete; this new migration slice has not been
compiled or tested in the cloud pass.

## Lifecycle and dependencies

- Cordis plugin name: `photag.tagging`; service: `photag/tagging` (`TaggingService`).
- Requires `DatabaseService`; contributes a tagging widget at `gallery.selection-actions`.

## Input / output

- `assign(media_ids, tag_ids)` applies requested tags, one-way associations, and ancestor tags while preserving `auto_applied` distinction.
- `remove(media_ids, tag_ids)`, `clear(media_ids)`, `for_media(media_id)`, and `associated_with(tag_id)` expose the corresponding source queries.
- `set_associations(tag_id, associated_ids)` updates association edges; `merge(source_id, target_id, reparent_children)` retains source merge behavior. Operations return `Result<_, TaggingError>`; database failures propagate.
- `move_tag(source_id, drop_position)` returns the media IDs whose implicit ancestor assignments changed and schedules metadata synchronization.
- Successful `assign`, `remove`, `clear`, `merge`, and `move_tag` calls schedule best-effort metadata rewrites for affected media. JPEG/JPEG files receive IPTC keywords embedded in APP13; other extensions receive an `.xmp` sidecar with Dublin Core subjects and Lightroom hierarchical subjects. Empty tag lists remove prior sidecars or IPTC keywords.
- `xmp::write_metadata(path, tag_paths)` is also public for explicit integration. The original access/modified times are restored best-effort after writing.

## Consumers and debugging

Use stable integer media/tag IDs from the database service. After mutation reload tag rows before showing results; verify ancestor and association flags when debugging unexpected tags. Metadata writes are detached and best-effort, matching the source; file errors are not returned by assignment methods. Compare `src/xmp.rs` byte behavior with `vendor/photag-source/src/xmp.rs` if local Windows testing exposes IPTC/XMP differences. Gallery F1–F9 hotkeys now call the service through Settings mappings; general tag picker/context menus, visual parity, and a user-visible metadata-write error channel remain pending.
