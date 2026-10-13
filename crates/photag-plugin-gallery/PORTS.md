# Gallery plugin ports / 图库数据对接

Status: query/window service and partial GPUI view are implemented. Full original interaction and visual parity are incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.gallery`; service: `photag/gallery` (`GalleryService`).
- Requires `DatabaseService`, `FileOperationsService`, `DragDropService`, `MediaPlaybackService`, `PreviewService`, and `SessionService`; registers its view/contributions in the Cordis-owned UI bridge.
- Optionally consumes `HistoryService`, `SettingsService`, and `TaggingService`. Gallery starts without these providers; history shortcuts/tag hotkeys/configurable opening are inactive when their provider is absent.
- Observes `FolderSelectionChanged`, `TagSelectionChanged`, `DirectoryPathSelected`, and `MediaIndexChanged`.

## Input / output

- `load(MediaQuery)` returns `GallerySnapshot`: compact ordered skeletons, total count, initial rich media window (up to 200), batched media tags, the full tag tree for naming active tag filters, available extensions, folder counts, and retained query.
- `ensure_window(snapshot, range)` returns `None` when current window covers range, otherwise a `GalleryWindow` with offset, media rows, and batched tags. `load_page(query, offset)` loads the next 200 rows.
- `media_by_id(id)` resolves an indexed row; `open_count(id)` reads the original informational counter; `apply_rating(ids, rating)` persists source-compatible clamped ratings. Filter mapping lives in `filter.rs`; selection behavior in `selection.rs`.
- The `Preview` button, Enter, and card double-click open selected images through `PreviewService` and videos through `MediaPlaybackService`; audio reports that preview is unavailable. Escape and `Return to gallery` close video playback. `Open` continues to launch the system default application.
- Starting a card drag calls `DragDropService::start_file_drag` on GPUI's drag-start callback. A selected card supplies the selected paths in selection order; an unselected card supplies only its own path. Ctrl/Shift-modified drags are suppressed, matching the source gallery. The native service starts a copy-only Windows CF_HDROP drag.
- The selected-item details area reads filesystem read-only/modified/created values, formats size and image dimensions/aspect ratio, displays the persisted open count, rating, capture date and hierarchical tags. Filesystem metadata failures are treated as unavailable fields.
- For selected images, `Show EXIF fields` lazily reads the source `kamadak-exif` 0.6.1 parser output, shows the original curated camera/exposure/GPS fields followed by the complete EXIF field list, and hides those fields again on request. Selection changes close the EXIF section so a new file is not read during each ordinary gallery render.
- The GPUI view restores and persists the source session's search, sort, media type, extension, date, rating, and tag filters through `SessionService`. Filter persistence preserves unrelated navigation/session fields. Search uses an input; date bounds use GPUI Kit calendar date pickers initialized from saved dates. Active include/exclude tag filters show individually removable pills with tag names (or a deleted-tag marker).
- The GPUI view restores saved gallery selection IDs only when they remain in the active query and restores the saved pixel scroll offset. It persists selection/filter changes, changed scroll offsets (sampled every 200 ms), and the current state when the view is dropped. Changing the query clears selection and returns the scroll to the top. A process terminated before the next save interval may lose the latest scroll movement.
- Thumbnail size (100–300 px) and the date-sorted month grouping toggle restore from and save to `SessionService`; the default thumbnail size is 160 px and month grouping defaults on.
- The gallery appends the next 200 rich media rows when downward scrolling reaches the end of the loaded card region; keyboard navigation materializes missing pages before scrolling to its target. Thumbnail display size can be adjusted from 100 to 300 px, and month headings can be toggled for newest/oldest date sorts. The scroll trigger estimates four columns, so its prefetch threshold can vary with window width. Smooth viewport row virtualization, database micro-thumbnail painting/prewarming, and rubber-band pointer selection remain incomplete.
- Image preview receives the active query's ordered image paths from compact gallery skeletons, so the preview filmstrip can navigate beyond the currently loaded rich-media page.
- Configured `Settings.open_key` opens the selected item when the Gallery root has focus and no bulk/delete confirmation is active (default `Space`; absent settings provider retains the current `Enter` behavior). F1–F9 settings mappings toggle their tag for the selected media, testing membership on the first selected item as in the source app; assignment/removal goes through `TaggingService`, preserving association/ancestor and metadata-write semantics. Successful toggles record source-shaped `AssignedTags`/`RemovedTags` entries when history is available.
- With media selected, the Gallery exposes a searchable hierarchical tag picker. Search matches tag names and full paths; selected choices can be assigned or removed from all selected media, or all assignments can be cleared. Successful operations reload results and record `AssignedTags`, `RemovedTags`, or pre-clear `ClearedTags` history entries when history is available.
- Successful Gallery rating changes record `RatingChanged` with the source's representative previous rating and clamped target rating; successful single-item rename records `Renamed` only after the indexed filename confirms the change. Ctrl+Z/Ctrl+Y call the optional `HistoryService`; completion status is visible in the Gallery and the query is reloaded after undo/redo.
- Current Gallery history producers are tag hotkeys and the tag picker, rating, and single-item rename. Delete, copy, move, and bulk file actions are not variants of the source `UndoEntry` and remain non-undoable. Settings open-key support depends on GPUI key names exposed by the adapter; Windows focus/shortcut behavior still needs native validation.

## Consumers and debugging

UI requests should use visible ranges and retain the query paired with the snapshot. After a filter or index event, reload the snapshot and reset selection/window state consistently. Inspect query arguments, offsets, and media IDs when rows or tags appear stale. Remaining original info-panel work: ffprobe video details, preview-only show/hide panel, and visual parity. EXIF values and Windows file attributes/timestamps require local real-file verification.

## Filter-driven bulk actions

The gallery's “Bulk actions…” panel ports the source `BulkActionState` workflow: choose Delete (Recycle Bin), Copy to, or Move to; scope matches to the whole library, the selected folder/path, selected tag(s), or current selection; then apply AND-combined date range, older/newer-than day count, inclusive rating range, filename substring, extension, and file-size bounds. It shows a live match count from compact gallery skeletons and uses those same IDs at execution. Current-selection mode deliberately skips the extra filters. Delete/Backspace and the delete button open a modal confirmation with a Cancel action; Enter confirms and Escape cancels. Bulk copy/move use native folder pickers and `FileOperationsService` per-item outcomes; move/delete refresh the gallery after completion. Single-item copy/move use source-compatible save-file pickers.

Limits: invalid bulk filters show an error and match zero items. Native file-operation behavior and bulk matching remain unverified on Windows until the concentrated Windows validation pass.

## Shared visual tokens

The GPUI gallery, cards, and bulk-action panel consume photag-ui-bridge::theme for source palette surfaces, text hierarchy, danger messaging, selection accent, and selected-card fill. Toolkit text-size presets and spacing helpers still do not map to every source token; Windows visual parity is unverified.
