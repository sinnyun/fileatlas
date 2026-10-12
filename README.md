# PhoTag Rust plugin migration

This repository contains a staged Rust migration of PhoTag. The pinned source,
feature audit, Cargo workspace, Cordis-managed SQLite plugin, lifecycle-owned UI
contribution bridge, and GPUI Kit shell are present. **This is an incomplete
migration; the gallery, preview, file operations, and many other product features are not yet at parity. Folder indexing and thumbnail backfill have an initial tested migration, but are not yet validated against real libraries and Windows media.**

- [Source pins](SOURCE_PINS.md)
- [Feature/source mapping and compatibility audit](docs/source-feature-audit.md)
- [Validation report](docs/validation-report.md)
- [Codex instructions](AGENTS.md)

## Workspace

- `crates/photag-shell`: Cordis runtime composition and GPUI Kit host; mounts
  plugin-created views into named app slots and dispatches UI command events.
- `crates/photag-ui-bridge`: typed UI contributions, GPUI view factories, and
  Cordis lifecycle cleanup for both metadata and executable mounts.
- `crates/photag-plugin-database`: original schema/query implementation wrapped
  as a Cordis service plugin, with separate reader/writer connections.
- `crates/photag-plugin-thumbnails`: original image thumbnail decoding and
  cache pipeline exposed as a Cordis service.
- `crates/photag-plugin-indexing`: recursive scanning, database indexing, poster pairing, thumbnail backfill and directory listing exposed through a Cordis service.
- `crates/photag-plugin-watcher`: registered-folder filesystem notifications, debounced SQLite reconciliation, thumbnail backfill and Cordis media-index change events.
- `crates/photag-plugin-gallery`: compact query results, windowed media loading with 200-row paging, filter/selection models, and a GPUI media-card view with cached thumbnails, plain/Ctrl/Shift selection, batch and keyboard 0–5 ratings, arrow/Home/End/Ctrl+A/Escape keys, media-type controls, all eight source sort orders, inclusive 0–5 rating range controls, include/exclude/clear extension filters, typed inclusive date-range filters, live search and Cordis-driven folder/tag/path selection; automatic viewport paging, keyboard target scrolling, Enter/preview, Delete confirmation, progressive thumbnail behavior, native date picker and full source filter-chip interactions remain pending.
- `crates/photag-plugin-navigation`: folder/tag snapshots, lazy subdirectory counts and expansion, database-backed tag commands, scan/watch-coordinated folder actions, and a GPUI folder/tag tree with path-based Add, rescan/remove, scan progress, and Cordis-driven folder/tag/path filters; the native folder picker remains pending.
- `crates/photag-plugin-tagging`: source-compatible media tag assignment, associated tags, automatic ancestor links, removal, clearing, and tag merge behind a Cordis service; assignment UI and XMP metadata writeback remain pending.
- `crates/photag-contracts`: reserved for stable domain contracts; migration is
  in progress.
- `vendor/photag-source`: unmodified source mirror for the pinned PhoTag commit.

## Build status

Run `cargo check --workspace` and `cargo test --workspace`. A Windows MSVC
toolchain and native dependencies are required for Windows execution. See
`docs/validation-report.md` for tested commands and remaining platform checks.
