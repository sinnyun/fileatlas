# UI render-time I/O audit (2026-10-15)

## Finding

The repeated `database/with_reader` records were genuine work triggered by
redraws, not a logging-only issue. GPUI render functions and helpers were
performing database reads for selected media, tags, and preview details. A
virtualized card list can rebuild during selection, scrolling, animation, or
window invalidation, so the same read ran repeatedly while the user did
nothing. Preview also synchronously resolved every file_manager ID for the
filmstrip, checked each path, read image dimensions, and reopened the source to
read EXIF orientation during rendering. These operations explain both log
growth and delayed preview refresh.

## Project-wide audit

The Rust code graph contains 71 direct database reader/writer call sites across
feature crates. Most are in service methods invoked by commands, event handlers,
background indexing, file operations, or navigation actions; their presence
alone is not evidence of a redraw loop. The UI source audit then followed
service calls from `Render::render` and its render helpers.

| Surface | Render-time work found | Result |
| --- | --- | --- |
| FileManager cards and right Inspector | Multi-selection path resolution, off-window Inspector media/tag lookup, and selection-detail filesystem metadata/open-count reads | Inspector loads once asynchronously by selected ID; drag paths are resolved once and cached by selected IDs. Selection-detail metadata and open count share a keyed background read. |
| FileManager tag picker, bulk tag filter and media action dialog | Tag tree, selected row and assigned-tag queries; media action dialog re-read its row on every paint | Render helpers now consume the loaded snapshot/Inspector fallback; action dialog reuses the row loaded when opened. |
| Preview image stage and information panel | Image dimensions, file metadata, EXIF fields, indexed media/tags/open count, video probe, and EXIF orientation could be read during render | Details load asynchronously and cache by path/media ID. Native asset decoding applies EXIF once; non-identity user transforms run in a keyed background task. |
| Preview toolbar and tag picker | Assigned-tag count, tag tree, and assignment reads on each render | Toolbar uses the async indexed-info cache; popup loads tree and assignments together off-thread and caches by media ID. |
| Media Playback tag panel | Tag tree and assigned IDs on each render | Reads once in a background task and caches until media identity or assignments change. |
| Preview AI contribution | Every 150 ms poll unconditionally notified, even without analysis work | Poll now reports whether key/task/result changed; unchanged/empty-channel polls cause no redraw. |
| Debug drawer and settled zoom | Accessibility logs drove the logger's own redraw; settled animation changed revision on every poll | Collection and presentation clocks are independent, and settled animations no longer change revision or write session. |
| Settings Shortcuts/AI pages | Navigation snapshot queried from paint to flatten tag choices | Shared background choices keyed by tag revision, updated while open and on reopen; no paint-time query. |
| FileManager statistics / bulk matching | Statistics/audio and matching queries ran during every paint; Debug's legitimate DB records closed a feedback loop | Shared keyed ReadModel snapshots on the model poller; index plus writer revision refresh data after mutations; execute retains fresh revalidation. |
| Inspector expanded EXIF | Selected path was queried before testing the EXIF result cache on every paint | Keyed background EXIF/path read; loaded/fallback path reused, errors cached and stale selection results discarded. |
| Preview mouse activity | Stage and root reset fade for repeated identical platform pointer observations | Coalesce coordinates; a stationary pointer and zero-distance pan do not reset activity or revision. Explicit moves/clicks/keys still wake controls. |
| Settings update monitoring | Every pending-task poll repainted even without new data; disconnected channel could stay pending | Notify on progress/result only; disconnect becomes terminal error. Completed/error checks settle in MCP; download/install not exercised. |
| Navigation tag associations, FileManager keyboard and file actions, Preview mutation handlers | Reads occur after explicit user actions (tag edit, shortcut, rename/delete/open), not during passive render | Kept as action-time operations; they are not redraw loops. |
| Grouping and indexing | Filesystem metadata and database work occurs while constructing/querying the model, outside the paint function | No render loop found in those paths. |

The search covered the active feature crates' `src/view` code and service
methods reached from render helpers. Test fixtures, startup migration, worker
loops, and explicit action callbacks were classified separately. A source
search is not proof against future indirect calls; the prevention rule below
applies to all current and new feature views.

