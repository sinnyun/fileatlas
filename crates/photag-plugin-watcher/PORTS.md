# Watcher plugin ports / 文件监视数据对接

Status: Cordis-managed watcher, bounded reconciliation, and Linux create/remove integration exist. Native Windows delivery and race behavior are unverified.

## Lifecycle and dependencies

- Cordis plugin name: `photag.watcher`; service: `photag/watcher` (`WatcherService`).
- Requires `DatabaseService` and `IndexingService`; emits `MediaIndexChanged` after database changes and startup scan commits.
- Plugin stop signals and joins its background reconciliation worker; do not start a second independent watcher loop.

## Input / output

- `watch(path)` / `unwatch(path)` schedule recursive folder monitoring.
- `poll_and_reconcile()` returns `ReconcileSummary { added_or_updated, removed }`; processing is bounded to 200 paths per poll.
- Added/updated files are reconciled into SQLite and missing thumbnail generation is delegated to Indexing. Successful changes dispatch Cordis event `photag/indexing/media-index-changed` with unit args.
- On startup, the existing background worker scans every persisted root through `IndexingService::scan_folder` before it begins polling notifications. `library_refresh()` returns `LibraryRefreshState { running, total_folders, completed_folders, failures }`. Plugin stop sets the shared cancellation flag and joins the worker.

## Consumers and debugging

Navigation registers folders and consumes scan lifecycle; gallery observes the index-changed event and reloads. Check watched root registration, debounce queue, event path classification, reconcile errors, and event routing. Keep the source 800 ms debounce and bounded drain behavior unless parity evidence supports a change.

## All-file notifications (2026-10-14)

Create/remove/rename paths are no longer restricted by suffix or dot-prefix. Reconciliation accepts ordinary files only (`metadata.is_file()`), preventing directory rows. Classification reuses the public Indexing `scan::classify_media_type`, including `audio` and generic `file`. The 800 ms debounce, 4,000-event drain and 200-path emission limits are unchanged. The Windows integration regression covers hidden unknown files, extensionless files, audio, directory exclusion and removal. Generic modify events remain ignored as before; the startup traversal refreshes file size and modification time after the next restart. This is not exhaustive race/rename-directory coverage.
