# PhoTag workspace instructions

## Status

This repository is an **incomplete migration**, not a completed PhoTag
replacement. The selected PhoTag source is preserved in
`vendor/photag-source/`. Consult `docs/source-feature-audit.md` before making
claims about parity. Any feature not marked `Migrated` there must remain
explicitly incomplete.

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
  cached image thumbnails, pages records in 200-row batches, supports
  plain/Ctrl/Shift card selection, batch and keyboard 0–5 ratings, basic
  arrow/Home/End/Ctrl+A/Escape selection keys, media-type segmented controls,
  all eight source sort orders, inclusive 0–5 rating-range controls,
  include/exclude/clear extension filters, filename/path/tag search scopes,
  typed date filters and clear-all filters, and reloads on Cordis
  folder/tag/path/index events. A selected-item details panel shows indexed
  path, type, file size, dimensions, capture date, rating and tags.
  Automatic viewport paging, scrolling to
  keyboard targets, Enter/preview and Delete confirmation, date grouping,
  rubber-band selection, native date-picker controls, per-filter source pills,
  tag filter pills and settings persistence are incomplete.
- `photag-plugin-navigation` preserves sidebar folder/tag tree data, lazy-loads
  nested directories, accepts folder paths, coordinates folder add/rescan/
  remove, scanner cancellation and watcher registration, and emits Cordis
  folder/tag/path selection events. Native folder picking, tag editing menus,
  drag/drop, expansion persistence, and full visual parity are incomplete.
- `photag-plugin-tagging` owns media/tag assignments and association/merge/
  reparent commands through source database queries. It reuses PhoTag's XMP/
  IPTC writer and schedules best-effort metadata rewrites after mutations.
  Picker/context-menu UI, hotkeys, visible write errors, and validation of the
  new writeback remain incomplete.
- `photag-plugin-settings` and `photag-plugin-session` preserve the source JSON
  models and paths as Cordis services. Their UI adapters and applying/restoring
  state to active views remain incomplete.
- `photag-plugin-metadata-export` wraps PhoTag's complete metadata query and
  JSON writer. The native save dialog, command event handling, and completion
  feedback remain incomplete; do not report the export UI as migrated.
- `photag-plugin-file-operations` preserves source single/bulk file and
  database operation order and emits `MediaIndexChanged` after row mutations.
  Folder/save dialogs, bulk-action UI, and preview/selection follow-through
  remain incomplete.
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
