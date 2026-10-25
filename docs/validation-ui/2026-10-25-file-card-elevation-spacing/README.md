# File card elevation and group spacing — 2026-10-25

## Reference and checklist

- Active main-window reference: `docs/reference-ui/concepts/multi-column-workspaces-2026-10-19.png` and its companion notes.
- Keep the existing file grid and group frame structure; add subtle theme-owned elevation to grid file cards.
- Add equal left/right breathing room between grouped file cards and their shared group frame. Keep list rows and group headers unchanged.

## Verification

- Windows, same-process FileAtlas MCP child, isolated profile `.local/validation/profiles/card-shadow-spacing`.
- Fixture source: 24 generated image copies inside `.local/validation/profiles/card-shadow-spacing/fixture`; production Indexing scan completed. Navigated to that fixture root, enabled descendants and subfolder grouping, then captured the live window.
- Screenshot: `.local/validation/profiles/card-shadow-spacing/screenshots/file-cards-group-spacing-many.png` (local-only).
- Visually checked the captured window with audit overlays disabled: all 24 cards fit within the group content bounds; the rightmost column remains inside the frame and both horizontal insets are visible.
- Static/build checks: `cargo +1.97.1 fmt --all -- --check`; UI theme, UI refresh, architecture and feature-ledger guards; locked offline checks for `fileatlas-plugin-file-manager` and `fileatlas-ui-bridge`; locked Shell build. All passed. Shell link emitted the existing LNK4098 runtime-library warning.
- Not checked: alternate display scales, dark theme, list layout, hover/selection shadow states, and small-window clipping.

## Implementation

- UI Bridge `WorkbenchStyle::workbench_media_card` applies the existing native-theme `card_shadow` to grid file cards.
- Grouped file rows use the shared standard horizontal spacing token on both sides, constrain their width, and include that inset in the available-width/column calculation.
