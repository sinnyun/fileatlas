# Tagging plugin ports / 标签数据对接

Status: database-backed assignment/association/merge service exists and synthetic assignment behavior is covered. Picker UI, hotkeys, and XMP rewrite integration are incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.tagging`; service: `photag/tagging` (`TaggingService`).
- Requires `DatabaseService`; contributes a tagging widget at `gallery.selection-actions`.

## Input / output

- `assign(media_ids, tag_ids)` applies requested tags, one-way associations, and ancestor tags while preserving `auto_applied` distinction.
- `remove(media_ids, tag_ids)`, `clear(media_ids)`, `for_media(media_id)`, and `associated_with(tag_id)` expose the corresponding source queries.
- `set_associations(tag_id, associated_ids)` updates association edges; `merge(source_id, target_id, reparent_children)` retains source merge behavior. Operations return `Result<_, TaggingError>`; database failures propagate.

## Consumers and debugging

Use stable integer media/tag IDs from the database service. After mutation reload tag rows before showing results; verify ancestor and association flags when debugging unexpected tags. XMP sidecar synchronization is not yet connected and must not be assumed.
