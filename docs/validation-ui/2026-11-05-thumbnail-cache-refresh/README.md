# Thumbnail cache refresh — 2026-11-05

## Behavior

- Tools → **Refresh current view thumbnails** clears only Shell-cache references for IDs in the current pane query, removes their transport copies, advances the cache path revision, and reloads the pane.
- Tools → **Refresh all system thumbnail cache** clears matching media references and the FileAtlas-owned `thumbs/system-shell` directory, then emits `MediaIndexChanged` so open panes reload.
- Both operations use Shell cache-only lookup when cards are requested again. They do not generate thumbnails. Board/project and other feature-owned cache directories are preserved.

## Verification

- `cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-database system_thumbnail_cache_tests -- --nocapture` — passed. Verifies current-ID scoping, all-media clearing, and preservation of a media reference under `thumbs/boards`.
- `cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-thumbnails system_cache_clear_keeps_other_feature_owned_cache_directories -- --nocapture` — passed. Verifies selected-path removal, revision advance, all-system clearing, and preservation of the board cache directory.
- `cargo +1.97.1 check --locked -j 2 -p fileatlas-plugin-file-manager -p fileatlas-plugin-indexing -p fileatlas-plugin-mcp` — passed.
- `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell` — passed; linker reported the existing `LNK4098` default-library conflict warning.
- `cargo +1.97.1 fmt --all -- --check`, theme guard, refresh guard, architecture guard, and feature-ledger guard — passed.
- Same-process isolated MCP run: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/thumbnail-cache-refresh-20261105 --plugins core --access-level standard --scenario scripts/validate-thumbnail-cache-refresh.cjs`. Both operations were accepted and completed; screenshots show the Tools controls and the final “Refreshed 0 system thumbnail cache entries.” status.

Screenshots and the MCP result remain under `.local/validation/profiles/thumbnail-cache-refresh-20261105/` and are ignored by Git. The synthetic fixtures had no Windows Shell thumbnail-cache hits, so actual provider-backed pixel reload remains unverified; cache invalidation, UI routing, and the preservation boundary were verified by the isolated runtime and unit tests.
