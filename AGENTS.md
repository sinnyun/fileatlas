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
- `photag-ui-bridge` is the shared UI registration contract and catalog.
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
- `photag-plugin-gallery` ports the source skeleton/window data model and owns
  its UI contribution metadata. Do not treat that service and its host-neutral keyboard selection model as
  completion of the GPUI gallery view, pointer selection, date grouping, or
  host key-event/action integration. The `filter` module maps source filter state
  into the existing database query; the original filter controls and settings
  persistence still require GPUI migration.
- `photag-plugin-navigation` preserves sidebar folder/tag tree data and routes
  tag mutations through the source query layer. It coordinates folder add,
  rescan, remove, scanner cancellation and watcher registration; keep native
  sidebar widgets/pickers, GPUI action dispatch, tag drag/drop and context menus
  marked incomplete until their GPUI event flow is implemented.
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

Record exact commands and outcomes in `docs/validation-report.md`. Rust checks
must run on a supported toolchain. Windows-only UI, shell, drag/drop, file
operations, and system integration require real Windows validation; mark them
unverified until tested.
