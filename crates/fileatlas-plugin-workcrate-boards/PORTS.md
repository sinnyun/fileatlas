# WorkCrate Boards ports

2026-10-23: `create_project_document` creates adjacent `.workcrate/<project UUID>` data before
writing initial previews/cache. Subsequent cache and canvas preferences use the existing
Projects.project_directory. `relocate_project(&str) -> Result<PathBuf, BoardsError>` migrates
only FileAtlas metadata/cache; `open_project_folder(&Path) -> Result<Vec<WorkCrateProject>,
BoardsError>` restores moved datasets. Cold migration/recovery starts no process and leaves
native/image bytes untouched. An existing canvas is released, saving only dirty content when
its old file still exists; recovery never saves to a vanished old folder. Native image links
and updates remain VectorCraft-owned. Cached first-image metadata uses root-relative paths
for in-folder files. Existing-session canvas_header_state also returns official links.list
under links, for same-process validation after folder moves.

Cordis plugin `fileatlas.workcrate-boards` publishes `fileatlas/workcrate-boards` and depends on Projects. `VectorCraftConfig { executable, cli_executable, port }` remains in `vectorcraft.json`; both executable fields are optional strings and `port` defaults to 17979. A blank CLI path is resolved as `vectorcraft-cli.exe` beside the configured GUI executable. The legacy loopback transport is used only by `connect()` and `open_editor()`; only an explicit editor action starts the GUI.

`board_catalog`, `load_board`, `board`, `boards_for_project` and `render_preview` read FileAtlas-owned `projects/<uuid>/boards-cache.json` and `native-previews/` only. They never start or contact VectorCraft. A missing catalogue is empty; a missing cached preview returns `PreviewUnavailable`. Cache entries are bound to the persisted native-document association UUID, so `attach_document` clears old cache and any binding rotation makes old board IDs unavailable.

`create_project_document(project, directory)` creates the first A3 landscape document through the official `vectorcraft-cli.exe mcp --headless` JSON-RPC stdio server, writes the `.vectorcraft` file into the selected folder without overwriting, adds the native material header, and stores catalogue/preview cache inside FileAtlas. `refresh_project(project)` is the explicit disk-to-cache synchronization action; it runs on the same routed session, so when the GUI already holds that document it reads the GUI's live state instead of refusing. Creating a document always uses the silent headless session: a new native file must never hijack the user's open GUI window. Both operations use bounded child lifetime, hidden Windows process creation, isolated CLI preferences under the FileAtlas project root, and one explicit native-folder read/write root. Preview PNG bytes are returned inline by the official renderer and written into FileAtlas by the manager.

`create_page`, `update_info`, `place_reference`, `place_file`, `export_native` and `refresh_project` run through `official_session`, which chooses the transport and refreshes the durable snapshot after a successful write: the user's running GUI when its control port answers, otherwise the silent headless session. A running GUI is no longer a reason to refuse a background write, because the write goes to that GUI's own engine instead of racing a stale in-memory copy. Placement still requires the native document and the source file to share one collapsed read folder; cross-volume sources are refused. Export reads from the native folder and writes to the chosen output folder. `open_editor` alone connects to or starts the GUI window, navigates to the live artboard and focuses it. The legacy TCP JSON-line client (`protocol.rs`) now serves only `connect()` and `open_editor()`; every other native call uses `mcp::McpSession`, which speaks newline JSON-RPC, bounds replies to 32 MiB, times out at 65 seconds, never retries a mutation, and poisons the child only after an uncertain outcome (timeout, response ID mismatch, oversized or unparsable line). A command-level `isError:true` reply is a well-formed answer from a server that keeps serving, so the session survives it; `command_error_keeps_the_headless_session_usable` guards that behavior.

## Canvas editing session

