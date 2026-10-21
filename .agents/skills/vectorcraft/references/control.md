# Live control protocol — driving the running VectorCraft app

`vectorcraft --control <port>` makes the desktop app listen on `127.0.0.1:<port>` (loopback only; port
1–65535, given once, or the app prints an error and exits before starting). One JSON request per line:

```json
{"id": 1, "method": "ui.inspect", "params": {}}
{"id": 1, "ok": true, "result": {…}}      or      {"id":1,"ok":false,"error":"…"}
```

Start it, then either speak the protocol directly or let the MCP server forward tool calls:

```sh
"C:\Program Files\VectorCraft\vectorcraft.exe" --control 7979 F:\art\card.vectorcraft
"C:\Program Files\VectorCraft\vectorcraft-cli.exe" mcp --connect 127.0.0.1:7979
node .agents/skills/vectorcraft/scripts/vc-mcp.cjs --connect 127.0.0.1:7979 --inspect
```

Sibling Craft apps use the same default port — if the log says binding failed, pick another port, or your
requests reach a different app.

**Status on this host:** the batch CLI and headless MCP paths are verified end to end. The `--control`
section below is documented from the pinned official docs plus the workspace's own client
([crates/fileatlas-plugin-workcrate-boards/src/protocol.rs](file:///F:/AI/geren/pho_cordis/crates/fileatlas-plugin-workcrate-boards/src/protocol.rs));
it was not exercised against a live GUI during this skill's creation, because launching or closing the user's
VectorCraft window needs their say-so.

## Methods

| Method | Params | |
|---|---|---|
| `engine.execute` | `{command, params}` | Run any engine or UI command (see `engine.commands`). Unknown top-level `params` keys error before anything runs. |
| `engine.commands` | | Full catalogue: label, shortcut, params doc, enablement. |
| `document.inspect` | `{depth?, childLimit?, ids?, kind?, limit?, offset?}` | Layer tree, selection, key object, history, paint defaults. |
| `ui.inspect` | | Tool, UI state, canvas rect, window size, perf, running background saves/exports, `fileDialog` while a native dialog is open. |
| `ui.menu.list` / `ui.menu.invoke` | `{command, params}` | Full menu tree / invoke an item. |
| `ui.contextMenu.list` | | Canvas context menu for the current selection. |
| `ui.tool.select` / `ui.tool.list` | `{tool}` | |
| `ui.pointer` | `{events:[{kind:down\|drag\|up\|move\|doubleclick, x, y, space?:"doc"\|"screen", mods?}]}` | Drives the active tool like the mouse; reply lists `selection`, `key`, `tool`. |
| `ui.key` / `ui.text` / `ui.move` / `ui.click` / `ui.drag` / `ui.wheel` | see docs | Synthetic keyboard, pointer, wheel input. |
| `ui.set` | `{brightness?, panel?, rulers?, outline?, grid?, smartGuides?, boundingBox?, controlBar?}` | |
| `ui.screenshot` | `{path?, data?}` | Capture the **window** as PNG; `data:true` also returns `pngBase64`. Needs a presented frame — fails after ~8 s when the screen is locked or the window is minimized/covered. |
| `ui.render` | `{path?, scale?, artboard?, data?}` | Render an artboard headlessly (0-based index) — the fallback when `ui.screenshot` can't. |
| `ui.resize` / `ui.focus` / `ui.dialog.set` / `.confirm` / `.cancel` | `{field, value}` | Window control; fill and submit the open dialog. |
| `app.open` / `app.save` / `app.export` / `app.quit` | `{path}` / `{path?}` / `{path?, format?, …}` | `app.open` returns `{index,title,format,warnings,…}` or null when a dialog asks first. `app.quit`, `file.close`, `file.closeAll` first return `{"pending":"saveChanges"}` per modified document. |
| `app.save` / `app.saveAs` / `app.saveCopy` / `app.saveAsTemplate` (via `engine.execute`) | `{path?, format?, options?, svg?}` | With `path` the file is written at once; without one the app shows a save panel, then the format's options dialog. |
| `file.place` | `{paths?}` or `{path\|name+dataBase64,…}` | Places files; with no file it opens the picker and the `place` dialog. |
| `document.exportForScreens` / `file.exportForScreens` | `{…}` or `{}` | With params it runs directly; without, it opens the dialog. No `folder` returns files as `dataBase64` (or one zip with `zip:true`). |
| `document.exportPdf` / `ui.savePdfDialog` | `{path?, preset?, range?, …}` | PDF options; `ui.dialog.set {field:"compatibility", value:"1.5"}` etc., `ui.dialog.confirm` writes. |
| `script.check` | `{path?\|data?\|dataBase64?, name?, includes?}` | Static report on a reference-app `.jsx` script — reads nothing else, changes nothing. |

Dialogs are scripted as a state machine: `ui.inspect` → `fileDialog`/pending dialog name →
`ui.dialog.set` fields → `ui.dialog.confirm`/`.cancel`. Replace-file conflicts answer `cancelled` and open a
`replaceFile` dialog; a discarded close is `ui.dialog.set {field:"discard", value:true}` then confirm.

## Transport rules (design around these)

- **Only requests are read.** Every line must be a JSON object with a string `method` (`id`, `params`
  optional; blank lines skipped). Anything else — non-JSON, an array, a line over 4 MiB, invalid UTF-8 — gets
  one error reply and the **connection closes**, so nothing queued after it on that connection runs.
- At most 16 concurrent connections; further ones get an error line and are closed. Reconnect after an error.
- Idle clients have 5 minutes before the next line; once the first byte arrives the whole line must land
  within an absolute 30 s (trickled bytes don't renew it); a reply has 60 s to be written.
- **No authentication on the port.** Enable `--control` only while you use it, and confine paths with
  `--automation-read-root` / `--automation-write-root` (same flags as the CLI; env
  `VECTORCRAFT_AUTOMATION_READ_ROOT` / `_WRITE_ROOT`). Confinement follows links, compares whole folder names,
  ignores case on Windows, refuses `..` below a nonexistent folder and Windows device names, and covers the
  frames carrying injected input — the person at the keyboard is not confined.
- A request unanswered after 60 s returns `{"ok":false,"error":"timeout: …"}`. "will not run" means it never
  started (safe to retry); "still running" means it may yet complete (inspect before retrying).
- Synthetic input budgets: `ui.drag` ≤256 `steps`; `ui.click`/`ui.drag` ≤16 `count`; all synthetic methods
  share a 1,024-event pending-queue limit; ≤64 control requests run per frame.

## Isolated profiles and safety

- Point `VECTORCRAFT_CONFIG_DIR` at a throwaway directory for native test sessions so preferences, logs,
  recovery and user libraries stay isolated.
- Pass `path` on every save/export: on Windows a dialog without a path runs inline and blocks the call.
- Never kill or restart the user's `vectorcraft.exe` to get a clean state, and never send `app.quit` or
  discard a `saveChanges` prompt without explicit permission. If a dirty document is in the way, say so.
- Background writes are refused while the same document is dirty in the GUI (the workspace's WorkCrate
  integration enforces this; keep that behaviour in any new caller).
