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

- `crates/photag-shell`: Cordis runtime composition and GPUI Kit host; currently
  renders the active contribution catalog and dispatches UI command events.
- `crates/photag-ui-bridge`: typed UI contributions and Cordis lifecycle cleanup.
- `crates/photag-plugin-database`: original schema/query implementation wrapped
  as a Cordis service plugin, with separate reader/writer connections.
- `crates/photag-plugin-thumbnails`: original image thumbnail decoding and
  cache pipeline exposed as a Cordis service.
- `crates/photag-plugin-indexing`: recursive scanning, database indexing, poster pairing, thumbnail backfill and directory listing exposed through a Cordis service.
- `crates/photag-plugin-watcher`: registered-folder filesystem notifications and debounced SQLite reconciliation.
- `crates/photag-plugin-gallery`: compact gallery query results, windowed media loading and gallery UI contribution metadata; the GPUI gallery itself is not yet migrated.
- `crates/photag-contracts`: reserved for stable domain contracts; migration is
  in progress.
- `vendor/photag-source`: unmodified source mirror for the pinned PhoTag commit.

## Build status

Run `cargo check --workspace` and `cargo test --workspace`. A Windows MSVC
toolchain and native dependencies are required for Windows execution. See
`docs/validation-report.md` for tested commands and remaining platform checks.
