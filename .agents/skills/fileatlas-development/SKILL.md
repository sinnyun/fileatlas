---
name: fileatlas-development
description: Develop FileAtlas features and maintain its Rust/Cordis plugin architecture, data behavior, and GPUI Kit interface. Use for implementation, debugging, refactoring, and review in this repository.
---

# FileAtlas Development

Use this skill when changing FileAtlas. First identify the feature owner and read its `PORTS.md`; keep implementation inside that boundary and use existing public services. The root `AGENTS.md` and linked project docs are the current source of truth. This skill summarizes the decisions that most often affect implementation.

## Start with the project shape

- FileAtlas is a Windows desktop file browser and workbench. File browsing, navigation, multi-column workspaces, and program workspaces are primary product capabilities; media indexing and metadata are integrated file-management features. Optional unrelated capabilities may be separate feature plugins.
- Rust workspace; GPUI and GPUI Kit render the native UI. Cordis owns compile-time feature plugin lifecycle, dependencies, services, events, and cleanup. Do not add dynamic DLL loading or another plugin manager.
- `fileatlas-shell` owns startup, runtime construction, and top-level composition only. Feature behavior belongs in feature crates.
- `fileatlas-ui-bridge` is the public UI registration/catalog, view-factory, shared component, theme gateway, and cleanup boundary. Reuse it for contributions and common UI instead of plugin-local infrastructure.
- The current product baseline is FileAtlas 0.2. Preserve its accepted layout and plugin architecture unless the request requires a focused change. Do not treat historical source UI or old phase documents as the active design.

For code discovery, follow the repository's Codebase Knowledge Graph instructions in `AGENTS.md`: use `search_graph`, `trace_path`, `get_code_snippet`, and `query_graph` before text search. Use text search for literals, configs, docs, and graph coverage gaps.

## Plugin boundaries and lifecycle

1. Locate the owning crate and inspect its `PORTS.md`, public contracts, callers, and relevant validation notes. Trace behavior before changing data or cross-plugin interactions.
2. Put feature behavior in its compile-time Cordis crate. Use public contracts/services/events between plugins; avoid private cross-crate imports and duplicate host facilities.
3. Register UI contributions (layout, dock, panel, slot, widget, command, menu, keymap) with plugin ownership so Cordis/UI Bridge removes them on stop. Background work and subscriptions must follow the same lifecycle.
4. Reuse the generic floating-panel facility for most plugin windows. Use the shared view catalog and registration contracts; do not build another panel framework.
5. Keep `PORTS.md` aligned with actual plugin/service identity, dependencies, inputs/outputs, event producers/consumers, storage, limits, and failure inspection points.

## Data and persistence

- Preserve established PhoTag-derived semantics and trace the matching implementation under `vendor/photag-source/` before changing backend behavior. Update the source mapping in the same change when applicable. Historical source UI is not a current UI specification.
- All SQLite access goes through the Cordis `fileatlas-plugin-database` service. Do not open SQLite directly from a feature plugin or duplicate the database layer.
- Choose persistence from the feature's data shape and record the decision in its `PORTS.md`:
  - Versioned JSON for modest settings, manifests, histories, and lists that are naturally read/written as whole documents. Use serde defaults, a same-directory temporary file and atomic replacement; document path, compatibility, migration, and recovery.
  - Shared SQLite for growing/query-heavy, relational, concurrent, incremental, or transactional data. Schema and typed APIs belong to the database crate.
  - Keep large binary assets separate from structured records. Distinguish rebuildable caches from user-created data; cleanup must not delete user data.
- Preserve existing SQLite schema and `%LOCALAPPDATA%\\FileAtlas` data unless a compatibility migration is supplied and tested. Do not replace working behavior with placeholders.

## GPUI Kit and interface work

Before an interface change, read `docs/ui-reference-workflow.md`, resolve the active screen in `docs/reference-ui/README.md`, and inspect the referenced image and concept notes. Turn the visible structure and states into a checklist before editing. For substantial visual changes, follow `AGENTS.md`'s concept-image workflow.

