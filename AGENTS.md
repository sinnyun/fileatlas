# PhoTag workspace instructions

## Status

This repository is an **incomplete migration**, not a completed PhoTag
replacement. The selected PhoTag source is preserved in
`vendor/photag-source/`. `docs/source-feature-audit.md` is the canonical,
current feature-to-source map. Read its status descriptions before making
parity claims. Do not use this file's architecture notes as a migration status
list: older feature summaries below were superseded by the audit dated
2026-10-13. Distinguish implemented behavior from behavior that still needs
Windows, media-corpus, or regression validation.

## Architecture target

- Rust Cargo workspace, Windows desktop target.
- `cordis-rs` owns plugin lifecycle, typed services, events, dependencies, and
  cleanup. Do not build a parallel plugin manager.
- GPUI and GPUI Kit own native GUI rendering. Do not add egui to the new app.
- `photag-shell` is only startup, runtime construction, and top-level app
  composition. Product behavior belongs to compile-time feature crates.
- Feature UI contributions (layout/dock/panel/slot/widget/command/menu/keymap)
  must have lifecycle ownership and be removed on plugin stop.
- `photag-ui-bridge` is the shared UI registration contract, metadata catalog,
  executable GPUI view-factory catalog, and Cordis-owned cleanup boundary.
- `photag-plugin-database` owns the source-compatible SQLite data layer. Keep
  all DB access behind its Cordis service as migration proceeds.
- `photag-plugin-thumbnails` retains PhoTag image thumbnail/cache generation.
- `photag-plugin-indexing` owns recursive scanning and thumbnail backfill; its
  service is the public entry point for scan progress/cancellation streams. Its
  current regression test uses a synthetic image and paired video poster; it
  does not establish Windows or full-format parity.
- `photag-plugin-watcher` owns registered-folder notifications and database
  reconciliation. Keep its 800 ms debounce and bounded event/path processing;
  native event delivery and filesystem race behavior still need Windows tests.
- `photag-plugin-gallery` ports the source skeleton/window data model, paints
  cached image thumbnails, pages records in 200-row batches, and supports
  source-style selection, ratings, keyboard navigation, filters, date grouping,
  rubber-band selection, named tag filters, hierarchical tag assignment,
  settings/session persistence, file actions, statistics, bulk operations,
  preview routing, and Cordis event reloads. See the canonical audit for the
  current interaction inventory. Windows behavior and visual parity remain
  unverified.
- `photag-plugin-navigation` preserves sidebar folder/tag tree data, lazy-loads
  nested directories, coordinates folder add/rescan/remove, scanner
  cancellation and watcher registration, emits Cordis selection events, and
  provides native folder picking, tag editing menus, drag placement, and
  persisted expansion. Windows interaction and visual parity remain
  unverified.
- `photag-plugin-tagging` owns media/tag assignments and association/merge/
  reparent commands through source database queries. It reuses PhoTag's XMP/
  IPTC writer, schedules metadata rewrites after mutations, and exposes
  writeback failures for retry or dismissal in Gallery. Temporary-media tests
  cover IPTC/XMP updates, timestamps, writeback failures and queue coalescing;
  real camera/editor interoperability remains unverified.
- `photag-plugin-settings` and `photag-plugin-session` preserve the source JSON
  models and paths as Cordis services. Settings controls and active-view state
  restore/save paths are integrated; see the audit for remaining validation
  limits.
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
- Reuse the pinned PhoTag implementation and data semantics. Never replace
  working behavior with placeholder behavior or report partial parity as full.

## Implementation rules

- Trace source behavior and callers in `vendor/photag-source/` before migration;
  update the mapping table in the same change.
- For every UI layout, style, or control-placement change, inspect the pinned
  source implementation and its exact dimensions, ordering, spacing, typography,
  colors, and interaction state before editing the migrated plugin. Screenshots
  are for checking the rendered result only; never infer layout values from
  pixels when source code provides them. Follow
  `docs/reference-ui/source-layout-spec.md` and add source file/line references
  to the change notes or validation record. If a value is computed or inherited
  from egui defaults, document that actual source rule instead of inventing a
  GPUI value. Keep every visible source control and state represented in its
  corresponding feature plugin; explicitly record source UI that has no
  migrated counterpart.
- Complete and review the source layout specification before making broad UI
  restructuring changes. Work from its source-to-plugin component map so
  framework edits proceed region by region without speculative placement.
- Preserve SQLite schema and existing `%LOCALAPPDATA%\\PhoTag` files unless a
  tested compatibility migration is supplied.
- Keep Rust files below 500 lines where practical and below 800 lines.
- Plugins communicate through public contracts; avoid cross-plugin private
  imports and duplicated framework facilities.
- Keep source attribution/license notices when copying upstream code.
- No dynamic DLL loading, plugin store, sandbox, or cloud sync.
- Do not add TODOs, mocks, or empty handlers as substitutes for functionality.

## Verification

The project is in the local validation phase. Run the checks needed for the
change, including `cargo fmt --all -- --check`, locked shell compilation, and
relevant crate or workspace tests. Use bounded parallel build jobs and keep
incremental compilation enabled when practical. Do not describe a compile or
headless test as proof of interactive parity.

For migration validation, compare each source feature in
`docs/source-feature-audit.md` with its pinned implementation under
`vendor/photag-source/` and its migrated counterpart. Start the Windows desktop
application locally and exercise the corresponding workflows where the
interactive environment permits. Record exact commands, environment, outcomes,
and untested cases in `docs/validation-report.md`; mark Windows UI, shell,
drag/drop, file operations, codecs, and system integration unverified until
they have actually been exercised. Keep the audit map and each crate's
`PORTS.md` aligned with observed behavior and public signatures.

For live verification and debugging, use the project's PhoTag MCP plugin when
an MCP client is available: configure the client to launch
`photag-shell --mcp`, then use its tools to read the running window, library,
media, preview/playback, and (with `dev-test` enabled) fixture/scale-test state.
The MCP server and GPUI window share that child process; it cannot attach to a
separately launched PhoTag process. Use an isolated `dev-test` profile for
fixture scans and virtual rows. Consult `crates/photag-plugin-mcp/PORTS.md` for
the tool list and startup diagnostics. MCP mutations remain disabled unless
`PHOTAG_MCP_ALLOW_MUTATIONS=1` is explicitly set; keep them disabled for
read-only validation and only enable them when the validation requires a
mutation.

Each crate must maintain a `PORTS.md` that documents Cordis plugin/service
identity, startup dependencies, input/output payloads, event producers and
consumers, current limits, and where to inspect failures. Keep it aligned with
actual public Rust signatures; it is an integration/debug guide, not a claim
that all behavior is complete.
