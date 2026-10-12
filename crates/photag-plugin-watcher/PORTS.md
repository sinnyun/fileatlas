# Watcher plugin ports / 文件监视数据对接

Status: Cordis-managed watcher, bounded reconciliation, and Linux create/remove integration exist. Native Windows delivery and race behavior are unverified.

## Lifecycle and dependencies

- Cordis plugin name: `photag.watcher`; service: `photag/watcher` (`WatcherService`).
- Requires `DatabaseService` and `IndexingService`; emits `MediaIndexChanged` after database changes.
- Plugin stop signals and joins its background reconciliation worker; do not start a second independent watcher loop.

## Input / output

- `watch(path)` / `unwatch(path)` schedule recursive folder monitoring.
- `poll_and_reconcile()` returns `ReconcileSummary { added_or_updated, removed }`; processing is bounded to 200 paths per poll.
- Added/updated files are reconciled into SQLite and missing thumbnail generation is delegated to Indexing. Successful changes dispatch Cordis event `photag/indexing/media-index-changed` with unit args.

## Consumers and debugging

Navigation registers folders and consumes scan lifecycle; gallery observes the index-changed event and reloads. Check watched root registration, debounce queue, event path classification, reconcile errors, and event routing. Keep the source 800 ms debounce and bounded drain behavior unless parity evidence supports a change.
