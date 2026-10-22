# WorkCrate Boards ports

Cordis plugin `fileatlas.workcrate-boards` publishes `fileatlas/workcrate-boards` and depends on Projects. `VectorCraftConfig { executable, cli_executable, port }` remains in `vectorcraft.json`; both executable fields are optional strings and `port` defaults to 17979. A blank CLI path is resolved as `vectorcraft-cli.exe` beside the configured GUI executable. The legacy loopback transport is used only by `connect()` and `open_editor()`; only an explicit editor action starts the GUI.

`board_catalog`, `load_board`, `board`, `boards_for_project` and `render_preview` read FileAtlas-owned `projects/<uuid>/boards-cache.json` and `native-previews/` only. They never start or contact VectorCraft. A missing catalogue is empty; a missing cached preview returns `PreviewUnavailable`. Cache entries are bound to the persisted native-document association UUID, so `attach_document` clears old cache and any binding rotation makes old board IDs unavailable.

`create_project_document(project, directory)` creates the first A3 landscape document through the official `vectorcraft-cli.exe mcp --headless` JSON-RPC stdio server, writes the `.vectorcraft` file into the selected folder without overwriting, adds the native material header, and stores catalogue/preview cache inside FileAtlas. `refresh_project(project)` is the explicit disk-to-cache synchronization action; it runs on the same routed session, so when the GUI already holds that document it reads the GUI's live state instead of refusing. Creating a document always uses the silent headless session: a new native file must never hijack the user's open GUI window. Both operations use bounded child lifetime, hidden Windows process creation, isolated CLI preferences under the FileAtlas project root, and one explicit native-folder read/write root. Preview PNG bytes are returned inline by the official renderer and written into FileAtlas by the manager.

`create_page`, `update_info`, `place_reference`, `place_file`, `export_native` and `refresh_project` run through `official_session`, which chooses the transport and refreshes the durable snapshot after a successful write: the user's running GUI when its control port answers, otherwise the silent headless session. A running GUI is no longer a reason to refuse a background write, because the write goes to that GUI's own engine instead of racing a stale in-memory copy. Placement still requires the native document and the source file to share one collapsed read folder; cross-volume sources are refused. Export reads from the native folder and writes to the chosen output folder. `open_editor` alone connects to or starts the GUI window, navigates to the live artboard and focuses it. The legacy TCP JSON-line client (`protocol.rs`) now serves only `connect()` and `open_editor()`; every other native call uses `mcp::McpSession`, which speaks newline JSON-RPC, bounds replies to 32 MiB, times out at 65 seconds, never retries a mutation, and poisons the child only after an uncertain outcome (timeout, response ID mismatch, oversized or unparsable line). A command-level `isError:true` reply is a well-formed answer from a server that keeps serving, so the session survives it; `command_error_keeps_the_headless_session_usable` guards that behavior.

## Canvas editing session

`canvas_open(board_id)` starts the page editor's official editing session, or moves an existing session to another page of the same document, and `canvas_close()` ends it; `canvas_is_open()` reports whether one is held, `canvas_open_quiet(board_id)` performs the same setup without a warm-up render, and `canvas_holds(board_id)` reports whether the held session already belongs to that page (field saves use these two to render once). One `mcp::McpSession` serves both transports: `McpSession::live(cli, port)` runs `mcp --connect 127.0.0.1:<port>` against the user's GUI when `port_is_listening` answers, otherwise `McpSession::headless` runs `mcp --headless` silently with `VECTORCRAFT_CONFIG_DIR` pointed at that project's own `cli-config` folder, so a long-lived session never starts a second engine over the shared preferences. Editing semantics stay official: `pointer_gesture`, `select_tool`, `object.move`, `object.setBounds`, `file.place`, `links.update`, `links.relink`, `text.create`, `text.setText`, `undo`, `redo`, `doc_inspect`, `render_preview` and `save_file`. `CanvasSnapshot` carries only what the official program reports (`revision`, `dirty`, `tool`, `selection`, `selectionBounds`, artboard geometry, the rendered preview path and size, last history step) and `canvas_object` returns the official node's kind, name, bounds, text and link status (`kind` is lowercased, link status is reported as `ok`/`modified`). `object.setBounds` is always sent with `reference:0` so x/y mean the box's top-left; the official default is the centre. A session that reports no open document is an empty state, not a failure: `open_document` opens the project's document, and `inspect_bound` reopens the bound file when a live GUI has closed it. If a reachable GUI still cannot make the document editable, `canvas_session` drops that transport and opens the saved document in the silent headless session, so the background editor never depends on a usable GUI window. A document that still fails to become active after that is reported in Chinese instead of the official protocol text.

Each edit writes the native document through `save_file` as soon as the official program reports `dirty`, so the `.vectorcraft` file, the GUI and `refresh_project` always describe the same page; `canvas_commit` covers leaving the canvas after an official change no command reported. `canvas_probe(known)` is the observer-side change probe: the caller passes the revision it last displayed, because a mutation made by another client (an automation call, a second panel) advances the session without that observer ever seeing the returned snapshot. `type_text` exists only on the live transport; the headless session creates or replaces text with `text.create`/`text.setText`.

The official launcher accepts exactly one `--automation-read-root` and one `--automation-write-root`, so a session's inputs collapse to the single deepest folder holding them (`automation_folder`). A canvas session is granted the document plus only those project references that already live inside the document's own folder; when the user points at a file elsewhere, `canvas_grant` commits the document, closes the running session and relaunches it with that folder inside the collapsed root, because the engine fixes its automation roots at launch. A live GUI keeps the roots its owner launched it with, so a refused path there stays a visible error instead of silently restarting the user's program. A live session shares the GUI, so every mutation first re-checks that the active document is still the bound one, and `open_document` activates an already-open window (found through `inspect_ui`, switched with `document.activate`) instead of re-reading it from disk.

`attach_document(project, path)` validates the existing `.vectorcraft` with a read-only headless session (`open_file` plus `doc_inspect` path check) and never contacts the GUI; `native::revalidate_open_headless` shares the open-report policy with the GUI opener. VectorCraft 0.14.0 lists the headless `export` tool with `additionalProperties:false` and only accepts `artboard`, `format`, `options`, `outlineText`, `path`, `range`, `scale`, `selection`; PDF uses `range` (never `useArtboards`, which exists only in the GUI command catalogue) and rasters use a single `artboard` plus `scale`. `operations.rs::OFFICIAL_EXPORT_ARGS` pins that list and the non-ignored `export_params_only_use_official_tool_arguments` test guards it. A headless `open_file` `missingLinks` entry is treated as a sandbox artifact only when the linked file still exists on disk outside the native document's folder (the read root cannot cover it); warnings, modified links, and genuinely absent links are blocking and are recorded in the diagnostics map until a successful revalidation.

Page identity remains project UUID + persisted association UUID + native numeric artboard ID. Header content is real native text tagged with versioned `fileatlas.workcrate` notes; edits are undo grouped. Raster/PDF exports are official CLI exports and are atomically placed at the requested output path. Inspect failures through returned BoardsError values and the WorkCrate UI/MCP operation message. No upstream source or renderer dependency is embedded.

## 2026-10-22 preview publication and text formatting

`canvas_edit` and `canvas_refresh` publish the official rendered PNG into the
existing version-1 boards-cache.json preview map via the existing atomic writer.
Only the edited page is copied; old cache assets use existing cleanup. No schema
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
reports whether the open session already belongs to the page; the field-save
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
