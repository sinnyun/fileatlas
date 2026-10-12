# Preview plugin ports

Status: image preview service and a native GPUI preview surface are implemented. Gallery selection wiring and source visual parity remain incomplete.

## Lifecycle and dependencies

- Cordis plugin name: `photag.preview`; service: `photag/preview` (`PreviewService`).
- The plugin owns its `preview/image` panel contribution and `preview/image-view` factory through `photag-ui-bridge`; Cordis disposal removes both.
- No startup service dependencies. Configure the transform cache with `PreviewConfig.cache_dir`; default is `%LOCALAPPDATA%/PhoTag/preview` (system temp directory when unavailable).

## Input / output

- `open(path)` selects an existing image and resets transforms to the initial fit state. `close()` clears current selection.
- `rotate_clockwise`, `rotate_counter_clockwise`, `flip_horizontal`, and `flip_vertical` adjust non-destructive preview transforms. `set_fit` restores fit mode; `set_zoom_percent` accepts the source presets 25, 50, 100, 125, 150, and 200. `toggle_info` controls the info strip.
- `snapshot()` returns current source path, transforms, fit/zoom settings, info visibility, and a revision counter.
- `display_path()` returns the original path when untransformed and otherwise returns a cached PNG copy with rotation and flips applied. It never writes the source media.

## Consumers and debugging

- A caller should invoke `PreviewService::open` when an image is selected for preview. The gallery does not yet publish this selection, so the view currently shows its empty state until another feature wires that call.
- Decode or cache write failures are returned by `display_path` as `PreviewError`; the GPUI surface currently leaves the image area empty on those failures. Image orientation metadata is not reapplied in this transform path, and the source preview's keyboard navigation, neighbor decode cache, progressive loading, pan/wheel interaction, auto-hide chrome, filmstrip, and info metadata panel have not been migrated.
- `photag-plugin-thumbnails::thumbs` remains the source-compatible thumbnail generator; it is intentionally not used for full-size preview decode.
