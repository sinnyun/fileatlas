# File operations dialog and copy flow

The FileManager bulk-operation dialog now uses the shared floating-panel title
height and compact horizontal inset. The right-click “Copy to…” single-item Save
File dialog and multi-item folder chooser now use `rfd::AsyncFileDialog` from a
detached GPUI task; selecting a destination starts the existing File Operations
service flow, and cancelling leaves the operation untouched. This removes the
synchronous native dialog call from the menu callback.

## Reference checklist

Checked `docs/reference-ui/final-v5/06-file-operations-dialogs.png` and its
companion notes. The bulk dialog retains the centered dimmed modal, action/source
columns, AND filter controls, matched-results area, and fixed footer. Its frame
keeps the shared workbench border/radius and native theme roles. The title row
uses `panels::HEADER_HEIGHT` (36 logical pixels), compact close control, shared
horizontal inset, and separator. Native destination selection remains the OS
dialog. The active concept notes carry the checklist and state requirements.

## Checks

- `cargo +1.97.1 fmt --all -- --check` — passed.
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2` — passed; one linker
  warning reports a default-library conflict (`LNK4098`).
- `node scripts/check-ui-theme.cjs`, `node scripts/check-ui-refresh.cjs`,
  `node scripts/check-architecture.cjs`, and
  `node scripts/check-feature-ledger.cjs` — passed.
- `node --check scripts/validate-file-operations.cjs` — passed.
- Same-process MCP command:
  `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/file-operations-review --access-level standard --scenario scripts/validate-file-operations.cjs`
- MCP opened the bulk-operation panel, captured the live window, and read UI state
  again successfully. Screenshot:
  `.local/validation/profiles/file-operations-review/screenshots/file-operations-panel.png`.
- The validation profile is isolated under `.local/validation/profiles/file-operations-review`.

## Limits

The MCP client cannot drive or inspect the native Windows save picker, so the
right-click context-menu selection through destination confirmation and the actual
Save File interaction remain unverified in this run. The code path is statically
compiled; the panel and same-process MCP responsiveness are runtime checked.
