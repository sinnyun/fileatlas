# Indexing plugin ports / 索引数据对接

Status: Cordis service and original scan workflow are present. Current tests use synthetic fixtures; full media-format and Windows parity remain pending.

## Lifecycle and dependencies

- Cordis plugin name: `photag.indexing`; service: `photag/indexing` (`IndexingService`).
- Requires `DatabaseService` and `ThumbnailService` to be published before startup.

## Input / output

- `scan_folder(folder_id, folder_path, cancel)` takes a registered PhoTag folder identity/path plus shared `AtomicBool` cancellation flag; returns `mpsc::Receiver<Option<ScanProgress>>`. Progress values carry scan updates; `None` is the completion sentinel.
- `generate_missing_thumbnails(items, cancel)` takes `(media_path, media_type)` pairs and returns the same progress channel shape.
- `list_subdirectories(path)` returns immediate `(name, path, has_children)` tuples using source filtering.
- Writes source-compatible media/folder rows through the database service and image cache through the thumbnail service.

## Consumers and debugging

Navigation starts/cancels scans; watcher backfill can request missing thumbnails. Keep channel draining off the UI render pass. Diagnose a stalled scan by checking cancellation ownership, receiver drain, filesystem access, database writes, and whether the producer reached the `None` sentinel. FFmpeg extraction and Windows paths require later Windows/media-corpus verification.
