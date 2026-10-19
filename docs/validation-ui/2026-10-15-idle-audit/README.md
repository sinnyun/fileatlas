# Full live idle audit — 2026-10-15

## Findings and repairs

The first actual same-process MCP run exercised 81 states and found 8 sustained
states that the previous source audit had missed. Their four shared causes
were Settings Shortcuts/AI reading Navigation snapshots during paint and Gallery
Statistics/Bulk matching executing queries during paint. The live Debug drawer
correctly collected those DB records, causing the next paint to read again.

| Initial reproduction | New records in a second 4 s observation | DB records |
| --- | ---: | ---: |
| Settings Shortcuts | 45 | 30 |
| Settings AI | 48 | 32 |
| Statistics | 112 | 64 |
| Bulk, empty library | 80 | 32 |
| Bulk, populated library | 80 | 32 |
| Bulk copy / move / delete, current selection | 48 / 47 / 48 | 32 / 32 / 32 |

These reads now use the shared `photag_ui_bridge::read_model::ReadModel` from
model pollers. The utility serializes work, coalesces edits to the latest key,
ignores stale results and caches errors as well as successes. A repeated empty
channel poll returns false. Render consumes the current result only. Navigation
publishes a tag revision so choices update while Settings remains open. Gallery
uses index events plus a new memory-only Database writer revision so direct
service writes also refresh Statistics/Bulk without reopening the dialog.
Writer failures conservatively invalidate too, since partial writes can exist.
Bulk execution still freshly revalidates its target set and delete confirmation.

The deeper Inspector section checks then found an additional cause: EXIF
looked up the selected media path before checking its cached EXIF result.
An expanded EXIF section therefore kept querying on every paint, even when the
fields were already cached. Its extended observation added 57 records in 5 s.
It now has a keyed background read model too, using the loaded/fallback path
when available and otherwise resolving it once off-thread. Empty/error results
are cached, and selection/index/writer identity rejects stale completions.

Longer Preview tests found duplicate stationary mouse-move observations
restarting chrome activity at both stage/root. Coordinate coalescing now
prevents that feedback and zero-distance pan invalidation. Real moves, clicks
and keys still wake controls. MCP exposes activity caller, count and last
coordinates without logging/invalidating, so actual input and a timer loop can
be distinguished. Separately, Settings update monitoring now repaints only
for result/progress and terminates disconnected channels rather than keeping
an endless pending poll.

No accessibility or DB records were filtered out to achieve these results.
Existing design/layout, SQL predicates, WAL connection separation, schema and
normal profile data remain unchanged. Source behavior mapping and affected
crate PORTS were updated with this change.

## Method and reproducibility

Windows 11 Pro 10.0.26200, Rust 1.97.1 Windows MSVC, GPUI 0.3.8, 150% DPI.
The normal `target/debug/photag-shell.exe` was rebuilt. The app is launched
through `scripts/photag-mcp-debug.cjs`, with MCP stdio and the real GPUI window
in the same child process. Native accessibility activation returns
`provider_response=true`; actual `Accessibility activated` and a11y paint
records remain in the reports. Debug remains open and following live during
the principal matrix. Paused/filtered/closed modes are also tested explicitly.

Each case waits for transitions, samples real UI/Preview/Debug state, waits
3–5 seconds, and samples again. A finite late completion or hover transition
is retained and followed by a bounded quiet recheck; it is never silently
removed. Longer follow-up retries wait four seconds before a five-second
observation, allowing the 2.8 s chrome fade to finish after genuine input.
Actual pointer movements were recorded during some earlier long observations;
those reports are retained and the final verdict requires a quiet recheck.
Ongoing work without a quiet sample, or a blocked state assertion, fails the
reusable scripts. Active playback is separately required to stay playing and
advance time without repeating DB queries, then settle on pause/close.

