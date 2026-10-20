# WorkCrate three-level implementation

This revision supersedes the initial two-level WorkCrate UI acceptance. The user
required concepts first, then project management → page management → editor.
The inspected concepts and user sketches are linked from
`docs/reference-ui/concepts/workcrate-three-level-2026-10-20.md`.

## Environment and commands

- Windows MSVC Rust 1.97.1; 150% scale; native GPUI Kit light theme. The host
  primary role is black in this profile; concept blue is not a private palette.
- Same-process Shell MCP with `mcp,workcrate-ui,dev-test`, Full access only in
  `.local/validation/profiles/workcrate-three-level-2026-10-20-r2`.
- `cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui`
- `cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell`
- Seed scenario on a fresh isolated profile:
  `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-three-level-2026-10-20-r2 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-three-level.cjs`
- Reopen and exercise painted controls:
  `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-three-level-2026-10-20-r2 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-ui-interactions.cjs`
- The same client with `--scenario scripts/validate-workcrate-global-name-ui.cjs`
  opened the actual global-name editor through a pointer click. Its draft shows
  the global project name while the page retains its visible local override;
  `editor-global-name.png` was opened and inspected. That scenario leaves the
  draft unsaved when its isolated process closes.
- Native sources are the repository's JPEG/PNG fixtures, not concept artwork.
  All reports, screenshots and output files are local-only in that profile.

## Observed behavior

| Check | Reference | Observation |
|---|---|---|
| Project screen | Title and main image only; management menu | Three project cards; no page metadata. Visible menu is anchored below ellipsis; cover click navigates to that project's pages. |
| Project operations | Create/rename/cover/duplicate/archive/restore/delete | MCP seeded cover copies, duplicated six pages with distinct IDs, archived/restored/deleted the duplicate. Original fixture still exists. Native dialog clicks remain separate coverage. |
| Page management | Three columns, six pages, independent selection, footer exports | Final directory screenshot contains all six previews. Actual pointer clicks selected 01 and 03, and footer reports two pages. Initial capture taken during load was rejected; scenario now waits for painted busy=false and page count. |
| Editor | Left page rail, central page, right information/content/images | Visible sections follow the reference. Fixed-height scrolling rails corrected initial top clipping. Form labels/values use horizontal rows. Selected image persists an upstream DesignCraft move via actual pointer drag. |
| Metadata | First image name/pixels; separate A3 size; global/local project name | New pages are 420 × 297 mm. First fixture reports 900 × 600 source pixels. Local override survives global rename and reopen; reset inherits in service tests. |
| Export content | Inside-page information band included; outside information excluded | DesignCraft-rendered header contains page number, inherited/local project name and content fields. PDF/image exports include that header and artwork, without inspector/chrome. |
| Output | Default A3; source-derived raster | Poppler reports six A3 pages, 1191 × 842 pt at this fixture's density. PNG/JPEG reopen with actual pixel dimensions; PNG pHYs/JPEG JFIF record print size. Physical rounding is documented in Export PORTS. |

Large-window screenshots are 3286 × 1831 physical pixels, with a 1240 × 860
logical host floating panel. The host also supports existing panel-only launch
settings; this feature adds no window framework. Small-window request
1280 × 960 is clamped by the host minimum to 1462 × 960 physical pixels.

## Corrections and remaining differences

- The first small-window screenshot exposed a stale fit scale: the old code
  read the panel definition key rather than its instance geometry. The editor
  now measures its actual center viewport, compares measurements before
  notifying, and applies FigCraft fit to that size. The final rebuilt screenshot
  was inspected: A3 fits entirely at 31%, with readable navigation/inspector and
  independently scrolling sidebars. The earlier clipped screenshot is rejected.
- Comparing original JPEG, exported PNG/PDF and native screenshots exposed a
  pre-existing WorkCrate RGBA/BGRA adapter error. All four native image paths now
  follow GPUI's BGRA contract; a boundary test checks channels and alpha. Export
  remains canonical RGBA. The final rebuilt screenshots were opened and checked:
  their blue-to-pink source gradient agrees with the original and exported PNG/PDF.
  Earlier red/blue-swapped screenshots are rejected as color evidence.
- Source-derived content and theme roles intentionally replace concept sample
  artwork and blue paint. The page's header is document content; it is rendered
  through DesignCraft, not a separate GPUI screenshot composition.
- Selection has a bounding outline and release-time drag; resize/rotation use
  explicit image controls. Interactive resize/rotation handles, cropping,
  masters/templates, custom reference-preview UI and external linked-file editing
  remain open. Page ordering is creation order; a reorder interface is not yet
  provided.
- Native image/cover/source pickers, save/folder dialogs and overwrite/delete
  confirmation interactions remain unverified. Progress/cancel, large batches,
  camera-level visible detail, dark/accent and all keyboard/focus states need
  broader coverage. Compilation is not recorded as proof of those flows.
- Shell linking emits the existing LNK4098 default-library warning; builds and
  same-process startup succeed.

Focused service/contract/UI coverage contains 13 passing tests, including A3
image print metadata, first-source refresh/replacement, name inheritance,
non-destructive project operations and native pixel-channel conversion. Shell
fmt/check/build and theme self-test, theme/refresh/architecture/layering/ledger
guards pass. The seed scenario's two quiet-state snapshots 1.2 seconds apart
have no differing fields; source-image and selection changes were separately
exercised through explicit actions.

Runtime evidence names include `project-menu.png`, `pages-selected.png`,
`editor-selected.png`, `editor-small.png`, `interactions.json`, `report.json`,
and `artifacts/all-pages.pdf`, `first-page.png`, `first-page.jpg` under the local
profile. No generated runtime artifacts are tracked.
