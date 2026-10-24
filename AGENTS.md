# FileAtlas workspace instructions

## Status

### Development baseline and current phase (FileAtlas 0.2, 2026-10-19)

The current project definition is FileAtlas 0.2, baseline
`dev-baseline-0.2-2026-10-19`. This is a project-development baseline, not the
Cargo/application release version (still `0.1.0`). Read
`docs/development-baseline-0.2-2026-10-19.md` and
`docs/development-baseline-2026-10-14.md` (historical layout baseline) together
with `docs/functional-development-plan.md` before continuing. The current phase is **functional operation
optimization and feature development**, not continued wholesale UI redesign.
Preserve the FileAtlas 0.2 layout and plugin architecture unless the user requests
a change or a feature needs a targeted update. The accepted multi-column and
program-workspace behavior is recorded in the 0.2 baseline and its validation
evidence; the October 14 document is historical context, not the current
complete layout specification.
FileAtlas is a personal Windows file browser and workbench manager. File browsing,
directory navigation, multi-column workspaces and program workspace management
are the product's primary purpose. Media indexing, thumbnails, tags and preview
are integrated capabilities of its file-management workspace. Add distinct,
optional capabilities as compile-time Cordis feature plugins. Future plugins may
serve unrelated needs and may integrate scripts, links, or suitable open-source
libraries; do not assume a fixed product category or that every proposed idea
will be built.

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
FileAtlas MCP with isolated test profiles. The client is
`scripts/fileatlas-mcp-debug.cjs`.

Automatic startup update offers remain suppressed; explicit manual checks and
installation remain available. The current complete main-window definition is
FileAtlas 0.2 in `docs/development-baseline-0.2-2026-10-19.md`; its active
multi-column visual reference is
`docs/reference-ui/concepts/multi-column-workspaces-2026-10-19.png` plus its
companion notes. The October 14 sidebar/preview image remains the reference for
the global sidebar and inspector structure. Other screens follow the active
entries in `docs/reference-ui/README.md`, including final-v5 where selected.
Mockups are visual direction, not pixel measurements.

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
- `fileatlas-shell` is only startup, runtime construction, and top-level app
  composition. Product behavior belongs to compile-time feature crates.
- The personal workbench grows through independent compile-time feature
  plugins. Most feature UIs should reuse the existing generic floating-panel
  facility. Plugins implement only their own feature behavior and internal
  layout, and reuse the host's existing data, persistence, settings, events,
  commands, and UI infrastructure through public contracts wherever those
  facilities apply. Do not create duplicate infrastructure or assume each
  plugin needs its own data subsystem. Cordis owns plugin lifecycle and cleanup.
  This does not imply dynamic DLL loading or an external plugin store.
- For every new feature, plan its persistence format from that feature's data shape:
  use a versioned JSON file for modest, whole-document settings/manifests/history;
  use the shared `fileatlas-plugin-database` SQLite service for growing/query-heavy,
  relational, concurrent or transactional data. Keep large binary assets in separate
  feature-owned files and reference them from structured records. Record the choice,
  data paths, migration and recovery behavior in the feature PORTS.md, following
  `docs/functional-development-plan.md`'s storage decision rules. Do not assume a
  plugin always needs SQLite or that each plugin should build a new storage framework.
- Feature UI contributions (layout/dock/panel/slot/widget/command/menu/keymap)
  must have lifecycle ownership and be removed on plugin stop.
- `fileatlas-ui-bridge` is the shared UI registration contract, metadata catalog,
  executable GPUI view-factory catalog, and Cordis-owned cleanup boundary.
- `fileatlas-plugin-database` owns the source-compatible SQLite data layer. Keep
  all DB access behind its Cordis service as the redesign proceeds.
- `fileatlas-plugin-thumbnails` retains FileAtlas image thumbnail/cache generation.
- `fileatlas-plugin-indexing` owns recursive scanning and thumbnail backfill; its
  service is the public entry point for scan progress/cancellation streams. Its
  current regression test uses a synthetic image and paired video poster; it
  does not establish Windows behavior or full media-format coverage.
- `fileatlas-plugin-watcher` owns registered-folder notifications and database
  reconciliation. Keep its 800 ms debounce and bounded event/path processing;
  native event delivery and filesystem race behavior still need Windows tests.
