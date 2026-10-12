# Preview plugin ports

Status: image preview service and a native GPUI preview surface are implemented. The Gallery opens selected images in this view. Source wheel multiplier, fit/preset zoom, cursor-anchored zoom, drag pan, and a toggled file/image/EXIF information overlay are available; source visual parity remains incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.preview`; service: `photag/preview` (`PreviewService`).
- The plugin owns its `preview/image` panel contribution and `preview/image-view` factory through `photag-ui-bridge`; Cordis disposal removes both.
- No startup service dependencies. Configure the transform cache with `PreviewConfig.cache_dir`; default is `%LOCALAPPDATA%/PhoTag/preview` (system temp directory when unavailable).

## Input / output

- `open(path)` selects an existing image and resets transforms to the initial fit state. `close()` clears current selection. The Info button toggles a scrollable overlay adapted from `vendor/photag-source/src/ui/preview.rs` and `src/ui/info.rs`.
- The panel reads filesystem name/path/type/extension/size/read-only/created/modified facts, image dimensions/aspect/megapixels, and the source's curated plus complete EXIF fields. Byte/date/aspect formatting follows the source conventions. EXIF dates remain in the decoded EXIF values; filesystem timestamps use local time.
- `rotate_clockwise`, `rotate_counter_clockwise`, `flip_horizontal`, and `flip_vertical` adjust non-destructive preview transforms. `set_fit` restores fit mode; `set_zoom_percent` accepts the source presets 25, 50, 100, 125, 150, and 200. `toggle_info` controls the information panel.
- GPUI wheel input reuses PhoTag's `old_zoom * (1 + scroll * 0.0025)` calculation and clamps the continuous scale to 5%–1600%. The default is center-anchored; `toggle_zoom_at_cursor` applies the source pan adjustment that keeps the cursor's image point stationary. Left-button drag pans by pointer delta. Fit uses the image/viewport ratio capped at 100%, as in the source preview.
- `snapshot()` returns current source path, transforms, fit/zoom settings, info visibility, and a revision counter.
- `display_path()` returns the original path when untransformed and otherwise returns a cached PNG copy with rotation and flips applied. It never writes the source media.

## Consumers and debugging

- A caller should invoke `PreviewService::open` when an image is selected for preview. The Gallery currently opens image previews through this service.
- Decode or cache write failures are returned by `display_path` as `PreviewError` and shown in the GPUI image area. Image orientation metadata is not reapplied in this transform path. The current `open(path)` contract has no database media record or application state, so the source panel's assigned tags, engagement/open count, indexed `date_taken`, video probe details, and source-themed modal framing are unavailable; the panel displays the metadata safely available from that path instead. The source preview's keyboard navigation, neighbor decode cache, progressive loading, smoothed zoom animation, toolbar exclusion during wheel zoom, auto-hide chrome, and filmstrip remain incomplete. Viewport dimensions are estimated from the window bounds and should be verified in the integrated Windows layout.
- `photag-plugin-thumbnails::thumbs` remains the source-compatible thumbnail generator; it is intentionally not used for full-size preview decode.
