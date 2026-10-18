# PhoTag workspace instructions

## Status

### Development baseline and current phase (2026-10-14)

The user accepted the current interface as development baseline
`dev-baseline-2026-10-14`. The current phase is **functional operation
optimization and feature development**, not continued wholesale UI redesign.
Read `docs/development-baseline-2026-10-14.md` and
`docs/functional-development-plan.md` before continuing. Preserve the accepted
layout unless the user requests a change or a feature needs a targeted update.
The product direction is a personal all-purpose workbench for the user's own
use: preserve the photo library as a completed core capability while adding
distinct, optional capabilities as compile-time Cordis feature plugins. Future
plugins may serve unrelated needs and may integrate scripts, links, or suitable
open-source libraries; do not assume a fixed product category or that every
proposed idea will be built.

**Automatic continuation:** when the user says “continue” without naming a
section, advance the first incomplete F0–F3 item in the functional development
plan. Start F0 with real folder/filter/selection/preview flows, then tagging,
undo/redo, file operations and indexing feedback. Explicit new feature requests
take priority and should be documented as feature plugins with clear purpose
and behavior. Existing architecture limits remain.

The C0–C4/V1–V4 ledger in
`docs/development-handoff-2026-10-14.md` is historical UI-phase context. Its
coding-only restrictions and old 19-error build snapshot do not define the
current work. Carry remaining unverified operations into F0; do not call them
passed merely because the layout was accepted. See
`docs/redesign-validation.md` for current evidence. Static checks use the
installed Rust1.97.1; live debugging, input and screenshots use same-process
PhoTag MCP with isolated test profiles. The client is
`scripts/photag-mcp-debug.cjs`.

Automatic startup update offers remain suppressed; explicit manual checks and
installation remain available. Main-window panel design is
`docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png` plus its
companion notes. Other screens follow `docs/reference-ui/final-v5/` and
concepts notes. Mockups are visual direction, not pixel measurements.

`vendor/photag-source/` remains the reference for backend behavior, data
semantics and queries. The migration-phase documents
`docs/source-feature-audit.md`, `docs/validation-report.md` and
`docs/reference-ui/source-layout-spec.md` are historical records of that phase:
useful for locating existing behavior, but they are not the current interface
specification and must not be used to demand source UI parity. Feature
completeness statements in them describe the state at the time they were
written.

## Architecture target

- Rust Cargo workspace, Windows desktop target.
- `cordis-rs` owns plugin lifecycle, typed services, events, dependencies, and
  cleanup. Do not build a parallel plugin manager.
- GPUI and GPUI Kit own native GUI rendering. Do not add egui to the new app.
- `photag-shell` is only startup, runtime construction, and top-level app
  composition. Product behavior belongs to compile-time feature crates.
- The personal workbench grows through independent compile-time feature
  plugins. Most feature UIs should reuse the existing generic floating-panel
  facility. Plugins implement only their own feature behavior and internal
  layout, and reuse the host's existing data, persistence, settings, events,
  commands, and UI infrastructure through public contracts wherever those
  facilities apply. Do not create duplicate infrastructure or assume each
  plugin needs its own data subsystem. Cordis owns plugin lifecycle and cleanup.
  This does not imply dynamic DLL loading or an external plugin store.
- Feature UI contributions (layout/dock/panel/slot/widget/command/menu/keymap)
  must have lifecycle ownership and be removed on plugin stop.
- `photag-ui-bridge` is the shared UI registration contract, metadata catalog,
  executable GPUI view-factory catalog, and Cordis-owned cleanup boundary.
- `photag-plugin-database` owns the source-compatible SQLite data layer. Keep
  all DB access behind its Cordis service as the redesign proceeds.
- `photag-plugin-thumbnails` retains PhoTag image thumbnail/cache generation.
- `photag-plugin-indexing` owns recursive scanning and thumbnail backfill; its
  service is the public entry point for scan progress/cancellation streams. Its
  current regression test uses a synthetic image and paired video poster; it
  does not establish Windows behavior or full media-format coverage.
- `photag-plugin-watcher` owns registered-folder notifications and database
  reconciliation. Keep its 800 ms debounce and bounded event/path processing;
  native event delivery and filesystem race behavior still need Windows tests.
- `photag-plugin-gallery` ports the source skeleton/window data model, paints
  cached image thumbnails, pages records in 200-row batches, and supports
  selection, ratings, keyboard navigation, filters, date grouping,
  rubber-band selection, named tag filters, hierarchical tag assignment,
  settings/session persistence, file actions, statistics, bulk operations,
  preview routing, and Cordis event reloads. Windows behavior remains
  unverified.
- `photag-plugin-navigation` preserves sidebar folder/tag tree data, lazy-loads
  nested directories, coordinates folder add/rescan/remove, scanner
  cancellation and watcher registration, emits Cordis selection events, and
  provides native folder picking, tag editing menus, drag placement, and
  persisted expansion. Windows interaction remains unverified.
