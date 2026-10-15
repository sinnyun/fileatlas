# Preview plugin ports

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status: S5 reflows the full-window image preview around a branded top bar, right-aligned metadata card, lower transform/tag/file toolbar, and image filmstrip with previous/next navigation. The stage uses the solid `SURFACE_EXTREME` token (audio uses its neutral audio surface). Information values render as compact label/value rows with collapsible File, Library, Image/Video, and EXIF groups. The Gallery opens selected media with its database identity. The AI suggestions surface polls local inference, reads live AI settings, and supports assigning/removing suggested tags. Source wheel multiplier, smooth zoom animation, fit/preset/continuous slider zoom, cursor-anchored zoom, drag pan, auto-hiding preview chrome, and the information overlay remain backed by existing service and session state. The AI top-bar action queues the existing request, while its result surface remains in the AI contribution slot. Integrated Windows visual and pointer validation is pending the required PhoTag MCP run.

## Lifecycle and dependencies

- Cordis plugin name: `photag.preview`; service: `photag/preview` (`PreviewService`).
- The plugin owns its `preview/image` and `preview/ai-suggestions` panel contributions and both view factories through `photag-ui-bridge`; Cordis disposal removes them.
- Requires `SessionService` and `DatabaseService` at startup to restore/save the last preview source path, show indexed metadata, and increment `open_count` once per newly opened media ID. Configure the transform cache with `PreviewConfig.cache_dir`; default is `%LOCALAPPDATA%/PhoTag/preview` (system temp directory when unavailable).
- Also requires `AiService`, `SettingsService`, `TaggingService`, and `HistoryService`. Shell starts Preview after all four providers; Gallery mode enables Tagging and History so the preview can apply and undo suggestions.

## Input / output

