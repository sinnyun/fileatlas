# WorkCrate UI plugin ports

## Cordis integration

Plugin `fileatlas.workcrate-ui` retains command `workcrate/open`, panel
`workcrate/overview`, sidebar launcher and `workcrate/settings` WorkCrate page.
Dependencies are Projects, Boards, Export, Thumbnails, FileOperations, Settings
and UiBridge. Cordis owns registrations and listener lifetime. Projects owns
project/reference/preset storage; Boards owns official application configuration,
native document binding and preview cache. UI owns no parallel document format.

## Native workflow

Projects show title/cover cards, search, name/creation sorting and archived filter.
Creation first asks for the user-owned native document folder, then creates the
internal project record and official `.vectorcraft` document with an initial
artboard. If native creation fails, UI removes the incomplete project record and
leaves the user folder untouched. Duplicate also asks for a destination folder,
then copies the native document and internal project metadata before refreshing
the duplicate's board/preview cache. Rename, cover, archive/restore and confirmed
deletion use the Projects service. Project listing decodes explicit covers only
and never opens native documents.

Project and artboard browsing reads the local project registry, cached artboard
catalogue and cached preview files. Opening a project or page never starts
VectorCraft, its CLI or a port connection. Missing local previews show a sync
prompt. The explicit “同步工程与预览” action calls `refresh_project` in a
background worker and reloads the resulting local cache. Associate an existing
`.vectorcraft` file or create an A3 landscape artboard. Existing artboard
dimensions remain unchanged. Multi-select, all/clear/exit, ordered image export
and all-artboard PDF use Export. Native file pickers choose existing documents,
placement inputs and export destinations.

Third level follows `old/src/components/AssemblyStudioView.tsx`: left page rail,
center canvas, right material information. The October20
projects/pages/editor concept images were also inspected. GPUI Kit/theme roles
own controls and colors. The center canvas has two modes. Preview paints the
cached official PNG and starts nothing. Edit (`view/canvas.rs`,
`view/canvas_surface.rs`, `view/object.rs`) opens one official canvas session
through Boards and overlays only what the official program reports: the rendered
page bitmap, the official `selectionBounds` box with eight handles, and a
translucent prediction box while the pointer is down. Pointer input is mapped
from window pixels to document points and forwarded as an official
`pointer_gesture`; numeric fields call `object.setBounds` with `reference:0`;
undo/redo and zoom act on the official session. The panel implements no hit
test, transform arithmetic, crop, text layout or private history, and complete
editing remains available via `open_editor`.

Actual width/height are read-only. Title, size annotation, project override with
inherit/reset, material, size, quantity and extras use native PageInfo/header
updates. Stable input entities hold drafts; an explicit save commits. Existing
field-preset settings and native dropdown menus remain available.

The WorkCrate settings page contains the VectorCraft executable path and local
control port alongside existing field presets. “保存程序设置” persists through
`Boards.configure`; it does not connect to a port or launch either program.
Background CLI actions use the configured executable and same-directory CLI
discovery. Only “在 VectorCraft 中编辑” starts the official GUI window; the edit
screen reaches a GUI that is already running through its configured control port,
and otherwise stays silent.

## Background work and UI inspection

Catalog/cover reads use keyed ReadModel workers, completion Wake/listen and
cached snapshots. Preview decoding uses one sequential ReadModel queue; generation
keys reject results after navigation. Jobs use background_executor and busy gating;
Boards serializes native transactions. Render projects cached snapshots only.
ChangeGate publishes `workcrate.ui` only when its serialized state changes.

Snapshot contains screen (`projects`, `pages`, `editor`), project_id, board_id,
busy, page_count, boards (id/native_id/index/title/dimensions/info/preview_ready/
error), previews_ready, preview_pending, selection, connection, executable, port,
document and message, plus `canvas`: mode (`preview`/`edit`), busy, zoom, `state`
(the official `CanvasSnapshot`: live/tool/revision/dirty/selection/
selection_bounds/artboard/preview/preview_size/last_step, null in preview mode),
`object` (official node detail: id/kind/name/bounds/text/linked/status/link_path)
and `drag` (kind `move` or `handle<N>`, origin, start, current) while a pointer
gesture is in flight. View drop clears it. Existing tracking measures controls
under `workcrate.control.<id>`, including artboard, inspector, page previews,
project actions, material fields, `field-edit-input`, `field-edit-save`,
`settings-vectorcraft-executable`, `settings-vectorcraft-port`,
`settings-vectorcraft-save` and `settings-vectorcraft-browse`, and the edit screen
`workcrate-canvas-preview`, `workcrate-canvas-edit`, `workcrate-canvas-zoom-in`,
`workcrate-canvas-zoom-out`, `workcrate-canvas-undo`, `workcrate-canvas-redo`,
`workcrate-canvas-handoff`, `workcrate-canvas-viewport`,
`workcrate-canvas-artboard`, `workcrate-canvas-object`, `canvas-x`, `canvas-y`,
`canvas-width`, `canvas-height`, `workcrate-canvas-apply-bounds`,
`canvas-text-input`, `workcrate-canvas-set-text`, `workcrate-canvas-create-text`,
`workcrate-canvas-links-update`, `workcrate-canvas-links-relink`,
`workcrate-canvas-place-link` and `workcrate-canvas-place-embed`.

## MCP

Tools are lifecycle-owned by the global McpRegistry and all named `workcrate-ui.*`.
Standard control tools: `navigate`, `open_board`, `manage_project`, `create_page`,
`update_page`, `place_image` (arbitrary official supported design file),
`field_presets`, `configure_vectorcraft`, `refresh_project`, `attach_document`,
`open_editor`, `state`. Project create and duplicate actions require a user-chosen
native document folder. Configuration only persists the executable/CLI/port values;
browsing does not connect or start VectorCraft. `navigate`, `open_board` and `state`
read cached local project, board and canvas data — they never start the official
program. Full/per-call confirm tools include `delete_project`, `export_pdf`,
`export_pages`.

The canvas tools drive the same official session the edit screen uses, so a call
made over MCP and a click in the panel land in the same `.vectorcraft` file and the
same running program: `canvas_open` (board id), `canvas_state` (read the official
snapshot), `canvas_gesture` (official pointer events), `canvas_set_bounds`
(`object.setBounds` with `reference:0`), `canvas_set_text`, `canvas_create_text`,
`canvas_place` (`path`, `at`, `link` — widens the session's granted folder when
needed), `canvas_links` (`update`/`relink`), `canvas_step` (`undo`/`redo`) and
`canvas_close` (commit, close, then re-read the catalogue). Every mutating canvas
tool requires `confirm=true` and returns the official snapshot; none of them accepts
raw official command ids or arbitrary parameters. See src/mcp.rs and
src/mcp/{operations,editing}.rs for exact JSON schemas. Native service calls run
through blocking workers. Obsolete synthetic text/typography/link/replace tools are
removed.

## Limits and verification

The official CLI must be available for native creation, synchronization, canvas
edits and exports; only `open_editor` starts the GUI window, and the edit screen
uses a running GUI only when its control port already answers. Cached projects and
previews remain browsable while the official programs are unavailable. Multi-project
PDF is limited by official export and refused by Export. Canvas edits write the
native document as they happen, so `refresh_project` only has to re-read the
catalogue and previews; there is still no embedded second editor. `type_text`
in-place typing exists only on the live route, so the headless route creates or
replaces text instead. Native dialogs and layout acceptance require same-process
FileAtlas MCP validation; compilation is not runtime evidence. See
`docs/validation-ui/2026-10-21-workcrate-canvas-editing/README.md` for what the
current run did and did not cover.