```powershell
rustup toolchain list
rustc +1.97.1 --version
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-database -p photag-ui-bridge -p photag-plugin-settings -p photag-plugin-navigation -p photag-plugin-gallery -p photag-plugin-preview -j 2
ffmpeg -hide_banner -loglevel error -f lavfi -i testsrc2=size=640x360:rate=24 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 12 -c:v mpeg4 -q:v 5 -c:a aac -y target/idle-suite-test.mp4
ffmpeg -hide_banner -loglevel error -f lavfi -i sine=frequency=220:sample_rate=44100 -t 30 -c:a pcm_s16le -y target/idle-suite-test.wav
New-Item -ItemType Directory -Force target/mcp-unified-refresh-final | Out-Null
Set-Content target/mcp-unified-refresh-final/settings.json '{"auto_check_updates":false,"show_filmstrip":true}'
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-unified-refresh-final --scenario scripts/validate-idle-mcp.cjs --allow-mutations
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-unified-refresh-final --scenario scripts/validate-idle-followup.cjs --allow-mutations
```

Use a fresh isolated profile for the complete matrix's initial empty-library
states. The runner copies `test-data/photag-library` into that profile and
optionally copies the synthetic 24 MP JPEG from the previous latency test.
Generated 12 s MP4 and 30 s WAV give meaningful playing/paused samples; the
original 0.25 s WAV was too short to establish sustained active playback.
Later scale tests inject 1000 virtual rows via the production DevTest service;
clearing them also invokes its existing fixture refresh, which may index the
read-only repository test fixture path into the isolated DB. Ratings in follow-up
only change that DB, and no tagging writeback is requested. Native Watcher
stimuli create/delete only `profile/fixtures/watcher-idle-probe.txt`.

No normal PhoTag DB/settings/session/cache were used. No app attachment,
generic computer automation or OS input injection was used. Sidebar + uses
GPUI pointer events at observed logical 228,72 and the native picker is then
read/cancelled by PhoTag MCP. Single/Multiple panels use registered plugin
factories and are closed through the existing PanelCatalog.

## Coverage and evidence

The final complete matrix has **95 cases: 93 quiet states and 2 active playback
states**, with no sustained or blocked case. It covers empty/populated Gallery;
all Sidebar and Inspector pages; six Settings pages; every filter popover;
seven grouping modes; Grid/List; root/nested folder browsing; single/multiple
selection; tag management dialogs; media actions; bulk modes and filter error
recovery; small/large/broken/GIF Preview, info/tags/more/zoom/rotate/flip/filmstrip;
audio/video paused, seeking, playing, paused again and closed; export success;
Debug pause/follow/category/page/close; 1000-row scrolling/off-window selection,
Statistics/Bulk; native picker open/close and manual update result.

The follow-up adds repeated longer Preview/reopen/pointer observations,
direct rating writes updating already-open Statistics and Bulk results,
settled hover animations, populated filter popovers, plugin Single/Multiple
panel disposal, Inspector sections, native Watcher create/remove, export
failure/recovery, Debug level/search/page-size/details/clear. Exact counts,
last sequence numbers, render/activity counters and per-case assertions are
in the exported summary and compressed raw reports.

Inspector EXIF is also exercised while switching/deselecting/reselecting,
scrolling with selection retained, and selecting a virtual record with a
missing source. The pre-fix EXIF run was stopped after the same root cause
continued into later states; its partial report is retained. Later states are
validated again after repair rather than interpreting those contaminated
observations as independent bugs. Each final suite records its executable hash.

Raw reports are gzip-compressed JSON to avoid committing tens of megabytes of
repeated log snapshots. They preserve every original sampled record, UI state,
action, retry and failure; expand with Node `zlib.gunzipSync` to inspect.
The evidence summary includes executable SHA-256 and final suite totals.

References: retained main layout `sidebar-pages-preview-2026-10-14-v2`; Settings
`final-v5/02` and `03`, menus `05`, bulk `06`, Statistics/Export/Updates `07`.
Native screenshots were inspected for actual structure, data/error states and
open Debug composition. This performance change omits no newly designed control.
Historical layout/feature limitations remain in the baseline validation ledger.

## Limits

This is comprehensive reachable-state testing for **idle refresh behavior**,
not complete functional acceptance of every operation/codec/OS configuration.
Destructive file actions, native save/destination/Recycle Bin confirmations,
OS drag/drop, updater download/install/restart, AI model download/inference,
third-party plugin unload, all codecs and other DPI/remote/network storage are
not established by these runs. Real WAV/MP4 playback clocks and frames were
checked; subjective sound/device behavior was not. Worker disconnect handling
is a code-level correction; these runs do not force a failed update worker.