`canvas_open(board_id)` starts the page editor's official editing session, or moves an existing session to another page of the same document, and `canvas_close()` ends it; `canvas_is_open()` reports whether one is held, `canvas_open_quiet(board_id)` performs the same setup without a warm-up render, and `canvas_holds(board_id)` reports whether the held session already belongs to that page (field saves use these two to render once). One `mcp::McpSession` serves both transports: `McpSession::live(cli, port)` runs `mcp --connect 127.0.0.1:<port>` against the user's GUI when `port_is_listening` answers, otherwise `McpSession::headless` runs `mcp --headless` silently with `VECTORCRAFT_CONFIG_DIR` pointed at that project's own `cli-config` folder, so a long-lived session never starts a second engine over the shared preferences. Editing semantics stay official: `pointer_gesture`, `select_tool`, `object.move`, `object.setBounds`, `file.place`, `links.update`, `links.relink`, `text.create`, `text.setText`, `undo`, `redo`, `doc_inspect`, `render_preview` and `save_file`. `CanvasSnapshot` carries only what the official program reports (`revision`, `dirty`, `tool`, `selection`, `selectionBounds`, artboard geometry, the rendered preview path and size, last history step) and `canvas_object` returns the official node's kind, name, bounds, text and link status (`kind` is lowercased, link status is reported as `ok`/`modified`). `object.setBounds` is always sent with `reference:0` so x/y mean the box's top-left; the official default is the centre. A session that reports no open document is an empty state, not a failure: `open_document` opens the project's document, and `inspect_bound` reopens the bound file when a live GUI has closed it. If a reachable GUI still cannot make the document editable, `canvas_session` drops that transport and opens the saved document in the silent headless session, so the background editor never depends on a usable GUI window. A document that still fails to become active after that is reported in Chinese instead of the official protocol text.

Each edit writes the native document through `save_file` as soon as the official program reports `dirty`, so the `.vectorcraft` file, the GUI and `refresh_project` always describe the same page; `canvas_commit` covers leaving the canvas after an official change no command reported. `canvas_probe(known)` is the observer-side change probe: the caller passes the revision it last displayed, because a mutation made by another client (an automation call, a second panel) advances the session without that observer ever seeing the returned snapshot. `type_text` exists only on the live transport; the headless session creates or replaces text with `text.create`/`text.setText`.

The official launcher accepts exactly one `--automation-read-root` and one `--automation-write-root`, so a session's inputs collapse to the single deepest folder holding them (`automation_folder`). A canvas session is granted the document plus only those project references that already live inside the document's own folder; when the user points at a file elsewhere, `canvas_grant` commits the document, closes the running session and relaunches it with that folder inside the collapsed root, because the engine fixes its automation roots at launch. A live GUI keeps the roots its owner launched it with, so a refused path there stays a visible error instead of silently restarting the user's program. A live session shares the GUI, so every mutation first re-checks that the active document is still the bound one, and `open_document` activates an already-open window (found through `inspect_ui`, switched with `document.activate`) instead of re-reading it from disk.

`attach_document(project, path)` validates the existing `.vectorcraft` with a read-only headless session (`open_file` plus `doc_inspect` path check) and never contacts the GUI; `native::revalidate_open_headless` shares the open-report policy with the GUI opener. VectorCraft 0.14.0 lists the headless `export` tool with `additionalProperties:false` and only accepts `artboard`, `format`, `options`, `outlineText`, `path`, `range`, `scale`, `selection`; PDF uses `range` (never `useArtboards`, which exists only in the GUI command catalogue) and rasters use a single `artboard` plus `scale`. `operations.rs::OFFICIAL_EXPORT_ARGS` pins that list and the non-ignored `export_params_only_use_official_tool_arguments` test guards it. A headless `open_file` `missingLinks` entry is treated as a sandbox artifact only when the linked file still exists on disk outside the native document's folder (the read root cannot cover it); warnings, modified links, and genuinely absent links are blocking and are recorded in the diagnostics map until a successful revalidation.

