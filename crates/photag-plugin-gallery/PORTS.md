# Gallery plugin ports / 图库数据对接

Status: query/window service and partial GPUI view are implemented. Full original interaction and visual parity are incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.gallery`; service: `photag/gallery` (`GalleryService`).
- Requires `DatabaseService`; registers its view/contributions in the Cordis-owned UI bridge.
- Observes `FolderSelectionChanged`, `TagSelectionChanged`, `DirectoryPathSelected`, and `MediaIndexChanged`.

## Input / output

- `load(MediaQuery)` returns `GallerySnapshot`: compact ordered skeletons, total count, initial rich media window (up to 200), batched tags, available extensions, folder counts, and retained query.
- `ensure_window(snapshot, range)` returns `None` when current window covers range, otherwise a `GalleryWindow` with offset, media rows, and batched tags. `load_page(query, offset)` loads the next 200 rows.
- `media_by_id(id)` resolves an indexed row; `apply_rating(ids, rating)` persists source-compatible clamped ratings. Filter mapping lives in `filter.rs`; selection behavior in `selection.rs`.

## Consumers and debugging

UI requests should use visible ranges and retain the query paired with the snapshot. After a filter or index event, reload the snapshot and reset selection/window state consistently. Inspect query arguments, offsets, and media IDs when rows or tags appear stale; UI rendering/paging parity remains for the second-round Windows verification.
