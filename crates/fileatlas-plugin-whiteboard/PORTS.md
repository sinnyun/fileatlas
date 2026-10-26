# FileAtlas canvas workspace plugin

## Ownership and source reuse

The Cordis product plugin remains `fileatlas.whiteboard` / `fileatlas/whiteboard`
for compatibility with the just-added settings, MCP names and user data. Its visible
product name is **画布**. It owns the unique canvas panel, compact top-left document
actions, bottom-center drawing toolbar, board switcher, resource list, layer list,
import/export and host callbacks. Style controls sit at the canvas upper-right and
undo/redo directly above the drawing toolbar. Cordis owns
startup and cleanup; UI Bridge owns the rail/panel/view registrations.

The engine is the local `fileatlas-canvas-engine` workspace crate. It contains a
source copy from the registry package `gpui-whiteboard 0.5.1`, upstream commit
`d94ade3eb162c041b71994681356fcbcb4dcfd8e` (MIT). `Cargo.lock` now has no
`gpui-whiteboard` package dependency; the product links only to the FileAtlas
workspace crate. The upstream crate boundary was confirmed from its published
manifest and package source. No Zorite application crate, note store or GPL
application code was copied. Preserve `LICENSE`, bundled JetBrains Mono OFL terms,
and `assets/icons/LICENSE` (Lucide/ISC). The engine PORTS records the local
adaptations and licenses.

The engine owns Scene JSON, camera mapping, GPUI input, hit testing, selection,
transforms, drawing, edit transactions and one undo/redo history. FileAtlas-added
entry points only expose embedded mode, current elements/selection, layer selection,
paint-order movement, and existing tool-group/color/font/template UI. Layer reorder
is an engine scene edit and runs its same history, `on_change` and save path.

## Public integrations and lifecycle

- `WhiteboardService` is `fileatlas/whiteboard`; its versioned JSON is a modest
  whole-document library. No new database or plugin manager is introduced.
- Existing `MediaResourceService` / `ResourceProvider` is the stable resource
  contract. Provider registration returns a drop token and removes the provider at
  plugin stop. Local image/video/audio/file links share a URI and provider open path.
- `CanvasToolContribution` is the only new toolbar extension. It carries command ID,
  icon catalog key, group/order and optional shortcut; `register_tool_for_plugin`
  binds its registration token to the contributing Cordis plugin's stop lifecycle.
  The command itself still uses the owner's normal UI Bridge command registration,
  permissions and `UiCommandInvoked` observer. Built-in drawing tools remain one
  engine-owned tool set rather than per-shape plugins.
- The host's file-card action resolves the same ResourceLink then asks the provider
  to open/play; if unsupported it hands the resolved file to QuickLook. A video
  inserted as a card carries a visible `视频 ·` title prefix, remains a normal
  selectable scene element, and appears in the resources list. There is no embedded
  video playback, duration/timeline annotation, or arbitrary external card renderer.
- Image insertion, clipboard paste, drops, templates, font selection, color editing,
  import/export and scene persistence call the copied engine's existing semantics.
  No parallel canvas scene or undo stack is created.

## Storage, compatibility and recovery

Normal path: `%LOCALAPPDATA%/FileAtlas/whiteboards/library.json`.
Isolated profile: `<FILEATLAS_DEV_TEST_DIR>/whiteboards/library.json`.
Library format remains version 2; it stores board titles/IDs, opaque Scene JSON,
font links, templates, user document colors, ResourceLink URIs and active board.
Large pasted images remain separate PNGs in `whiteboards/pasted-images/`. Source
media remains external.

Version 1 `path` values migrate losslessly to version 2 `uri`; old absolute image
paths still resolve. Zorite Scene JSON import/export remains compatible and leaves
image paths unchanged. Writes use a sibling temporary file and backup; a valid
`.bak` is restored on primary corruption while retaining `.corrupt`. Unsupported
future versions stop startup without overwriting source data. Import validates the
Scene before changing the library. No SQLite schema or existing FileAtlas media rows
are changed.

## MCP and debugging

Existing registered tools remain `whiteboard.state`, `.create`, `.open`, `.rename`,
`.import`, `.add_images`, `.flush`, `.export`. Read state includes board scenes and
resource links. High-impact export retains Full access plus per-call confirmation.
Canvas pointer interactions use the FileAtlas MCP `control_ui` event path against the
same GPUI child; no alternate drawing endpoint exists. Errors are visible in the
panel or save status. Inspect `.local/validation/profiles/<run>/whiteboards`,
primary/backup/corrupt files, the same-process MCP log and plugin startup diagnostics.

## Current feature limits

Implemented in the FileAtlas shell: unique drawing toolbar, select/pan/text and tool
families for shapes, pen/connectors and resources; color/font/template access; one
undo/redo pair; image/file insertion; image/video resource listing; board list; layer
selection and up/down reorder through engine history/save; existing save/restore and
Scene import/export.

Still open: command-backed toolbar extensions cannot define new Scene element
semantics, and there is no arbitrary card renderer API; properties inspector is
read-only beyond selecting/reordering layers; layer grouping, hide/lock; resource
search/filter and video playback; nested layer UI; shortcut dispatch for the
optional toolbar shortcut metadata; full light/dark, changed-accent and narrow-window
visual coverage. The concept marks these states as future work, not shipped behavior.

Validation evidence: `docs/validation-ui/2026-10-26-fileatlas-canvas/README.md`.
