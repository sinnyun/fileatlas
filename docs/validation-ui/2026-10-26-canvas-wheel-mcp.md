# Canvas wheel MCP control — 2026-10-26

## MCP operation

Added Standard-gated `whiteboard.scroll` with inputs:

- `id`: target board ID
- `delta_x`, `delta_y`: wheel delta in logical pixels, bounded to ±4096
- `control`: false zooms at the cursor; true pans
- `at`: `[x, y]` canvas-local logical coordinates, bounded to 0–4096

The tool queues the operation to the live Whiteboard host, which calls the same
`WhiteboardView::scroll_canvas` function as native wheel input. `whiteboard.state`
provides the persisted Scene camera for readback. Camera changes from this MCP
operation are saved to the existing board JSON. No extra data store was added.

## Verification

Command:

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/canvas-wheel-mcp-final-20261026 --plugins whiteboard --access-level standard --scenario scripts/validate-canvas-wheel-mcp.cjs
```

Result: passed in the same FileAtlas MCP child process. With initial camera
`{x:0,y:0,zoom:1}`, ordinary wheel input at `[120,80]` with `delta_y:160`
returned `{x:34.285713,y:22.857141,zoom:1.4}`. The world point beneath the
cursor was unchanged. Ctrl+wheel input with `delta_y:80` returned
`{x:34.285713,y:-34.285713,zoom:1.4}`: zoom stayed fixed and vertical pan
matched `-80/1.4`. The test scenario is
`scripts/validate-canvas-wheel-mcp.cjs`.

This exercises the exact shared canvas wheel handler through the feature MCP
tool; it does not synthesize a Windows hardware mouse wheel event.
