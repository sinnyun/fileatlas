# Watcher plugin ports / 文件监视数据对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

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

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.


## Browser runtime update — 2026-10-15

Browser runtime skips StartupRefresh and does not watch every favorite. `set_browse_directory(Option<&str>,bool)` replaces the active subscription; recursive=false is default, true only for descendant view. `retry_browse_watch()` retries registration on explicit refresh/reactivation. `invalidate_browse_directory`/`take_directory_dirty` coordinate full rereads with Gallery.

notify RecommendedWatcher falls back to PollWatcher (2 seconds) at initialization or registration failure. Subscription operations execute off-thread and check a generation before changing the actually registered path, preventing stale workers from unwatching a newer location. Registering emits one reconciliation hint to cover the listing/subscription gap. File Create/Remove/Rename/Data/Metadata/Any events are verified against local stat; AccessTime is ignored. Preserve 800 ms debounce, at most 4000 channel drains and 200 emitted paths; incoming and coalesced path capacity is 4096. Overflow collapses to root invalidation. External removal clears visible membership, retaining tag records. Errors log exact paths and fallback failures. Native create/delete/overwrite/directory-create were exercised; forced fallback and disconnected network drives have not been exercised.


## Registration sequencing — 2026-10-16

WatcherService::browse_registration_pending()->bool exposes whether the latest asynchronous watch registration attempt has finished. FolderWatcher keeps requested/completed generation atomics, publishing completion under the same native-watcher lock so old workers cannot overwrite a newer completion. Gallery's cancellable directory worker waits for readiness before read_dir. No synthetic root event is emitted after registration; real Create/Remove/Modify events, 800ms debounce, bounded processing and overflow invalidation remain. This closes the enumeration-before-subscription gap without a second full scan. Native watch failure still attempts PollWatcher; terminal errors remain logged rather than preventing directory browsing.
