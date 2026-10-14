# Startup data refresh verification / 启动数据刷新验证

Date: 2026-10-14. The accepted UI baseline remains `dev-baseline-2026-10-14`; this is a targeted data freshness repair.

## Cause

The `start-dev.bat` launcher selects `PHOTAG_DEV_PLUGINS=all`, including the isolated development profile under `target/dev-test`. The all-file scanner had been updated, but the app's startup path only loaded persisted SQLite rows and registered native folder watches. A full scan ran only after adding a folder or explicitly requesting Rescan. The watcher handles later create/remove/rename notifications, so files already present when the app starts, and file changes made while it is closed, stayed stale in the persisted index.

The same-process PhoTag MCP report from the old profile copy showed the registered test folder at 40 rows and the built-in sample at 9 rows. The full test folder on disk contained 79 ordinary files. This distinguished a stale persisted index from a Gallery display/filter issue.

## Change

Watcher now starts a cancellable background reconciliation for each registered folder through the production Indexing service. It upserts all ordinary files, refreshes file size and modification time, preserves existing row IDs and user metadata, and invalidates/rebuilds cached image thumbnails when file metadata changes. It removes rows for offline-deleted files after successful traversal. Unavailable roots retain their cached rows and expose a failure. Index commits emit `MediaIndexChanged`; Gallery reloads while keeping valid selection and scroll position. The read-only MCP `get_ui_state.library_refresh` field exposes progress and failures.

## Verification

The source profile and real user library were left unchanged. A copy of the old persisted profile was launched with the rebuilt default shell executable and the same `all` plugin selection used by `start-dev.bat`:

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p photag-shell
cargo +1.97.1 build --locked -j 2 -p photag-shell
cargo +1.97.1 test --locked -j 2 -p photag-plugin-indexing
cargo +1.97.1 test --locked -j 2 -p photag-plugin-watcher
cargo +1.97.1 test --locked -j 2 -p photag-plugin-gallery
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-startup-refresh --plugins all --plan target/mcp-startup-refresh-plan.json
```

Results: fmt/check/build passed; Indexing 3 passed and 1 existing FFmpeg test ignored; Watcher 2 passed; Gallery 16 passed. In the same-process GPUI/MCP run, the two persisted roots finished with `running=false`, `completed_folders=2`, and no failures. Counts changed from 40 to 79 and from 9 to 12; the full indexed library now has 91 rows. `search_media` returned all 79 rows for the test folder, including 39 generic `file` records. The existing Gallery path showed 60 rows after the index event, and its preexisting selected ID remained selected. Forty previously indexed test-folder paths retained their IDs and ratings; tagged fixture media IDs 13, 17, and 18 retained their existing tag assignments and ratings (4 on IDs 17 and 18).

Offline deletion cleanup and unavailable-root retention are covered by `successful_startup_scan_removes_offline_deletions_but_keeps_unavailable_roots`. The Indexing regression also replaces an image with a different size at the same path and verifies refreshed dimensions/file size while preserving its ID, rating, capture date, and tag links; this catches stale path-hash thumbnail reuse. A separate Watcher integration test covers live create/remove delivery. This does not establish Windows notification race coverage or startup performance for very large, slow, or network libraries. The persisted Windows user profile itself was not rewritten; the verification profile and screenshot are local files under `target/mcp-startup-refresh/`.
