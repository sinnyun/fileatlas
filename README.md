# PhoTag Rust plugin migration

This repository contains a staged Rust migration of PhoTag. The pinned source,
feature audit, Cargo workspace, Cordis-managed SQLite plugin, lifecycle-owned UI
contribution bridge, and GPUI Kit shell are present. **This is an incomplete
migration; the gallery, preview, file operations, and many other product features are not yet at parity. Folder indexing and thumbnail backfill have an initial migration; settings and session persistence now retain their PhoTag JSON schemas behind Cordis services. These slices are not yet validated against real libraries and Windows media.**

- [Source pins](SOURCE_PINS.md)
- [Feature/source mapping and compatibility audit](docs/source-feature-audit.md)
- [Validation report](docs/validation-report.md)
- [Cloud development environment guide](docs/cloud-development-environment.md)
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
- `crates/photag-plugin-gallery`: compact query results, windowed media loading with 200-row paging, filter/selection models, and a GPUI media-card view with cached thumbnails, plain/Ctrl/Shift selection, batch and keyboard 0–5 ratings, arrow/Home/End/Ctrl+A/Escape keys, media-type controls, all eight source sort orders, inclusive 0–5 rating range controls, include/exclude/clear extension filters, typed inclusive date-range filters, filename/path/tag search scopes, live search, active search/type/date/rating/extension filter pills, clear-all filters, Cordis-driven folder/tag/path selection and a selected-item details panel for path/type/size/dimensions/date/rating/tags; automatic viewport paging, keyboard target scrolling, Enter/preview, Delete confirmation, progressive thumbnail behavior, native date picker and tag filter pills remain pending.
- `crates/photag-plugin-navigation`: folder/tag snapshots, lazy subdirectory counts and expansion, database-backed tag commands, scan/watch-coordinated folder actions, and a GPUI folder/tag tree with path-based Add, rescan/remove, scan progress, and Cordis-driven folder/tag/path filters; the native folder picker remains pending.
- `crates/photag-plugin-tagging`: source-compatible media tag assignment, associated tags, automatic ancestor links, removal, clearing, tag merge/reparent, and JPEG IPTC/XMP sidecar writeback behind a Cordis service; assignment UI and user-visible metadata-write errors remain pending.
- `crates/photag-plugin-settings` and `crates/photag-plugin-session`: PhoTag-compatible JSON settings/session schemas and Cordis persistence services; GPUI controls and applying/restoring live state remain pending.
- `crates/photag-plugin-metadata-export`: source-compatible all-library metadata records and pretty JSON writing behind a Cordis service; save dialog and command execution remain pending.
- `crates/photag-plugin-file-operations`: Cordis service for source-compatible single/bulk recycle, copy, move, rename, folder creation, open, and reveal actions; user dialogs and UI action wiring remain pending.
- `crates/photag-contracts`: reserved for stable domain contracts; migration is
  in progress.
- `vendor/photag-source`: unmodified source mirror for the pinned PhoTag commit.

## Build status

For the cloud code-generation pass, use incremental compilation and bounded jobs:
`CARGO_INCREMENTAL=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo check -p photag-shell --locked`. Run `cargo fmt --all -- --check` separately. Plugin tests are deferred to the local second-round debugging pass by request. A Windows MSVC toolchain and native dependencies are required for Windows execution. See `docs/validation-report.md` for exact results and remaining platform checks.
