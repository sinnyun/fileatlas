# Unified tag capsule — 2026-10-17

Implemented the shared tag capsule concept at `docs/reference-ui/concepts/unified-tag-capsule-2026-10-17.md` and its generated concept image. One reusable `photag_ui_bridge::tag_capsule::TagCapsule` now renders tag color, mapped icon, label, selected state and consistent compact spacing.

Applied to gallery cards (up to three capsules, then `+N`), file inspector tags and candidate picker, gallery tag filters/bulk choices, navigation tag tree/address bar/tag dialogs, tag inspector children, and settings tag choices (AI groups and shortcut selection). Generic labels without tag metadata use the shared name-only form. Native OS popup menu rows remain text-only because that menu API does not accept custom capsule children.

## Same-process UI evidence

Windows, 150% DPI. Launched `target/tag-picker-build/debug/photag-shell.exe --mcp` through `scripts/photag-mcp-debug.cjs` with the isolated `dev-test` profile `target/mcp-unified-tag-capsule`; the user's normal PhoTag profile was not used. The isolated fixture has five colored/icon-tagged tags assigned to `small.png`.

- `gallery-capsules.png`: three colored/icon capsules and a `+2` overflow marker on the gallery card.
- `sidebar-capsules.png`: consistent capsule styling in the navigation tag tree.
- `inspector-tag-picker.png`: assigned capsules and the separate add-tag popover; candidate rows carry the same icon/color/text styling.
- `settings-shortcut-capsule.png`, `settings-ai-capsules.png`: settings tag selections.

The shared row's narrower surfaces can truncate long names; gallery capsules are width-limited to preserve the image card. The candidate picker and native popup-menu interaction beyond the captured open state were not separately exercised.

## Checks

- `cargo +1.97.1 fmt --all -- --check` — passed.
- `cargo +1.97.1 check --locked -p photag-shell -j 2` — passed.
- `cargo +1.97.1 build --locked -p photag-shell -j 2 --target-dir target/tag-picker-build` — passed; linker emitted existing `LNK4098` warning.
- `node scripts/check-ui-refresh.cjs` — passed.
- `node scripts/check-architecture.cjs` — passed (84 edges, 711 debt entries; existing orphan-directory warnings for `photag-plugin-media-playback` and `photag-plugin-preview`).
- `node scripts/check-feature-ledger.cjs` — passed.
