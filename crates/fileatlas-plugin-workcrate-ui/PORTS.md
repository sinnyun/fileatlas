# WorkCrate UI plugin ports

2026-10-23: New/duplicate projects use adjacent metadata automatically. The project-card
more menu adds “迁移项目数据”; the overview adds “打开项目文件夹” to recover moved projects
or import into a fresh profile. Reuses native folder picker and existing background/reload
and Cordis cleanup. MCP `workcrate-ui.project_folder` accepts action relocate (project_id),
open (folder), inspect (project_id); returns folder/ID, recovered projects, or resolved
storage/document/reference paths. It is a Standard mutation tool on the same UI service paths.
Official links diagnostics are available in header_state; no custom relinking subsystem.

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
document, message and `auto_sync` (syncs/pending/error), plus `canvas`: busy, zoom, `state`
(the official `CanvasSnapshot`: session_id/live/tool/revision/dirty/selection/
selection_bounds/artboard/preview/preview_size/last_step, null before a session opens),
`object` (official node detail: id/kind/name/bounds/text/linked/status/link_path)
and `drag` (kind `move` or `handle<N>`, origin, start, current) while a pointer
gesture is in flight. View drop clears it. The editor screen is a single interface:
showing a page opens the official session automatically (`ensure_canvas_session`)
and the former preview/edit mode split is gone; entering the page list or project
list commits and closes the session as before. Existing tracking measures controls
under `workcrate.control.<id>`, including artboard, inspector, page previews,
project actions, material fields, `field-edit-input`, `field-edit-save`,
`settings-vectorcraft-executable`, `settings-vectorcraft-port`,
`settings-vectorcraft-save` and `settings-vectorcraft-browse`, and the edit screen
`workcrate-editor-rail`, `workcrate-canvas-zoom-in`,
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

## 2026-10-22 single merged editor interface

The preview/edit mode toggle is gone. `canvas_mode` was removed from
`WorkCratePanel`; `toggle_canvas_mode` and `canvas_mode_button` no longer exist
and the `workcrate-canvas-preview` / `workcrate-canvas-edit` controls were
removed. Showing a page (rail click, restore, remote selection or a landed
snapshot) runs `ensure_canvas_session`, which opens the official session the
same way the edit toggle did; autosync, field saves and the preview watcher
keep refusing only while `canvas.is_some()`. The editor toolbar now always
shows zoom/undo/redo plus the former preview-only `place-design-file` and
`export-current-image` actions beside `workcrate-canvas-handoff`. The right
rail (`workcrate-editor-rail`, 310 px) stacks the canvas object panel and the
page material inspector; `inspector` and `canvas_object_panel` render inner
content only. The former local-bitmap preview branch in `render_editor` is
removed — the artboard is always the official frame. `canvas.mode` no longer
appears in `workcrate.ui` state.

## 2026-10-22 no panel-wide disabling during canvas jobs

`canvas_action`, position/size number inputs and the toolbar actions no
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

## 2026-10-22 canvas accepts dropped images

The canvas viewport (preview and edit mode) registers `on_drop`/
`drag_over` for GPUI's external `ExternalPaths`, `FileManagerFileDragPayload`
(only when `allows_drop()`) and `FileStagingDragPayload`. All three feed
`canvas_drop_files`: extensions are filtered against the image whitelist,
non-images hint "拖入的内容里没有可放置的图片。", a busy canvas hints
"画布操作进行中，请稍候…", and each image runs
`canvas_place_dropped(board, path, centre+24·i)` — opening the session quietly
first when none holds the page, and copying sources the official single read
root can never reach into `<document folder>/materials` (see the boards
PORTS) before placing the copy linked.
Hovering a drag over the viewport highlights its border with `accent_hover`.

Because GPUI 0.2.2 seals `ExternalPaths` construction, MCP validation uses the
semantic equivalent `control_ui {action:"canvas_drop_files"}` → surfaces key
`workcrate.canvas.drop-request`, consumed by the panel's preview watcher
(1.5 s) into the same `canvas_drop_files`; `ui.drops` counts accepted drop
requests. `boards.canvas_holds` now returns `Ok(false)` when no session is
open (it previously errored, contradicting its name; both earlier callers
guarded with `canvas_is_open()` first). Validation:
`docs/validation-ui/2026-10-22-workcrate-canvas-drop/README.md`.

## 2026-10-22 projects screen redesign: shared tier band and unified icon buttons

The Tier 1 projects screen follows `docs/reference-ui/concepts/
workcrate-projects-user-2026-10-21.png`: a centered three-tier entry band
(`WorkCratePanel::tier_band`, shared by all three screens), a hero header with
the unified search field plus `icon_button` sort/archive toggles and a primary
新建工程, redesigned project cards (icon row 重命名/复制/删除/更多-menu, meta
line "N 个画布 · 相对时间", first-page info band from the boards cache,
preview caption, ".vc 关联工程 / 进入页面矩阵" footer) and a centered
创建新物料工程 tile. All paints come from the native theme and
`fileatlas_ui_bridge::components`; no local palette.

