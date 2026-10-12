# Gallery plugin ports / 图库数据对接

Status: query/window service and partial GPUI view are implemented. Full original interaction and visual parity are incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.gallery`; service: `photag/gallery` (`GalleryService`).
- Requires `DatabaseService`, `FileOperationsService`, `PreviewService`, and `SessionService`; registers its view/contributions in the Cordis-owned UI bridge.
- Observes `FolderSelectionChanged`, `TagSelectionChanged`, `DirectoryPathSelected`, and `MediaIndexChanged`.

## Input / output

- `load(MediaQuery)` returns `GallerySnapshot`: compact ordered skeletons, total count, initial rich media window (up to 200), batched tags, available extensions, folder counts, and retained query.
- `ensure_window(snapshot, range)` returns `None` when current window covers range, otherwise a `GalleryWindow` with offset, media rows, and batched tags. `load_page(query, offset)` loads the next 200 rows.
- `media_by_id(id)` resolves an indexed row; `open_count(id)` reads the original informational counter; `apply_rating(ids, rating)` persists source-compatible clamped ratings. Filter mapping lives in `filter.rs`; selection behavior in `selection.rs`.
- The `Preview` button and Enter open the selected image through `PreviewService`; other media types report that image preview is unavailable. `Open` continues to launch the system default application.
- The selected-item details area reads filesystem read-only/modified/created values, formats size and image dimensions/aspect ratio, displays the persisted open count, rating, capture date and hierarchical tags. Filesystem metadata failures are treated as unavailable fields.
- For selected images, `Show EXIF fields` lazily reads the source `kamadak-exif` 0.6.1 parser output, shows the original curated camera/exposure/GPS fields followed by the complete EXIF field list, and hides those fields again on request. Selection changes close the EXIF section so a new file is not read during each ordinary gallery render.
- The GPUI view restores and persists the source session's search, sort, media type, extension, date, rating, and tag filters through `SessionService`. Filter persistence preserves unrelated navigation/session fields. Search and date text inputs are initialized from the stored values.

## Consumers and debugging

UI requests should use visible ranges and retain the query paired with the snapshot. After a filter or index event, reload the snapshot and reset selection/window state consistently. Inspect query arguments, offsets, and media IDs when rows or tags appear stale. Remaining original info-panel work: ffprobe video details, preview-only show/hide panel, and visual parity. EXIF values and Windows file attributes/timestamps require local real-file verification.

## Filter-driven bulk actions

The gallery's “Bulk actions…” panel ports the source `BulkActionState` workflow: choose Delete (Recycle Bin), Copy to, or Move to; scope matches to the whole library, the selected folder/path, selected tag(s), or current selection; then apply AND-combined date range, older/newer-than day count, inclusive rating range, filename substring, extension, and file-size bounds. It shows a live match count from compact gallery skeletons and uses those same IDs at execution. Current-selection mode deliberately skips the extra filters. Delete requires a second explicit confirmation. Copy/move require a destination path and use `FileOperationsService` per-item outcomes; move/delete refresh the gallery after completion.

Limits: destination folder picking has not yet migrated, so copy/move use a typed path. Invalid filters show an error and match zero items. Native file-operation behavior and bulk matching remain unverified on Windows until the concentrated Windows validation pass.