Page identity remains project UUID + persisted association UUID + native numeric artboard ID. Header content is real native text tagged with versioned `fileatlas.workcrate` notes; edits are undo grouped. Raster/PDF exports are official CLI exports and are atomically placed at the requested output path. PDF `options` starts with `VectorCraft Default`, disables `preserveEditing`, and uses color/gray JPEG `high` compression with bicubic downsampling to 300 PPI above 300 PPI and `compressText: true`. Other PDF settings retain official defaults. Official JPEG compression requires opaque images; text/vectors remain native PDF content. Inspect failures through returned BoardsError values and the WorkCrate UI/MCP operation message. No upstream source or renderer dependency is embedded.

## 2026-10-22 preview publication and text formatting

`canvas_edit` and `canvas_refresh` publish the official rendered PNG into the
existing version-1 boards-cache.json preview map via the existing atomic writer.
Only the edited page is published; old cache assets use existing cleanup. No schema
migration or extra data store. Cached render_preview therefore sees edits before
canvas_close/refresh_project. UI frames reuse the same decoded bitmap in the rail.

`canvas_text_style(id: u64, font: Option<&str>, size: Option<f64>,
justify: Option<&str>) -> Result<CanvasSnapshot, BoardsError>` forwards
text.setStyle; positive finite size and supported paragraph alignment are validated.
canvas_set_bounds rejects empty selection/nonfinite values/nonpositive dimensions.
Probe validates the active native document path before returning a changed frame.
Opening a replacement canvas and ordinary native transactions commit/close an
existing canvas first, preventing stale headless sessions from overwriting edits.
The UI may retain an explicit session in preview mode; navigation still ends it.

## 2026-10-22 page switching without a reconnect

One canvas session belongs to the whole native document, not to one page.
`canvas_open_inner` keeps a held session whose `holds_document` check confirms the
same `.vectorcraft` file is still bound, clears the old page's official selection
(`select.none`), moves `board_id`/`native_id`/`artboard`/`preview`, and renders
that artboard through the same process. `CanvasSnapshot.session_id` names the
official process: it is unchanged across any number of page switches and changes
only when the service relaunches (close/reopen, `canvas_grant`, a lost document, or
a native catalogue transaction). Nothing is saved on a switch; the document is
committed when the editor leaves. The ignored official test
`page_switches_reuse_the_same_official_session` covers reuse, per-page preview
paths, cleared selection and the new id after a real close.

## 2026-10-22 compact material band and one-page field saves

