# File staging basket validation (2026-10-20)

## Scope

Tracks `fileatlas-plugin-file-staging`: JSON
persistence, path deduplication/status, batch copy and move through
`FileOperationsService`, retry retention on conflicts, same-process panel
rendering, and restart persistence. The current visual reference is
`docs/reference-ui/concepts/file-staging-grid-drag-2026-10-20.png`.

## Environment and commands

- Windows workspace: `F:\AI\geren\pho_cordis`
- Rust toolchain: `1.97.1-x86_64-pc-windows-msvc`
- Isolated MCP profile: `.local/validation/profiles/file-staging-final`
- Scenario fixtures, screenshots, reports, and profile data remain under the
  git-ignored `.local/validation/` directory.
- Static commands run:

  ```powershell
  cargo +1.97.1 fmt --all
  cargo +1.97.1 check --locked --offline -j 2 -p fileatlas-plugin-file-staging -p fileatlas-plugin-file-manager -p fileatlas-plugin-file-operations -p fileatlas-plugin-i18n -p fileatlas-shell
  cargo +1.97.1 build --locked --offline -p fileatlas-shell -j 2
  ```

- Same-process MCP scenarios run against the built shell child:

  ```powershell
  node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/file-staging-final --plugins dev-test --access-level full --scenario scripts/validate-file-staging.cjs
  node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/file-staging-final --plugins dev-test --access-level full --scenario scripts/validate-file-staging-restart.cjs
  ```

Both scenarios exited successfully. The first used actual temporary files and
verified: paths from multiple directories, canonical deduplication, panel
rendering, batch copy, a pre-existing target collision without overwrite,
retention of failed items for retry, batch move/source removal, missing-file
status and pruning. The second started a fresh MCP child against the same
isolated profile and verified the remaining staged path persisted.

The build emitted the existing MSVC linker warning `LNK4098` for a default
library conflict; compilation and linking succeeded.

## Visual and interaction review

The initial same-process screenshot is stored locally at
`.local/validation/profiles/file-staging-final/screenshots/staging-basket.png`.
The updated grid screenshot and drag probes are under
`.local/validation/profiles/file-staging-grid-20261020/screenshots/`. The new
panel renders thumbnail/title cards and remains modeless so FileManager stays
visible as a drop target. The synthetic MCP drag shows the drag ghost and
highlights the destination group, but releasing over the group did not move the
file or remove it from the basket. Immediate move-on-drop is therefore
implemented but not runtime-verified yet. The probe is
`scripts/validate-staging-grid-drag.cjs`; its local report is
`.local/validation/profiles/file-staging-grid-20261020/staging-grid-probe.json`.
The first modeless-panel change still showed the shell overlay because the
overlay predicate counted every open custom panel. Shell now excludes panels
marked non-modal. A same-process MCP pointer click selected the alpha folder
while the staging panel remained open; the result was `modeless-interaction-passed`.
Its screenshot is
`.local/validation/profiles/file-staging-grid-20261020/screenshots/staging-modeless-open.png`.
Dark theme and a standard window were used; empty, small-window, light-theme,
and hover/focus/disabled states were not checked.

Incoming FileManager-to-staging drag is verified with the same-process MCP
scenario `scripts/validate-file-to-staging-drop.cjs` using the isolated
`staging-pointer-baseline-20261020` profile. Dragging
`test-data/fileatlas-fixtures/01-small/tiny.jpg` into the empty staging panel
added its path and preserved the source file. The success result was
`file-to-staging-passed`; its hover screenshot is
`.local/validation/profiles/staging-pointer-baseline-20261020/screenshots/file-to-staging-drag-hover.png`.

MCP covered the underlying add/remove/copy/move/status operations and panel
rendering. Direct pointer activation of the FileManager selection context-menu
entry, native multi-file picker, and native destination picker was not covered.
The selection event path and dialog handlers compile, but their real UI
interaction remains unverified. Windows-specific cross-volume moves, permission
failures, and collision races also remain unverified. Batch cancellation,
pause/resume, and interrupted-transfer recovery are not implemented.

## Governance

`cargo +1.97.1 fmt --all -- --check`, architecture, layering, UI theme (including
self-test), UI refresh, and feature-ledger checks passed after the final docs
updates. The grid-drag probe exited on its move assertion. This record
distinguishes service-level file-I/O coverage from drag/drop and native dialog
interaction coverage.
