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
deletion use the Projects service. Project listing decodes explicit covers or the first locally cached page
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
own controls and colors. The center canvas has two modes. Initial preview paints the cached official PNG and starts nothing. After explicit
editing or GUI handoff, preview retains that session and follows official revisions. Edit (`view/canvas.rs`,
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
document, message and `auto_sync` (syncs/pending/error), plus `canvas`: mode (`preview`/`edit`), busy, zoom, `state`
(the official `CanvasSnapshot`: session_id/live/tool/revision/dirty/selection/
selection_bounds/artboard/preview/preview_size/last_step, null before a session opens),
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
`workcrate-canvas-place-link`, `workcrate-canvas-place-embed` and one
`rail-<board_id>` per page in the editor's page rail.

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

## 2026-10-22 refresh and inspector revision

Preview/edit toggles retain the explicitly opened session and undo history. Each
frame updates the current BoardCard/rail; `boards[].canvas_revision` identifies
that frame for MCP inspection. Page/project exits still commit and close. Explicit
GUI handoff now opens an observer session in preview mode; passive browsing never
opens one. Native catalogue mutations retire the old editing session before their
transaction. Stale probe and file-picker results are ignored after navigation.

Background reload retains same-project images and reads the selected page first.
Project cover fallback reads the first cached artboard locally; explicit covers
remain authoritative. Project search responds to input changes. No new storage
format, private renderer or history is introduced.

The inspector uses a multiline Textarea, clears object fields on selection changes,
rejects nonfinite geometry, disables text/link operations for incompatible selections,
and adds font/point-size/left-center-right alignment through official text.setStyle.
`canvas_text_style {id, font?, size?, justify?}` is a Full/confirm MCP tool;
justify is left/center/right/justifyAll. Controls are canvas-font-input,
canvas-size-input, canvas-apply-style and canvas-align-{left,center,right}.
These format inputs are change requests (blank preserves the current attribute),
not a computed mixed-style font inspector. See the October 22 refresh validation.

## 2026-10-22 single-field saves

`save_field(id, key, value, label)` is the inspector's save path and deliberately
does not use `perform`: it never sets `busy`, never reloads the project, and never
clears the rail. Each in-flight draft is tracked in `field_saves` as
`(board id, field key)`, which renders as the row's own 保存中 marker and blocks a
second save of that same field only. The panel state exposes it as
`field_saves: ["<boardId>#<key>"]` beside `editing: "<boardId>#<key>"`, so MCP can
tell which row is open even though every edit row shares the `field-edit-input`,
`field-edit-save` and `field-edit-cancel` geometries.

The job re-reads the page record from Boards, applies one field
(`management::apply_field`) and calls `update_info`. Before that it reuses the
session already held for this page (`canvas_holds`) or warms one quietly
(`canvas_open_quiet`), so a cold save opens the silent official session itself
and later saves keep using it; the page stays in preview mode throughout. The
official frame is decoded and its selected object queried on the same background
thread (`canvas_frame`), never on the UI thread. `apply_field_save` patches only
that card: a returned frame goes straight into the overlay
(`boards[].canvas_revision` then equals the session revision), otherwise only that
page is queued in the preview queue. An unchanged draft (`changed:false`) queues
nothing at all and closes only a session this job opened. A refused write keeps
the draft row open and reports `保存失败：…` with the Chinese message from Boards;
internal role ids never reach the strip. `management::field_label` supplies the
Chinese field name used by `已保存：<字段>`.

Saves are serialized: another save is refused while `field_saves` is non-empty,
and page switching, canvas mode toggles and canvas jobs are refused until it
returns. A save that warms a session starts the canvas probe poller after
applying its frame, so changes made through the official program still reach the
preview.

## 2026-10-22 inspector cards and stable rows

The inspector is two `section_card` groups, 页面信息 · 画板规格 and 页面内容 ·
同步物料栏. Every field row is a fixed 32 px; a draft replaces only that row's
value with the shared `field-edit-input` plus 保存/取消, so opening, committing or
cancelling a draft never moves the rows below or resizes the panel. `begin_edit`
focuses the input and its `PressEnter` subscription commits the draft. The panel
footer is a fixed 24 px status line (busy / saving / last message), so save
feedback no longer changes the content height. The add action is the full-width
`page-add-field` row labelled 新增物料参数行.

## 2026-10-22 automatic preview acquisition

`view/autosync.rs` keeps an idle watcher (`refresh::poll`, 1500 ms) that asks
Boards for `preview_state` only while the panel is not busy, no canvas session is
held or wanted, no field save is in flight and no state read is pending. An
unchanged stamp publishes nothing and starts no process. A stale stamp starts one
`auto_refresh_project` job on the background executor; a failure keeps the cached
previews, records `auto_sync.error`, shows `自动同步失败：…` once and retries
after 30 s. Success patches `boards_view` without reloading the project: existing
decoded images stay as placeholders, `load_generation` invalidates old decodes,
the selected page is queued first, selection and editing state survive when their
page still exists, no official editor session opens, and the footer reports
`已自动同步 VectorCraft 的更改`. State JSON gains
`auto_sync { syncs, pending, error }`. While a canvas session is open the watcher
stays idle because the existing canvas probe owns that page; leaving the canvas
refreshes the catalogue. This path never drives, focuses or activates the user's
GUI, and it sees only the saved file.

## 2026-10-22 page switching inside the editor

A page pick while the page editor holds an official session no longer reopens it.
`switch_canvas_page` calls `canvas_open` for the selected board and Boards moves the
same official process to that artboard; `session_id` stays the same, the edit or
preview mode is untouched, and the landing frame patches that page's card
(`boards[].canvas_revision`). The page rail and `workcrate-ui.open_board` both reach
this path. Only selecting another project, returning to the page list or project
list, or closing the panel commits and closes. A pick that arrives while a frame
or a field save owns the session is re-applied when it lands (`apply_canvas_frame`,
field-save completion), and entering edit re-converges on the selected page. A new
native page remains a catalogue transaction, so it saves and closes the preview
session before the editor reopens. Rail entries expose
`workcrate.control.rail-<board_id>` geometry for pointer validation. Evidence:
`scripts/validate-workcrate-page-switch.cjs` and
`docs/validation-ui/2026-10-22-workcrate-page-switch/README.md`.

## 2026-10-22 localized refresh while editing

An open editor no longer reloads the whole catalogue for ordinary actions. The UI
state listener compares `(selected_project, selected_board)` and both refresh
revisions: a page pick while a session is held goes to `switch_canvas_page`, a
catalogue revision clears the canvas, a board-only revision calls
`refresh_visible_board` (reopens/moves the held session and repaints that one card),
and only a selection/catalogue change calls `reload`. `projects.wake` is handled
by `deliver_project_change`, which re-reads `projects(true)` and `field_presets()`,
returns `false` when nothing changed and otherwise patches `items`/`preset_fields`
without touching cards, previews or the session. Reference placement and relink
therefore no longer reload. `CanvasFrame` carries the page record
(`board: Option<WorkCrateBoard>`), so `apply_canvas_frame` patches the matching
card without a catalogue read. Project rename patches the item in place; export
only commits/closes the canvas it must and never reloads; placement re-reads the
placed page only. `perform` remains only for actions that really change the
catalogue or must close the session: create project/page, attach document, sync,
project archive/duplicate/cover/delete, leaving the canvas and canvas errors.

`ui.reloads` counts whole-catalogue reloads, so MCP can assert that a localized
action kept the count, the `session_id` and the card id list unchanged.
`WorkCratePanel` is still a single GPUI view: any notify rebuilds its element
tree. What was removed is the catalogue data reload and its re-decode/re-queue
work, not element-level incremental painting. Auto-sync keeps its whole-catalogue
card swap (images retained, counter untouched) because the native document exposes
no per-page content revision. Evidence: `scripts/validate-workcrate-local-refresh.cjs`
and `docs/validation-ui/2026-10-22-workcrate-local-refresh/README.md`.

## 2026-10-22 no toolbar-wide disabling during field saves

`WorkCratePanel::action` no longer counts `field_saves` in its `disabled`
state; a click while a save is in flight sets the hint "正在保存页面内容，请稍候…"
and is refused. Every action handler already refuses while `field_saves` is
non-empty, so no new gating was added. A `renders` counter in
`workcrate-ui.state` measures whole-view renders; screenshot row diffs showed
typing changes ~58/1831 pixel rows and a mid-save frame ~74/1831 rows after the
fix (previously 840/1831 because every button greyed out). Wrapping inputs in
child view hosts does not scope GPUI dirty propagation (ancestors are always
marked dirty) and was reverted.

## 2026-10-22 no panel-wide disabling during canvas jobs

`canvas_action`, `canvas_mode_button` and the position/size number inputs no
longer disable while `canvas_busy`. Structural disabled conditions remain (no
canvas, set-text needs a type object, links-update needs a linked object); a
click while `busy || canvas_busy` sets the hint "画布操作进行中，请稍候…".
`WorkCratePanel::action` (canvas toolbar zoom/undo/redo, top toolbar) also
drops `canvas_busy` from its disabled state with the same click hint. Numeric
drafts are gated by the apply buttons' refusal, not by input disabling.

## 2026-10-22 proportional corner scaling in the edit canvas

`CanvasDrag` steers corner-handle samples (handles 0/2/5/7) onto the
ratio-preserving path through the opposite corner (`constrain_corner`: the
dominant axis sets the scale, crossing both axes flips the box; edge handles
stay single-axis). The official engine still performs the hit test and the
scale — only the sampled points change. This is implemented here because the
installed vectorcraft-cli 0.14.0 ignores `pointer_gesture`'s `mods` parameter
in headless sessions (verified: four spellings all scale freely). The visual
prediction and the forwarded gesture share the same constrained points.