- `photag-plugin-tagging` owns media/tag assignments and association/merge/
  reparent commands through source database queries. It reuses PhoTag's XMP/
  IPTC writer, schedules metadata rewrites after mutations, and exposes
  writeback failures for retry or dismissal in Gallery. Temporary-media tests
  cover IPTC/XMP updates, timestamps, writeback failures and queue coalescing;
  real camera/editor interoperability remains unverified.
- `photag-plugin-settings` and `photag-plugin-session` preserve the source JSON
  models and paths as Cordis services. Settings controls and active-view state
  restore/save paths are integrated; their panels are rebuilt on the current
  design while keeping these persisted models compatible.
- `photag-plugin-metadata-export` wraps PhoTag's complete metadata query and
  JSON writer. Its command, native JSON save dialog, progress, result, and
  error feedback are integrated; Windows interaction remains unverified.
- `photag-plugin-file-operations` preserves source single/bulk file and
  database operation order and emits `MediaIndexChanged` after row mutations.
  Gallery and Preview integrate destination/confirmation dialogs, bulk actions,
  selection repair, and filmstrip follow-through; native Windows behavior
  remains unverified.
- `photag-plugin-dev-test` is opt-in development tooling, not product behavior.
  It scans `test-data/photag-library` through the production Indexing service
  and seeds up to 100,000 virtual rows in an isolated DB. Its profile must keep
  the normal PhoTag DB, settings, session and thumbnail cache untouched.
- Reuse the pinned PhoTag implementation and data semantics for backend
  behavior. Never replace working behavior with placeholder behavior, and never
  report partial coverage as complete.
- This is a personal-use workbench. There is no project-wide allowlist or
  restriction on open-source licenses for feature dependencies; choose the
  library that best provides the requested behavior. Keep upstream attribution
  and license files, record the license and pinned revision, and note any
  distribution obligations. The user accepts GPL-3.0 dependencies for personal
  use; do not silently change the workspace's declared license.

## Implementation rules

- Global styling follows the clipboard-v2-derived workbench theme. Before UI work,
  read `docs/ui-theme.md` and its active concept/checklist. `photag-ui-bridge::theme`
  owns colors, typography, spacing, component states and GPUI Kit Theme;
  `components` owns shared cards/navigation/actions/headings, and `panels` owns
  floating chrome. Reuse these public facilities; do not add plugin-local palettes,
  hardcoded paint colors or independent Theme updates. Preserve feature-specific
  layout and existing Settings/Session geometry persistence. Run
  `node scripts/check-ui-theme.cjs` alongside the other governance checks, and
  verify default/changed accent, selected/hover/focus/disabled and small-window
  states through same-process PhoTag MCP. Full developer rules: `docs/ui-theme.md`.

- Before any interface change, read `docs/ui-reference-workflow.md` and resolve
  the active screen reference in `docs/reference-ui/README.md`. Open the actual
  image, not only its filename or generation prompt. Convert its visible
  structure, alignment, sizing hierarchy and states into an implementation
  checklist before editing. Compare a same-process MCP screenshot with that
  checklist after editing. “Visual direction, not pixel measurements” permits
  adaptive sizing; it does not permit replacing horizontal rows with stacked
  controls, omitting designed states, or declaring clipped content complete.
  Document every remaining discrepancy and its reason. Do not mark visual
  acceptance passed merely because compilation or a state assertion passed.
- Trace source behavior and callers in `vendor/photag-source/` before changing
  backend/data behavior; update the mapping table in the same change.
- Interface work follows the user's current design. Before implementing a
  screen, read the corresponding `docs/reference-ui/final-v5/` mockup and its
  concepts notes, then reproduce its structure, controls, and states in GPUI
  Kit. The user's design decisions and in-session direction override any older
  source-layout document. Record which mockup a change implements, and note
  designed controls or states that are still missing from the app.
- For a substantial visual/style change to a project interface, first use the
  built-in image generation capability to create a concept image, then
  implement against that visual reference. Describe the visual hierarchy,
  palette, spacing/gaps, component separation, and representative states
  (such as selected, hover, and nested rows) explicitly in the prompt. Inspect
  the concept before coding; revise it when the user gives visual feedback.
  Save project-relevant concepts under `docs/reference-ui/concepts/` and record
  the concept used in the validation notes. Treat generated images as visual
  direction, not pixel measurements or product data. After implementation,
  compare a same-process PhoTag MCP screenshot with the concept, refine visible
  mismatches, and record the result and remaining gaps. Small local tweaks do
  not need a new concept unless the user asks for one.
- Preserve SQLite schema and existing `%LOCALAPPDATA%\\PhoTag` files unless a
  tested compatibility migration is supplied.
- Keep Rust files below 500 lines where practical and below 800 lines.
- Plugins communicate through public contracts; avoid cross-plugin private
  imports and duplicated framework facilities.
- Keep source attribution/license notices when copying upstream code.
- No dynamic DLL loading, external plugin store, sandbox, or cloud sync.
- Do not add TODOs, mocks, or empty handlers as substitutes for functionality.

