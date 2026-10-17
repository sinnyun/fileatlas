# Version Info plugin ports / 历史版本信息对接

## Lifecycle and ownership

- Cordis plugin: `photag.version-info`; service: `photag/version-info` (`VersionInfoService`).
- Starts before `photag.versions`; owns `version-info.sqlite3` beside PhoTag's active database.
- Stores display metadata, current-revision flags, branch ancestry projections and pending-merge conflict state. Lore owns file contents and revision objects.
- Separate database file; PhoTag's existing `photag.db` / `photag-tags.db` schema is not migrated.

## Service contract

- `list(entity_id)` returns revisions newest first, including revision ID/number, parent IDs, title, timestamp, size, MIME, preview path and current status.
- This complete persisted catalog owns saved-version visibility. Lore's current-HEAD ancestry is not a filter for it; `set_current` changes flags without deleting older or newer records.
- `save(record)` upserts metadata transactionally and maintains one current revision per entity.
- `set_current(entity_id, version_id)` rejects unknown revisions and updates current status in one transaction.
- `put_graph(entity_id, graph)` stores the latest Lore branch projection and recoverable pending merge; `graph(entity_id)` returns an empty projection when none is present.
- Entity IDs are path-derived by the Lore plugin. Current limitation: renaming/moving a file creates a new history identity; path rebind support is not implemented.

## Errors and inspection

SQLite, JSON and I/O errors are returned as `VersionInfoError`; a poisoned service lock fails the call. Inspect this crate's service methods and `version-info.sqlite3` in the isolated profile. It does not expose arbitrary SQL or rewrite PhoTag annotation records.
