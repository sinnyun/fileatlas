# Everything search adapter

## Lifecycle

- Cordis plugin: `fileatlas.everything-search`; service: `fileatlas/everything-search`.
- Shell publishes the optional service before FileManager. Publication and software checking do not launch external software.
- A filename/path search connects to the existing Everything 1.5 client. If absent, it locates an installed compatible executable and starts it with official `-startup -first-instance` arguments without opening a search window. No download, installation, upgrade, service/configuration change, or process termination is performed.
- Startup attempts are serialized and limited to one per 30 seconds. An index-loading result starts one bounded readiness worker; a genuine transition to ready increments `revision()`. FileManager retries the current query through its existing refresh monitor. Service disposal cancels readiness polling; Everything remains the user's external application.

## Public contract

- `search_paths(query: &str, roots: &[String]) -> Result<Vec<String>, EverythingSearchError>`: synchronous worker-only query; returns deduplicated full file paths. Current-folder searches use that folder; collection searches use registered roots; an empty roots slice searches Everything's whole index.
- `inspect_installation() -> EverythingInstallation`: blocking registry/file-version inspection. Call through a ReadModel, never render. States: `missing|incompatible|ready|unsupported`; includes optional executable path and actual four-component EXE version. SDK 3 requires Everything 1.5 or later.
- `revision() -> u64` and `index_loading() -> bool`: passive in-memory reads. The readiness worker changes the revision once at completion, not on each timer wake.
- Portable installations can use `FILEATLAS_EVERYTHING_EXE`; an explicit missing path is authoritative. Detection also checks App Paths/uninstall locations, Program Files, Local AppData and PATH.
- Search uses SDK 3 regex mode with escaped text/root paths. FileAtlas substring semantics, including existing SQL LIKE `%|_` wildcards, are retained; valid names beginning with `!` are not interpreted as Everything operators.
- SDK 3.0.1.12 viewport setters always return FALSE despite updating state. The adapter verifies their values with getters. Unicode output buffers use the SDK's required byte count, including NUL, rather than silently accepting truncated paths.
- Busy-pipe probes avoid the SDK's default 30-second busy wait. Per-client shutdown watchdogs bound query RPCs to 15 seconds and readiness probes to 2 seconds; a lease joins its watchdog before destroying the client. Restoring a search queues the first query on FileManager's ReadModel and never waits for external IPC on the GPUI thread.
- Above 250,000 results, unsupported/missing/incompatible clients, or other IPC failures return an error. FileManager uses SQLite fallback; a successful provider result is never inferred from installation detection.

## Distribution and diagnostics

- Official SDK 3.0.1.12 x64 static library/header and MIT notice: `vendor/everything-sdk`. The Everything application is not bundled.
- Settings > Software remains read-only installation detection. FileManager exposes installation/version/loading state and installation links immediately after the search input/scope button.
- Inspect `FileManagerSnapshot.search_provider`, `search_media.search_provider` and `get_ui_state.file_manager.everything`. Each rendered snapshot retains its own provider so a concurrent MCP query does not overwrite its diagnostic state.
- The earlier Session 0 incompatibility explanation was incorrect: PID 38460 was `Everything.exe -svc`, the indexing service, not an SDK search client. Starting the existing 1.5.0.1423 search client in the user session fixed IPC; no Everything upgrade was needed.
- Runtime evidence: `docs/validation-ui/2026-10-17-everything-search/README.md`.

