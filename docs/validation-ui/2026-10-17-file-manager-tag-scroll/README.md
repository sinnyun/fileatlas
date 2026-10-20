# FileManager tag updates preserve the current scroll position — 2026-10-17

## Root cause and change

The file inspector's add/remove/clear handlers called `FileManagerView::reload()` after writing tag assignments. That is an explicit filter reload: it requests window offset 0, clears the row layout, selection and `ListState`, then restores selection from the old snapshot. As a result, assigning one tag from the middle of a long file_manager visibly jumped the view to the beginning.

Tag changes now refresh only the affected media tag cache in the common case. The operation reads tags for those media IDs and updates `FileManagerSnapshot.media_tags` plus the selected-file fallback. The card chips, inspector and tag popover therefore repaint without reloading the file_manager rows. When an include/exclude tag filter is active, a tag change can alter which files match; that path uses the existing background `refresh_index()` window refresh, which captures and restores the logical scroll anchor and selection. Explicit navigation/filter changes retain their prior reset behavior.

Implementation: `crates/fileatlas-plugin-file-manager/src/view/tagging_picker.rs`, helper `refresh_tag_assignments`. Interface contract: `crates/fileatlas-plugin-file-manager/PORTS.md`, “Tag assignment refresh scope”.

## Same-process FileAtlas MCP verification

Windows, 150% DPI. Ran `node scripts/fileatlas-mcp-debug.cjs --exe target/tag-picker-build/debug/fileatlas-shell.exe --profile target/mcp-tag-scroll-position-20261017 --plugins dev-test --scenario target/repro-tag-scroll.cjs --allow-mutations`. The fixture was 600 copies of the small synthetic JPG in a temporary directory under `test-data/fileatlas-fixtures`; all copies and the database/profile were isolated from the user's FileAtlas profile. The temporary fixture directory was removed after verification.

Scrolled to the middle, selected media ID 276 (`image-0274.jpg`), then clicked a candidate tag in the inspector popover. `get_media` confirmed the tag assignment. Before and after MCP snapshots both reported:

- `window_offset`: 227
- `selected_media_ids`: `[276]`
- `matched_count`: 612
- `loaded_count`: 200

The file_manager screenshot remained on the same middle rows, and the selected media's tag capsule appeared on its card and in the inspector. See `before-add.png` and `after-add.png`.

## Checks

- `cargo +1.97.1 fmt --all -- --check` — passed.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` — passed.
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2 --target-dir target/tag-picker-build` — passed; linker emitted the existing `LNK4098` warning.
- `cargo +1.97.1 test --locked -p fileatlas-plugin-file-manager --lib -j 2` — 23 passed.
- `node scripts/check-ui-refresh.cjs` — passed.
- `node scripts/check-architecture.cjs` — passed (existing orphan-directory notices only).
- `node scripts/check-feature-ledger.cjs` — passed.

A separate interaction with an active include/exclude tag filter was not exercised. That branch uses `refresh_index()` rather than the local cache patch so result membership can update while retaining the scroll anchor.