- `fileatlas-plugin-file-manager` owns the primary file-management window and
  workbench content view. It owns directory/file browsing, filtering, selection,
  view state and integrated media-specific actions. Its package identity, Cordis
  service ID, MCP names and runtime view keys use the file-manager naming
  consistently.
  It ports the source skeleton/window data model, paints
  cached image thumbnails, pages records in 200-row batches, and supports
  selection, ratings, keyboard navigation, filters, grouping, rubber-band
  selection, named tag filters, hierarchical tag assignment, settings/session
  persistence, file actions, statistics, bulk operations and Cordis event
  reloads. It supplies up to four independent browser views for the columns
  feature, each with local address, query, selection and scroll state. Ordinary
  file viewing is handed to the user's running QuickLook. Record platform and
  interaction coverage from the linked MCP evidence; do not describe all
  Windows paths as either verified or unverified as a group.
- `fileatlas-plugin-columns` owns the active single/dual/quad layout state and
  toolbar contribution. Shell composes the views; FileManager owns each pane's
  browsing behavior. Shared filter/view controls target the active pane.
- `fileatlas-plugin-workspaces` owns the versioned JSON program-workspace list,
  default selection and manager UI. It saves startup layout and each pane's
  browsing/view configuration, not media rows. Its normal path is
  `%LOCALAPPDATA%\\FileAtlas\\workspaces\\workspaces.json`; dev-test can supply
  an isolated profile path.
- `fileatlas-plugin-navigation` preserves sidebar folder/tag tree data, lazy-loads
  nested directories, coordinates folder add/rescan/remove, scanner
  cancellation and watcher registration, emits Cordis selection events, and
  provides native folder picking, tag editing menus, drag placement, and
  persisted expansion. Main-window folder/tag selection is routed to the
  currently active FileManager pane. Check per-flow validation notes for native
  picker, watcher and platform-specific coverage.
- `fileatlas-plugin-tagging` owns media/tag assignments and association/merge/
  reparent commands through source database queries. It reuses FileAtlas's XMP/
  IPTC writer, schedules metadata rewrites after mutations, and exposes
  writeback failures for retry or dismissal in FileManager. Temporary-media tests
  cover IPTC/XMP updates, timestamps, writeback failures and queue coalescing;
  real camera/editor interoperability remains unverified.
- `fileatlas-plugin-settings` and `fileatlas-plugin-session` preserve the source JSON
  models and paths as Cordis services. Settings controls and active-view state
  restore/save paths are integrated; their panels are rebuilt on the current
  design while keeping these persisted models compatible.
- `fileatlas-plugin-metadata-export` wraps FileAtlas's complete metadata query and
  JSON writer. Its command, native JSON save dialog, progress, result, and
  error feedback are integrated; Windows interaction remains unverified.
- `fileatlas-plugin-file-operations` preserves source single/bulk file and
  database operation order and emits `MediaIndexChanged` after row mutations.
  FileManager integrates destination/confirmation dialogs, bulk actions and
  selection repair; QuickLook is the external file-view handoff. Native Windows
  behavior remains subject to the specific validation evidence.
- `fileatlas-plugin-quicklook` sends a request to an already-running user-owned
  QuickLook instance. FileAtlas does not start, install, update or terminate it.
- `fileatlas-plugin-versions` and `fileatlas-plugin-version-info` provide Lore-backed
  per-file version history and persisted version metadata through the shared
  database service. This is distinct from global undo/redo.
- WorkCrate (`fileatlas-workcrate-contracts` plus the `workcrate-projects`,
  `workcrate-boards`, `workcrate-export` and `workcrate-ui` plugins) is a document
  manager for the official VectorCraft program. One project associates one native
  `.vectorcraft` document saved in the user's chosen folder; its pages are that
  document's real artboards. Projects keeps the association, references, main image
  and field presets as versioned JSON; Boards keeps the official executable/port
  configuration, the durable artboard catalogue and the PNG preview cache; Export
  runs official CLI PNG/JPEG/PDF output with atomic handoff; UI keeps project
  management, the page matrix and the material-bar forms on the current design.
- WorkCrate browsing never touches the official program: project, page and cached
  preview reads stay local to FileAtlas data and open no process or port, showing a
  placeholder instead. Create, sync, material-bar edits, file placement, canvas edits
  and export run through the official automation session; only `open_editor` starts
  the official GUI window. The official launcher takes exactly one read folder and one
  write folder, so a session's inputs collapse into the single deepest folder holding
  them. A canvas session is granted the document's own folder up front and is
  relaunched with a wider collapsed root only when the user points at a file elsewhere;
  a live GUI keeps the roots its owner launched it with, so a refused path there stays
  a visible error instead of silently restarting the user's program.