- `open(path)` selects a source and resets its view; `open_media(path, media_id)` additionally carries the indexed row identity in `PreviewSnapshot::media_id` for consumers such as AI suggestions. `request_ai_suggestions(media_id)` queues a one-shot explicit request in the snapshot; the AI surface consumes it through `take_ai_suggestion_request()` and runs the same request path as its manual Analyze control, including downloading a missing model. `close()` clears current selection. The Info button toggles a scrollable overlay adapted from `vendor/photag-source/src/ui/preview.rs` and `src/ui/info.rs`.
- The `preview/ai-suggestions` surface reads a fresh AI settings snapshot on request and automatic image changes. Auto-analysis runs only when enabled and the model is already present; manual Analyze downloads a missing model before inference. It shows worker results/errors and routes Apply/Remove actions through `TaggingService`, which creates unknown labels under `AI Suggested` and synchronizes metadata using its standard path. A successful Apply pushes `UndoEntry::AssignedTags` with the exact direct label IDs after the DB mutation succeeds, matching the source AI action; failed DB mutations create no history entry. Video and audio extensions are excluded.
- A result is scoped to `(source path, media ID)`. Changing preview discards the receiver and visibly reports that the result is abandoned; the current AI engine has no cancellation token, so the detached inference worker can finish in the background. Automatic analysis attempts each image once per panel lifetime, including errors, matching the source's no-repeat auto-trigger; manual Analyze can retry.
- The panel reads filesystem name/path/type/extension/size/read-only/created/modified facts, indexed `date_taken`, assigned tag names, `open_count`, image dimensions/aspect/megapixels, and the source's curated plus complete EXIF fields. For video extensions it invokes the source-compatible `ffprobe` lookup and shows dimensions, duration, frame rate, video codec, bitrate, and audio codec. Probe results are cached per path and file modification time. Byte/date/aspect formatting follows the source conventions. EXIF dates remain in the decoded EXIF values; indexed and filesystem timestamps use local time.
- `rotate_clockwise`, `rotate_counter_clockwise`, `flip_horizontal`, and `flip_vertical` adjust non-destructive preview transforms. `set_fit` restores fit mode; `set_zoom_percent` accepts the source presets 25, 50, 100, 125, 150, and 200, while `set_zoom_scale` stores a continuous 5%–1600% value for the v5 toolbar slider. `toggle_info` controls the information panel.
- GPUI wheel input reuses PhoTag's `old_zoom * (1 + scroll * 0.0025)` calculation and clamps the continuous scale to 5%–1600%. Wheel updates ease zoom and cursor-anchored pan toward their targets with source exponential smoothing; animation advances on the preview's 150 ms refresh loop and saves the settled transform once. The default is center-anchored; `toggle_zoom_at_cursor` applies the source pan adjustment that keeps the cursor's image point stationary. Left-button drag pans by pointer delta. The wheel handler is attached to the image hit area; the overlaid toolbar owns its own wheel event, so scrolling controls cannot zoom the image. Fit uses the image/viewport ratio capped at 100%, as in the source preview.
- `snapshot()` returns current source path, transforms, fit/zoom settings, info visibility, and a revision counter. Pointer motion, clicks, scrolling, and focused key input keep preview chrome visible. After two seconds idle it fades over 0.8 seconds; the toolbar and filmstrip are removed from hit-testing when fully hidden and return on the next pointer or key activity.
- The Preview toolbar is centered near the bottom of the stage. Its drag handle applies transient pointer deltas to the stored center-bottom offsets and writes the final offsets to `SettingsService` on mouse release. A locked toolbar does not start a drag; effective offsets are clamped to the current viewport after resize and persist when the user next drags. Persistence failures are shown on the surface. Settings also provides lock and reset controls. Preview toolbar visibility follows the persisted zoom, transform, tags, file, and view button preferences. File actions are grouped behind More. Toolbar quick actions open the indexed file in its default application or reveal it in the file manager; both run through `FileOperationsService` and report failures in the preview surface.
- `PreviewPlugin` reopens `session.json`'s `preview_path` when the image still exists and clears the saved path when that file is missing. Opening and closing updates that field. Transform, zoom, pan, zoom-anchor mode, and information visibility are saved in backward-compatible optional session fields and restored for the matching path. Keyboard `F` fits, `+`/`-` steps through source zoom presets, number-row `0`–`5` rates the current indexed item, configured F1–F9 toggles that item's assigned tag, arrows navigate the image/audio filmstrip, Escape closes, `I` toggles info, and `R` rotates clockwise. For video previews, Space toggles playback, Left/Right seek by five seconds, and Escape closes the preview, following `src/video.rs`. Rating changes update SQLite, record `UndoEntry::RatingChanged` when the previous value is available, and emit `MediaIndexChanged`; tag changes use `TaggingService`, record history when available, and emit the same event. These shortcuts are ignored while the tag search input or rename input has focus.
- The Tags toolbar button opens a searchable hierarchical picker for the active indexed item. Collapsed parent IDs restore from and save to `SessionService.collapsed_dropdown_tag_ids`; a nonempty search expands matching descendants regardless of the saved collapsed state. With an empty search it also shows up to six recently assigned tags. Each choice toggles assignment, and assigned tags have direct remove actions. Recent IDs are shared across Gallery and Preview through Tagging and capped at eight, matching source app state. Mutations use `TaggingService`, so associations, metadata rewrites, history, and Gallery refresh follow the same shared paths. Right-clicking the image stage opens a source-style action menu with Open externally, Show in folder, explicit AI Suggest Tags, 0–5 rating, and confirmed Delete actions; AI and rating items are omitted when the preview has no media ID.
- Preview requires `FileOperationsService` and routes F2 or the Rename control through its source-compatible rename path. The service updates the matching preview path in place, preserving transform state, media identity, and filmstrip navigation. Delete/Backspace and the Delete control show a native confirmation before calling `delete_to_recycle_bin`; successful row removal advances to the next supplied filmstrip item, or the previous item when deleting the last one, and closes when no neighbor remains. It retains each neighbor's media ID so the AI surface remains scoped correctly. `MediaIndexChanged` triggers the existing Gallery refresh. Recycle Bin errors are shown even when the source row was removed. If event delivery fails after a mutation, the action reads the row again before repairing the preview.
- `set_filmstrip(paths)` supplies ordered paths; `set_filmstrip_media((path, media_id))` retains row identity. Gallery supplies the filtered query's complete ordered media sequence (images, videos, and audio), so Left/Right navigation follows source order across media kinds. The panel displays up to seven nearby thumbnails and supports previous/next navigation with the arrow keys while focused. Nearby 96 px filmstrip images and 1600 px preview frames are generated through a two-worker background queue under the preview cache.
- The native stage selects ready image data in order: full raster, 1600 px progressive image, Gallery thumbnail. `open_indexed_media(&MediaItem)` supplies that thumbnail and indexed identity. The progressive image remains available until the full native asset is ready; it does not expire based on file age. The active fast decode has queue priority, JPEG uses DCT scaling, and neighbors no longer start speculative 1600 px decodes that compete with the current original. Ready-cache lookups are memory-only. Worker file-cache checks validate source modification time; the queue stops when the service is dropped.
- `display_path()` returns the original path when no user transform is set: GPUI's native decoder already applies EXIF orientation, so another oriented full-size PNG is unnecessary. With user rotation/flips it returns a cached PNG after EXIF and user transforms. The UI materializes these copies asynchronously and never writes the source media.