## Shared implementation, 2026-10-15 follow-through

At the user's request the repair is now a workspace-wide common mechanism,
not separate timer/receiver fixes. `refresh::poll` is the sole GPUI timer owner;
`ChangeGate` handles passive signatures, and `ReadModel` handles passive reads.
All prior GPUI timers in feature views, Shell and shared frames use this entry.
Preview's seven manual read workers, Playback tags and Gallery's Inspector,
selection details and drag-path workers now reuse ReadModel too. Feature
code retains semantic identity/revision keys and result presentation. Explicit
commands, writes and fresh bulk-execution checks keep their existing services.

Scan and developer-task monitors no longer redraw just because work is still
running. Playback uses revision/position/playing/popover keys and still advances
normally; paused samples must settle. ReadModel queues only the latest demand,
discards obsolete results, caches missing/error outcomes and maps crashed
workers to one terminal error. Runtime tests include real selection changes,
missing sources, pause/resume and already-open view invalidation.

The architecture contract and caller example are in
[ui-refresh-contract](../../ui-refresh-contract.md). The new
`node scripts/check-ui-refresh.cjs` guard is part of the existing CI workflow;
it catches independent GPUI timers but does not replace runtime testing.
The final local workspace command is
`cargo +1.97.1 test --workspace --locked -j 2`: 89 tests pass, 3 are ignored
(the existing FFmpeg thumbnail fixture test and two network updater download
checks). Locked shell check/build and rustfmt pass; the guard and script syntax
checks pass. These are compile/automated checks, distinct from the MCP evidence.

Final evidence is generated only after both suites finish and their binary
SHA-256 matches `target/debug/photag-shell.exe`:
`node scripts/export-idle-evidence.cjs target/mcp-unified-refresh-final`.
The exporter refuses incomplete, sustained or blocked reports. It retains
original failing/interrupted runs separately and includes quiet rechecks and
native-input/window interruption data rather than deleting those samples.

## Final unified binary results

Both final suites ran against SHA-256
`ace27723b7bf8d8848531cd88e9fa2c37ac8253828207e584c8435569813b7de`:
95 matrix cases (93 settled, 2 actually-playing) plus 41 settled follow-up cases,
**136 total, 134 quiet and 2 active, zero sustained/blocked**. The follow-up
also requires the Preview render counter to stop, not just the log sequence.
Neither final suite needed to restore a minimized window.

Four initial observations required one bounded quiet recheck each. Preview Info
and the two live Bulk rating cases recorded real pointer-position changes;
original samples and counters are retained. Scale off-window selection produced
14 a11y-only updates over about 0.53 seconds with no input/database/Preview
render growth, followed by a completely quiet 4-second recheck. This finite GPUI
transition is recorded without attributing an unproven root cause. No case is
passed by filtering out its records or averaging away sustained activity.

First-open native paint measurements in the final matrix: the small PNG
thumbnail/full were 21.13/58.95 ms; the 6000×4000 JPEG thumbnail/full were
32.64/486.80 ms. They measure paint submission in this isolated development
process, not physical GPU presentation or a promise for all user files/storage.

Evidence: [compact summary](../../../.local/validation/artifacts/2026-10-15-idle-audit/summary.json), [full matrix](../../../.local/validation/artifacts/2026-10-15-idle-audit/unified-matrix.json.gz),
[extended follow-up](../../../.local/validation/artifacts/2026-10-15-idle-audit/unified-followup.json.gz), and original historical runs
listed in summary.history. [Preview screenshot](../../../.local/validation/artifacts/2026-10-15-idle-audit/preview-image.png),
[empty-library Statistics](../../../.local/validation/artifacts/2026-10-15-idle-audit/statistics.png), [broken-image state](../../../.local/validation/artifacts/2026-10-15-idle-audit/preview-broken.png)
and [manual updater result](../../../.local/validation/artifacts/2026-10-15-idle-audit/manual-update-result.png) are same-process captures.
Build/check/test/format/architecture logs are alongside these records.
