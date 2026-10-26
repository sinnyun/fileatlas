# Canvas image quality tiers and durable previews — 2026-10-26

## Implementation

Canvas images still reference their original local/Eagle `ResourceLink`. The
existing ThumbnailService supplies a preview with a maximum long edge of 300 px;
the whiteboard copies that JPEG to
`%LOCALAPPDATA%/FileAtlas/whiteboards/previews/<board-id>/` and stores its safe
filename and pixel dimensions in an additive `Board.image_previews` field. The
library remains version 2; old board records deserialize with an empty preview
list. Original media remains external and is never copied into this directory.

The image renderer starts with the thumbnail and measures the full transformed
image rectangle in physical screen pixels using the monitor scale factor.
Viewport clipping controls painting only; it does not lower the selected image
quality as an image moves out of view. The original is requested at 500,000
screen pixels or more. It returns to the thumbnail at 350,000 pixels or fewer
only after remaining below that threshold for 350 ms. This hysteresis plus
debounce avoids unnecessary tier changes during camera motion. The two decoded
sources crossfade with a 180 ms smoothstep transition.

Decoded originals remain memory-only for up to 30 seconds after last use. Idle
originals share a 128 MiB cache budget and the least-recently-used entries are
released first. A visible original is kept available; after expiration or cache
pressure it is decoded again only if requested. Thumbnail JPEGs remain in the
board-owned preview directory. Missing previews are regenerated from the linked
source.

The policy follows mature canvas patterns: tldraw separates viewport culling
from source asset lifetime, selects image resolution from on-screen scale and
device pixel ratio, and debounces resolution changes during zoom. See [tldraw
performance](https://github.com/tldraw/tldraw/blob/main/apps/docs/content/sdk-features/performance.mdx)
and its [asset resolution contract](https://tldraw.dev/sdk-features/assets).
Excalidraw's renderer also caches per-element raster canvases, regenerates them
when zoom/device scale changes, and caps backing-canvas dimensions to avoid
oversized allocations; that supports keeping render caches separate from scene
visibility and bounding their memory cost. See its [renderElement
implementation](https://github.com/excalidraw/excalidraw/blob/master/packages/element/src/renderElement.ts).
FileAtlas keeps its two existing source tiers (thumbnail/original), since its
local source contract does not provide generated intermediate sizes, and applies
these lifecycle ideas to decoded original bitmaps.

## Verification

- Prior checks on the base image-tier implementation: Rust 1.97.1 fmt, locked
  Shell check/build, theme/refresh/architecture/feature-ledger guards and Eagle
  same-process MCP passed. That run used monitor scale 1.5 and confirmed
  777,924 physical pixels at zoom 2.8 and 222,661 at zoom 1.498. Its screenshot
  set is under `.local/validation/profiles/canvas-image-area-final2-20261026/`.
- Current implementation: `cargo +1.97.1 fmt --all -- --check` and
  `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` passed; locked Shell
  build passed with the existing linker `LNK4098` warning. UI theme, refresh,
  architecture and feature-ledger guards passed.
- Same-process Eagle/MCP scenario passed:
  `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/canvas-image-residency-final-20261026 --plugins whiteboard --access-level standard --scenario scripts/validate-eagle-canvas-drop.cjs`.
  At monitor scale 1.5, the image measured 777,924 physical pixels at zoom
  2.8 and 222,661 at zoom 1.498. While at the original tier, the scenario
  panned the image out of view for 500 ms, returned to the same camera position,
  then captured the restored high-resolution image. It subsequently crossed
  back to the thumbnail tier and switched boards. Screenshot evidence is in
  `.local/validation/profiles/canvas-image-residency-final-20261026/screenshots/`:
  `canvas-high-visible-pixel-area.png`,
  `canvas-original-after-pan-return.png`,
  `canvas-thumbnail-screen-area.png`, and `canvas-board-switched-back.png`.

## Cache statistics MCP — 2026-10-27

Added read-only `whiteboard.cache_stats`. It reports cache entries and decoded
image counts separately, RGBA bitmap byte estimates, current quality-state
counts, visible image count, idle original bytes/age, active transitions,
pending downgrades, original requests and memory hits, decode requests, TTL and
budget evictions, plus the configured thresholds and retention policy. The byte
estimate excludes GPUI/GPU and allocator overhead. Counters reset when the active
board changes.

The expanded Eagle scenario passed in an isolated same-process MCP run:
`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/canvas-cache-stats-verified-20261026 --plugins whiteboard --access-level standard --scenario scripts/validate-eagle-canvas-drop.cjs`.
It confirmed the thumbnail decoded first with zero decoded originals; the
original was decoded after zooming in; panning out and back increased original
memory hits while retaining the decoded source; zooming down returned quality to
the thumbnail while the original remained cached; after the image stayed
offscreen beyond 30 seconds, the tool reported one TTL eviction and zero decoded
originals. The final snapshot reported one decoded thumbnail (202,800 RGBA
bytes), no original cache entries, `original_ttl_evictions=1`, and the configured
128 MiB idle budget. The run also crossed the previously validated physical
screen-area thresholds and switched boards successfully. The budget eviction
counter remains unexercised by memory pressure because this Eagle fixture stayed
below 128 MiB.

## Remaining runtime coverage

The whiteboard-owned MCP wheel tool exercises the same handler and can verify
camera math, including pointer anchoring and Ctrl+wheel panning; those checks
are recorded in [the wheel-control validation](2026-10-26-canvas-wheel-mcp.md).
MCP camera values and the known monitor scale confirm that the test image's
full physical display area crossed both source-quality thresholds. The new
cache statistics MCP verifies TTL eviction; budget eviction under memory
pressure remains untested. Mid-fade reversal is not directly asserted. The
board-switch screenshot confirms the persisted preview is visible after
reopening, but does not establish zero decode latency.
