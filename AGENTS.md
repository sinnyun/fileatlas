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
  and reloads on Cordis
  all eight source sort orders, inclusive 0–5 rating-range controls and
  include/exclude/clear extension filters, and
  reloads on Cordis folder/tag/path/index
  events. Automatic viewport paging, scrolling to
  keyboard targets, Enter/preview and Delete confirmation, date grouping,
  rubber-band selection, source search scopes, date filters
  and settings persistence are incomplete.
- `photag-plugin-navigation` preserves sidebar folder/tag tree data, lazy-loads
  nested directories, accepts folder paths, coordinates folder add/rescan/
  remove, scanner cancellation and watcher registration, and emits Cordis
  folder/tag/path selection events. Native folder picking, tag editing menus,
  drag/drop, expansion persistence, and full visual parity are incomplete.
- `photag-plugin-tagging` owns media/tag assignments and association/merge
  commands through source database queries. Keep picker/context-menu UI,
  hotkeys, and XMP rewrite integration marked incomplete until migrated.
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
