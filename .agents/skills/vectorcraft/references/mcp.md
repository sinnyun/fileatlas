# VectorCraft MCP server

`vectorcraft-cli mcp` runs a Model Context Protocol server on stdio: newline-delimited JSON-RPC 2.0.
Verified on 0.14.0 (32 tools). Use `scripts/vc-mcp.cjs` to drive it without hand-writing protocol lines, or
register it as a real MCP server.

## Modes

| Mode | How | Works |
|---|---|---|
| Remote | `mcp --connect 127.0.0.1:7979` (app started with `vectorcraft --control 7979`) | Everything, and you watch the app change live. Tool calls forward over the control protocol. |
| Headless | `mcp --headless` | In-process engine + CPU renderer. Everything except UI-only tools: `inspect_ui`, `type_text`, `open_panel`, `screenshot {window:true}`. |
| Auto (no flag) | `mcp` | Tries `127.0.0.1:7979`, falls back to headless. |

Logs go to stderr; stdout carries only protocol messages. When the app isn't listening, `--connect` fails
with `cannot connect to 127.0.0.1:7979: connection timed out` and exit 1.

## Register it in Qoder

This host has a user-scope entry `vectorcraft` → `C:\Program Files\nodejs\node.exe` +
`args: ["F:\\AI\\geren\\pho_cordis\\.agents\\skills\\vectorcraft\\scripts\\vc-mcp-launcher.cjs"]` (stdio,
non-secret, auto mode, no automation roots). It was added 2026-10-21 pointing straight at
`C:\Program Files\VectorCraft\vectorcraft-cli.exe` `args: ["mcp"]` and repointed at the launcher on
2026-10-22, so Qoder and Codex share one path-discovery layer. Equivalent hand-written config:

```json
{"mcpServers": {"vectorcraft": {"command": "C:\\Program Files\\nodejs\\node.exe",
  "args": ["F:\\AI\\geren\\pho_cordis\\.agents\\skills\\vectorcraft\\scripts\\vc-mcp-launcher.cjs"]}}}
```

Add it with the `manage_mcp` tool (`action: add`, then `action: update` with
`expectedSourceRevision` from `query_mcp get_config`) instead of editing a config file. Handshake latency
(spawn → first protocol line, median of 5 runs, no GUI running, `.local/validation/vectorcraft-mcp-launcher/latency.cjs`):
`mcp --headless` 58 ms direct and 151 ms through the launcher; `mcp` auto 873 ms direct and 963 ms through the
launcher (the ~0.8 s is the `127.0.0.1:7979` probe before it falls back); `mcp --connect` with no GUI replies
nothing and exits fast, so a pinned-connect server shows as failed until the app is started. With auto, start
the GUI first and then `manage_mcp action: retry` to respawn the server into connected mode; confirm it with
`Get-NetTCPConnection -RemotePort 7979` showing an `Established` socket owned by the `vectorcraft-cli mcp`
pid (its parent is the launcher's `node.exe`). Installing never starts a server. A freshly added server is not
callable the instant `add` returns — the conversation's MCP tool table refreshes later (observed inside the same
conversation), so verify with `query_mcp list_tools` in the meantime. An `update` leaves the previous upstream
process running as a child of Qoder (observed: one stale `vectorcraft-cli mcp`); it clears when the app restarts.

## Generic access for other agents

`scripts/vc-mcp-launcher.cjs` (zero dependencies, CommonJS) is the portable front door for any MCP client —
Codex, Claude Code, Cursor, VS Code, or a hand-written one. It resolves the official binary on the host
(`VECTORCRAFT_CLI` / `VECTORCRAFT_MCP_EXE` / `FILEATLAS_VECTORCRAFT_EXE` → Program Files → `%LOCALAPPDATA%\Programs`
→ macOS `.app/Contents/MacOS` → Linux `/opt` and `~/.local/bin` → bare name on `PATH`), starts
`<cli> mcp` and pipes newline-delimited JSON-RPC bytes both ways untouched. It re-declares no tool, so the
surface stays exactly the official 32 and cannot drift from the installed build. Launcher diagnostics and the
upstream log go to **stderr only**; stdout carries protocol bytes alone.

```sh
node scripts/vc-mcp-launcher.cjs --which                    # resolved exe + version
node scripts/vc-mcp-launcher.cjs --headless                 # or --connect 127.0.0.1:PORT, --cwd DIR, --exe PATH
node scripts/vc-mcp-launcher.cjs --print-config codex       # also claude|cursor|vscode|json; --direct skips Node
node scripts/vc-mcp-launcher.cjs --install codex --dry-run  # writes the block into $CODEX_HOME/config.toml
```

The Codex block it emits on this host:

```toml
[mcp_servers.vectorcraft]
type = "stdio"
command = 'C:\\Program Files\\nodejs\\node.exe'
args = ['F:\\AI\\geren\\pho_cordis\\.agents\\skills\\vectorcraft\\scripts\\vc-mcp-launcher.cjs']
startup_timeout_sec = 60
```

