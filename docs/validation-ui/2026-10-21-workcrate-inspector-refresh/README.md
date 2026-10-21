# WorkCrate inspector layout, preset menus and refresh audit

Date: 2026-10-21. Runtime checks used the same-process FileAtlas MCP child with an
isolated dev-test profile. Product window screenshots are in the ignored profile path
listed below; they are deliberately not copied into Git.

## Design reference and visible structure

The inspector refinement follows
[`workcrate-inspector-2026-10-21.png`](../../reference-ui/concepts/workcrate-inspector-2026-10-21.png)
and the existing WorkCrate editing checklist. It keeps the left page rail, center A3,
and independent right inspector. The inspector now has compact horizontal field rows,
thin theme-colored section dividers, and small adjacent preset controls. Text settings
and linked-image actions stay in their existing order. The concept image is guidance;
it does not add page or canvas behavior.

Same-process screenshots:

- `.local/validation/profiles/workcrate-inspector-refresh-2026-10-21/screenshots/field-preset-dropdown.png`
  shows the selected page, custom field rows, and open `制作工艺` menu with `写真覆膜`
  and `UV 打印`.
- The same folder's `native-character-and-frame-options.png` shows the text editor and
  the current native advanced-settings layout in the same inspector.

The second screenshot is intentionally dense because the full native typography panel
is expanded. The inspector remains independently scrollable. The current window has a
dark native theme; this run does not separately establish the appearance of every theme
role or narrow-window state.

## Refresh audit

| Path | Before | Current behavior | Check |
|---|---|---|---|
| Field value / preset choice, image import/replace/update, crop restore, text delete | Shared `perform` reloaded the entire board catalog and incremented `load_generation` | Fetch and replace the selected board in place; update the A3 information/header layer and that page preview. Text selection is cleared only when the text was deleted. | Preset scenario retains page generation and linked-image texture IDs while the A3 header texture changes for the selected value. |
| External navigation wake | Reloaded even when the same project and board were already selected | Compare both IDs; duplicate navigation returns without reload. Explicit MCP board/project refresh revisions distinguish same-page writes from navigation. | Same-process `update_page` on the selected board updates its A3 header without changing `load_generation`; linked image texture IDs remain. |
| Settings preset save | Editor could depend on a render-time array captured by a popup closure | Settings wake updates the cached field list; popup also reads the current Projects service snapshot when opening. | Add fields/options in Settings while editor stays mounted, then open and use the new popup. |
| Text body / size / color / frame | Per-text bounded write and texture update | Preserve input entity and unchanged image textures; refresh page preview after a typing pause. | Existing inline text regression. |
| Project service wake | Any project-list change reloaded the page catalog | Update project list in place. Reload only if the selected project disappears; a selected project's name change refreshes its A3 header and preview. Preset-only changes update menus without catalog reload. | Source-path audit plus the preset scenario; project name change is code-reviewed, not a separate live action in this run. |
| Project manager mutations | Rename/archive/delete/duplicate/cover used the same full reload path | Update visible project rows in place; discard only deleted/changed cover caches and decode missing/new covers. | Same-process rename/archive/restore service mutations update the project list without changing `load_generation`; local button dispatch is source-audited. |
| Project or page navigation/creation | Catalog membership/selection changes | Full catalog load remains intentional. | Source paths audited; exercised by existing project/editor scenarios. |
| Project-wide image update | Updates linked sources across multiple boards | Full catalog refresh remains because every affected board needs new metadata/previews. | Code-path audit; exhaustive per-image failure permutations not repeated here. |
| Export | Stage-driven finite poll | Only stage changes notify; task stops on completion/error. | Existing progress implementation and refresh guard. |

Render still diffs the retained GPUI tree when `cx.notify()` runs; the one WorkCrate
panel remains one GPUI view entity, so a notification schedules its render function.
The repaired behavior removes catalog reloads, page-list churn, load-generation changes,
and unnecessary linked-image texture replacement from page-local writes. The canvas
header layer is re-rendered when page information changes because its displayed content
has changed.

## Verification

Toolchain: installed `1.97.1-x86_64-pc-windows-msvc`.

```text
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked --offline -j 2 -p fileatlas-plugin-workcrate-ui
cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-projects
cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
```

All commands passed. Project/UI tests: 6 passed. Architecture remained 125 edges and
803 baseline debt items; UI theme, refresh ownership, feature ledger and layering guards
passed. The linker emitted its existing `LNK4098` runtime-library warning.

Live MCP checks:

```text
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-inspector-refresh-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-typography.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-inspector-refresh-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-field-presets-auto.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-inspector-inline-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-inline-text.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-project-manager-refresh-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-project-manager-refresh.cjs
```

All exited successfully. Preset auto-save works without pressing Save; two added fields
each have two choices, the new menu appears and selects a value. The field edit keeps
`load_generation` and linked-image texture IDs stable while the A3 header layer updates.
The same-board MCP `update_page` also refreshed the A3 header in place without a catalog
reload. Project rename/archive/restore also updated the open project list without changing
the catalog generation. The inline editor regression passed. The quiet-state assertions
found no continued refresh after work completed. Native Explorer drag/drop, physical IME
composition, multimonitor scaling, and each error branch were outside this audit.

## Remaining boundaries

- Project-wide image refresh intentionally reloads the catalog after changing sources
  across pages; a per-board progress/update flow is not part of this request.
- One GPUI view redraws through retained rendering after notification. This change
  prevents the expensive data/catalog reload and unrelated texture reload; it does not
  split the inspector into another Cordis/GPUI view entity.
- Typography advanced settings remain scrollable, with compound native values edited as
  JSON, as recorded in the existing WorkCrate editing validation.
- The older `validate-workcrate-editing.cjs` all-in-one scenario was tried once but is
  stale at its text step: after Add Text, the current design deliberately presents the
  single text input on the canvas, while that script still waits for the inspector-only
  `text-body` control. Its preceding image replacement/crop assertions completed; text
  behavior is covered by the dedicated inline and typography scenarios above.
