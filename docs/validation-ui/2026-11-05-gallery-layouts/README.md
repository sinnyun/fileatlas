# FileManager gallery layouts and system thumbnails — 2026-11-05

Reference: [gallery layout concept](../../reference-ui/concepts/gallery-layouts-2026-11-05.png) and its checklist.

## Implementation state

- Ordinary FileManager cards ask the Windows Shell for a cached thumbnail only (`THUMBNAILONLY | INCACHEONLY`). A cache miss uses the shared file-type placeholder.
- Shell bitmap pixels are copied without resizing to a versioned PNG transport file under `FileAtlas/thumbs/system-shell`; the card uses `Contain` on the shared neutral frame. Legacy custom card thumbnails and micro-thumbnails are ignored.
- Index scans no longer decode ordinary image sources or invoke FFmpeg. The legacy missing-thumbnail hook completes without generating ordinary card artwork.
- Waterfall and justified layouts, layout-specific options, and their interaction review remain unimplemented. The active concept is the target reference, not a claim of visual parity.

## Verification

- `cargo +1.97.1 check --locked -j 2 -p fileatlas-plugin-thumbnails -p fileatlas-plugin-indexing -p fileatlas-plugin-file-manager` — passed.
- `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell` — passed; linker emitted existing `LNK4098` default-library conflict warning.
- `cargo +1.97.1 fmt --all -- --check` — pending final rerun.
- `node scripts/check-ui-theme.cjs`, `node scripts/check-ui-refresh.cjs`, `node scripts/check-architecture.cjs`, and `node scripts/check-feature-ledger.cjs` — passed before final small card/cache edits; rerun before closeout.
- Same-process MCP (`dev-test` isolated profile, access Off) returned workspace/UI state and captured `filemanager-gallery-current.png` at `.local/validation/profiles/gallery-layouts-system-thumbnails-retry/screenshots/`. The visible folder was empty, so this screenshot verifies window startup only; card aspect fit, Shell cache hit/miss, and alternate layout interactions were not observable in this run.
- Windows Shell-provider behavior depends on an existing Shell thumbnail-cache entry. Compilation does not establish format/provider coverage. No automated tests were run.

Runtime profiles, screenshots and reports stay under `.local/validation/profiles/` and are intentionally not checked in.
