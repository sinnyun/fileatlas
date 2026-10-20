# FileManager tag chips — 2026-10-17

The grid file_manager cards now show up to three assigned, display-enabled tag chips
at the bottom-right of the thumbnail. If more tags are assigned, a `+N` chip
shows how many remain. The tag row is width-constrained to the thumbnail, and
individual names are ellipsized to keep card content visible. List layout does
not add thumbnail overlays.

## Validation

- `cargo +1.97.1 fmt --all -- --check`: passed.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2`: passed.
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2 --target-dir target/tag-picker-build`: passed; existing linker warning LNK4098.
- `node scripts/check-ui-refresh.cjs`: passed.
- `node scripts/check-architecture.cjs`: passed (84 edges, 711 debt entries); reports existing orphan directories `fileatlas-plugin-media-playback` and `fileatlas-plugin-preview`.
- `node scripts/check-feature-ledger.cjs`: passed.
- Same-process FileAtlas MCP used the isolated `target/mcp-gallery-tag-chips2` profile, `dev-test` plugin, and mutation guard enabled only for this fixture. It scanned `test-data/fileatlas-fixtures`, created five test tags, assigned them to `small.png`, refreshed the FileManager, and captured the live window. The screenshot shows three tag capsules and `+2` on the thumbnail. No normal user database was touched.

Screenshot: [gallery-tag-chips.png](../../../.local/validation/artifacts/2026-10-17-file-manager-tag-chips/gallery-tag-chips.png).

## Compact thumbnail sizing — 2026-10-17

FileManager-only capsules now use 18px height, 4px horizontal padding, 10px icons,
and tiny text. Their widths are divided across the available thumbnail row, so
long names ellipsize before forcing the chip row to clip. On thumbnails below
112px, two tags are shown before the `+N` count to preserve readable capsules.
Other uses of the shared capsule keep the regular size.

- `cargo +1.97.1 fmt --all -- --check`: passed.
- `cargo +1.97.1 check --locked -p fileatlas-ui-bridge -p fileatlas-plugin-file-manager -j 2`: passed.
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2 --target-dir target/tag-picker-build`: passed; existing linker warning LNK4098.
- Same-process FileAtlas MCP using isolated `target/mcp-tag-capsule-size-20261017`, `dev-test`, and test-only mutations: passed. Five fixture tags were assigned to `small.png`; screenshot confirms the compact three-capsule plus `+2` layout.

Screenshot: [gallery-capsules.png](../../../.local/validation/artifacts/2026-10-17-unified-tag-capsule/gallery-capsules.png).