- `load_status() -> serde_json::Value` exposes per-open generation/source/media ID, first-frame and full-frame paint timings, first/current quality, dimensions, error, render count and poll-wake reasons. Timings begin at `open_media` and are completed by a marker after the ready image's paint command; they are separate from the open-method span and do not claim a display scanout/GPU fence. Late paint callbacks from old generations are ignored. Diagnostics do not invalidate the view.
- The AI contribution's 150 ms polling only calls `cx.notify()` when the media key or worker state/result changes. Empty channels and unchanged selections do not repaint the app. Settled wheel zoom also stops incrementing revision and saving session state. Regression tests cover both idle paths.

## Consumers and debugging

- A caller should invoke `PreviewService::open_media` when an indexed item is selected for preview, so per-image AI actions retain the media ID; `open` remains available to path-only callers. The Gallery opens previews through this service.
- Full-size and progressive image decodes apply decoder-provided EXIF orientation before user rotate/flip transforms, matching PhoTag's preview image path. The preview surface renders images and a dark audio stage with filename, rating controls, tags, and an Open in default player action routed through `FileOperationsService`. Video items in the same ordered sequence open through `MediaPlaybackService`; moving away from video closes its playback surface. When FFprobe cannot inspect a video, Preview launches its path with the OS default handler and closes its in-app preview, matching `VideoPlayer::open`. The current right-click action panel uses the GPUI surface layout rather than reproducing the source's floating egui menu placement. Viewport dimensions are estimated from the window bounds and should be verified in the integrated Windows layout. `ffprobe` metadata remains unverified on Windows deployment layouts.
- `photag-plugin-thumbnails::thumbs` remains the source-compatible thumbnail generator; it is intentionally not used for full-size preview decode.

## Shared visual tokens

The preview information panel and status messages use photag-ui-bridge::theme for source modal/base surfaces, accent headings, secondary text, and danger text. The live validation verified the full-window dim overlay, centered image, and info panel with a generated PNG; other settings, devices, codecs, metadata corpora, and interaction states remain unverified.

## Code map

- `src/view.rs` defines the Preview view state and contribution factory.
- `src/view/render.rs` implements the Preview GPUI layout and stage; `src/view/actions.rs`, `context_menu.rs`, `keyboard.rs`, and `tag_picker.rs` own their respective action, input, and menu surfaces.
- `src/view/support.rs` contains shared button and toolbar-position helpers; metadata rendering is in `src/info.rs`.


## Localization integration (2026-10-13)

Visible control labels consume the public UI Bridge translator supplied by
photag.i18n. Stable commands, source settings/schema values, media paths and
user-created tag data are unchanged. Missing translations fall back to English;
OS/backend errors and some generated diagnostics are not fully localized.
The real MCP-only validation coverage and remaining source layout differences
are recorded in docs/validation-report.md; screenshots do not prove pointer
interaction or animation parity.