`--install codex` replaces only the `[mcp_servers.vectorcraft]` block (idempotent on repeat), leaves every
other `[mcp_servers.*]` entry alone, and writes `config.toml.<utc-stamp>.bak` next to the file first.
Automation roots and `--connect` conflict upstream, so the launcher refuses that combination itself with exit 2
(`automation roots imply the headless engine`).

Session model worth stating to whoever connects: each stdio client gets its **own** server process, hence its
own untitled document, and they never see each other's edits. With `vectorcraft --control 7979` running, every
auto-mode client attaches to that one window instead, so agents share one document and the engine serializes
calls (a call still running after 2 s makes the next one return `-32003 busy`).

Verified on this host (vectorcraft-cli 0.14.0, harness in `.local/validation/vectorcraft-mcp-launcher/`):
16/16 launcher checks and 10/10 config checks — handshake + `tools/list` = 32 through the launcher in headless
and auto; headless `file.new` → `draw_shape` (id 2) → `export` wrote a real 2388-byte PNG; spawn from an
unrelated cwd worked; `--install` into a temp `CODEX_HOME` parsed back with smol-toml 1.9.1 keeping
context7/node_repl/codebase-memory-mcp; repeat installs produced one block. With the GUI up, auto mode
returned `ui_inspect` canvas data (live attach); with it closed, the same call correctly fell back to headless.
**Not yet exercised:** a real Codex session — this host has the Codex Windows app without a `codex` CLI on
`PATH`, so the TOML was validated by parsing, not by a Codex handshake. The Qoder entry above still launches
the exe directly (registered 2026-10-21); pointing it at this launcher is optional and only buys portable
path discovery.

## Protocol essentials

- Revision `2025-06-18` (`2025-03-26` and `2024-11-05` accepted). Every request/notification must carry
  `"jsonrpc":"2.0"` or it is refused with `-32600`.
- One message per line, ≤16 MiB; a batch holds ≤64 messages; `command_batch` ≤256 steps with ≤8 MiB of kept
  results. Replies from a connected app are refused past 256 MiB and drop the connection.
- A malformed/id-mismatched reply drops the connection and reports **uncertainty** — the failed operation is
  never replayed automatically. Treat a lost reply as "may have happened": `inspect_document` before retrying
  a mutation.
- The engine runs one call at a time. After 2 s of a running call, new calls get `-32003` (`data.busy` names
  the running call) and were **not** run. `ping`, `tools/list`, `resources/list`, `prompts/list`, `prompts/get`
  answer immediately.
- `notifications/cancelled {requestId}` removes a queued call or stops a running one at its next checkpoint —
  that request then gets **no reply**, and a cancelled export writes nothing.
- `_meta.progressToken` yields `notifications/progress` before the reply (PDF pages, raster artboards).
- Not implemented: `resources/subscribe`, reading the document during a long call, `sampling`/`elicitation`,
  Streamable HTTP/SSE (stdio only).

## The 32 tools

Common-style names first; the older aliases stay listed for existing workflows.

