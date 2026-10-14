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
