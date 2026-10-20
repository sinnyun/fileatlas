# Clipboard workbench initial integration — 2026-10-18

2026-10-18 correction: this is historical initial evidence. `core` alone did not
enable the shell's explicit dev-test paths; a LOCALAPPDATA override does not
guarantee Windows Known Folder isolation. Current styling, classification,
persistence and verified isolation supersede it in the [v2 record](../2026-10-18-clipboard-workbench-v2/README.md).

## Implemented

- Added `fileatlas-plugin-clipboard` and wired it into the normal `core` shell composition.
- Registered a lifecycle-owned left rail button and single generic floating panel.
- Added a Ctrl+Shift+V registration alongside the existing Ctrl+F shortcut; a Ctrl+Shift+V registration conflict leaves Ctrl+F registered.
- Uses `clipboard-master` 4.0.0 (MIT) for clipboard change notifications and `arboard` 3.7.0 (MIT OR Apache-2.0) for text access.
- Stores text history and category records in additive tables owned by the shared DatabaseService. Captures non-empty text up to 1 MB, deduplicates content, and prunes unpinned rows above 500. Selecting a history item puts its text back on the OS clipboard.
- Passive UI refresh is driven by `refresh::Wake`; the database snapshot is cached outside render.

## Verification

- `rustup toolchain list`: selected installed `1.97.1-x86_64-pc-windows-msvc`.
- `cargo +1.97.1 fmt --all -- --check` — passed.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` — passed.
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2` — passed (existing linker warning LNK4098).
- `node scripts/check-ui-refresh.cjs` — passed.
- `node scripts/check-architecture.cjs` — passed (edges=93, debt=713, UI crates=11; two existing orphan-directory warnings).
- `node scripts/check-layering.cjs` — passed (30 crates, 9 layers, 16 approved peer edges).
- `node scripts/check-feature-ledger.cjs` — passed.
- Same-process runtime: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-clipboard-final-verified --plugins core --scenario scripts/validate-clipboard-workbench.cjs --allow-mutations`. The scenario mounted the panel, verified the system-scope Ctrl+Shift+V command contribution, seeded isolated sample rows, checked content-free metadata, created a category through actual GPUI pointer/text events, paused/resumed capture, and captured [clipboard-workbench.png](../../../target/mcp-clipboard-final-verified/screenshots/clipboard-workbench.png).
- The MCP mutation opt-in applied only to the isolated validation profile. No real clipboard text was read during fixture setup.

## Remaining work and verification limits

- The current panel exposes search, content editing, category creation/assignment/filtering, pin/unpin, delete/clear, and copy-back. It does not yet support category rename/delete or non-text formats.
- Same-process MCP uses `add_clipboard_text` to seed only the isolated database and `get_clipboard_state` to inspect metadata without returning clipboard contents. The populated panel rendered seeded text items, search, category field, pause control, and edit inspector; category creation and pause/resume via actual GPUI pointer/text events passed. The OS clipboard itself was not read or written during this check.
- No system-wide keypress was exercised. No generic OS input or external application was used as a substitute. Ctrl+Shift+V conflict behavior and paste into another application remain unverified.
- The panel now includes a session-only pause-capture control and clear-unpinned action. Persistent privacy preferences and private-app exclusions are not implemented.