- GPUI Kit `ThemeRegistry` / `ThemeConfig` / `Theme` owns resolved UI colors and states. Consume colors through UI Bridge and shared components. Never add plugin-local RGB palettes, `mode_color`, direct Theme updates, or colors used only to style plugin UI.
- Use UI Bridge `components`, `panels`, settings-page declarations, and other public facilities. Prefer native themed controls and shared builders (`button`/`action`/`icon_button`, `text_input`/`search_input`/`textarea`, sections/cards/navigation). If a common component is missing, extend UI Bridge and use it consistently.
- Follow `docs/ui-theme.md` for semantic color families, typography, spacing, dimensions, interaction states, floating panels, and theme-generator ownership. Do not work around theme or design guards by rewriting expressions.
- All passive database/filesystem/metadata reads must use `fileatlas_ui_bridge::read_model::ReadModel` with explicit identity/revision keys, coalescing, failure caching, and stale-result rejection. Rendering declares a nonblocking request; it does not perform IO. Use `refresh::poll` and `ChangeGate`; wakeups or unchanged state do not justify notifications.
- Keep action-time target validation fresh. Preserve event/action semantics and avoid repeated polling, repeated notifications, and hidden logs as a workaround.
- For interface changes, compare an actual same-process FileAtlas MCP screenshot and exercised states with the active reference. Record the reference, exact commands/environment, outcomes, visible gaps, and unverified behavior in `docs/redesign-validation.md` and a focused `docs/validation-ui/` record. Compilation is not visual or Windows interaction evidence.

## Validation and local workflow

- Follow `docs/rust-toolchain.md`. On the documented Windows host, inspect `rustup toolchain list` at the start of a validation session and explicitly use installed Rust 1.97.1 (`cargo +1.97.1`), `--locked`, bounded jobs (`-j 2` for builds/tests), and existing incremental artifacts. Never infer incompatibility from the older default toolchain.
- Run checks relevant to the change: formatting, locked crate/shell check or build, focused tests, and applicable governance scripts. For UI/refresh/architecture changes consult `docs/engineering-governance.md` and run relevant guards, including `node scripts/check-ui-theme.cjs --self-test`, `node scripts/check-ui-theme.cjs`, `node scripts/check-ui-refresh.cjs`, `node scripts/check-architecture.cjs`, and `node scripts/check-feature-ledger.cjs` as applicable.
- Do not claim a build or headless test proves an interface works. Live UI/debugging and interaction validation use the project's FileAtlas MCP plugin. Attach to the running same-process HTTP MCP for inspection; use `scripts/fileatlas-mcp-debug.cjs` only for isolated regressions. Do not substitute generic OS UI automation or launch another instance to inspect the user's current window.
- Put generated validation profiles, screenshots, logs, and reports under git-ignored `.local/validation/`; use isolated profiles for tests that seed data. Keep reusable scenario code in `scripts/` or the feature source.
- Keep Rust files under 500 lines where practical and under 800 lines. Retain upstream attribution and licenses for copied code; document pinned revision and license in the owning feature's `PORTS.md`.

## Project references

Read the relevant documents rather than copying their full rules into feature code:

- Current architecture and product baseline: `AGENTS.md`, `docs/development-baseline-0.2-2026-10-19.md`
- Feature priorities and storage decisions: `docs/functional-development-plan.md`
- Theme and shared UI contracts: `docs/ui-theme.md`, `docs/ui-reference-workflow.md`, `docs/ui-refresh-contract.md`
- Architecture and governance checks: `docs/engineering-governance.md`
- Rust selection and FileAtlas MCP workflow: `docs/rust-toolchain.md`, `crates/fileatlas-plugin-mcp/PORTS.md`
- Backend source semantics: `vendor/photag-source/` and `docs/source-feature-audit.md` (historical mapping; confirm actual current code and behavior)
- Per-feature API and limits: the owning crate's `PORTS.md`

