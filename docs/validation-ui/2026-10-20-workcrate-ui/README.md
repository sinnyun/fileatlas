# WorkCrate project and board UI validation

## Environment and commands

- Windows, `1.97.1-x86_64-pc-windows-msvc`, 150% display scale.
- Same-process MCP launched the Shell child with `mcp,workcrate-ui,dev-test`,
  Standard access and profile
  `.local/validation/profiles/workcrate-ui-2026-10-20/`.
- `cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell`
- `cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui`
- MCP client: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-ui-2026-10-20 --plugins mcp,workcrate-ui --access-level standard --plan .local/validation/workcrate-ui-panel-open.json`

## Observed results

- `tools/list` and `command_list` expose `workcrate/open`; command execution
  opened the WorkCrate panel in the same Shell/GPUI process.
- The native UI visibly created a project and a board through its controls. The
  board catalog screenshot shows its numbered card, blank page preview, image
  count and 595 × 842 pt dimensions. Empty-project and empty-board copy were
  visible and readable.
- Reference comparison: the active three-column catalog hierarchy is present;
  with one board the card starts at the left and leaves the rest of the row open.
  The host was using its native dark theme, as expected from GPUI Kit. The first
  editor screenshot exposed a clipped page. The fit implementation now mirrors
  FigCraft `engine::ViewState::fit`: 64pt margins, a 50pt minimum usable axis,
  fit capped at 100%, measured panel content size, and current device scale.
  The revised same-process screenshot shows the entire A4 page in the viewport.
- `workcrate-ui.state`, `workcrate-ui.open_board`, and `workcrate-ui.place_image`
  were added as lifecycle-owned MCP tools after generic pointer delivery did
  not switch the selected card. The service route placed both a fixture PNG and
  a generated 1024×768 JPEG converted from the repository's BMP fixture. The
  editor screenshot shows both placements in their saved locations, with no
  thumbnail substituted for the placed source.
- Full-access MCP `workcrate-ui.export_pdf` with per-call `confirm=true` rendered
  those sources to a one-page PDF. The MCP result reported 175.54 DPI and
  1451×2053 px; Poppler reopened it as one A4 page (595.137×842.051 pt). Its
  110-DPI render was inspected and showed both images fully within the page.
  This verifies the core output-resolution path for a 1024×768 fixture, not
  camera-resolution sharpness or native Save dialog behavior. After adding the
  same-directory atomic replacement, the same-process MCP export was repeated
  against that existing PDF path and succeeded with the same raster dimensions.
- The native image picker, high-zoom interaction, missing-source recovery,
  small-window fit, and resize/crop controls remain unverified or unimplemented.

Screenshots and MCP reports are local-only under the profile's `screenshots/`
directory and `report.json`; generated validation data is not checked in.

## Checks

WorkCrate service/contract/UI tests: 9 passed. Shell check/build and formatting
passed. `node scripts/check-ui-theme.cjs`, `node scripts/check-ui-refresh.cjs`,
`node scripts/check-feature-ledger.cjs`, `node scripts/check-architecture.cjs`,
and `node scripts/check-layering.cjs` passed. Shell linking emitted LNK4098 for
conflicting default libraries; it did not prevent the build or MCP startup.
