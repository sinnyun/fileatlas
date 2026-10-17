# Indexing plugin ports / 索引数据对接

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: Cordis service and original scan workflow are present. Current tests use synthetic fixtures; full media-format and Windows parity remain pending.

## Lifecycle and dependencies

- Cordis plugin name: `photag.indexing`; service: `photag/indexing` (`IndexingService`).
- Requires `DatabaseService` and `ThumbnailService` to be published before startup.

## Input / output

- `scan_folder(folder_id, folder_path, cancel)` takes a registered PhoTag folder identity/path plus shared `AtomicBool` cancellation flag; returns `mpsc::Receiver<Option<ScanProgress>>`. Progress values carry scan updates; a terminal `ScanProgress.error` is populated for directory traversal, media indexing/finalization, thumbnail database batch writes, or scan worker panic failures. `generate_missing_thumbnails` reports thumbnail database batch write errors and worker panics through the same terminal field. `None` remains the completion sentinel. Cancellation completes without a failure payload.
- `generate_missing_thumbnails(items, cancel)` takes `(media_path, media_type)` pairs and returns the same progress channel shape.
- `list_subdirectories(path)` returns immediate `(name, path, has_children)` tuples including dot-prefixed directories.
- Writes source-compatible media/folder rows through the database service and image cache through the thumbnail service.

## Consumers and debugging

Navigation starts/cancels scans; watcher backfill can request missing thumbnails. Keep channel draining off the UI render pass. Diagnose a stalled scan by checking cancellation ownership, receiver drain, filesystem access, database writes, and whether the producer reached the `None` sentinel. FFmpeg extraction and Windows paths require later Windows/media-corpus verification.

## All-file indexing (2026-10-14)

`classify_media_type(extension: &str) -> &'static str` is the shared, case-insensitive classifier used by scanning and Watcher. It returns `image`, `video`, `audio`, or `file`; empty and unrecognized extensions return `file`. Recursive scans index every accessible ordinary file, including dot-prefixed files/directories and extensionless files. Image files paired with a video remain independent rows while still supplying its poster. Only recognized image/video types request decoder thumbnails; unsupported files retain no fabricated thumbnails. `list_subdirectories` includes dot-prefixed directories. Traversal errors still produce a terminal error, and cancellation/batch limits remain in force. Reconciliation deletes rows for offline-deleted files only after a complete traversal of an available root; an unavailable root keeps its cached rows. Repeated scans refresh file size and modified time while preserving IDs, ratings, dates, tags, and cached dimensions. Navigation Rebuild/Rescan retains its explicit clear-and-rebuild behavior.

Repeated image thumbnail-cache hits may return no decoded dimensions/micro bytes. Scan batch updates retain existing width/height/micro via COALESCE rather than clearing those values. Regression and same-process repeat-scan query cover retained dimensions.

## Startup refresh (2026-10-14)

Watcher starts one cancellable background refresh over every registered folder using `IndexingService::scan_folder`. This adds file types excluded by older builds and removes offline deletions, then emits `MediaIndexChanged` as rows are committed so Gallery can reload while the refresh proceeds. `WatcherService::library_refresh()` reports running state, folder progress and failures; `get_ui_state` exposes the same snapshot for live diagnostics. Missing/unavailable roots report an error and retain their existing database rows. This is a full recursive traversal at each application start. When file type, size, or modification time changes, the scanner invalidates its old cached thumbnail and regenerates it; unchanged paths reuse the thumbnail cache. Windows traversal and very large-library startup time still need user-scale verification.

## Checked immediate directory reads (2026-10-14)

`IndexingService::try_list_subdirectories(&self, path: &str) -> std::io::Result<Vec<(String,String,bool)>>` returns the same name/path/has_children payload, including dot-directories, but reports read_dir/entry/file_type failures for Gallery's browser error state. Existing list_subdirectories keeps its best-effort skip/error-empty behavior through the shared reader; no scanning/writing/event semantics change. Child-presence probes remain best effort. Used only for immediate directory cards, including empty directories.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics observe tracked production progress streams: phase transitions, errors, scan_id and stream completion with duration_ms, failed and completed flags. Observation still forwards all messages and cleans its active entry even if downstream closes; no fabricated scan result. Stream elapsed includes work/wait, not CPU time.


## Browser runtime update — 2026-10-15

Indexing obtains video cache paths and micro-thumbnail generation through `ThumbnailService`; it does not import thumbnail implementation modules.

Normal startup, folder browsing and favorite clicks do not invoke recursive indexing. `visible_thumbnail(&str,&str)->Option<(String,Option<u32>,Option<u32>,Option<Vec<u8>>)>` reuses the image thumbnail and ffmpeg video poster pipelines for current visible demand. Paths inside the derived thumbnail directory are excluded to prevent cache self-generation. The recursive scan API remains for isolated dev fixtures and legacy regression coverage, not the browser product entry point.
