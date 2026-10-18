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

`MediaQuery::external_search_paths` optionally supplies candidate full paths
from Everything. The database still applies active filename/path substring
scopes and tag search, along with all PhoTag-owned filters. The field is a
query optimization and does not alter the persistent SQLite schema. Gallery
owns provider selection and fallback; database code never calls Everything.

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

## Everything search cache

Everything search integration adds `cache_search_files(&[BrowserFile]) -> Result<Vec<String>, DatabaseAccessError>`.
It materializes real SDK hits in the session database without changing live-directory membership.
Normal/extended Windows path aliases reuse row IDs; same path/version sets are a no-op and do not
advance the write revision. New search sets prune old derived hits while retaining directory scopes,
ratings/tags and preview/history pins. `search_paths` is memory-only; durable annotation persistence
does not save search inventories. Directory and search ingestion share `search_cache::cache_file`.

## Lore version metadata


`versions`, `version_graph`, and `version_metadata_imports` are durable typed
tables owned by this service. `VersionInfoService` delegates to typed methods;
feature crates must not open a second SQLite connection or use raw SQL
callbacks. Browser-mode persistence includes these tables so version history
survives process restarts. Legacy `version-info.sqlite3` is imported
idempotently and preserved as a rollback copy.

## Program launcher tables (2026-10-17)

The shared PhoTag database schema includes `app_launcher_categories` and
`app_launcher_entries`; they live in the same database service and connection
pool as the rest of the application. Browser-mode `photag-tags.db` persistence
copies these tables into its durable store and synchronizes removals, so launcher
categories and edits survive restarts without creating a plugin-owned DB file.
The typed methods `launcher_snapshot`, `launcher_merge_discovered`,
`launcher_add_category`, `launcher_rename_category`, `launcher_delete_category`,
`launcher_rename_entry`, `launcher_move_entry`, `launcher_set_hidden`,
`launcher_remove_entry`, and `launcher_add_manual` provide the feature boundary.
Schema setup is idempotent in `db::initialize` and adds tables only; it does not
alter existing photo rows or columns.

Search-location contract (2026-10-17): `MediaQuery.search_entire_computer: bool`
is a Gallery orchestration flag (default false), not a SQLite schema field.
Gallery normalizes active search directory constraints before invoking the DB;
SQL continues to apply `directory_tree`, tag/date/rating and candidate predicates.
No migration or persistent inventory changes are introduced by this flag.

### Launcher installation metadata (2026-10-17)

The existing database owns additive `app_launcher_metadata(entry_id INTEGER PRIMARY KEY REFERENCES app_launcher_entries(id) ON DELETE CASCADE, installed_on TEXT, size_bytes INTEGER, pin_scope INTEGER NOT NULL DEFAULT 0)`. `pin_scope` uses 0=none, 1=category top, 2=all-programs top. Startup migrates an earlier metadata table by adding the defaulted column. `launcher_set_pin_scope` validates and writes through the typed service; rescans preserve the value while updating installation metadata. Browser-mode persistence synchronizes the new column with its table. The regression checks prior-schema migration, restart persistence, scope validation and cascade deletion.
