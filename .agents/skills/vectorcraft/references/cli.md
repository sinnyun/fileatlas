# vectorcraft-cli — batch automation reference

Verified against `vectorcraft-cli 0.14.0` on Windows (`C:\Program Files\VectorCraft\vectorcraft-cli.exe`).
`node scripts/vc.cjs which` re-resolves it. `--help` on the installed binary is the current truth.

## Subcommands

| Subcommand | What it does |
|---|---|
| `commands` | Print the whole command catalogue as JSON (792 entries on 0.14.0: `id`, `label`, `menu`, `shortcut`, `params`, `enabled`). |
| `run [--in FILE] [--cmd ID [--params JSON]]... [--export FILE]... [--scale N]` | Headless batch: open `FILE` (or start a fresh document), run commands in order, export. Prints one JSON line per step. |
| `convert IN OUT [flags]` | Open any readable format, write `OUT` in the format its extension picks. |
| `info FILE` | JSON summary: import `warnings`, title, colour mode, units, artboards, object counts by kind, fonts. |
| `mcp [...]` | MCP stdio server — see `mcp.md`. |
| `bench FILE [--size WxH] [--iters N]` | ms/frame (multi- and single-threaded) for one file fitted to WxH (default 2880x1800). |
| `perf [--paths N] [--deep]` | Performance budgets on a synthetic N-path document (default 50000). Non-zero exit when a budget is missed, unless the machine is busy. |

`run` also accepts the host commands `file.open`, `file.save`, `file.export`, `file.exportForScreens`,
`tool.select`. `run --in`, `convert` and `info` read every format `document.open` reads.

## `convert` flags

`--scale N` (raster size, pixels per point), `--artboard I` (0-based) **or** `--range R` (1-based,
`"1-3,5"`), `--outline-text`, `--decimals N` (SVG precision 1–7, default 3), `--replace-lossy`,
`--editing-data`, `--ai-version 7|pdf`, `--hidden-objects layer|omit|visible`,
`--transparent-images flatten|omit`.

A PDF gets every artboard unless one of them is given; EPS gets the bounds of the art; other formats get
the first artboard. `OUT` may not equal `IN` when reading `IN` left things out, unless `--replace-lossy`.
The result carries the export's `warnings` plus `importWarnings` for what the read dropped.

## Parameter keys are checked, strictly

`--params` keys are validated against the command's `params` doc (the same check runs in
`command_run`, `command_batch` and `engine.execute`). An unlisted top-level key is refused **before the
command runs**, and the error names the nearest key:

```
vectorcraft-cli: shape.ellipse: invalid parameters for `shape.ellipse`: unknown parameter `cx`; did you mean `x`?
```

Only the top level is checked; keys inside a nested object belong to the command. Effect `params` are an
exception — `effect.apply` / `effect.setParams` check them against `effect.list`.

Verified geometry shapes (query `params <id>` before using others):

```
shape.rectangle  {x, y, width, height, radius?, rotation?} → {id}
shape.ellipse    {x, y, width, height}                      → {id}   # not cx/cy
shape.star       {cx, cy, radius1, radius2, points}         → {id}
shape.polygon    {cx, cy, radius, sides}                    → {id}
shape.line       {x1, y1, x2, y2}
```

`shape.*` take no paint. Paint the selection afterwards:

```
paint.setFill    {color?|none?|swatch?|tint?|gradient?, item?}
paint.setStroke  {…as paint.setFill}
stroke.set       {weight?, cap?, join?, align?, dash?, startArrow?, endArrow?, …}
object.setProps  {ids?|id?, name?, visible?, locked?, opacity?, blend?, isolate?, knockout?, data?}
text.create      {x, y, text, size?, font?, style?, color?, vertical?, area?:{width,height,fit?}, justify?, …}
```

`enabled=false` in the catalogue is normal: enablement is evaluated at listing time, without a document or
selection. Most commands still run once a document and a selection exist.

## Step results and references