## Local Rust toolchain selection

This Windows host already has Rust toolchains above the workspace MSRV installed.
Its default `stable` is Rust 1.94.1, while `Cargo.toml` requires Rust 1.95.
Do not repeatedly run unqualified Cargo build/check/test commands on this host:
they select the old default and fail before checking the project.

- At the start of a build/validation session, inspect `rustup toolchain list`.
- Use the installed `1.97.1-x86_64-pc-windows-msvc` explicitly here:
  `cargo +1.97.1 ...`, including fmt, check, build, test, run, clippy and tree.
- Keep `--locked`, bounded jobs (`-j 2` for builds/tests) and incremental settings.
- If installed toolchains change, choose an already-installed Windows MSVC
  toolchain meeting the current workspace `rust-version`; verify with
  `rustc +<toolchain> --version`. Only install a toolchain if none is suitable.
- An old default is not evidence that Rust is missing or that the project/GPUI
  Kit is incompatible. Select the suitable installed toolchain first; do not
  lower `rust-version`, change the global default or bypass the version check.

See `docs/rust-toolchain.md` for exact local commands and MCP configuration.

## Verification

Redesign-phase changes are verified against the current design, not against the
egui source. Run the checks needed for the change, including
`cargo fmt --all -- --check`, locked shell compilation, and relevant crate or
workspace tests. Use bounded parallel build jobs and keep incremental
compilation enabled when practical. Do not describe a compile or headless test
as proof that an interface works.

For interface work, launch the Windows application and exercise the redesigned
flows, comparing the rendered window against the corresponding
`docs/reference-ui/final-v5/` mockup: layout, visible controls, states, and
copy. Record the mockup checked, exact commands, environment, outcomes, and the
designed controls or states still missing in `docs/redesign-validation.md`.
Windows UI, shell, drag/drop, file operations, codecs, and system integration
remain unverified until actually exercised.

For backend behavior, keep comparing the affected feature with its pinned
implementation under `vendor/photag-source/` and record results in
`docs/validation-report.md`, which continues to hold the migration-phase
history. Keep each crate's `PORTS.md` aligned with observed behavior and public
signatures.

For live debugging, runtime tests, and interface/interaction verification,
**always use the project's PhoTag MCP plugin directly**. This is the required
workflow, not an optional fallback. Do not substitute generic computer-use,
OS mouse/keyboard injection, or a separately launched application for plugin
MCP debugging. Cargo fmt/check/build/test remain required static/build and
automated regression checks; they do not replace live MCP verification.

Configure the MCP client to launch
`photag-shell --mcp`, then use its tools to read the running window, library,
media, preview/playback, and (with `dev-test` enabled) fixture/scale-test state.
The MCP server and GPUI window share that child process; it cannot attach to a
separately launched PhoTag process. Use an isolated `dev-test` profile for
fixture scans and virtual rows. Consult `crates/photag-plugin-mcp/PORTS.md` for
the tool list and startup diagnostics. MCP mutations remain disabled unless
`PHOTAG_MCP_ALLOW_MUTATIONS=1` is explicitly set; keep them disabled for
read-only validation and only enable them when the validation requires a
mutation.

If a required interaction or state is not exposed by the plugin, inspect and
extend the PhoTag MCP public tools and their PORTS.md as part of the debugging
work, then test through those tools. If MCP startup or access is blocked,
record the exact blocker and leave the affected runtime path unverified;
do not silently switch to generic UI automation or claim runtime coverage.

Each crate must maintain a `PORTS.md` that documents Cordis plugin/service
identity, startup dependencies, input/output payloads, event producers and
consumers, current limits, and where to inspect failures. Keep it aligned with
actual public Rust signatures; it is an integration/debug guide, not a claim
that all behavior is complete.

## UI refresh and passive read ownership (2026-10-15)

- All GPUI timer-driven model/view polling uses `photag_ui_bridge::refresh::poll`.
  Return `PollResult` with actual changes; a timer wake, empty receiver, or an
  operation merely remaining active is not a reason to notify. Finite workers
  use `PollResult::until(changed, active)` so completion stops the timer.
- Use `refresh::ChangeGate` for passive state signatures/duplicate pointer input.
  Playback keys include changing positions/frames; settled animations return idle.
- Passive DB/filesystem/metadata reads use `read_model::ReadModel` with explicit
  identity/revision keys. Cache failures, coalesce requests, discard stale results.
  Render may declare a nonblocking request and read its snapshot; it must not
  execute IO or log/notify repeatedly. Pollers deliver read completions.
- Preserve event/action semantics and fresh action-time target validation.
  Do not hide logs to disguise repeated work. New/changed flows require same-process
  MCP quiet-state and real-change checks. Run `node scripts/check-ui-refresh.cjs`,
`node scripts/check-architecture.cjs` and `node scripts/check-feature-ledger.cjs`
(see `docs/engineering-governance.md`; architecture baselines regenerate with
`--update-baseline` after reviewing the diff).
- Contract and examples: `docs/ui-refresh-contract.md`.
