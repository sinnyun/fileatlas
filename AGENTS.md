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
- Preserve SQLite schema and existing `%LOCALAPPDATA%\\PhoTag` files unless a
  tested compatibility migration is supplied.
- Keep Rust files below 500 lines where practical and below 800 lines.
- Plugins communicate through public contracts; avoid cross-plugin private
  imports and duplicated framework facilities.
- Keep source attribution/license notices when copying upstream code.
- No dynamic DLL loading, plugin store, sandbox, or cloud sync.
- Do not add TODOs, mocks, or empty handlers as substitutes for functionality.

## Verification

During the cloud code-generation pass, run only `cargo fmt --all -- --check` and
`cargo check -p photag-shell --locked` (with incremental compilation and bounded
parallel jobs). Do not run workspace or individual plugin tests in this pass;
reserve those for the user's second-round concentrated debugging. The shell
check compiles the host dependency graph but does not prove plugin behavior or
produce a tested Windows executable. Record exact commands and outcomes in
`docs/validation-report.md`. Windows-only UI, shell, drag/drop, file operations,
and system integration require real Windows validation; mark them unverified
until tested.

Each crate must maintain a `PORTS.md` that documents Cordis plugin/service
identity, startup dependencies, input/output payloads, event producers and
consumers, current limits, and where to inspect failures. Keep it aligned with
actual public Rust signatures; it is an integration/debug guide, not a claim
that all behavior is complete.
