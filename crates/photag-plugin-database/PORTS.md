# Database plugin ports / 数据对接

Status: implemented source-compatible SQLite service; broad query-by-query regression coverage remains pending. Source of truth: `src/lib.rs` and `src/db/`.

## Lifecycle and dependency

- Cordis plugin name: `photag.database`; service: `photag/database` (`DatabaseService`).
- No plugin dependency. `DatabaseConfig.path = None` opens `%LOCALAPPDATA%\PhoTag\photag.db`; `Some(path)` overrides it.
- Cordis publication is scoped to plugin lifetime. Consumers obtain it with `Context::try_service::<DatabaseService>()`.

## Input / output

- Read: `with_reader(|&Connection| -> rusqlite::Result<T>)`; use only for short read operations.
- Write: `with_writer(|&Connection| -> rusqlite::Result<T>)`; serialized through the writer mutex.
- Query/domain API: `DatabaseService::db::queries` retains PhoTag query functions and source schema. A callback returns `T`, wrapped in `Result<T, DatabaseAccessError>`; SQLite and poisoned-lock failures are surfaced.
- Persistent contract: existing `photag.db`, tables, columns, migrations, WAL settings, and separate read connection. Do not change schema or row semantics without a legacy fixture and reopen test.

## Consumer rules / debugging

Keep closures short; move expensive scans/metadata work off the GPUI render thread. On a missing-service error, check startup ordering and Cordis plugin lifetime first; on a database error, record the operation and SQLite error without logging media contents.

## Generic file type (2026-10-14)

`MediaItem.media_type` / `MediaQuery.media_type` support `file` alongside `image`, `video`, and `audio`. The existing SQLite text column and unfiltered queries already accommodate this value; no schema or persisted-file migration is needed. Decoder support does not determine row visibility. Image/video statistics remain media-specific; all-file totals include generic rows.

`queries::scan_upsert_media(conn, folder_id, filename, full_path, media_type, file_size, modified_at, poster_path) -> rusqlite::Result<Option<String>>` is the scanner-specific upsert and returns the old cache path when source file metadata changes. On a full-path conflict it refreshes folder/name/type/size/modification time and supplied video poster without replacing the row; it keeps IDs, ratings, tag links and capture dates. If type, size, or modification time changes it clears cached dimensions/micro-thumbnail data so Indexing regenerates the image cache. Existing `insert_media` and file-operation semantics are unchanged. This avoids the source scanner's `INSERT OR REPLACE` metadata loss when reconciling persisted indexes.

## Compact grouping inputs (2026-10-14)

`db::queries::get_media_grouping_fields(&Connection, &MediaQuery) -> rusqlite::Result<Vec<MediaGroupingFields>>` returns filtered distinct id/full_path/modified_at/root_path using the existing filter builder and registered folders. Call through DatabaseService. It ignores paging like skeletons; no created-at column or schema migration is added. Grouping owns filesystem creation-time reads. Gallery fetches viewport rich rows by arranged IDs; SQL query/SortOrder meanings remain unchanged.

## Immediate-directory query (2026-10-14)

`MediaQuery.direct_directory: Option<String>` restricts all existing query/skeleton/count builders to ordinary indexed rows directly below the supplied path. SQL parameters compare a slash-normalized literal prefix, then require no further separator; `%` and `_` are literal directory characters. Source path_prefix recursion remains unchanged. This is an in-memory query field, not a schema change. Gallery browser projection uses it; recursive callers retain None.245-row cache/scoped-query regression is in Gallery browser_tests.rs.

## Runtime diagnostics (2026-10-14)

Runtime diagnostic wrappers retain separate WAL reader/writer connections, locking, SQL operation order and error types. with_reader/with_writer now record caller location and wall-clock spans including lock wait; SQL failures emit ERROR. No schema change or SQL rewriting. The logging subscriber belongs to photag.debug.

## In-memory writer revision (2026-10-15)

`DatabaseService::write_revision() -> u64` reads an atomic invalidation identity
without logging or querying SQLite. It advances after each writer closure
returns, even on error because the closure may have performed partial writes.
It does not advance for readers. This is conservative invalidation, not a
durable transaction ID or count of changed rows. No schema, SQL, lock order or
closure results are changed; direct external SQLite writes are not tracked.
Gallery statistics/bulk read models use this with their existing event revision
to remain fresh after direct service operations without repaint-driven reads.
