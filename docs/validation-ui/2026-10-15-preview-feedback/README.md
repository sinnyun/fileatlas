# Preview refresh feedback and displayed-image latency — 2026-10-15

## Cause and correction

The screenshot's accessibility messages were observations of actual repeated
paints. `AiSuggestionsView` notified every 150 ms even with no work. It now
notifies only on identity, request, task/result or error transitions. Settled
wheel zoom also stops changing revision and saving sessions. The Debug drawer
keeps every accessibility record, but renderer observation records do not
advance its own content invalidation clock. Other actions/errors still update
the live drawer; closing or pausing it stops log-driven repainting.

The image stage previously did not consume its progressive cache correctly;
the cache also expired after 500 ms regardless of full image readiness. It now
selects ready native full data, then progressive data, then the existing FileManager
thumbnail. Replacement follows asset readiness. Current-image work precedes
queued filmstrip work, JPEG fast decoding uses DCT scaling, and neighboring
full-size previews are no longer speculatively generated. Ordinary EXIF-oriented
images use the native loader's orientation rather than generating another
full-size oriented PNG. Explicit user transformations remain asynchronous.

Development builds now optimize `image`, `jpeg-decoder`, `zune-jpeg` and
GPUI's pixel conversion; application crates remain incremental debug builds.
The normal `target/debug/fileatlas-shell.exe` was rebuilt and is the final tested
executable. Its SHA-256 is in [summary.json](../../../.local/validation/artifacts/2026-10-15-preview-feedback/summary.json).

## Environment and commands

Windows 11 Pro 10.0.26200, Windows MSVC Rust 1.97.1, GPUI 0.3.8,
150% display scale; native screenshot is 1942×1256. Read the baseline
`sidebar-pages-preview-2026-10-14-v2` and `final-v5` directions; retained their
layout and current floating Preview/Debug composition. No new designed control
is omitted by this performance fix. Only the isolated `target` profile was
modified. Graph discovery was attempted but returned `Transport closed`, so
source discovery fell back to local reads/searches.

```powershell
rustup toolchain list
rustc +1.97.1 --version
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-preview -p fileatlas-plugin-debug -p fileatlas-plugin-file-manager -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-preview-feedback-final --plan target/preview-feedback-final-verified.json --allow-mutations
```

All static checks/build passed; Preview 5, Debug 6 and FileManager 19 tests passed.
Regressions cover unchanged AI polls/empty channels, settled zoom, retained
renderer logs without feedback, decode priority/nonexpiring progressive data,
JPEG scaling and ignoring stale paint generations.

## Fixture preparation and runtime evidence

Two synthetic RGB fixtures were generated with Python/Pillow/NumPy, RNG seed
20261015: `cold-24mp.jpg`, 6000×4000, JPEG quality 90, 21,513,628 bytes;
`cold-8mp.tiff`, 3200×2400, 23,040,140 bytes. They contain random pixels, not
user photographs. An initial isolated `dev_test_scan_fixture` indexed both
using `FILEATLAS_TEST_LIBRARY=F:/AI/geren/pho_cordis/target/preview-latency-fixtures`.
Settings suppress startup updates. Before the final launch the isolated
session's `preview_path` was cleared and selection set empty, preventing startup
restoration from warming the native full-image cache. FileManager thumbnails and
on-disk progressive caches already exist, as in normal indexed browsing.

The [final plan](final-plan.json) and [19-step report](../../../.local/validation/artifacts/2026-10-15-preview-feedback/final-report.json) use
the project MCP server and GPUI window in the same child process. The server
activates the native accessibility provider using WM_GETOBJECT on its own
verified HWND; the report retains real `Accessibility activated` and subsequent
`Sending a11y tree update` messages. Debug stays open and following latest.
Double-click down/up events are dispatched through the Shell's GPUI pointer
queue, at logical 525,230 for JPEG and 360,230 for TIFF. No OS pointer injection
or separately launched application was used. A previous attempt with separate
single/double click sequences and shorter waits did not open the preview;
the final explicit double-click sequence succeeded for JPEG, TIFF and reopening.

| Case | First image paint | Full-resolution paint |
| --- | ---: | ---: |
| JPEG, first full asset load in final process | 31.95 ms, FileManager thumbnail | 400.30 ms |
| TIFF, first full asset load in final process | 26.18 ms, FileManager thumbnail | 74.58 ms |
| JPEG, reopen in same process | 20.24 ms, full image | 20.24 ms |
| JPEG, earlier unoptimized development build | 64.02 ms, progressive cache | 9806.41 ms |

The slow baseline is preserved in [unoptimized-report.json](../../../.local/validation/artifacts/2026-10-15-preview-feedback/unoptimized-report.json)
and [its plan](unoptimized-plan.json). Its original executable was rebuilt during
optimization and is not an independently retained baseline binary. The baseline
already included the paint instrumentation/progressive corrections, so it
isolates remaining decoder cost rather than representing all previous defects.

Final report steps 7/8 are separated by five seconds after zoom to 150% and an
eight-second settling period: both show `render_count=37`, identical
`poll_wakes={chrome:14,decode:1,progressive:1,state:2}`. Steps 9/10, another five
seconds apart, both show `logs.latest_id=167`, `total=167`, `view_revision=[1,123]`.
Thus neither image redraw nor a11y log collection keeps growing while idle.
These checks were asserted against the actual report during evidence export.

The first/full markers run after the decoded image's draw command during GPUI
paint, not when an open handler returns. They do not notify another render.
This measures CPU paint submission readiness, not a GPU fence or physical panel
scanout. The previous 3.84 ms open-handler measurement was not image latency.
Filesystem cache was not flushed; these are local single-run measurements,
not promises for RAW, corrupt media, network drives, all codecs or every DPI.
AI model download/inference and all transform combinations were not exercised.
No claims about those paths are added here.

![Final native window: full JPEG with Debug still expanded](../../../.local/validation/artifacts/2026-10-15-preview-feedback/cold-jpeg-full.png)