The initial source audit missed Settings and Statistics/Bulk's paint-time
queries. Those were subsequently reproduced by the full live MCP matrix;
the table above includes the corrections. Graph results are a discovery aid,
not complete coverage proof. The live suite now fails for sustained/blocked
observations and checks real cache freshness and error recovery as well as
absence of work.

## Shared repair rule

Treat `Render::render` and every function called only to construct elements as
pure projection: they may read immutable view snapshots, local presentation
state, and caches. They must not query a database, inspect media files, decode
images, enumerate directories, mutate a service, or persist session state.

When an identity or user action changes, start the required work once on a
background executor. Store the pending identity to coalesce duplicate requests,
cache both success and missing/error results under the identity plus relevant
revision, and notify the view when the result arrives. Ignore stale completions
whose identity no longer matches. Mutations invalidate only the affected cache
and then emit the existing domain event. Prefer the owning service's existing
snapshot/window over issuing another query.

This rule fixes the work rather than hiding its tracing records. Database spans
remain visible so any future violation is diagnosable. Review new UI code by
tracing each service call out of render helpers and checking that the operation
is event-driven, cached, and off the UI thread when it can block.

Follow-up: an I/O-only audit does not cover unconditional timer invalidation.
Every poller must return/change a revision only for a model transition; an
animation must stop changing the model after it settles. An observability
counter may record paint timing but must not trigger another paint. Never use
the duration of an open command as image-load latency: separately measure a
first image paint and full-resolution paint. Debug/render telemetry must remain
queryable without feeding the diagnostic view's own invalidation clock. Unit
regressions cover idle AI polls, settled zoom, retained accessibility logs,
decode priority, progressive-cache lifetime, and stale paint generations.

## Preview open path

FileManager now opens a double-clicked item from the already-loaded `MediaItem`,
immediately sets a filmstrip from the current 200-row media window, and
resolves the complete filtered strip through `spawn_blocking`. A late result
only replaces the strip if the same media is still open. The indexed strip
setter skips synchronous per-path existence checks; opening a missing item is
still checked by `PreviewService::open_media`. FileManager selection details resolve
filesystem timestamps/permissions and open count once off-thread. Preview image
dimensions, file metadata, EXIF/video details, and indexed info no longer block
the paint function. The native GPUI asset loader applies EXIF orientation;
ordinary images no longer require a second oriented full-size PNG.

When a restored/user transform is non-identity, its transformed cache file is
now materialized in a keyed background task. The original raster remains
visible until the transformed result is ready; stale transform results are
discarded when source or transform identity changes. Live latency evidence is
recorded in `docs/redesign-validation.md` after same-process MCP verification.

## Follow-up runtime evidence

The initial 3.84 ms `preview_media` span measured handler dispatch only. It did
not establish image readiness. A 6000×4000 JPEG exposed a 9806 ms full paint in
the unoptimized development decoder. The final standard executable paints its
existing FileManager thumbnail at 31.95 ms and full image at 400.30 ms; reopening
within that process takes 20.24 ms. A 3200×2400 TIFF paints thumbnail/full at
26.18/74.58 ms. These are single local fixture runs, not codec-wide benchmarks.

With native accessibility activated and Debug open, five-second idle samples
hold Preview render count at 37 and Debug latest ID/total at 167. Zoom is 150%,
with the animation and chrome fade settled. The earlier AI timer was the
remaining periodic invalidation source; removing I/O alone did not fix it.
See `validation-ui/2026-10-15-preview-feedback/README.md` for exact commands,
reports, sample details, retained accessibility records and measurement limits.

The subsequent complete-state idle audit and additional pointer/writer/observer
checks are recorded in `validation-ui/2026-10-15-idle-audit/README.md`.

## Unified caller contract after the live audit

All GPUI timer loops now use `fileatlas_ui_bridge::refresh::poll`; shared
`ChangeGate` performs signature detection. Shared `ReadModel` replaces the
remaining per-view passive metadata/tag/selection worker templates in Preview,
Playback and FileManager. Render may declare a nonblocking demand, never executes
IO; model pollers deliver completions. PollResult separates actual change from
finite-operation liveness. This also fixes unchanged scan/developer progress
redraws while preserving active transport/animation updates. The new architecture
guard runs in CI. See [UI refresh contract](ui-refresh-contract.md) for signatures,
keys and error/stale-result requirements, and the final same-process idle evidence
for what was actually exercised. Future semantic key choices still belong to
feature owners; the common helper is not proof of every possible state transition.