- WorkCrate canvas editing forwards every editing semantic to the official program.
  The page editor's edit mode only captures input, maps overlay pixels to document
  points, displays the official `selectionBounds` box, and moves that box as a visual
  prediction while the pointer is down. Selection, hit testing, moving, scaling,
  image place/link/update/relink and text editing all run as official pointer
  gestures or official commands. FileAtlas paints no artwork of its own, implements
  no crop, transform arithmetic, hit test or text layout, keeps no separate undo
  history, and does not link or patch VectorCraft sources.
- One automation surface serves both states: an editing session uses the live control
  port when the official GUI is reachable there, and otherwise runs the official
  headless session silently. A running GUI is never a reason to refuse an edit, and
  editing must never require the user to close the GUI or keep it running first.
- `fileatlas-plugin-dev-test` is opt-in development tooling, not product behavior.
  It scans `test-data/fileatlas-fixtures` through the production Indexing service
  and seeds up to 100,000 virtual rows in an isolated DB. Its profile must keep
  the normal FileAtlas DB, settings, session and thumbnail cache untouched.
- Reuse the pinned PhoTag upstream implementation and data semantics for backend
  behavior. Never replace working behavior with placeholder behavior, and never
  report partial coverage as complete.
- This is a personal-use workbench. There is no project-wide allowlist or
  restriction on open-source licenses for feature dependencies; choose the
  library that best provides the requested behavior. Keep upstream attribution
  and license files, record the license and pinned revision, and note any
  distribution obligations. The user accepts GPL-3.0 dependencies for personal
  use; do not silently change the workspace's declared license.

## Native theme ownership (2026-10-19; supersedes manual palette rules)

- GPUI Kit ThemeRegistry/ThemeConfig/Theme owns every plugin's colors, including
  custom-painted controls. Never introduce a Rust RGB palette, mode_color, private
  Theme state, or per-plugin Theme updates. UI Bridge theme/runtime is the sole
  application gateway. Existing role aliases only project native ThemeColor.
- Material is a compile-time Cordis generator plugin. It outputs native GPUI
  ThemeSet JSON from seed/style/contrast; it must not apply paints itself.
  Reuse its public service and existing Settings persistence, not another manager.
- The configurable UI palette is limited to eight semantic families: primary,
  neutral surface, text, secondary, success, warning, danger and info. Native
  foreground/background/interaction roles are tonal members of those families;
  the Material generator owns their role fan-out. Do not expose arbitrary
  per-role colors that create additional palette branches.
- Feature plugins consume UI colors only through GPUI Kit Theme roles and the
  shared UI Bridge components. Shared UI Bridge helpers own product styling;
  plugins may pass content data colors only for actual user data or native
  document artwork, never to style their own UI.
- Pair fills with their native foreground roles; media badges and custom widgets
  obey the same native theme. Preserve real media pixels and user tag data.
- Read docs/ui-theme.md and the Material plugin PORTS before extending themes.
  Run the theme guard including self-test: it also scans the bridge theme itself.
  Old manually coded palettes in historical validation notes are not a template.

## Implementation rules

- Global styling follows the clipboard-v2-derived workbench theme. Before UI work,
  read `docs/ui-theme.md` and its active concept/checklist. `GPUI Kit Theme` owns all resolved colors and component states;
  `fileatlas-ui-bridge::theme` owns the native theme gateway and product geometry;
  `components` owns shared cards/navigation/actions/headings, and `panels` owns
  floating chrome. Reuse these public facilities; do not add plugin-local palettes,
  hardcoded paint colors or independent Theme updates. Preserve feature-specific
  layout and existing Settings/Session geometry persistence. Run
  `node scripts/check-ui-theme.cjs` alongside the other governance checks, and
  verify default/changed accent, selected/hover/focus/disabled and small-window
  states through same-process FileAtlas MCP. Full developer rules: `docs/ui-theme.md`.

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
  screen, resolve its active reference in `docs/reference-ui/README.md`, read
  the linked mockup and concept notes, then implement its structure, controls
  and states in GPUI Kit. Use final-v5 only for screens where the active
  reference still selects it. The user's design decisions and in-session
  direction override older source-layout documents. Record which reference a
  change implements, and note designed controls or states that are still
  missing from the app.