`run` prints one JSON line per step (`{"step":"cmd","command":…,"result":…}`, plus `open` and `export`
steps) and stops at the first failure with a non-zero exit. A parameter string that is exactly `$N` or
`$N.path` takes that value from step `N` (0-based, in run order; `run --in`'s open is not a step);
`$last…` refers to the previous step. Paths index objects by key and arrays by position: `"$1.id"`,
`"$2.ids.0"`, `"$last.ids[0]"`. `$$` escapes a literal `$`. A reference naming no value fails that step.

```sh
vectorcraft-cli run --cmd file.new --params '{"width":600,"height":200}' \
  --cmd text.create --params '{"x":10,"y":60,"text":"plain and bold","size":30}' \
  --cmd text.setRangeStyle --params '{"id":"$1.id","start":10,"end":14,"style":"Bold"}'
```

## Verified output shapes

```
cmd file.new:        {"index":0,"previewMode":"default"}
cmd shape.rectangle: {"id":2}
cmd paint.setFill:   null
cmd text.create:     {"bounds":{…},"id":4}
cmd document.save:   {"bytes":31279,"format":"vectorcraft","path":"…","warnings":[]}
cmd document.export: {"bytes":23570,"format":"png","path":"…","warnings":[]}
```

Always read `warnings` — they say what a format lost or approximated, and what an import couldn't read
(e.g. an EPS shown as its preview, with the PostScript error named).

## Reproducible output

PNG, PDF, EPS, legacy `.ai`, packaged zips and saved `.vectorcraft` embed a timestamp. Set
`SOURCE_DATE_EPOCH` to a non-negative whole second count to pin it and make bytes repeat:

```sh
SOURCE_DATE_EPOCH=1791208980 vectorcraft-cli run --cmd file.new --params '{}' --export out.pdf
```

A command's own date parameter wins; a blank/negative/non-numeric value is ignored. SVG carries no time.
Password-encrypted PDFs stay non-reproducible (their encryption seed is random).

## Headless session memory

`run` and headless `mcp` share the desktop app's preferences folder, so both see:

- **Saved PDF presets** (`pdf.preset.save/.delete/.import`) — one `.vcpdfpresets` file each in `PDF Presets`.
- **ICC profiles** (`color.loadProfile`) — copied into `Color Profiles`, re-registered at the next start.

Nothing else persists (not Color Settings, not the document). `VECTORCRAFT_NO_PREFS` disables all reads and
writes; a server confined with `--automation-*-root` keeps nothing either. Damaged/oversized files are
skipped and logged.

## Environment variables

| Variable | Effect |
|---|---|
| `VECTORCRAFT_CLI` / `FILEATLAS_VECTORCRAFT_EXE` | Absolute path to `vectorcraft-cli.exe` used by the wrapper scripts. |
| `VECTORCRAFT_CONFIG_DIR` | Isolated config root (preferences, logs, recovery, user libraries) — use it for throwaway test profiles. |
| `VECTORCRAFT_NO_PREFS` | Disable preference reads/writes entirely. |
| `VECTORCRAFT_AUTOMATION_READ_ROOT` / `_WRITE_ROOT` | Default confinement roots for the app and the control channel. |
| `SOURCE_DATE_EPOCH` | Pin export timestamps. |

## Gotchas

- The wrapper scripts print compact, human-readable output with a trailing `exit=N` line, so do not pipe
  `vc.cjs raw commands` (or any step output) into a JSON parser — call `vectorcraft-cli commands` directly
  when you need machine-readable JSON. `vc.cjs info` is the exception: it echoes the CLI's JSON unchanged.
- Windows error text is localized (`系统找不到指定的文件。 (os error 2)`). Branch on exit codes and
  `warnings`, not on English message matching.
- Use absolute paths for `--export`/`document.save` targets; relative paths resolve against the CLI's cwd,
  which is not necessarily the shell's.
- `vectorcraft-cli mcp --version` is not a flag (`unknown mcp option`); use `--version` on the top-level CLI.
- A source build does not put the CLI on `PATH`; only `cargo install --path apps/vectorcraft-cli --locked`
  or a release install does.
- New objects become the selection, so selection-based commands (`paint.*`, `object.*`, `pathfinder`) apply
  to what you just drew. A command that omits an object nested inside a selected group also omits it when
  `ids` lists it alongside that group.