## v5 design 01 / 07 presentation (2026-10-14)

The image view uses an opaque stage, an opaque full-width toolbar anchored immediately above
the optional 100px filmstrip, filename captions and a controlled GPUI Kit More popover. Stored
custom toolbar offsets and lock/reset controls remain compatible. The information card uses
real indexed filename/rating/open count/tags and filesystem/image metadata; ratings call the
existing preview rating handler. EXIF and path sections start collapsed. More/Tags/filmstrip
can be requested through the project MCP; see its PORTS. Pointer hit propagation and dragging
are separate from typed-request/state/screenshot coverage.

## 2026-10-14 current implementation and MCP evidence

Reference02 rendered with real image info/ratings, fixed opaque toolbar, filenames in filmstrip, controlled More/Tags popovers. MCP exercised zoom/rotation/reset/filmstrip showhide/info; native file dialogs, toolbar drag and per-key keyboard behavior unverified. See current redraw evidence in docs/redesign-validation.md.

## Shared floating frame (2026-10-14)

Image preview content is mounted in the UI Bridge frame ID preview, default1100×760, with drag/resize/maximize/restore/close. Existing PreviewService ownership, transforms, filmstrip, tag/file/AI controls remain unchanged. `panels::content_bounds` reads measured GPUI content geometry; fit sizing uses that area, cursor zoom and right-click menu coordinates subtract its actual origin. Earlier whole-window viewport estimates above are historical for this path. Preview and Playback services still hold one active media each; the generic panel catalog does not imply multi-session decoding.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.

## Render-time reads and preview opening (2026-10-15)

Gallery double-click supplies its already loaded media row to Preview. The
current 200-row indexed filmstrip appears immediately; the full filtered path
order is resolved off-thread and applied only while that media remains active.
Indexed paths skip synchronous per-file existence checks. Preview image
dimensions, filesystem metadata, EXIF/video details and indexed information
load asynchronously and cache by path/media ID. EXIF orientation is cached for
the active source. The tags popover loads
its tag tree and assignments on a background executor only while open; toolbar
assignment count uses the indexed-info cache. Oriented/rotated image cache
materialization is also keyed and asynchronous; the original remains visible
until the processed raster is ready. These changes keep database and file reads
outside repeated paint work; see `docs/ui-render-performance-audit-2026-10-15.md`.

## Stationary pointer and activity diagnostics (2026-10-15)

Preview's stage/root coalesce mouse-move coordinates before restarting chrome
activity. Repeated stationary observations do not keep showing/fading controls;
real coordinate changes and explicit click/key/service activity still wake
them. Zero-distance pan observations do not change the model. `load_status()`
adds `activity {count, caller:[file,line]?, pointer:[x,y]?}` alongside poll_wakes.
These are memory-only diagnostics, do not emit per-frame logs or notify views,
and distinguish input-driven animation from periodic invalidation in MCP tests.

## Shared read and refresh ownership (2026-10-15)

Preview/AI timers use UI Bridge refresh::poll; revision/decode/progressive/chrome/pointer changes use shared gates. Preview's seven passive reads now use ReadModel, including cached failures and latest-request coalescing. The model timer delivers completions; render may declare a nonblocking demand. Tag mutations reset their read model; errors are visible in the tag popup. Original raster remains visible while transformed data loads. Native paint/latency diagnostics remain independent of notify/logging.

## Per-file decode diagnostics (2026-10-15)

Background queue jobs now have correlated `preview decode started` / `preview
decode finished` records with a process-monotonic job_id, source/cache paths,
extension-based format, file_bytes, role (filmstrip_thumbnail / progressive_preview),
max_dim, enqueue priority, cached/written/status and failure stage/error. Completion
records expose source/output dimensions, decoder (jpeg_dct/generic/cache_hit) and
JPEG fallback error. Each real job logs start/result once; no timer/render-time logging.

