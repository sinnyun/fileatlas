# Open Eagle item from canvas properties

## Behavior

The Eagle metadata block in the selected image inspector includes an external-link
icon button with tooltip “在 Eagle 中打开并定位此图片”. It sends the existing
Eagle item ID through `eagle://item/{id}` to the OS registered handler. If there is
no persisted Eagle source or Eagle Links service, the button is omitted. Dispatch
errors appear in the canvas status message. The action opens/locates the Eagle
library item and does not alter media or metadata.

Implementation: `EagleLinkService::open_item`; Eagle Links owns the protocol
handoff, and Whiteboard owns the inspector affordance.

## Validation

- `cargo +1.97.1 fmt --all -- --check`
- `cargo +1.97.1 check --locked --offline -p fileatlas-plugin-whiteboard`
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`
- `node scripts/check-ui-theme.cjs --self-test`; `node scripts/check-ui-theme.cjs`; `node scripts/check-ui-refresh.cjs`; `node scripts/check-architecture.cjs`; `node scripts/check-feature-ledger.cjs`
- Same-process MCP scenario: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/canvas-eagle-open-item-20261026 --plugins whiteboard,dev-test --access-level standard --scenario scripts/validate-eagle-open-item.cjs`
- The scenario inserted a real image resolved through the live Eagle API, clicked the icon through GPUI pointer events in the FileAtlas MCP child, and completed the Eagle deep-link handoff for the resulting item ID. Screenshot: `.local/validation/artifacts/2026-10-26-canvas-sidepanels/eagle-open-item-clicked.png`.
- The MCP child verifies successful URL-handler dispatch; Eagle does not expose selected/open UI state in its documented Web API, so this does not claim an independently observed Eagle window selection.
- No test suite was run.
