---
name: vectorcraft
description: Operate the official VectorCraft vector-illustration app and automate it from the terminal. Use when creating, editing, inspecting, converting or exporting .vectorcraft/.svg/.pdf/.ai/.eps/PNG artwork, when driving vectorcraft-cli (run, convert, info, commands, bench, perf), when using the vectorcraft-cli mcp stdio server or its tools, and when driving a running VectorCraft window through the --control port. 触发词：VectorCraft、vectorcraft-cli、画矢量图、导出 SVG/PDF/PNG、.vectorcraft 文档、控制协议/MCP 驱动矢量程序。
argument-hint: <task or document path>
---

# VectorCraft

## Overview

VectorCraft is an open-source Rust vector-illustration app (Adobe-Illustrator-class) that ships a
native automation surface. Three ways to drive it, from cheapest to most interactive:

| Surface | Command | Use for |
|---|---|---|
| Batch CLI | `vectorcraft-cli run / convert / info / commands` | one-shot draw → save → export pipelines, format conversion, file inspection, benchmarks. No protocol code needed. |
| MCP stdio server | `vectorcraft-cli mcp [--headless \| --connect ADDR]` | many interactive steps, inline paint shortcuts, screenshots as image content, resources/prompts. Register it as an MCP server or drive it with `scripts/vc-mcp.cjs`. |
| Live control port | `vectorcraft --control 7979` + JSON lines on `127.0.0.1:7979` | watch the real GUI change: menus, panels, pointer/keyboard input, window captures. Reached from MCP `--connect` mode. |

## Locate the binary first

The MSI/portable install is **not on `PATH`**. Resolve it once instead of guessing:

```sh
node scripts/vc.cjs which        # prints the exe path + version, e.g. C:\Program Files\VectorCraft\vectorcraft-cli.exe
```

Search order: `VECTORCRAFT_CLI` → `FILEATLAS_VECTORCRAFT_EXE` → `C:\Program Files\VectorCraft\` →
`%LOCALAPPDATA%\Programs\VectorCraft\` → `vectorcraft-cli` on `PATH`. The desktop app sits beside it as
`vectorcraft.exe`. Verified host: `vectorcraft-cli 0.14.0`.

Both scripts are self-contained CommonJS (`node scripts/vc.cjs`, `node scripts/vc-mcp.cjs`) and print
compact results with an explicit `exit=N` line. Run them from this skill's directory, or with its absolute
path (e.g. `node .qoder/skills/vectorcraft/scripts/vc.cjs …` from a project root); they resolve the binary
themselves, so no `PATH` setup is needed.

## Golden rules

1. **Never guess command ids or parameter keys.** The catalogue is the truth, and unknown keys are
   refused before anything runs (`invalid parameters for \`file.new\`: unknown parameter \`widht\`; did you
   mean \`width\`?`). Introspect:
   ```sh
   node scripts/vc.cjs search "align"          # id | label | enabled | menu
   node scripts/vc.cjs search "gradient" --full
   node scripts/vc.cjs params shape.rectangle  # exact accepted keys
   ```
   Through MCP: `list_commands {filter}` / `run_command document.formats` / `vectorcraft://command/{id}`.
2. **Headless state lives and dies with one process.** Each `vectorcraft-cli run` and each `mcp` server
   process starts a fresh untitled 612×792 pt document. Draw *and* export in the same invocation, or save
   a `.vectorcraft` and reopen it. Exporting from a second process that drew nothing yields a blank image
   with no error. Only saved PDF presets and ICC profiles persist between runs
   (`VECTORCRAFT_CONFIG_DIR` relocates them, `VECTORCRAFT_NO_PREFS` disables them).
3. **CLI shape commands take geometry only; the MCP `draw_shape` tool also paints.**
   `shape.rectangle` rejects `fill` outright — `unknown parameter 'fill'; the command takes: x, y, width,
   height, radius, rotation` — so paint is a separate command applied to the selection (new objects become
   the selection):
   ```sh
   vectorcraft-cli run --cmd shape.rectangle --params '{"x":40,"y":40,"width":220,"height":120,"radius":12}' \
                       --cmd paint.setFill --params '{"color":"#1e88e5"}' --cmd paint.setStroke --params '{"color":"#0d47a1"}'
   ```
   ```jsonc
   // MCP: one call
   {"tool":"draw_shape","args":{"shape":"rectangle","x":60,"y":60,"width":240,"height":140,"radius":16,"fill":"#1e88e5","stroke":"none","name":"card"}}
   ```
4. **Engine commands are not MCP tool names.** `--call file.new` fails with `unknown tool`. Use the 32
   real tools, or `run_command {command, params}` for the other ~760.
5. **Verify the picture, don't assume it.** `node scripts/vc.cjs info FILE`, `--inspect`,
   `render_preview`/`screenshot`, then actually open the exported PNG. A zero exit code only proves the
   commands ran.

## Recipes

