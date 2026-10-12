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
