# Database-only tagging and inline Inspector picker — 2026-10-17

## Behavior

- Normal tag assignment, removal, clearing, merge, reparent and record relinking
  update PhoTag's SQLite data only. They no longer schedule automatic IPTC or
  XMP file writes.
- The file Inspector's Tags section expands a searchable hierarchical
  multi-select list in place. “Add selected” applies the selected tags to the
  current file and closes the list. Existing tag chips and their remove
  controls remain.
- The broad Gallery selection picker remains available for bulk tagging.
- Explicit metadata-writer APIs remain opt-in for integrations.

Reference comparison: `vendor/photag-source/src/xmp.rs` and its caller paths in
`vendor/photag-source/src/app.rs`. The source automatically writes JPEG IPTC or
non-JPEG sidecars; this default is intentionally disabled by the user's
2026-10-17 instruction. Assignment/association/ancestor semantics still use
the same database service.

## Validation

- `rustup toolchain list`: selected installed Windows MSVC Rust 1.97.1.
- `cargo +1.97.1 fmt --all -- --check`: passed.
- `cargo +1.97.1 check --locked -p photag-shell -j 2`: passed.
- `cargo +1.97.1 build --locked -p photag-shell -j 2`: passed with the existing
  linker warning LNK4098.
- `node scripts/check-ui-refresh.cjs`: passed.
- `node scripts/check-architecture.cjs`: passed (84 edges, 711 debt entries).
- `node scripts/check-feature-ledger.cjs`: passed.
- Same-process MCP used the isolated `target/mcp-tagging-inspector-live5`
  profile and the built `target/debug/photag-shell.exe`, with mutations enabled
  only for that profile. It browsed the synthetic `small.png`, opened the
  Inspector's inline list using actual GPUI pointer events, selected two tags,
  and clicked “Add selected”. `get_media` returned both database tag
  assignments. The source image SHA-256 was unchanged
  (`15f095b8be6c5fec062b1a9c15cebd44e242a0941bc4bcf33cba83535a59f8c7`),
  `small.xmp` did not exist before or after, and no writeback failures were
  reported.

Screenshots from that child process:

- [Inline multi-select open](../../../.local/validation/artifacts/2026-10-17-tagging-inspector/dropdown-open.png)
- [Two tags assigned in the Inspector](../../../.local/validation/artifacts/2026-10-17-tagging-inspector/tags-added.png)

Real third-party editor interoperability is not applicable to normal tagging
now that file metadata is not written. Explicit integration writeback remains
outside this UI flow.

## Popover and immediate assignment update — 2026-10-17

The Inspector picker now opens as a GPUI Popover anchored to Add tags, so it overlays the Tags section without increasing the Inspector body height. It shows the file's currently assigned tags in 已选中 and remaining tags in 所有值. Clicking an available value calls the existing database assignment action immediately; clicking an assigned value or its chip removes it immediately. There is no staged selection or confirmation button. The existing metadata-writeback policy remains database-only.

Same-process MCP with the isolated profile 	arget/mcp-tag-picker-final captured the open popover after a real GPUI click; the floating layer does not move the EXIF/file-path sections. Screenshot: [popover-open.png](../../../.local/validation/artifacts/2026-10-17-tagging-inspector/popover-open.png). Static checks after this UI change: fmt, locked shell check/build, and the three governance scripts passed. Pointer-driven add/remove through the new popover still needs a confirmed real-change check; the attempted MCP pointer target did not produce a DB assignment, so this is not reported as interaction-verified.

