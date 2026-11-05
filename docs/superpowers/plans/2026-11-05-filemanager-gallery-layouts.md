# FileManager Gallery Layouts Implementation Plan

> **For agentic workers:** Execute inline in this session. Follow the project Windows MCP workflow for runtime UI review when available.

**Goal:** Add Waterfall and Justified layouts, configurable Grid/List display options, aspect-preserving uncropped thumbnails, and Windows Shell thumbnails for FileManager without generating card thumbnails in FileAtlas.

**Architecture:** Keep behavior in `fileatlas-plugin-file-manager`, persist mode/options per pane through the existing `ColumnDefinition`, and isolate pure geometry from card rendering. Add a Windows Shell thumbnail adapter to `fileatlas-plugin-thumbnails`; FileManager reads Shell-provided images only, while unrelated feature-owned previews retain their existing contracts.

**Tech Stack:** Rust workspace, GPUI/GPUI Kit, Cordis services, Windows Shell `IShellItemImageFactory`, existing SQLite and versioned workspace JSON.

## Global Constraints

- Preserve SQLite schema and existing user files unless a tested compatibility migration is supplied.
- FileManager feature behavior stays in its Cordis feature crate; use shared theme and UI Bridge components.
- Persist FileManager mode/options in existing versioned workspace JSON and preserve old `list_layout` values.
- Use `cargo +1.97.1`, `--locked`, and bounded jobs for build/check commands on this host.
- Do not generate FileManager card thumbnails from source files; use Windows Shell output or a file-type placeholder.
- Keep thumbnails aspect-preserving and uncropped in every gallery mode.

## File map

- `crates/fileatlas-plugin-thumbnails/src/`: Windows Shell bitmap request, lossless transport cache and public service contract.
- `crates/fileatlas-plugin-indexing/src/`: remove source-decoder/FFmpeg backfill from the ordinary FileManager thumbnail path; expose Shell-backed lookup.
- `crates/fileatlas-plugin-file-manager/src/view/`: gallery geometry, layout state, option controls, uncropped card fitting and virtualized visible placements.
- `crates/fileatlas-plugin-workspaces/src/lib.rs`: version-compatible per-column layout and display options.
- `docs/reference-ui/` and `docs/validation-ui/`: active concept checklist and observed implementation boundaries.

## Tasks

1. Implement the Shell thumbnail adapter and update FileManager lookup so ordinary cards never fall back to FileAtlas decoders or FFmpeg.
2. Add a pure gallery layout model for Waterfall, Justified, Grid and List; preserve variable aspect ratios and visible-range lookup.
3. Extend per-column workspace persistence and add the layout/options controls with legacy mode migration.
4. Integrate placements with selection, drag/drop, grouping, list columns and viewport restoration; change image fitting to contain/letterbox.
5. Update PORTS, the active reference index, feature validation notes and governance records; run the applicable static checks and build, then perform same-process MCP UI review when available.
