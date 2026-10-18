# JSON feature storage migration — 2026-10-18

## Scope

Clipboard history/categories now use `%LOCALAPPDATA%/PhoTag/clipboard/clipboard.json`;
captured images use `clipboard/images/<sha256>.png`. Program launcher categories and
entries use `%LOCALAPPDATA%/PhoTag/launcher/launcher.json`; icon PNG/meta cache uses
`launcher/icons/`. In MCP debug runs both roots are the explicit profile directory.

On first launch without a JSON file, each plugin imports its old SQLite projection.
Clipboard import copies available referenced image files before publishing the JSON.
Launcher import carries names, paths, categories, hidden state, source, install metadata,
pin scopes, custom-name/category flags and the original discovery key used to prevent
duplicate entries during later scans; legacy icon cache files are copied. JSON writes
use a sibling temporary file and replacement with a recovery backup on Windows. Existing
SQLite rows and old resource files are retained for recovery and are not changed by feature
runtime operations after import.

## Verification

Environment: Windows; installed Rust `1.97.1-x86_64-pc-windows-msvc`; isolated profiles
under `target/`; same-process PhoTag MCP launched by `scripts/photag-mcp-debug.cjs`.

- `cargo +1.97.1 fmt --all` — passed.
- `cargo +1.97.1 check --locked -p photag-shell -j 2` — passed.
- `cargo +1.97.1 build --locked -p photag-shell -j 2` — passed (existing linker warning LNK4098).
- `node scripts/check-ui-theme.cjs` — passed.
- `node scripts/check-ui-refresh.cjs` — passed.
- `node scripts/check-architecture.cjs` — passed (edges=96, debt=713, ui crates=12; existing orphan-directory notices remain).
- `node scripts/check-feature-ledger.cjs` — passed (4 stage rows, links resolve).
- Migration fixture: seeded an isolated legacy SQLite database with one clipboard favorite/category and one launcher entry/category carrying custom name/category, hidden state, install date/size and category pin. The MCP process created both JSON files; parsed values preserved every seeded field.
- Discovery identity fixture: repeated the migration into a fresh isolated profile after preserving the exact legacy discovery key; JSON retained `path:c:\\legacy\\legacy.exe`, preventing a path-key mismatch on future scans.
- Second process on the discovery-key profile reopened one clipboard item and exactly one entry with that legacy key; the ongoing host discovery scan did not create a duplicate.
- Restart fixture: a second PhoTag MCP process on the same profile read one clipboard item and the same launcher category/entry from JSON. No duplicate migration rows appeared. A separate isolated MCP profile added a clipboard item and launcher category; a new process reopened both from the JSON files.

Commands for the MCP scenarios:

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-json-import-clean-20261018 --plugins clipboard,launcher --plan target/mcp-json-import-clean-plan.json
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-json-import-clean-20261018 --plugins clipboard,launcher --plan target/mcp-json-import-restart-plan.json
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-json-storage-20261018 --plugins clipboard,launcher --allow-mutations --plan target/mcp-json-migration-write-plan.json
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-json-storage-20261018 --plugins clipboard,launcher --plan target/mcp-json-migration-read-plan.json
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-json-import-key-20261018 --plugins clipboard,launcher --plan target/mcp-json-import-key-restart-plan.json
```

The legacy fixture databases, JSON, logs, and command plans are generated under `target/`
and are local validation artifacts, not committed product data. No normal `%LOCALAPPDATA%`
PhoTag database or user profile was modified. The Windows system clipboard was not mutated.

## Boundaries

The feature panel layout and OS clipboard capture behavior were not retested in this storage
change. Clipboard image import was not seeded with a binary fixture; migration copies referenced
files when present and preserves their original reference if a file is missing. User data in
the old SQLite feature tables is deliberately retained; removing it would need a separate,
verified backup/cleanup decision.
