# Version Info plugin ports / 历史版本信息对接

## Lifecycle and ownership

- Cordis plugin: `photag.version-info`; service: `photag/version-info` (`VersionInfoService`).
- Starts after `photag.database` and before `photag.versions`; obtains the shared `DatabaseService` and publishes the typed `VersionInfoService` facade.
- Stores display metadata, current-revision flags, branch ancestry projections and pending-merge conflict state. Lore owns file contents and revision objects.
- Version rows and branch snapshots live in typed tables managed by `photag-plugin-database`, including browser mode's durable store. On startup, a legacy sibling `version-info.sqlite3` is imported once, idempotently, and retained as a rollback copy; it is never opened for writes.

## Service contract

- `list(entity_id)` returns revisions newest first, including revision ID/number, parent IDs, title, timestamp, size, MIME, preview path and current status.
- This complete persisted catalog owns saved-version visibility. Lore's current-HEAD ancestry is not a filter for it; `set_current` changes flags without deleting older or newer records.
- `save(record)` upserts metadata transactionally and maintains one current revision per entity.
- `set_current(entity_id, version_id)` rejects unknown revisions and updates current status in one transaction.
- `put_graph(entity_id, graph)` stores the latest Lore branch projection and recoverable pending merge; `graph(entity_id)` returns an empty projection when none is present.
- Entity IDs are path-derived by the Lore plugin. Current limitation: renaming/moving a file creates a new history identity; path rebind support is not implemented.

## Errors and inspection

Database and JSON errors are returned as `VersionInfoError`. Inspect typed service methods and the `versions` / `version_graph` tables through DatabaseService diagnostics. The facade does not expose arbitrary SQL or rewrite PhoTag annotation records.