`update_info(id, title, info) -> Result<InfoSave, BoardsError>` answers with
`InfoSave { board, frame: Option<CanvasSnapshot>, changed: bool }` and takes
exactly one of three paths. A draft equal to the stored page returns
`changed:false, frame:None`: nothing is written, so the official session gets no
step and no page is re-rendered. A session already held for this page (for
example the UI's explicit editing session, or one warmed by the previous save) is
written through that same session (`canvas_write_info`) and returns its rendered
`frame`, so the overlay paints the saved page without the change probe rendering
it a second time. Otherwise a routed session writes the page and only that
page's catalogue record and preview are replaced (`store_page`); the other pages
keep their existing preview files. `canvas_open_quiet(board_id)` shares
`canvas_open`'s setup but skips the warm-up render, and `canvas_holds(board_id)`
reports whether the open session already belongs to the page (`Ok(false)` when
no session is open — it previously errored; 2026-10-22 drop placement relies on
that); the field-save
caller uses them to warm a session silently and render exactly once, and closes
only a session it opened itself when the draft turns out unchanged. Before any
of this the id's association segment is compared with
`projects.document_binding(project)`, so a rotated binding is refused instead of
being written into a stale page.

The band itself is the compact pale horizontal strip of the October 21 studio
sketch: page number plus prominent project name on the left, material / size /
quantity separated on the same baseline at the right, extra fields flowing into
subordinate value+label rows below the primary row. The page number sits in a
white rounded chip; band and chip are official rounded rectangles sent behind
every header text node, so the owned artwork stays below the labels.
`header_layout` is crate-private pure geometry: the official program supplies
every text box through `document.node`, and the module only turns those
measurements into target boxes applied later as `object.transform` deltas. The
role contract between the driver and the layout is exact — `page_number`,
`page_number_chip`, `project_name`, `material`/`size`/`quantity`, `size_label`,
`extra:<key>`, `label:<role>`, `band`, and each separator under its own index
`separator:<n>`
(the bare `separator` role is never sited, and `each_separator_is_measured_under_its_own_index`
guards that). Pass one sites the primary row and its separators and asks whether
the group fits; when it does not, `Layout::group_in_primary` is false and the
material group is demoted. Pass two then sites both the value and the label of
every detail row at `SUB_SIZE`, because a metric for text that is not created
leaves the layout with nothing to place.

Two official behaviors shape the tests rather than being bugs: the official
program applies smart guides to pointer drags (an observed ≈2.4 pt snap onto
another node's edge, which is why the canvas drag test targets the middle of the
page), and `text.create` auto-wraps long text to the artboard width, so
`HeaderLayoutError::TooWide` is a defensive branch while ordinary over-long
content is refused by the request bounds. Every rejection the user can reach is a
Chinese actionable sentence; `describe` keeps the layout's internal reason
(role ids, metric names) out of the panel message.

Native resource diagnostics no longer echo the official report into the panel:
`describe_open_report` lists up to three missing file names (then a count), the
number of link rewrites and warnings, and tells the user to restore or relink
those resources in VectorCraft, save, and re-select the document through
“关联工程” so the headless session revalidates it. Links that the sandboxed
headless session flags only because they sit outside its read root are filtered
out when the file exists on disk, so the panel does not blame a healthy file. A
cached blocking report produces the same sentence, and
`ACTIVE_DOCUMENT_CHANGED`, the pending-dialog message and `DirtyDocument` are
Chinese as well. Link ids, `\\?\` paths and internal property names never reach
the interface.

## 2026-10-22 automatic preview acquisition

`BoardCache.source: Option<DocumentStamp { modified_ns, len }>` records the saved
native file a catalogue was built from (`serde(default)`, so an older version-1
cache simply has no source and is refreshed once). Every cache writer stamps it:
`refresh_project` (after `official_session`, which may commit a warm canvas
first), `refresh_cache_from_client`, `create_project_document`, `store_page` and
the canvas frame publication. Sampling the file before a write would leave the
cache looking stale, so each writer stamps after its own native write.

`preview_state(project) -> PreviewState { project_id, stamp, cache_stamp }` is a
local stat plus the cached stamp; it starts no process and opens no port.
`auto_refresh_project(project) -> AutoSync { synced, boards }` returns
`synced:false` when the cache already matches, otherwise reads the saved file
through a silent `mcp --headless` session only (never the live GUI transport and
never `official_session`), rebuilds catalogue and previews with
`rebuild_cache`, and publishes them through the atomic writer; `write_cache`
then removes preview files the new map no longer references. A missing document
is not stale: with no saved file there is nothing to read, so the cached view
stays. Only the saved file is read; unsaved state in a running GUI becomes
visible when the user saves it there.

## 2026-10-22 drag-and-drop placement copies unreachable sources

`canvas_place_dropped(board_id, source, centre)` is the drop path: it first
tries `canvas_place(source, centre, link=true)`; when the official single read
root makes the source unreachable (`Invalid` errors starting with "CLI
inputs" from `automation_folder` — another volume, or folders meeting only at
a drive root), the source is copied into `<document folder>/materials`
(content-stamp dedupe: same name+content reuses the file, a collision becomes
`name-<stamp>.ext`), the quiet session is reopened (the failed grant already
closed it), and the copy is placed linked and registered as the reference.
Grantable sources keep linking in place. MCP `canvas_place` is unchanged.

## 2026-10-22 grouped material bars, content groups and panel-only size label

The material bar is now grouped. Every page's band nodes (band path, page-number
chip, all header text) live inside that page's subgroup — a group tagged
`role:"header_page"` with the page's board id — and every subgroup lives inside
one document-wide master group tagged `role:"header_root"` with no board id.
`header::write` unlocks the master (`object.setProps locked:false`) at the start
of the undo group, orders the page's nodes with explicit `node.move` indexes
(band first, then chip, then text) instead of selection-based z-order calls,
creates the subgroup/master as single-child `object.group`s when missing, moves
the subgroup into the master idempotently, and re-locks the master as the last
write. Verified official behavior behind this design: `locked` persists through
save/reopen, and locked parents do not block official headless commands on
their children — so the panel keeps writing while the VectorCraft GUI cannot
select or move a band node. Legacy flat documents and the pre-compact
`role:"header"` group migrate to subgroups on the next field save; `extract_owned`
still reads tagged text fields through the whole nesting depth.

Page content is grouped too: `native::group_content` moves every freshly created
text (`canvas_create_text`) and placed image (`canvas_place`, `place_at`) into
that page's content group — a group tagged `role:"content_root"` with the board
id — creating it from the first node when absent. Official `file.place` returns
`ids`; created ids move with `node.move`, keeping ids stable for selection.

`PageInfo.size_label` (尺寸标记) is panel-only now: `header::write` no longer
paints the 规格 row, `catalog` no longer reads it from nodes, and stale
size-label nodes are deleted by the existing stale-field cleanup on the next
save. The value lives in `boards-cache.json`; both catalogue rebuilds
(`refresh_cache_from_client`, autosync `rebuild_cache`) merge it back from the
previous cache by board id via `merge_panel_fields`, so canvas saves and external
saves never lose it. Old documents keep their painted 规格 row until the next
save removes it.

## 2026-10-22 canvas picks and transforms stay on the child

The official hit test always reports the outermost group, so after the material
and content grouping a canvas click selected the whole page's content group and
any gesture moved everything at once. `canvas_gesture` now plans each batch:

- A still `down`/`up` pair is a click: forwarded, then `native::drill_click`
  re-points the selection at the child under the pointer. It walks the official
  `document.inspect` layer tree (the only official tree that carries `bounds`;
  `document.json` has none) top-down in reverse z-order, selects the deepest
  child whose bounds contain the point, recursing through nested groups. A
  click that landed inside a locked group (the band master and its subgroups)
  is deselected: the panel owns that content.
- A drag is decided by `native::run_drag`. When the press lands inside the shown
  selection, a shown child is moved by `object.move` on its id and re-selected
  (the official press would re-grab the group); a shown group or multi-selection
  is forwarded unchanged, since the user picked it as a whole. When the press
  starts elsewhere, a synthetic click picks the node under it first: a child is
  moved and re-selected, empty paper forwards the original marquee, and a press
  inside the locked band swallows the gesture.
- Handle drags never reach the gesture transport: the overlay commits its
  predicted box with one official `object.setBounds` (`reference:0`), which the
  workcrate-ui `canvas_up` sends for `CanvasDragKind::Handle`.

`gesture_points` only interprets plain `down`/`drag`/`up` batches; anything else
is forwarded untouched. Official facts behind this design: the press re-runs the
hit test even when the target is already selected, `document.inspect` reports
`selection`, `selectionBounds` and the full layer tree with `locked` flags at
`depth:16`, and headless marquee gestures select nothing (a live GUI keeps its
own official marquee). The ignored official test
`click_drill_picks_the_topmost_child_under_the_pointer` covers the tree walk.


## 2026-10-23 independent material category

`PageInfo.material_category: String` is the independent material category, default empty; no project-title fallback remains. `project_name_override: Option<String>` is retained for old cache compatibility only. On cache read an explicit old override moves into category; native catalogue reads old `project_name` text only when its ownership note is not inherited. Legacy inherited title text is not adopted. Header writes retain the stable native `project_name` role but always mark it non-inherited and use the incoming PageInfo category (not the pre-save board). The next successful save removes a stale inherited/cleared name node. No document rewrite occurs just from opening a project. Empty header text skips creation/measurement/group order and existing empty roles are handled by stale cleanup. This avoids grouping deleted empty text ids when creating or clearing a page category. Native artboards/other artwork and SQLite remain unchanged.


## 2026-10-23 single-line material bar (supersedes per-field layout above)

`header::write` now installs layout 2 once, then patches three persistent official point-text objects: `page_number`, `project_name` (independent category) and `details`. The right object is native `text.create { justify: "right" }` with no area/wrapping frame; its right anchor is fixed at the page inset. Only non-empty field values aggregate, without field names, joined by ` | `, including all future extra fields. Display folds whitespace to one line. Only the changed complete text is sent as one `text.setText`; no per-field widths, separator nodes, repeated text styles, transforms, grouping or background recreation occur on normal saves. The official whole-object bounds of a changed text are checked against fixed left/right regions; overflow cancels the undo group and returns a Chinese error. It never wraps, truncates or silently shrinks the text. The initial vertical alignment uses official metrics only once per text.

The native `details` text's existing namespace note (`version:1`, `layout:2`) contains `details { version:1, material, size, quantity, extra }`; category's note contains its original `value`. Native catalogue reads structured values, never splits the visible | text. Literal pipes/newlines/empty extras survive reopening. A single space keeps an empty text object's anchor/ID alive; metadata retains an actually empty value. Namespace notes with unknown/corrupt details versions are refused. Other PageInfo/cache/project JSON models are unchanged. Existing separate field/label/separator nodes migrate only on the first successful field save; native undo grouping rolls back failed migration. Background, chip and grouping are created once for a new layout, retained on all ordinary changes. Title/annotation-only updates cause no header text writes. Header root remains locked during normal child text patches; only initial install/migration unlocks and relocks it.

`WorkCrateBoardsService::canvas_header_state() -> Result<Value, BoardsError>` explicitly inspects only the already-open official session; it opens no file/process/port. Response includes board_id, layout, owned text ids/text/notes/official summaries+models, furniture summaries and `last_write { board_id, report { initialized, commands:[{ command,id,ids }] } }`. `initialized:true` means this write performed the one-time installation. Report is bounded in-memory diagnostics on the last successful write, not persistence or a polling/event stream. Normal writes contain undo-group begin/end, one changed text set and its structured note; it does not record payload text. Errors remain in existing Boards/UI error paths. MCP UI exposes this as read-only `header_state {}`. Official saved metadata is authoritative; manually unlocking and editing only the derived right-side text in VectorCraft does not reverse-map values into the individual fields.

New code is split into header/payload, editing/header_inspection, editing/state and native/tests to keep touched Rust modules below 800 lines. The retired field-width geometry implementation/tests are removed and replaced with fixed-anchor/aggregate roundtrip tests and same-process native-id/command assertions. Current evidence: docs/validation-ui/2026-10-23-workcrate-single-line-header/README.md.


### Material-bar black ink (2026-10-23)

All three native text roles use #000000. On action-time save, existing layout-2 text summaries are inspected; any non-black ids receive one text.setStyle { ids, fill: "#000000" } within the same undo group before content patching. Unchanged black text receives no style mutation. Existing node ids, geometry and furniture remain intact; offline browsing never rewrites documents. This is document ink, independent of UI Theme.


## 2026-10-23 native page order and generated titles

`page_title(index: usize) -> String` yields 页面01, 页面02… (minimum two digits).
`reorder_pages(project: &str, order: &[String]) -> Result<PageOrderSave, BoardsError>`
requires one complete permutation of current bound board ids, rejecting duplicates, missing or foreign ids
both against the local cache and the fresh native catalogue. It reuses a held canvas session for this project (otherwise the existing session router),
then executes official artboard.reorder { index, to } and repairs changed generated
artboard names and numeric material-header text in one undo group. Header::write_in_group shares the
transaction without nested groups. An edit error cancels the group; no cache publication occurs.
Successful save patches records and changed previews atomically; file/cache errors use existing BoardsError paths.
This changes list order only, preserving stable artboard ids, coordinates, artwork and layout-2 text ids.
No-op UI moves return early. Reordering an old layout performs its existing one-time header migration.

First and added pages use generated native names. artboard.new gets an explicit position 20pt right of
the rightmost existing official bounds, at the topmost board y, because its default follows list-last
and overlaps existing boards after a permutation. This does not move any existing board or artwork.
The read-only existing-session canvas_header_state diagnostics now include official artboards geometry.

Cache reads project old names to the automatic title without rewriting disk or launching a process;
new creation and explicit reorder persist native names. update_info retains its public title parameter
for callers but rejects a manually supplied name differing from the generated name. Removed page-info
metadata members remain serde-compatible, with existing size_label merge/recovery intact. No new schema,
sequence file, database, event system or plugin lifecycle is introduced.

Current evidence: docs/validation-ui/2026-10-23-workcrate-page-order/README.md.


### Order updates keep the native session (2026-10-23 follow-up)

PageOrderSave { boards: Vec<WorkCrateBoard>, changed: Vec<String>, frame: Option<CanvasSnapshot> }
separates the ordered records from changed images and the selected canvas frame. Same-project
canvas_reorder_pages holds the existing session, keeps session_id/native page/tool/selection,
updates only its preview index by native id, and saves once. No viewport/navigation command
is sent (view.goToArtboard is unavailable headlessly and must not move the live GUI viewport).
The selected changed page renders once, its returned frame is reused for the cache and UI;
other changed pages render in the same background operation; unmoved pages retain their PNG paths.
A shared OwnedLayout scan serves the batch, refreshed only after a legacy header installation.
Unmoved automatic-name pages skip header updates entirely. No per-page process launch, canvas
reopen, or whole-cache preview rebuild occurs in this path. header_state.previews exposes
existing cached paths solely for explicit diagnostics.

header_state.last_write remains the last update_info header write report; reorder diagnostics use PageOrderSave.changed instead. It is not a unified history ledger.


2026-10-23: Newly created body text explicitly sends text.create font=Microsoft YaHei, sharing header_layout::TEXT_FONT with material headers. canvas_create_text still accepts its positive size and the existing bounds/gesture services remain intact. CanvasObject.font_family is a read-only projection of the official text.getRange first run style, used to verify the actual font; it does not add a font editor or rewrite existing artwork.


### One durable frame per page (2026-10-23)

`PREVIEW_DIR` (`native-previews/`) is the only cached-frame folder, and `preview_frame(project,
native_id) -> Result<(String, PathBuf), BoardsError>` is its only writer-side naming rule:
`native-previews/page<native_id>-<association prefix>-1600.png`. The name is per page and per
native-document association, never per render, so a re-render replaces its own file in place
(`mcp::McpSession::render_preview` already writes through a sibling temp file plus atomic
rename). `make_previews` renders straight into that path; the page editor's `canvas_open`
targets the same path, and `publish_canvas_frame` / `store_order` / `store_page` only record it
in the cache map keyed `{board_id}@1600`. `durable_preview_relative(project_dir, reported)`
accepts exactly one `native-previews/*.png` component (the session reports `/` separators, so
that spelling is rebuilt as a native path) and refuses anything else instead of copying it
somewhere new. `write_cache` owns reclamation: entries dropped by the new map are deleted after
the write succeeds, newly written files are deleted when it fails, and
`remove_legacy_canvas_scratch` deletes the `canvas/` render mirror older releases wrote beside
each cached preview. Browsing, the page grid and `render_preview` still never start a session.
Re-associating a document rotates the binding, so `attach_document` clears every old frame and
the new association names new files. Inspect failures through BoardsError values,
`header_state.previews` and the cache map. Evidence:
docs/validation-ui/2026-10-23-workcrate-preview-dedup/README.md.
