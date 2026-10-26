# Canvas side panels: compact icon and parameter rows

## Reference and result

Implemented the user-requested side-panel clarity pass against
`docs/reference-ui/concepts/fileatlas-canvas-tldraw-layout-2026-10-26.png` and
its supplement `fileatlas-canvas-inspector-compact-2026-10-26.png`. The canvas
and drawing toolbar remain in place. The left panel now presents board/resource
rows with type icons, decoded names, and compact type/extension detail. The
right panel uses one row per layer with type icons and icon-only reorder actions;
selected-object properties use semantic icons, short values, and copy controls
for full paths/URLs. Long values truncate in place instead of widening the panel.

## Same-process evidence

- Command: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/canvas-sidepanels-20261026 --plugins whiteboard,dev-test --access-level standard --scenario scripts/validate-eagle-canvas-drop.cjs`
- Result: passed. The scenario resolved the running Eagle library item `WILDWOOD - LAIKA.jpg`, inserted its original image through `whiteboard.add_images`, and verified the saved Eagle name/ID, folder, original path, original URI, thumbnail path, and API URL.
- Window/screenshot was captured from the same FileAtlas MCP child process. The isolated profile disabled `show_ui_audit_frames` and `show_typography_audit_colors` so the screenshot reflects product styling.
- Screenshot: `.local/validation/artifacts/2026-10-26-canvas-sidepanels/eagle-image-properties.png`.

The screenshot confirms compact icon/parameter rows and no horizontal overflow in the right panel. The left board list is populated and readable; the resource subsection is empty in this run because the Eagle image was inserted as a canvas image rather than a file-card resource. Multi-layer density, narrow-window layout, and native cross-app mouse drag were not covered by this scenario.

## Static checks

Passed: `cargo +1.97.1 fmt --all -- --check`; `cargo +1.97.1 check --locked --offline -p fileatlas-plugin-whiteboard`; `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`; `node scripts/check-ui-theme.cjs --self-test`; `node scripts/check-ui-theme.cjs`; `node scripts/check-ui-refresh.cjs`; `node scripts/check-architecture.cjs`; `node scripts/check-feature-ledger.cjs`.

No test suite was run.

## Copying an original image path into Windows Explorer — 2026-10-26

The canvas resource URI (`file://...`) and an Explorer filesystem path serve
different purposes. The URI percent-encodes Chinese characters by design and is
intended for FileAtlas resource references. The properties now label these as
`画布资源 URI` and `本地文件路径`. Copying the latter strips Windows' internal
`\\?\UNC\` long-path prefix and yields a normal `\\server\share\...` UNC path.

Rebuilt the Shell and reran the Eagle drop scenario through same-process MCP.
Verified that the normalized path for the live Eagle source exists with PowerShell
`Test-Path -LiteralPath`; refreshed screenshot shows the two distinct labels.
The clipboard itself is not exposed by FileAtlas MCP, so validation confirmed the
copy payload's normalization logic and filesystem existence, not a synthetic paste
into Explorer. Build/check, fmt, and theme/refresh/architecture/feature-ledger
guards passed.

The user's specific URI (`LUE2D9HWV51NS`) was also resolved with the current Eagle
API. Its percent-decoded UNC path is
`\\merittanas\设计里\meritta同步文件夹\设计灵感.library\images\LUE2D9HWV51NS.info\这是一张描绘现代都市夜晚室内场景的数字插画，通过窗户框架分割画面，展现三个人物在温馨灯光下的互动。.jpg`;
PowerShell confirmed `Test-Path -LiteralPath` is true. Eagle returns that same long
Chinese phrase as the item's `name`, so it is source metadata rather than a canvas
encoding error.
