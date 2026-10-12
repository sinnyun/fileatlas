# Thumbnail plugin ports / 缩略图数据对接

Status: original image thumbnail pipeline is wrapped as a service and cache reuse has a synthetic-image regression test. Full format and Windows parity are not established.

## Lifecycle and dependency

- Cordis plugin name: `photag.thumbnails`; service: `photag/thumbnails` (`ThumbnailService`).
- No Cordis service dependency. `ThumbnailConfig.directory = None` selects `%LOCALAPPDATA%\PhoTag\thumbs`; a path override is available for isolated use.

## Input / output

- Request: `generate_image(source: AsRef<Path>)`.
- Result: `Option<(cache_path: String, width: u32, height: u32, micro_thumb: Option<Vec<u8>>)>`; `None` means generation could not produce an image result. A cache hit reports `(0, 0)` dimensions per the retained implementation and may still return micro-thumbnail bytes.
- `directory()` returns the cache root. Cache filenames and image processing remain source-compatible with PhoTag.

## Consumers and debugging

Indexing calls this service to generate image caches and fills missing thumbnail rows. Do not call synchronously from GPUI render. For missing thumbnails inspect source path accessibility, decoder result, cache directory permissions, and the indexed `thumb_path`; do not infer video decoding success from this image API.
