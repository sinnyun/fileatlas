# Theme modes and plugin-owned shell actions — 2026-10-19

Palette values and the pre-refinement screenshots below record the earlier
two-mode implementation. They are superseded by the low-saturation semantic
palette and paired-contrast checks in [the 2026-10-19 follow-up](../2026-10-19-muted-semantic-colors/README.md).

## Implementation record

- Reference: [light-mode concept](../../reference-ui/concepts/light-workbench-theme-2026-10-19.png) and [checklist](../../reference-ui/concepts/light-workbench-theme-2026-10-19.md); dark mode remains tied to the [accepted navy concept](../../reference-ui/concepts/unified-workbench-theme-2026-10-18.png).
- Settings reference: [Appearance minimal concept](../../reference-ui/concepts/settings-minimal-2026-10-18.png). The user-directed extension inserts a horizontal Color mode row after Accent color and before Interface layers, with Dark / Light segments.
- Persistence: `color_mode` is stored in the existing Settings JSON. Missing keys default to dark; no new persistence path or DB migration.
- Plugin ownership: Settings contributes its theme and Settings rail controls in `workspace.sidebar-footer`; `fileatlas-plugin-shell-controls` contributes sidebar, inspector, workspace and Tools actions. Cordis removes those views on stop.
- Semantic colors: mode-aware accessors drive surfaces, text, separators, control fills, borders, overlays and the GPUI Kit theme across consumers. Uppercase constants remain dark-palette compatibility values for const contexts.
- Shell audit: functional action buttons have moved to plugin views. Remaining Shell-rendered elements are the FileAtlas brand, layout and status composition, resize interaction, and generic host rendering of plugin command contributions.

## Static checks

Commands: `cargo +1.97.1 fmt --all -- --check`; `cargo +1.97.1 check --locked -p fileatlas-shell -j 2`; `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`; `node scripts/check-ui-theme.cjs`; `node scripts/check-ui-refresh.cjs`; `node scripts/check-architecture.cjs`; `node scripts/check-feature-ledger.cjs`.

Result: formatting, locked shell compilation/build and all four governance checks passed after the shell-controls crate was added. The linker emitted LNK4098 (`LIBCMT` conflict), but the build completed successfully. Feature ledger checks relative links.

## Live visual and interaction evidence

Same-process MCP sessions used `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-theme-modes --plugins core --access-level standard --interactive` for the rail toggle, then isolated plan sessions `--plan target/theme-settings-click-plan.json` and `--plan target/theme-settings-mode-plan.json` for the Settings entry and Appearance row. Each plan's UI actions and MCP reads ran against the same `fileatlas-shell --mcp` child. The isolated window reported 3286×1831 physical pixels at scale 1.5. Pointer control opened Settings from the plugin-owned footer. The Settings row changed `color_mode` from dark to light and back to dark; MCP reported the semantic base changing from `0x0e1823` to `0xf3f6fa` and back. The main-window screenshot shows FileManager, Navigation, inspector and footer colors changing with it. Settings JSON was dark after the test.

Evidence: ![dark-main.png](../../../.local/validation/artifacts/2026-10-19-theme-modes/dark-main.png), ![light-main.png](../../../.local/validation/artifacts/2026-10-19-theme-modes/light-main.png), ![settings-page-dark.png](../../../.local/validation/artifacts/2026-10-19-theme-modes/settings-page-dark.png), ![settings-page-light.png](../../../.local/validation/artifacts/2026-10-19-theme-modes/settings-page-light.png), and ![settings-page-dark-restored.png](../../../.local/validation/artifacts/2026-10-19-theme-modes/settings-page-dark-restored.png). The theme button is directly above Settings. The Settings entry and mode row both worked. Still unverified: menu overlays, focus/disabled states, small-window behavior, restart restoration, and a floating panel screenshot. Overall visual acceptance remains partial.