MCP surface: `workcrate-ui.state` now reports `sort_by_name`/`show_archived`.
The pages/editor breadcrumb was replaced by the same tier band plus a slim
action bar (关联工程/新增 A3 页面/同步工程与预览 with icons); the tier pills
keep the old control ids `workcrate-projects-home`/`workcrate-pages-home`, so
older scenarios still navigate. Page counts and first-page summaries are read
from each project's boards cache inside the snapshot read (no official
process). Relative-age formatting is a pure function with unit tests.
Validation: `docs/validation-ui/2026-10-22-workcrate-projects-redesign/README.md`.

## 2026-10-22 pages and studio screen redesign: unified icon buttons and section cards

The Tier 2 pages screen follows `docs/reference-ui/concepts/
workcrate-pages-user-2026-10-21.png`: a hero header (eyebrow, project-name
title, "共 N 个独立物料 · 已选中 M 个" statistics) with a multi-select
`icon_button` toggle (ListChecks), 全选/取消全选 ghost buttons, 关联工程/
同步工程与预览 outline actions and a primary 新增画布; page cards with an
always-visible checkbox icon (Square/SquareCheck, selecting turns multi-select
on), spec line, optional material info band, preview (unchanged open/toggle
semantics) and a "第 N 页 · 编辑" footer; and an export bar with the
PNG/JPEG toggle, 导出选中页为图片 (FileImage) and 导出全部为 PDF. The slim
screen bar now shows only on the editor screen.

The Tier 3 editor follows `workcrate-studio-user-2026-10-21.png` +
`workcrate-inspector-2026-10-21.png`: the canvas toolbar uses unified icon
controls — 添加图片 (ImagePlus), 新建文字 (Type), ZoomOut/ZoomIn, Undo2/Redo2
via the new `WorkCratePanel::icon_action` (same field-save/canvas-busy guards
as `screen_action`; the old text-only `action` helper was removed),
放置设计文件 (FilePlus), 导出本页图片 (FileImage) and a primary 在 VectorCraft
中编辑 (ExternalLink); the left rail renders numbered mini-cards (accent
number, truncated title, physical-size line, accent border when active); the
object panel and page inspector render as shared `section_card`s
(画布对象·官方选区, 位置与尺寸·官方 bounds, 文字/文字内容·官方文本,
链接图片·外部文件, 文字格式·字体与字号, 画板·添加图片, 页面信息·画板规格,
页面内容·同步物料栏). Canvas semantics (gestures, mapping, drop, official
session) are unchanged.

All previous control ids are preserved (`pages-multi-select`, `pages-select-all`,
`pages-clear-selection`, `pages-exit-multi-select`, `select-page-{id}`,
`open-page-{id}`, `page-preview-{id}`, `export-*`, `rail-{id}`, `field-*`,
`preset-*`, `page-add-field`, `workcrate-canvas-*`, `place-design-file`,
`export-current-image`, `workcrate-canvas-handoff`), so existing scenario
scripts keep working. Validation:
`docs/validation-ui/2026-10-22-workcrate-pages-studio-redesign/README.md`
(scenario `scripts/validate-workcrate-pages-redesign.cjs`).


## 2026-10-23 settings and scoped defaults (supersedes name inheritance above)

Settings → Plugins → WorkCrate uses shared SettingsPage schema for default project sorting, image format, official GUI executable, optional separate CLI executable, control port, all builtin/custom default-information columns and their values. Program configuration remains Boards `vectorcraft.json`; sort/image-format defaults use shared SettingsService `Settings.workcrate { sort_by_name, image_export_jpeg }` (serde-default compatible, applied when opening a new panel). Existing per-panel sort/export controls remain temporary choices. Startup mode/enabled controls remain owned by the settings host. No configuration action starts VectorCraft.

The tier bar offers 项目默认信息 (or 程序默认信息 outside a project), opening the Cordis-owned generic panel `workcrate/project-defaults` (960×800, minimum 700×480). The same form has a scope selector for program or an existing project. Project values auto-save through Projects, can be added/edited/deleted, custom fields can be added/removed. Each project column has 合并到程序默认信息, a non-destructive deduplicating union that preserves unrelated columns. Busy/pending/invalid drafts block scope switches and promotions. External project preset changes refresh a clean mounted form from memory via existing Wake; unchanged snapshots do not notify. Page dropdowns query effective presets when opening; custom project fields also appear in the inspector. 页面内容 now starts with independent 物料类别; inheritance/global-name/reset controls are removed. Title renaming remains project management behavior.