Durations in milliseconds (rounded to0.001ms): queue_ms (enqueue to worker start),
cache_check_ms (metadata freshness), read_ms (compressed bytes), header_ms,
decode_ms, orientation_ms (includes EXIF lookup/application), resize_ms,
write_ms (JPEG encoding, cache file write and publication), duration_ms (entire
worker execution, excludes queue) and total_ms (queue+worker). Stage sums may
exclude small worker bookkeeping overhead. Cache hits skip decode stages and
report unknown dimensions as None, avoiding new IO merely for logging. JPEG
fallback stage timings accumulate both attempted decoders. Generic formats read
compressed bytes once into memory so disk-read timing is separate from decoding;
orientation and decoded pixel semantics are preserved. This adds a compressed
input buffer alongside the existing decoded image for generic formats.

Read/decode/cache-write failures produce ERROR with the actual reason/stage;
failed jobs still clear pending state. Shared temp-file publication semantics
retain an existing completed target on rename races. Failed temporary writes
are cleaned up. First/full paint records add source, generation, painted
 dimensions and duration_scope=open_to_paint; this differs from worker durations.
Existing raw message limits and click-to-open scrollable debug details apply.
No schema/settings/service identity changes. Evidence and field guide:
docs/validation-ui/2026-10-15-decode-details/README.md.

## Demand-driven decode and shared Gallery thumbnails (2026-10-15)

`FilmstripMedia { path: PathBuf, media_id: i64, thumbnail: Option<PathBuf> }`
and `set_indexed_filmstrip_items(impl IntoIterator<Item=FilmstripMedia>)` carry
the existing database thumbnail with each ordered identity. Gallery's initial
loaded window and background full-result handoff, and MCP `open_media`, use
this public contract. Older path/ID setters remain compatible. Paths are not
stat'ed synchronously by the indexed setter. Render uses memory-only lookups.

Workers validate supplied thumbnail mtime against the source and reuse it
without original-image reads, decoding, resizing or writing another cache.
Missing/older thumbnails fall back to the existing 96px generation path;
completion replaces the displayed candidate. The bounded seven-image
neighborhood remains. A fresh but corrupt thumbnail is not detected by this
metadata check; native image loading retains its error behavior.

The former unconditional 1600px work is replaced: only JPEG/JPEG sources that
remain incomplete after 150ms qualify, sampled by the existing shared poller
(actual dispatch can be later). PNG and other full-decode formats go directly
from the existing thumbnail to the native full raster. Each open generation
permits at most one progressive attempt, including failures. Native full paint
cancels queued progressive work; switch/close cancels all old request work.
Workers check tokens before decoding, before encoding and before publication.
An already running decoder/encoder cannot be forcibly interrupted. Old
generation completions cannot remove a new generation's pending key or publish
an obsolete result. Plugin stop closes the existing workers; no new scheduler.

Completion logs add `generation`, `reused`, `cancellation_reason`, and decoder
`gallery_thumbnail`; cancelled work has status=cancelled. A queued job discarded
before worker start instead emits `preview queued decode cancelled`. Skipped
decode stages are zero, with unknown dimensions as None. `load_status()` adds
`cache_policy { progressive: "jpeg_if_pending_after_150ms", nearby:
[{source,indexed_thumbnail,display_thumbnail}], pending }`, all memory-only.
Existing service IDs, Cordis events, SQLite schema and user cache paths remain.
Tests cover cached reuse, stale/missing fallback, request ownership and finite
failed attempts. Live timing/coverage: docs/validation-ui/2026-10-15-preview-optimization/README.md.


## Browser runtime update — 2026-10-15

Browser files are pinned into the session cache by path for existing preview/filmstrip contracts; no untagged row is persisted. File revision includes path, size and nanosecond mtime. `refresh_local_source` on watcher change closes missing sources or reopens changed sources. Property/EXIF/dimensions and original GPUI assets invalidate with file revision; progressive cache filenames include it. Informational open counts update session cache. A same-path PNG overwrite 60x30 -> 100x80 was exercised with actual full-frame presentation diagnostics. Existing priority, cancellation, seven-item neighborhood and transform behavior are retained.