| Tool | Arguments → result |
|---|---|
| `command_list` / `list_commands` | `filter`\|`query`, `menu`, `enabled_only`, `brief` → command array. No `limit` argument. |
| `command_run` / `run_command` | `id` (command_run) or `command` (run_command), `params` → runs any engine or UI command. The schemas are `additionalProperties:false`: `{"command":…}` to `command_run` is `-32602`. |
| `command_batch` | `steps:[{id,params}]` ≤256, `stop_on_error` (default true), `atomic`, `label` → `completed`, `failed`, `results`, `created`. `id` may be `draw_path`/`draw_shape` with that tool's args. Distinct from engine command `command.batch` (the Actions-panel player). |
| `doc_inspect` / `inspect_document` | `depth`, `childLimit`, `ids`, `kind`, `limit`, `offset` → artboards, layer tree, selection, history, current tool, paint defaults. |
| `render_preview` | `max_side` (1–4096, default 1024), `artboard` (0-based) → inline PNG, without editing. |
| `ui_inspect` / `inspect_ui`, `ui_screenshot` | Connected desktop state / window capture; tool errors in headless mode. |
| `select_tool` | `tool`: `selection`, `directSelection`, `pen`, `rectangle`, `ellipse`, `polygon`, `star`, `lineSegment`, … |
| `pointer_gesture` | `events:[{kind:down\|drag\|up\|move\|doubleclick, x, y, mods?}]`, `tool?`, `mods?` — same path as the mouse. |
| `draw_path` | `points` (`[[x,y],…]` or `[{x,y,in?,out?,smooth?},…]`) **or** `d` (SVG path data), `closed?`, paint + `dash/cap/join/opacity/name/effects`. |
| `draw_shape` | `shape` + geometry (`rectangle`/`ellipse`: `x,y,width,height`,`radius?`; `polygon`: `cx,cy,radius,sides`; `star`: `cx,cy,radius1,radius2,points`; `line`: `x1,y1,x2,y2`) + the same paint/style keys. |
| `set_paint` | `fill?`, `stroke?`, `strokeWidth?`, `ids?` → applies to selection and becomes the new-art default. |
| `press_key` | `key`, `mods?` — remote: a real key event; headless: runs the command bound to that shortcut or feeds the busy tool. |
| `type_text`, `open_panel` | Remote only (`panel` = id such as `layers`, `swatches`, `colorGuide` or its label). |
| `invoke_menu` | `command`, `params?` — includes UI-only commands (`view.*`, `window.*`) in remote mode. |
| `screenshot` | `path?`, `scale?`, `artboard?`, `window?` → MCP image content plus text; `window:true` is remote-only. |
| `open_file` | `path` → `document.open`'s result including `warnings`. |
| `save_file` | `path?` → the document's own file/format; extension picks the format. |
| `export` | top level only: `path format options scale artboard range selection outlineText` (`additionalProperties:false`). |
| `add_text` | `text, x?, y?, width?, height?, fit?, path?, mode?, pathEffect?, size?, font?, color?, justify?, vertical?` → `{id}`, plus `overflow:true` when it doesn't fit. |
| `apply_effect` | `effect?`, `params?`, `ids?`; omit `effect` to get the effect catalogue with defaults. |
| `pathfinder` | `operation`: `unite minusFront intersect exclude divide trim merge crop outline minusBack`, `ids?`. |
| `transform` | `ids? dx? dy? rotate? scale? scaleX? scaleY? reflect? shear? origin? copy?` — runs move, rotate, scale, reflect, shear in that order. |
| `create_graph` | `type? x y width height csv?` or `series? categories? rows?` — the nine graph types. |
| `text_wrap` | `ids? offset? invert? release?`. |
| `undo` / `redo` | `{}`. Calls that change nothing record no undo step, so `undo` then reverts the earlier step; `inspect_document.history` lists steps. |

Paint values accept `"#rrggbb"`, `"none"`, `[r,g,b]` (0..1), `{"c","m","y","k"}`, `{"gray"}`, or a full
`paint.setFill` params object (`{"gradient":…}`, `{"swatch":"name"}`). Omitted paint on `draw_shape`/`draw_path`
inherits the current new-art default — which is why type created right after a yellow fill comes out yellow.
Pass `stroke:"none"` explicitly for a filled shape with no outline; lines start unfilled with a visible stroke.

## Resources and prompts

Fixed: `vectorcraft://document` (summary), `vectorcraft://document/json` (whole model),
`vectorcraft://commands` (live catalogue). Templates: `vectorcraft://object/{id}`,
`vectorcraft://command/{id}`, `vectorcraft://effect/{id}`, `vectorcraft://swatch/{name}` (percent-encode
spaces). Prefer drilling over dumping on a real document: `inspect_document {depth:0}` →
`run_command document.find {name?,kind?,text?,limit?}` → `document.node {id, summary:true, depth?, childLimit?}`.
A truncated level reports `childCount`, so slicing is never silent.

Prompts (`prompts/get` fills one templated user message): `poster` (brief, palette, text), `icon-set`
(subject, count, detail), `recolor` (palette, method), `trace-and-style` (path, preset, effect),
`export-set` (formats, directory, scale).

`completion/complete` answers prompt arguments and template variables from the live catalogues (formats,
Image Trace presets, effect ids, object ids, swatch names), capped at 100 with `total`/`hasMore`.

## Verified gotchas

- A freshly launched `vectorcraft --control 7979` opens with no document. `ui_inspect` still answers
  (`documents: []`, live `perf.fps`), but every editing tool returns
  `command \`shape.ellipse\` is not available right now: no document open`. Run
  `command_run {"id":"file.new","params":{"width":612,"height":792}}` first; `undo` then reports
  `nothing to undo` until something has changed.
- Tool names are not command ids: `--call file.new` → `unknown tool \`file.new\` (see tools/list)`. Use
  `run_command {"command":"file.new","params":{…}}`.
- Unknown top-level tool arguments are JSON-RPC `-32602` (`unknown argument \`background\` for \`export\``),
  and unknown `params` keys name the nearest valid one — check `tools/list` input schemas on the installed
  build before trusting newer upstream docs.
- Errors from a command come back as a normal result with `isError:true` and a readable message, so the loop
  can correct and retry; the server keeps serving.
- Each server process is a fresh untitled 612×792 document. Keep a whole session in one process
  (`--script`, or `command_batch`), or reopen the saved `.vectorcraft`.
- `screenshot`/`ui.screenshot` need a presented frame; in headless mode use `render_preview` or `export`.