```sh
# Draw, save natively, export PNG + SVG in one batch (one process keeps the document)
node scripts/vc.cjs run --cmd file.new --params '{"width":600,"height":400,"units":"Pixels","name":"card"}' \
  --cmd shape.ellipse --params '{"x":60,"y":40,"width":180,"height":120}' \
  --cmd paint.setFill --params '{"color":"#e53935"}' \
  --cmd text.create --params '{"x":60,"y":320,"text":"hello","size":28,"color":"#222222"}' \
  --cmd document.save --params '{"path":"F:/out/card.vectorcraft"}' \
  --cmd document.export --params '{"path":"F:/out/card.png","ppi":144,"background":"white"}' \
  --cmd document.export --params '{"path":"F:/out/card.svg"}'

# Convert / inspect / query
node scripts/vc.cjs convert in.svg out.pdf --range 1-3
node scripts/vc.cjs info art.vectorcraft
node scripts/vc.cjs search pathfinder          # count + matching ids; --full for params text

# Multi-step MCP session (steps file = one process, so state carries over)
node scripts/vc-mcp.cjs --headless --script steps.json --inspect
node scripts/vc-mcp.cjs --headless --call draw_path '{"d":"M72 400 C 150 300 250 500 330 400","stroke":"#e53935","strokeWidth":4,"fill":"none"}' \
                        --call export '{"path":"F:/out/curve.png","format":"png","options":{"ppi":144,"background":"white"}}'
node scripts/vc-mcp.cjs --headless --tools          # the 32 tool names
```

`steps.json` is `[{"tool":"draw_shape","args":{…}}, {"method":"tools/list","params":{}}]`.
Reuse one session for many calls: `command_batch` (≤256 steps, `atomic:true` = one undo step) or
`$N.path` / `$last.path` step references.

To hand the same server to **another agent** (Codex, Claude Code, Cursor, VS Code) without hardcoding this
host's exe path, use the portable launcher: `node scripts/vc-mcp-launcher.cjs --which`,
`--print-config codex|claude|cursor|vscode|json`, `--install codex` — it finds the binary, starts `mcp` and
forwards the protocol bytes untouched, so the tool surface stays the official 32. See
`references/mcp.md`#generic-access-for-other-agents.

## Format facts worth knowing

- Readable: `.vectorcraft .drawcraft .vctemplate .svg .svgz .pdf .ai .ait .eps .epsf .ps .dxf .emf .wmf`
  `png jpg gif webp tif bmp tga psd psb af afdesign afpub`. Writable: `.vectorcraft .svg .svgz .pdf .ai .png
  jpg gif webp tif bmp .vctemplate .txt .dxf .eps .emf .wmf .tga .psd`.
- Coordinates: points, **y down**, origin = first artboard top-left; new document defaults to 612×792 (US
  Letter) unless `file.new` sets `width/height/units`.
- `.vectorcraft` is JSON plus a `VCBLOBS` binary section — not plain JSON; edit it through commands, not by
  text patching. `document.save {compress:true}` gzips; `version:1|2` writes legacy `.drawcraft`.
- Export extras belong in `options` for the MCP `export` tool (its schema is `additionalProperties:false`;
  top level is only `path format options scale artboard range selection outlineText`).
- Rasters: `ppi`, `background` (`transparent|white|black|"#rrggbb"`), `antiAlias`, `interlaced`;
  `useArtboards:true` + `range:"1-3,5"` writes `<stem>-<artboard>.<ext>`.
- Set `SOURCE_DATE_EPOCH=<seconds>` for byte-reproducible PNG/PDF/EPS/native output (a password-encrypted
  PDF stays non-reproducible).

## Confining file access

Give the agent bounded roots instead of free filesystem access; a root you omit grants none of that access:

```sh
vectorcraft-cli mcp --headless --automation-read-root F:/proj --automation-write-root F:/proj/out
vectorcraft --control 7979 --automation-read-root F:/proj --automation-write-root F:/proj/out   # confines the live app
```

The roots imply `--headless` on the CLI (combining them with `--connect` is an error) — confine the app
itself instead. A refused path is an ordinary, readable error the agent can correct, and the call exits
non-zero: `isError export: automation path rejected: outside the write root F:\proj\out: F:/proj/escaped.png`.
Links are followed, whole folder names are compared (case-insensitively on Windows), and
`--automation-read-root` alone means nothing can be written at all.

## Safety

- **Never kill, restart or replace the user's running `vectorcraft.exe`.** Before touching a live session,
  check whether the port is taken and whose process owns it; ask first. Only `open_editor`-style work should
  start the GUI, and never a second instance on a document the app already holds dirty.
- `app.quit`, `file.close*` open a `saveChanges` dialog for each modified document — confirm/cancel
  deliberately, and never discard the user's unsaved work to make a test pass.
- The control port has **no authentication** on loopback: enable `--control` only while you use it, and pass
  `path` to save/export commands so no native file dialog blocks the call (on Windows dialogs run inline).
- Write generated artwork under the caller's output folder or `.local/`; do not commit `.vectorcraft`,
  PNG or profile artifacts as evidence.

## References

- `references/cli.md` — every subcommand, flags, parameter checking, step references, reproducibility, verified output shapes, gotchas.
- `references/mcp.md` — server modes, protocol limits, the 32 tools, resources, prompts, Qoder registration, strict-schema errors.
- `references/control.md` — driving the running GUI: methods, dialogs, synthetic input budgets, transport limits.
- `scripts/vc.cjs` — locate the CLI; `which | version | search | params | run | convert | info | bench | perf | raw`.
- `scripts/vc-mcp.cjs` — MCP driver; `--tools | --resources | --prompts | --call | --script | --inspect | --shot`, `--read-root/--write-root`.

Upstream docs (newer than the installed build — trust the local binary first):
`https://github.com/storytold/vectorcraft` › `docs/mcp.md`, `docs/control-protocol.md`, `README.md`.
