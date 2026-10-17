# Database plugin ports / 数据对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: implemented source-compatible SQLite service; broad query-by-query regression coverage remains pending. Source of truth: `src/lib.rs` and `src/db/`.

## Lifecycle and dependency

- Cordis plugin name: `photag.database`; service: `photag/database` (`DatabaseService`).
- No plugin dependency. `DatabaseConfig.path = None` opens `%LOCALAPPDATA%\PhoTag\photag.db`; `Some(path)` overrides it.
- Cordis publication is scoped to plugin lifetime. Consumers obtain it with `Context::try_service::<DatabaseService>()`.

## Input / output

- Preferred interface: typed `DatabaseService` operations for folders, media, Gallery snapshots/pages, tag and rating mutations, scan batches, watcher reconciliation, browser cache updates, metadata export, and statistics. Composite reads/writes keep their SQL and lock/transaction scope inside the database crate.
- Compatibility/debug interface: `with_reader` and `with_writer` still exist for database-owned operations and focused tests. Production feature crates must not pass arbitrary SQL callbacks; `scripts/check-layering.cjs` rejects their use outside the database crate (inline unit-test modules are excluded).
- Query facade: `photag_plugin_database::queries` remains public for source-compatible query/domain types and database crate internals. New production behavior should call typed service methods rather than execute query functions from a consumer crate. The SQLite connection and `db` implementation module are private. Typed methods return `Result<T, DatabaseAccessError>`; SQLite and poisoned-lock failures are surfaced.
- Persistent contract: existing `photag.db`, tables, columns, migrations, WAL settings, and separate read connection. Do not change schema or row semantics without a legacy fixture and reopen test.

## Consumer rules / debugging

Keep closures short; move expensive scans/metadata work off the GPUI render thread. On a missing-service error, check startup ordering and Cordis plugin lifetime first; on a database error, record the operation and SQLite error without logging media contents.

## Generic file type (2026-10-14)

`MediaItem.media_type` / `MediaQuery.media_type` support `file` alongside `image`, `video`, and `audio`. The existing SQLite text column and unfiltered queries already accommodate this value; no schema or persisted-file migration is needed. Decoder support does not determine row visibility. Image/video statistics remain media-specific; all-file totals include generic rows.

`queries::scan_upsert_media(conn, folder_id, filename, full_path, media_type, file_size, modified_at, poster_path) -> rusqlite::Result<Option<String>>` is the scanner-specific upsert and returns the old cache path when source file metadata changes. On a full-path conflict it refreshes folder/name/type/size/modification time and supplied video poster without replacing the row; it keeps IDs, ratings, tag links and capture dates. If type, size, or modification time changes it clears cached dimensions/micro-thumbnail data so Indexing regenerates the image cache. Existing `insert_media` and file-operation semantics are unchanged. This avoids the source scanner's `INSERT OR REPLACE` metadata loss when reconciling persisted indexes.

## Compact grouping inputs (2026-10-14)

`queries::get_media_grouping_fields(&Connection, &MediaQuery) -> rusqlite::Result<Vec<MediaGroupingFields>>` returns filtered distinct id/full_path/modified_at/root_path using the existing filter builder and registered folders. Call through DatabaseService. It ignores paging like skeletons; no created-at column or schema migration is added. Grouping owns filesystem creation-time reads. Gallery fetches viewport rich rows by arranged IDs; SQL query/SortOrder meanings remain unchanged.

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


## Browser runtime update — 2026-10-15

Runtime now uses `DatabaseService::open_browser_at(path)`; the plugin opens **photag-tags.db**, leaving legacy photag.db untouched. `main` is an in-memory SQLite session cache, `saved` is the attached durable database. Browsing calls `with_cache_writer`; annotation/bookmark commands call `with_writer`, which synchronizes only tagged/rated media and user-owned tables. Unchanged durable rows are not rewritten. No old library migration is performed.

Public additions: `is_browser() -> bool`, `with_cache_writer<T>(FnOnce(&Connection)->rusqlite::Result<T>)`, `begin_directory_cache(&str)`, `pin_file(&str,&str,Option<i64>)->Result<i64,_>`, `pin_history(&[i64])`, `flush_annotations()`, `forget_records(&[String])->Result<usize,_>`. The serialized browser reader uses the same memory connection. Cache IDs are real session row handles; persistent selection uses absolute paths. Recent scope budget: four directories, prune older scopes above 200,000 rows; current directory, annotated/rated files, active preview and history pins may exceed it. `file_versions` and path lists are session only. `favorite_meta` stores aliases/order; `tag_previews` stores independent durable identification thumbnail paths. The hidden folder owner 0 is not a favorite.

`MediaQuery::{local_only,direct_directory,directory_tree,annotated_only}` separate visible filesystem membership from annotation queries. `MediaItem::missing` is a presentation field, not persisted. `LibraryStats::total_files` includes all types. Inspect SQL errors in DatabaseAccessError/tracing; persistence tests cover restart, no untagged inventory, literal path prefix and history cache retention.