MCP `field_presets` accepts optional `project_id` to select project storage; absent selects program. New Standard tool `promote_project_field { project_id, key }` performs the same per-column action. `state` accepts optional `project_id` and returns `project_presets`, `effective_presets`, plus `defaults_form` (scope, draft columns, save/error/notice, program-path drafts) from the shared view snapshot. New control ids include project-defaults, settings-preset-scope, settings-vectorcraft-cli, settings-promote-<key>; existing settings value/add/delete controls stay stable. Errors surface in the form; failed writes retain previous saved data. Evidence and remaining interaction boundaries: docs/validation-ui/2026-10-23-workcrate-defaults/README.md.


## 2026-10-23 native single-line field writes

The inspector still edits individual values through the same `save_field` pipeline; Boards now aggregates the right material/size/quantity/extra values into one native right-aligned point-text object. Category is its own native text. This supersedes the old native per-field layout while preserving the existing per-page card/frame patch and warm session. Overflow returns 保存失败 with a Chinese single-line capacity message, keeps the UI draft and leaves all native nodes/previous values intact. There is no new UI input scheme or user persistence file.

New read-only MCP `workcrate-ui.header_state {}` calls public `canvas_header_state`; it requires an existing session and neither starts nor refreshes one. Its IDs, official models/bounds and last successful header command report let live debugging distinguish per-page preview repaint from native object mutation. It registers and cleans up with existing WorkCrate MCP registration. See Boards PORTS for payload/limits and scripts/validate-workcrate-single-line-header.cjs for native IDs, rollback, legacy migration and roundtrip assertions.


## 2026-10-23 page order and read-only page information

Both the Tier-2 matrix and Tier-3 page rail expose native themed up/down buttons (first/last disabled)
and a GPUI typed PageDrag grip. Drop uses the target page's current position, in either direction;
project and captured id-order validation reject stale/cross-project payloads. A draft, field save,
canvas job or catalogue job disables ordering. Ordering runs a background order_job and patches the existing cards once, preserving the
selected page id, multi-selection, canvas session/frame and unaffected decoded images.
No custom pointer injector, drawing implementation or independent drag manager was added.

The 页面信息 card now contains only a read-only generated title; title/physical edit controls and
physical/pixel/point-size placeholders in the matrix, rail and canvas context have been removed.
Material size is still editable in 页面内容. Old title and physical inspector keys are refused.

Standard MCP `workcrate-ui.reorder_pages { project_id, board_ids:[string] }` returns `{ boards, changed, session_id }`
and publishes a coalesced order result through the existing UI-state Wake (no catalogue reload). update_page now permits material-category/material/size/
quantity/extra patches only; the title parameter is removed and legacy page-info edits are rejected.
MCP registrations and cleanup still belong to the UI Cordis plugin.


OrderFrame decodes only changed previews off the UI thread and merges records by stable page id
in one delivery. Pending order jobs have a generation so an older completion cannot replace a
newer result. The existing read-model probe generation is invalidated while retaining the
visible canvas; busy ordering pauses probes, and delivery restarts them on the same session.
Passive canvas jobs retain action errors instead of clearing them on a later probe. MCP and
manual ordering share this patch path. No new polling/connection manager or separate page store.


## 2026-10-23 remove position/size inspector

The right canvas inspector no longer renders the position/size section, canvas-x/y/width/height controls or workcrate-canvas-apply-bounds. Its private NumberInput entities, synchronization, parser and apply handler are removed. Text editing/typography, image links, generated page title and material fields remain. Native selection geometry, drag/scale gestures and the public Boards/MCP bounds APIs remain unchanged. No persistence or service schema change. Evidence: docs/validation-ui/2026-10-23-workcrate-no-bounds/README.md.


## 2026-10-23 plain text controls

The typography card, font/size input entities and font/size/alignment handlers are removed. Content editing remains, as do native object drag/scale handles, canvas zoom and official transformations. No document-wide reformatting is performed. Updated no-bounds scenario now expects the font input to be absent; current full evidence is docs/validation-ui/2026-10-23-workcrate-plain-text/README.md.


## 2026-10-23 object info at the footer

The right canvas object information card is removed. Editing now projects the existing official selection count, single object name/type, or empty-selection text onto the right of the existing fixed-height status footer. Status/errors remain on the left. The right text is one truncated right-aligned line capped at 60%/600 logical px; no IO or polling is added. Read-only workcrate-ui.state UI snapshot includes object_status, and control canvas-object-status exposes geometry. Editing controls, default font and object/viewport scaling remain unchanged. Evidence: docs/validation-ui/2026-10-23-workcrate-object-footer/README.md.


## 2026-10-23 material content first

The right-side page information/automatic-title card is removed, and inspector(card) is composed before canvas_object_panel. The rail starts with 页面内容, followed by existing text/image controls. Automatic titles still appear in page lists and editor context. Preview/read errors formerly in the removed card now remain visible inside the material content card. No backend, persistence, session or edit semantics changed. Evidence: docs/validation-ui/2026-10-23-workcrate-content-first/README.md.