- For a substantial visual/style change to a project interface, first use the
  built-in image generation capability to create a concept image, then
  implement against that visual reference. Describe the visual hierarchy,
  palette, spacing/gaps, component separation, and representative states
  (such as selected, hover, and nested rows) explicitly in the prompt. Inspect
  the concept before coding; revise it when the user gives visual feedback.
  Save project-relevant concepts under `docs/reference-ui/concepts/` and record
  the concept used in the validation notes. Treat generated images as visual
  direction, not pixel measurements or product data. After implementation,
  compare a same-process FileAtlas MCP screenshot with the concept, refine visible
  mismatches, and record the result and remaining gaps. Small local tweaks do
  not need a new concept unless the user asks for one.
- Preserve SQLite schema and existing `%LOCALAPPDATA%\\FileAtlas` files unless a
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
- If a running FileAtlas process prevents overwriting the target EXE, the user
  explicitly authorizes closing that process and rebuilding the normal EXE.
  Verify its executable path belongs to this workspace, close it normally first,
  and terminate only that verified process if it does not exit. Do not ask again.
  Reuse the existing target directory and incremental artifacts. Do not work
  around an EXE lock with a separately named EXE, a new target directory,
  `cargo clean`, or a full rebuild. See `docs/rust-toolchain.md`.
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

For interface work, launch the Windows application through the project's
same-process MCP and exercise the changed flows, comparing the rendered window
against its active reference from `docs/reference-ui/README.md`: layout, visible
controls, states and copy. Record the reference checked, exact commands,
environment, outcomes and remaining designed controls/states in
`docs/redesign-validation.md` and a focused `docs/validation-ui/` record.
Windows UI, shell, drag/drop, file operations, codecs, and system integration
remain unverified until actually exercised.

For backend behavior, keep comparing the affected feature with its pinned
implementation under `vendor/photag-source/` and record results in
`docs/validation-report.md`, which continues to hold the migration-phase
history. Keep each crate's `PORTS.md` aligned with observed behavior and public
signatures.

For live debugging, runtime tests, and interface/interaction verification,
**always use the project's FileAtlas MCP plugin directly**. This is the required
workflow, not an optional fallback. Do not substitute generic computer-use,
OS mouse/keyboard injection, or a separately launched application for plugin
MCP debugging. Cargo fmt/check/build/test remain required static/build and
automated regression checks; they do not replace live MCP verification.

For live validation configure the MCP client to launch
`fileatlas-shell --mcp` as an isolated stdio child, then use its tools to read the running window, library,
media, preview/playback, and (with `dev-test` enabled) fixture/scale-test state.
The MCP server and GPUI window share that child process; it cannot attach to a
separately launched FileAtlas process. Use an isolated `dev-test` profile for
fixture scans and virtual rows. Consult `crates/fileatlas-plugin-mcp/PORTS.md` for
the tool list and startup diagnostics. MCP access is controlled by Settings →
Plugin settings → MCP → AI control: Off is read-only, Standard permits UI
control and ordinary mutations, and Full permits plugin-declared high-impact
tools. High-impact calls still require per-call `confirm=true`. New profiles
default to Off; isolated validation may set a profile's `mcp_access_level` to
the minimum stage needed. Do not use the retired `FILEATLAS_MCP_ALLOW_MUTATIONS`
environment variable as an authorization mechanism.

All generated test data, intermediate MCP reports, logs and runtime screenshots
must live under the git-ignored `.local/validation/` directory. New MCP runs use
`.local/validation/profiles/<run-name>` for isolated profiles, fixtures, reports
and screenshots; reusable scenario scripts remain in `scripts/` or their feature
source directory. Keep concise human-readable findings and reproducible scenario
code in the repository. If a validation document links an output artifact, link
its corresponding `.local/validation/artifacts/<original-relative-path>` file.
Never check in generated screenshots, databases, profiles, raw MCP logs or result
JSON just to make evidence links portable; record their local-only location and
verification summary in Markdown instead. Existing `target/` profiles are legacy;
new validation runs must use `.local/validation/profiles/`.

If a required interaction or state is not exposed by the plugin, inspect and
extend the FileAtlas MCP public tools and their PORTS.md as part of the debugging
work, then test through those tools. If MCP startup or access is blocked,
record the exact blocker and leave the affected runtime path unverified;
do not silently switch to generic UI automation or claim runtime coverage.

Each crate must maintain a `PORTS.md` that documents Cordis plugin/service
identity, startup dependencies, input/output payloads, event producers and
consumers, current limits, and where to inspect failures. Keep it aligned with
actual public Rust signatures; it is an integration/debug guide, not a claim
that all behavior is complete.

## UI refresh and passive read ownership (2026-10-15)

- All GPUI timer-driven model/view polling uses `fileatlas_ui_bridge::refresh::poll`.
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
