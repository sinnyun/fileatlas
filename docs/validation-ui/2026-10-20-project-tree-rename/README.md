# Project tree naming and migration validation — 2026-10-20

## Static/build validation

- Toolchain: `1.97.1-x86_64-pc-windows-msvc` (`rustup toolchain list`).
- `cargo +1.97.1 fmt --all` and `cargo +1.97.1 fmt --all -- --check`: passed.
- `cargo +1.97.1 check --locked --offline -p fileatlas-plugin-project-tree -p fileatlas-shell -j 2`: passed.
- `cargo +1.97.1 build --locked --offline -p fileatlas-shell -j 2`: passed. Linker emitted LNK4098 default-library conflict warning; build completed successfully.
- `node scripts/check-ui-theme.cjs`, `node scripts/check-ui-refresh.cjs`, `node scripts/check-architecture.cjs`, `node scripts/check-feature-ledger.cjs`, `node scripts/check-layering.cjs`: passed. Architecture baseline regenerated after reviewing the crate-identity rename.

## Same-process Windows MCP

- Command: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/project-tree-rename-migration-20261020 --plugins core --access-level standard --scenario scripts/validate-project-tree-switch.cjs`.
- Runtime: Windows, Rust 1.97.1 MSVC, same-process `fileatlas-shell --mcp`, isolated profile, 150% GPUI scale factor.
- Before startup, the isolated profile contained the old `projects/projects.json` path and version 1 `{ "projects": [...] }` data. After startup, the plugin loaded that root, wrote `project-tree/project-tree.json` using `{ "roots": [...] }`, and retained the old file. The scenario then added a second root and verified switching the expanded tree, navigating the active browser pane, and collapsing the tree. MCP `navigation_tab` reported `project-tree`.
- Screenshot checked: `.local/validation/profiles/project-tree-rename-migration-20261020/screenshots/after-add-1.png`; the Navigation label renders as “项目树”. Other generated screenshots and reports remain under the ignored profile directory.
- The first migration attempt used an extra root that shifted fixed pointer coordinates in the existing scenario; it stopped at the root-switch step. The successful rerun used the legacy root as `project-alpha`, keeping the scenario geometry stable. This was a scenario-coordinate issue, not a product failure.

The clean-profile regression scenario also passed with the same root-add/switch/collapse checks at `.local/validation/profiles/project-tree-rename-clean-20261020`.
