# MCP plugin ports / MCP 对接

## WorkCrate — 2026-10-20

Three-level revision adds Standard-gated `workcrate-ui.navigate`,
`manage_project` (create/rename/cover/duplicate/archive/restore), `create_page`
(A3 landscape), and `update_page` (name, physical dimensions and content).
`state.ui` publishes the actually mounted screen, busy state and selections from
the shared UI surface. `delete_project` and `export_pages` require Full plus
per-call confirmation; deletion retains feature data in trash. Ordered PDF and
PNG/JPEG exports use original-image DesignCraft renders. See WorkCrate UI PORTS
for exact payloads and remaining limits.

`workcrate-ui.state` returns project names, reference counts and board metadata
from JSON manifests. It does not read linked image bytes. `workcrate-ui.open_board`
opens an existing board in the current FileAtlas UI; `workcrate-ui.place_image`
adds a path as a project reference, validates and places the actual JPEG/JPG/PNG
bytes, then opens that board. Both mutations require Standard MCP access and
change only WorkCrate records/board files; they do not write to source images.
`workcrate-ui.export_pdf` writes a PDF to the requested path and requires Full
access plus `confirm=true`. The UI plugin owns and unregisters these tools. The
schemas and error handling are maintained in
`fileatlas-plugin-workcrate-ui/PORTS.md`.

## FileManager group-frame pointer diagnostics (2026-10-20)

`get_ui_state.file_manager.group_frames` reports primary-pane painted frame segments:
row-index keys map to `{path,first,bounds:[x,y,width,height]}` in logical client
coordinates. Multi-row groups retain the same path on every segment. Geometry
is memory-only, republished on paint and may briefly be empty during layout.

Standard-gated `control_file_manager {action:"debug_internal_file_drag",enabled:true}`
is rejected unless an explicit `FILEATLAS_DEV_TEST_DB` isolated dev-test profile is
present. The live FileManager then keeps its typed GPUI drag/drop path while skipping
the native OLE drag-out handoff, which otherwise blocks synthetic pointer delivery.
Disable it with `enabled:false`; the default is false and it is never persisted.
`file_manager.debug_internal_file_drag` reports the primary pane's state. Pointer tests
use existing `control_ui` events and real filesystem outcomes, rather than the
direct `move_files_to_subfolder` control route. See FileManager PORTS and
`docs/validation-ui/2026-10-20-group-frame-drop/README.md`.

The `get_ui_state.file_manager.file_copy_buffer_count` field reports the active
primary view's in-memory Ctrl+C selection count. `control_ui {action:"key_press",
value:"ctrl-c"|"ctrl-v"}` dispatches chords into the same GPUI process; pointer
position determines the Ctrl+V folder target. See
`docs/validation-ui/2026-10-20-file-keyboard-parent-drop/README.md`.

## Live column toolbar diagnostics — 2026-10-19

`get_ui_state.theme` now reports `mode` (`dark`/`light`), the native theme name as `id`,
`framework: "GPUI Kit ThemeRegistry"`,
and current base/panel/card/border/text semantic colors so live mode checks do
not mistake the former fixed navy values for the selected palette.

Material tools are lifecycle-owned by `fileatlas.material-theme`, not hardcoded here.
`material-theme.state` returns generator domain errors as `last_error`; a failed
generation call uses the ordinary MCP failure result. See that plugin's PORTS.

`get_ui_state.program_workspace.current_snapshot` reads the existing public
`workspace.current_snapshot` surface as a typed `ProgramWorkspace`, or null
before publication. Its `columns[i].list_layout` and `show_subfolders` describe
each live pane's view controls. `active_workspace` remains the saved definition;
the legacy top-level `file_manager`/`browser` diagnostics describe the primary pane.
This read-only projection adds no polling, IO or state mutations.

## Access levels — 2026-10-19

The MCP plugin contributes the `fileatlas.mcp` settings page. `Settings.mcp_access_level`
has Off, Standard and Full stages. Standard permits UI control and ordinary
mutations; Full additionally permits tools whose plugin-owned MCP metadata marks
`confirmation_required`, and those calls still require `confirm=true`. Calls
read the current SettingsService snapshot, so stage changes apply immediately.
The previous `mcp_allow_mutations` boolean is accepted as a legacy JSON alias
(`false` → Off, `true` → Full). See
`docs/validation-ui/2026-10-19-mcp-access-stages/` for runtime evidence.
Older validation entries below preserve the environment-variable values used
by those historical builds; current authorization is always controlled by the
SettingsService access level described here.

## Clipboard workbench — 2026-10-18

- `get_clipboard_state` is read-only and reports count, category names, item IDs,
  length, category assignment, content `kind` (text/link/code), revision, favorite state (legacy pinned alias) and timestamps. `ui` reports current filter, visible IDs, selection and editor/group-name lengths through `UiSurfaceState`. It deliberately omits
  clipboard text.
- `add_clipboard_text {content, category_id?, favorite?}` is mutation guarded and
  adds a text row directly through `ClipboardService`; it never reads or writes
  the OS clipboard. Use only an isolated profile for UI fixture data.
- OS clipboard capture and system-wide keypress are not exposed as MCP actions;
  those Windows paths still require dedicated runtime evidence.
- `scripts/fileatlas-mcp-debug.cjs` always enables `dev-test` and supplies an explicit profile-local `FILEATLAS_DEV_TEST_DB`, even when the requested plugin list is `core`. Windows Known Folder resolution can ignore a `LOCALAPPDATA` override; the old client therefore did not establish isolation for `core` alone. Reports now include effective plugins, child PID and isolation status. Explicit profile paths are visible in the running UI.
- `control_ui input_text` dispatches printable Unicode characters with their actual keystroke/character values, avoiding the earlier synthetic `text` key that did not reliably reach focused inputs. It remains same-process GPUI input, with the existing mutation guard and 1000-character cap. `scripts/validate-clipboard-workbench.cjs` exercises Chinese group typing, immediate assignment, filters, search, rename/delete and capture pause; `scripts/validate-clipboard-restart.cjs` checks independent child-process restarts without OS clipboard access.

## Shortcut Settings validation — 2026-10-18

`command_list` includes each registered shortcut's `id`, default `key`,
`effective_key`, enabled state, and `scope` (`application` or `system`). The settings panel discovers these rows
from the same catalog and applies persisted overrides by contribution ID.
`control_ui {action:"key_press", value:"ctrl-alt-q"}` supports GPUI's
hyphen-separated modifier chord syntax for same-process capture tests. The MCP
key press is a synthetic in-window event and does not simulate an OS-wide key
press while another application is foregrounded.

## Program launcher diagnostics — 2026-10-17

When `fileatlas.launcher` is enabled, `get_launcher_state` reads its in-memory
snapshot from `LauncherService` in the same FileAtlas process. It returns scan
status, categories and app entries (display name, launch path, category,
hidden/available flags, discovery source, pin scope, install date/size, and
whether an uninstall command is registered). Mutation-guarded tools cover
adding and refreshing discovery; category create/rename/delete; program
display-name/category/hidden edits; pinning; launch; list-only removal; and
uninstall. Launch, removal, category deletion, and uninstall require
`confirm=true`; uninstall runs the registered command without another native
prompt after explicit MCP confirmation. List-only removal never deletes files.
Mutations require Standard or Full in Settings → MCP → AI control. High-impact
launcher actions additionally require Full and per-call `confirm=true`. An absent launcher service returns an explicit
error. Use `control_ui` pointer actions and `screenshot_window` for live panel
interaction.

`scripts/validate-program-launcher.cjs` exercises category CRUD, partial
program edits, discovery refresh, confirmation gates, launcher-only removal
and re-addition, plus live panel/icon/sort/pin flows. It does not invoke a real
uninstaller; `confirm=false` is verified to leave state unchanged. Uninstall
execution against installed software remains a manual system integration
check.

## Everything search diagnostics — 2026-10-17

`get_ui_state.file_manager.search_provider` reports `everything` only after a successful
SDK IPC query; `sqlite-fallback` means FileManager used its original database search
because the optional adapter/query was unavailable. `search_media` routes
through FileManagerService and exercises the same provider as the FileManager query.
`search_media.search_provider` and `get_ui_state.file_manager.search_provider` retain
the provider of that specific result/snapshot. `file_manager.everything` contains
`state`, actual EXE `version`, `path`, and `index_loading`. Installation inspection
is read-only; only a real filename/path search can prepare the background client.
`control_file_manager {action:"everything_recheck"}` refreshes the installation
ReadModel and retries the current text search. The state transition scenario is
`scripts/validate-everything-status.cjs`; it reads actual 1.4/1.5 EXE resources in
an isolated portable-path override without installing or executing those fixtures.
The isolated integration scenario is `scripts/validate-everything-search.cjs`.
`control_file_manager {action:"search_scopes_open",enabled}` opens/closes the live
top-bar scope panel; `search_scope` accepts `filenames|paths|tags|time` and
uses FileManager's persisted scope toggle. `get_ui_state.file_manager.search_scopes`
and `search_scopes_open` expose the rendered choices for same-process checks.

## Unsaved version status — 2026-10-17

`get_file_version_state {media_id}` is read-only and returns the path plus
`state.status`, `currentVersionId/currentTitle`, `matchingVersionId/matchingTitle`.
It compares immutable content and never auto-creates or switches versions.
`get_ui_state.file_manager.version_states` exposes visible-window read results,
per-file errors and `badge_visible`; `version_states_loading` reports bounded
ReadModel work. Only confirmed `modified` shows the amber card dot.

## History visibility diagnostics — 2026-10-17

`get_ui_state.inspector_history.visible` is true only when the Inspector is open,
the History tab is selected and FileManager's current Inspector context is a file.
The other history surface fields retain their last rendered values when hidden;
use `visible` and `file_manager.selected_media_ids` to verify the live page after a
version switch rather than treating stale count/current values as visibility.

## QuickLook replacement — 2026-10-16

This section supersedes old Preview/Playback tool contracts below.
MCP now depends on `QuickLookService`; PreviewService and MediaPlaybackService
and their crates are removed. `control_preview` and `control_playback` and
their request payloads are removed from tools/list. `open_media {media_id}` is
an async, mutation-guarded call to the installed QuickLook for any ordinary
file type. `get_view_state` returns `{quicklook: LaunchSnapshot}`; dispatch
success is not external rendering/visibility evidence. `get_ui_state` includes
the same diagnostics and `quicklook_notice` (the last FileManager failure message).
`control_file_manager {action:"preview",media_id}` exercises the real FileManager alert
flow; `control_ui {action:"key_press",value:"space"}` dispatches a real
same-process GPUI key. Settings Toolbar is no longer an accepted tab, and the
fixed panel diagnostics list only Settings. Other preview terminology, such
as bulk-match previews and inspector fields, retains its own meaning.

## Context-inspector validation additions — 2026-10-16

- `get_ui_state.file_manager.inspector_context` reads the live rendered context key
  `program|tag|folder|file` without another database query.
- `control_file_manager {action:"inspector_tag_name",value:string}` queues
  `FileManagerUiRequest::InspectorTagName(String)` (1–1024 bytes). It changes the
  live input only. Click Save through same-process pointer events to commit.
- `control_navigation {action:"tag_appearance",tag_id,color?,icon?}` is mutating
  and uses `NavigationService::set_tag_appearance`. It checks the ID and hex
  color, sets both fields, and clears omitted values. It seeds isolated fixtures.
- `get_ui_state.inspector_demo` exposes enabled/selected/active/count/filter
  from the explicit visual demo through existing UiSurfaceState. Normal launches
  return enabled=false. Actual GPUI pointer events exercise node selection,
  menus, branch filtering and in-memory Create/Switch; these fields do not imply
  a persistent file-version backend.

## Lore-backed historical versions — 2026-10-17

`list_file_versions {media_id}` reads the Lore revision history for the indexed
file and returns titled revisions, timestamps, sizes, thumbnails, parents, and
the active revision. `create_file_version {media_id,title,confirm:true}` saves
the current file bytes as a titled revision. `activate_file_version
{media_id,version_id,confirm:true}` restores a revision and refreshes that
media row's derived file metadata and thumbnail. Create/activate require
Full access in Settings → MCP → AI control and explicit `confirm:true`. The same-process
FileManager History tab invokes the same Versions service. Plugin lifecycle
dependencies are `fileatlas/versions` → `fileatlas/version-info` and
`fileatlas/thumbnails`; durable version and branch metadata uses typed
`DatabaseService` tables in the FileAtlas browser database. Existing sibling
`version-info.sqlite3` data is imported idempotently and retained as a rollback
copy. MCP service operations have passed
same-process validation. The History tab, revision selection and version switch
were clicked through the MCP child's GPUI window; pending conflicts were read
and aborted after restarting that child with the same profile. See
`docs/validation-ui/2026-10-17-lore-version-history/README.md`.

`list_file_branches {media_id,include_archived?}` reads branch heads, ancestry,
per-branch revisions and pending merge conflicts. `manage_file_branch` accepts
`action`=`create|switch|archive|merge|resolve|abort` plus the relevant `name`,
`source_branch`, `strategy` and `overwrite` fields. All writes require the
mutation guard and `confirm:true`. Byte-changing switch/merge operations refresh
the FileManager row and emit `MediaIndexChanged`; resolve accepts `ours` or `theirs`.

The mutation guard and same-process child/window boundary remain in force.
Version/branch round trips, FileManager History pointer interaction and pending
merge recovery across restart are exercised by the
`scripts/validate-lore-*` scenarios through `fileatlas-mcp-debug.cjs` with isolated
profiles. Evidence:
`docs/validation-ui/2026-10-17-lore-version-history/README.md` and
`docs/validation-ui/2026-10-16-inspector-complete/README.md`.

Current browser runtime supersedes the older indexed-library descriptions below; see the 2026-10-15 update at the end and `docs/validation-ui/2026-10-15-browser/README.md`.

Status (updated 2026-10-19): normal FileAtlas startup hosts Streamable HTTP MCP in the GUI process at `http://127.0.0.1:38473/mcp` (`FILEATLAS_MCP_HTTP_PORT` overrides the port). The legacy `--mcp` stdio mode remains for isolated debugging. Bind is loopback-only; rmcp validates Host against its loopback allowlist. Modification access is controlled live by Settings → Plugin settings → MCP → AI control and persists as `Settings.mcp_access_level` in the existing Settings JSON. Off allows reads only; Standard allows UI control and ordinary mutations; Full additionally allows tools whose plugin metadata marks them high-impact. Missing values default to Off. Every host and contributed mutating call reads the current setting at call time; there is no restart or environment variable override. High-impact operations still require their per-tool `confirm:true` checks. The legacy JSON key `mcp_allow_mutations` maps false to Off and true to Full.

MCP feature tools are contributed through `fileatlas_contracts::mcp::McpToolRegistry`. The host publishes the registry before feature startup; each Cordis feature plugin contributes its own namespaced tool definitions, JSON input schema, handler and mutation/confirmation metadata, then retains the returned registration guard in a plugin-specific Cordis lifecycle service. These wrapper services have distinct Cordis identities, so multiple plugins can publish guards in one realm. MCP resolves contributions at `tools/list`, `tools/call`, and `get_tool`, so stopping a plugin removes its tools from the live server. The MCP crate does not own contributed feature parameter definitions. Launcher, FileManager, Navigation, Tagging, Clipboard, File Operations, Metadata Export, QuickLook, Versions, Debug, and opt-in DevTest contribute their tools; each plugin's `PORTS.md` documents its names, schemas, and behavior. The old unnamespaced aliases are no longer advertised by `tools/list`; window capture/resize, cross-plugin UI inspection/control, and the global command catalog remain host tools because they depend on the running window or shared UI Bridge registries.

Runtime functional validation on 2026-10-19 used `start-dev.bat all` and its
same-process HTTP endpoint at `127.0.0.1:38473/mcp`. The profile stored its
database/settings/session/thumbnails under `target/dev-test`. The run created
`test-data/mcp-live-2026-10-19` with a copied JPEG and text fixture. `tools/list`
returned 62 tools including 52 namespaced plugin contributions; no retired
unnamespaced aliases were listed. `file_manager.get_media`, `file_manager.search_media`,
`navigation.list_library`, `tagging.list_tags`, `clipboard.snapshot`,
`launcher.snapshot`, `quicklook.snapshot`, `quicklook.inspect`, `debug.query`,
`get_ui_state`, `command_list`, and `screenshot_window` completed; the screenshot
returned a real MCP image block.

With `FILEATLAS_MCP_ALLOW_MUTATIONS=1` only in the isolated test process, the run
registered and rescanned the fixture directory, indexing its JPEG/text/README;
set and restored the image rating; created, assigned, read, removed and deleted
a temporary tag; copied the test JPEG, renamed and moved that copy, then deleted
only that test copy to the Recycle Bin; exported 10 metadata records to the test
directory; and created/listed/activated a Lore version of the text fixture. Its
SHA-256 remained unchanged after activation. Launcher category create/rename/
delete succeeded. DevTest scanned all 12 checked-in fixture files, found the
nested JPEG through FileManager search, injected 1,000 virtual rows and cleared them.
During injection the DevTest status reported 1,000 rows, while
`file_manager.search_media` still returned 12 results even when queried with the
fixture folder ID; this is recorded as an observed FileManager/benchmark visibility
gap, not a successful FileManager scale display. The temporary tag/category and
test-folder registration were removed. The fixture scan remains in the isolated
DevTest profile; generated `metadata-export.json` and physical test fixtures
remain in the test directory. The app was restarted without the mutation
environment variable, and a final `file_manager.set_rating` attempt was correctly
rejected with `FILEATLAS_MCP_ALLOW_MUTATIONS=0`.

Current API names and full parameter schemas live beside their providers:
`clipboard.*`, `file-operations.*`, `file_manager.*`, `launcher.*`,
`metadata-export.*`, `navigation.*`, `quicklook.*`, `tagging.*`,
`versions.*`, `debug.*`, and (when enabled) `dev-test.*`. The remaining host
tools are window screenshots/resizing, cross-plugin `control_ui`,
`control_navigation`/`control_file_manager` UI-driver operations, and the global
`command_list`/`command_run` bridge.

## Cordis lifecycle and dependencies

- Cordis plugin: `fileatlas.mcp`; lifecycle guard service:
  `fileatlas/mcp-task-guard`; host window-handle service:
  `fileatlas/mcp-window-handle` (`CaptureWindowHandle`).
- Requires `FileManagerService`, `NavigationService`, `TaggingService`, `QuickLookService`, `MetadataExportService`, `CaptureWindowHandle`, the UI Bridge catalogs/surfaces, HistoryService, and the shared `McpToolRegistry`. Feature data operations are dispatched through each feature's registry contribution; the host does not acquire Clipboard, Launcher, File Operations, Versions, Debug, or DevTest services for MCP handlers.
- The Shell publishes `CaptureWindowHandle` before providers start, fills it with the actual FileAtlas HWND when GPUI creates the window, and stops the MCP plugin before stopping its providers.
- MCP uses the official Rust `rmcp` SDK. Normal startup serves Streamable HTTP from the GUI process at `http://127.0.0.1:38473/mcp`; `FILEATLAS_MCP_HTTP_PORT` changes the port. `--mcp` retains stdio for diagnostics.
- Cordis disposal aborts the MCP task. If Cordis cannot publish the lifecycle
  guard during startup, the transport task is aborted before the plugin returns
  an error. No localhost/network listener is started.

## Tool contracts

The exact current feature API is listed in the owning plugin's `PORTS.md`.
The `tools_read.rs`/`tools_write.rs` names in historical validation notes below
refer to retired unnamespaced entry points; the host rejects those calls and
clients should use the namespaced plugin tools shown by `tools/list`.

- `command_list` reads the live UI Bridge command registry and reports command
  IDs, owner, label, menu/keybinding placement, `CommandNeeds`, and current
  availability. `command_run { command_id, confirm? }` dispatches the same
  `UiCommandInvoked` Cordis event used by UI command contributions. Undo/redo
  require an available history entry and MCP mutation opt-in; confirmed
  mutations additionally require `confirm: true`. The response means the event
  was queued, not that an asynchronous action completed. The current metadata
  export UI command opens its panel; the separate `export_metadata` tool runs
  an export with its destination and confirmation contract.

- `screenshot_window`: captures only the FileAtlas top-level HWND on Windows with `PrintWindow(PW_RENDERFULLCONTENT)`, encodes the result as PNG, and returns MCP image content plus pixel dimensions. A missing, minimized, invalid, or unsupported window returns an error. Visual output has been inspected for FileManager and Preview on the isolated validation profile; other screens and DPI/window-size configurations remain to be checked.
- `resize_window`: changes the current FileAtlas HWND outer size for isolated layout checks. Width is limited to 640–4096 and height to 480–2160; the result includes requested and actual window bounds because Windows DPI and monitor work-area limits can clamp the request. It does not move, activate, or reorder the window; capture afterward with `screenshot_window` to check the rendered layout.
- `list_library`: reads registered folder IDs, paths, last-scan times/counts, and the hierarchical tag tree.
- `search_media`: supports a phrase across filename/path/tag, folder ID, tag IDs, media type, offset, a maximum page size of 100, and the source sort orders.
- `get_media`: returns a single indexed media row and its current tags.
- `get_view_state`, `open_media`, `control_preview`, and `control_playback`: inspect/change preview and playback state using the existing preview/playback services. Preview controls include `close`; live image rendering was exercised with the generated PNG. Video/audio codecs and device output still need Windows runtime validation.
- `cancel_scan`: requests cancellation for a folder's active scan.
- `dev_test_status`, `dev_test_scan_fixture`, `dev_test_inject_virtual`, and
  `dev_test_clear_virtual`: control the optional `DevTestService` in the same
  FileAtlas process. These tools are available when `FILEATLAS_DEV_PLUGINS` contains
  `dev-test`; virtual injection accepts only 1,000, 10,000, or 100,000 rows.
  Start operations return immediately; poll `dev_test_status` until `running`
  is false, then use `search_media` / `list_library` and `screenshot_window` to
  inspect the same running application. The isolated `target/dev-test` data
  directory is selected by the Shell profile, not by MCP tool arguments.
- `set_rating`, `assign_tags`, `remove_tags`, `create_tag`, `rename_tag`, `delete_tag`, `add_folder`, `remove_folder`, `rescan_folder`, `rename_media`, `copy_media`, `move_media`, `delete_media`, and `export_metadata` are mutating tools. Ordinary mutations require Standard or Full; plugin metadata marks the high-impact file/export, folder, tag-delete, and rescan operations, which require Full plus `confirm: true`. Batch limits are 100 media IDs and 50 tag IDs. Tag assignment/removal update SQLite only and do not write IPTC or create XMP sidecars.
- `delete_tag` removes the requested tag and its direct associations; it passes `clean_parents: false` because the MCP request only confirms deleting that tag and does not separately request ancestor-assignment cleanup. The Navigation service's affected media IDs are intentionally omitted from the existing `{ "ok": true }` response.

MCP access defaults to Off. Standard permits UI control and ordinary mutations; Full additionally permits plugin-declared high-impact mutations. `confirm: true` is an explicit call-site acknowledgement, not proof of a human interaction; configure the MCP host to ask the user before invoking mutating tools. Copy/move destinations and media paths use the existing FileOperations service. Do not run this MCP server for an untrusted client.

## Current limits and debugging

- `get_view_state.preview.image_load` exposes Preview's actual first/full paint timing, quality, dimensions, errors, render count and polling wake reasons. `get_debug_info.view_revision` exposes the drawer's independent UI/content clocks. These read-only diagnostics do not generate logs or invalidate views.
- `control_ui { action: "accessibility_activate", confirm: true }` requires Full access and per-call confirmation, and asks only the Shell-published HWND belonging to this process for its native Windows UI Automation provider (`WM_GETOBJECT`, UIA root ID). This activates real GPUI accessibility updates for feedback-loop regression testing; no external window attachment or OS pointer/keyboard injection is used. Timeout is bounded to 2 seconds; non-Windows returns an unsupported error.

- Normal HTTP MCP calls target the already-running GUI process. The `--mcp` stdio endpoint remains a legacy isolated-debug path.
- MCP tool availability does not complete native UI behavior listed in `docs/source-feature-audit.md`; tools call the same Cordis services and their existing limits apply.
- Check Settings → MCP → AI control access level, whether `--mcp` was passed, Shell plugin dependency order, and stderr when tools are missing or startup fails.
- If a screenshot fails, check HWND publication and window state first. If it returns a blank or stale image, inspect PrintWindow/GPUI Windows rendering behavior; do not treat protocol compilation as screenshot parity.
- Batch scan receivers are drained by background workers so the indexing worker can finish; scan progress is exposed through `get_ui_state.scan_progress` when a scan is active.

## UI validation tools (2026-10-13)

`get_ui_state` reads settings_open/settings_tab, language, translation_revision
and preview_chrome_alpha. These supplement get_view_state; they do not describe
OS pointer position or all GPUI widget state. `control_ui { action, value? }`
requires Standard or Full access and supports settings_open/settings_close,
settings_tab, language (en/zh-CN), statistics, bulk_actions, file_manager_dialogs_close
select_media (value is an indexed media ID string), and deselect. Settings uses public
UiSurfaceState; language uses I18nService; FileManager uses FileManagerUiRequest.
Settings and FileManager consume requests asynchronously. Wait for rendering before
screenshot_window. No OS mouse/keyboard injection is used.

Shortcut recording diagnostics (2026-10-18): `get_ui_state.settings_shortcuts`
contains `capturing_key: bool`, `binding_id: String`, and `error: String`.
`binding_id`/`error` are empty when idle; invalid/conflicting chords keep the
binding in capture mode and expose the visible inline error without persisting
it. Values come from Settings' capture transitions; reading does not mutate the
UI. The `capturing_key` flag also covers the existing open-media key capture.
Use real `control_ui` pointer clicks/keypresses in
`scripts/validate-shortcuts-layout.cjs` to enter these states.

`control_ui {action:"pointer_scroll", value:"x,y,delta_y"}` queues one GPUI
`ScrollWheelEvent` through the existing bounded same-process input queue.
Position is logical client pixels in 0–4096; delta is finite logical pixels in
-4096–4096 (negative scrolls down). Requires mutations enabled, does not send
OS input, and returns the same pending-queue error as other pointer actions.

`control_ui {action:"canvas_drop_files", request:{"paths":["…"]}}` hands file
paths to the WorkCrate page editor exactly like dropping files on its canvas;
the view filters images, opens the official session when needed, and places
each file linked. GPUI seals the OS `ExternalPaths` drag value, so a real
`FileDropEvent` cannot be synthesized in-process — this is the semantic
equivalent through the `workcrate.canvas.drop-request` surfaces key, consumed
by the panel's preview watcher. Requires mutations enabled.

Run `node scripts/validate-ui-mcp.cjs` after a locked Shell build to scan the
isolated fixture profile, capture 23 UI states, and sample the real Preview
chrome fade curve. Output lives in target/mcp-ui-screenshots; the script never
executes delete/copy/move/update operations. Hover/tooltip activation, native
dialogs and pointer-driven animations remain untested by this interface.

## S2 and startup-update diagnostics (2026-10-13)

`get_ui_state` additionally returns boolean `inspector_open`,
`update_checking`, `update_available`, and `update_offer_open` from shared
UiSurfaceState. `control_ui` additionally accepts `inspector_open`,
`inspector_close`, and `settings_check_updates` (no value required). The
existing mutation guard still applies. Settings consumes manual-check
requests on its 100 ms GPUI poller; update completion is asynchronous.
Inspector controls operate the same surface state as the Shell toggle and
FileManager close button. These service-level requests and screenshots verify
state/rendering; they do not prove pointer click propagation inside filter
popovers, native dialogs, or every designed state.

## v5 live FileManager/Navigation controls (2026-10-14 handoff)

`get_ui_state` adds `file_manager_popover`, `tools_open` and the live FileManager
snapshot (filters, selected IDs, matched count, month grouping and inspector
sections). `control_file_manager` controls the same FileManager actions and controlled
Popover state as the UI. Actions: popover, search, media_type, sort, rating,
extension, tag, tag_mode, extension_mode, date_from, date_to, clear_filters,
inspector_section, tag_picker, month_grouping, tools_open and tools_close.
Read tools/list for exact optional fields and accepted string values.
`control_file_manager` additionally accepts bulk_open, bulk_close, and
bulk_configure; configuration uses `value` for delete/copy/move and
`bulk_source` for all/folder/tag/selection. `get_ui_state` reports the live
bulk action/source. `open_media_menu` with `media_id` and `close_media_menu`
control a live FileManager-owned media action surface; its state and indexed ID are
reported by `get_ui_state`. The `preview` action with `media_id` routes through
FileManager's selected-card preview handler and is available for same-process
validation of its filmstrip handoff. The native right-click ContextMenu remains
pointer-triggered. `control_navigation` accepts create/edit/delete/merge/move/close with
optional required-on-target tag_id; these open dialogs, never commit
deletion/merge. Both tools require the existing mutation flag and asynchronous
view polling.
`get_ui_state.scan_progress.error` can report a terminal failure while the
Indexing progress observer still has that scan active. Navigation consumes the
same terminal event and retains its Retry card independently; the MCP progress
snapshot may disappear after the completion sentinel.

MCP `open_media` routes image IDs to Preview and video/audio IDs to Playback
using the indexed `media_id`; `get_view_state` includes the playback ID.
`export_metadata` sets the destination and runs the shared
MetadataExportService state machine; `get_ui_state.metadata_export` reports
idle/exporting/succeeded/failed status read by the visible panel. Tool text
JSON may carry an error without CallToolResult.isError; clients must check both. The new
scripts/fileatlas-mcp-debug.cjs does so, uses an isolated target profile, captures
via screenshot_window and keeps one child window alive in interactive mode.
It cannot attach to another instance. Current continuation changes are
unverified; the older successful exe predates final layout edits.
## v5 redesign validation interfaces (2026-10-14)

`control_ui` additionally accepts `sidebar_open/sidebar_close`, `export_open/export_close`,
`writeback_retry/writeback_dismiss` (value is an exact recorded failure path). Sidebar controls
use the same UI Bridge state and Shell toggle; writeback actions use the production retry
payload or failure dismissal, never a synthesized successful result. `get_ui_state` exposes
sidebar/export visibility and the recorded writeback failures with path/error.

`control_preview` additionally accepts `next/previous`, `tags_open/tags_close`,
`more_open/more_close`, and `filmstrip_show/filmstrip_hide`. Requests affect the mounted Preview
view's controlled GPUI Kit popovers and existing SettingsService filmstrip flag. These do not
inject OS input. `control_playback` adds `set_volume` with finite `volume` in [0,1.5],
`tags_open/tags_close` and `more_open/more_close`; `get_view_state.playback` returns actual volume
and the controlled surfaces. UI requests are asynchronous; read state after the GPUI poller.
The existing isolated-profile mutation policy remains applicable.

## 2026-10-14 current implementation and MCP evidence

control_file_manager actions select_many (1–1000 unique existing media_ids) and scroll (zero-based current-result index in value) are mutation-guarded UI requests, including unloaded records. get_ui_state file_manager includes window_offset/loaded_count and folder_id/path_prefix. 100012-row runtime observed window_offset99812/loaded_count200 and bulk2 across pages. Detailed profile/plans/reports and current limits: docs/redesign-validation.md latest section.

## v2 panel and GPUI input validation (2026-10-14)

control_ui adds navigation_tab(folders/tags/extensions), inspector_tab(information/history/extensions; info alias), sidebar_width/inspector_width(value:logical pixels100–1000 then hostclamp), pointer_down/pointer_drag/pointer_up(value:x,y logicalclient coordinates0–4096). These UI-control actions require Standard or Full access. CaptureWindowHandle now holds the nativehandle and bounded single pending McpPointerInput{kind,x,y}; queue_pointer rejects busy/poisoned and shell take_pointer drains/deferred GPUI event dispatch. No OS pointer/keyboard injection and no external-process attach. get_ui_state exposes active pages and applied widths. control_navigation adds expand_folder(folder_id), expand_path(value:directory), select_path(value:directory), folder_search(value:query); requests validate registeredroot containment and perform canonical path selection/lazy loading. Live event drags/clicks and read-only rejection verified; see docs/redesign-validation.md.

## Floating panels and list validation (2026-10-14)

`control_file_manager layout` accepts value grid/list through FileManagerUiRequest::Layout. `get_ui_state.file_manager.layout` is the live rendered choice. `get_ui_state` adds panels(builtin visibility/frame/client bounds), panel_instances(all currently open frame/client bounds), open_panel_ids and active_panel. Bounds are comma-separated logical pixel strings; client_bounds are GPUI-measured, independent of screenshot DPI/nonclient offsets.

Mutation-guarded `control_ui panel_open` accepts a registered definition ID in value and returns instance_id; panel_close accepts that returned instance ID. Both call UI Bridge PanelCatalog; they cannot manufacture unregistered content. dev-test supplies actual task-status definitions for isolated single/multiple checks. pointer_move/pointer_right_down/pointer_right_up additionally queue same-process GPUI events using x,y; existing pointer_down/drag/up remain. No OS injection or separate process attachment. Wait for the host/feature poller before reading screenshots/snapshots.

## Native folder picker validation (2026-10-14)

get_ui_state.folder_picker returns pending and native_open(the Navigation provider actually found its current native window). Mutation-guarded control_navigation folder_picker_cancel calls NavigationService::cancel_folder_picker; it only cancels an open picker. Read-only screenshot_folder_picker captures the identified same-process native picker through existing PrintWindow encoding, with no external attachment. The debug client accepts capture_tool for a planned capture; default remains screenshot_window. Actual Sidebar + pointer clicks, native screenshots, two cancel/reopen cycles and unchanged library were recorded. These controls do not simulate folder acceptance or claim its coverage.

## All-file query contract (2026-10-14)

`search_media.media_type` now accepts `file` in addition to `image`, `video`, and `audio`; omission queries all types. `file` identifies indexed unsupported/extensionless files, not directories. Same-process screenshots, FileManager controls, native GPUI pointer dispatch and Navigation add/rescan tools validate these cards with an isolated profile.

## Startup refresh diagnostics (2026-10-14)

`get_ui_state.library_refresh` returns `running`, `total_folders`, `completed_folders`, and `[path, error]` failures from the Watcher-owned startup scan. The read-only field lets the same-process MCP client wait for reconciliation and inspect failures without invoking a rescan mutation. `list_library` and `search_media` then show current persisted row totals.

## Grouping controls (2026-10-14)

`control_file_manager` accepts action `grouping`, value `none|month|file_type|modified|created|folder|subfolder`; `toggle_group` uses an exact get_ui_state group label; `popover` accepts `grouping`. The existing `month_grouping` boolean remains an alias for Month/None. These actions require Standard or Full access and queue the real FileManager actions. get_ui_state.file_manager exposes grouping_mode, ordered groups as [label,count], collapsed_groups, window_offset and loaded_count. search_media remains a database query in query sort order; it does not pretend to return grouped UI order. Runtime commands/restart/screenshots are recorded under docs/validation-ui/2026-10-14-grouping.

## Directory-browser controls (2026-10-14)

control_file_manager adds open_directory(value=an entry in the current directory-card list), directory_up and refresh. It rejects invisible paths and no-parent views; these queue FileManager's real actions and require Standard or Full access. get_ui_state.file_manager publishes browsing, directories[{name,path}], current_directory, parent_directory, directory_error and selected_directory; matched_count remains the displayed indexed-file count. Directory cards have no synthetic media IDs. control_ui adds pointer_double_down/pointer_double_up (same x,y logical client coordinates), dispatched by Shell as same-process GPUI MouseDown/MouseUp with click_count2. Single/drag/right-click behavior is unchanged. See docs/validation-ui/2026-10-14-folder-browser for84-step evidence including mouse hit tests and read-only restarts.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics tools: get_debug_info(category?,level?,search?,page?,page_size?) returns actual drawer ui and logs Page from optional fileatlas/debug service. Read-only and deliberately does not log its own query. control_debug(action,value?) shares DebugService::control with native controls and requires Standard or Full access. Unknown service/action/invalid values return normal error JSON. MCP rejected requests are logged as ERROR; stdout remains reserved for protocol traffic.

## Idle regression matrix (2026-10-15)

`get_ui_state` adds `settings_tag_choices` and `file_manager.dialog_reads` from the
actual view caches, not new DB queries. Dialog reads report statistics
state/error/photos/videos/audio/rating_counts and bulk state/error/count/preview_ids.
Mutation-guarded `control_file_manager action=bulk_field` requires `field_index`
0..7 and text in `value`: older/newer days, min/max rating, name, extensions,
min/max MB. It updates the real InputState and resets confirmation; it cannot
execute an operation. The reusable same-process runner supports `--scenario
scripts/validate-idle-mcp.cjs`: isolated fixtures, native accessibility enabled,
Debug following live, settling and paired idle observations, stale-data/error
checks and separate active playback observations. Sustained/blocked cases cause
a failing suite. Read the report's coverage/limits; this is not an assertion of
all codec/OS integration behavior.

`get_view_state.preview.image_load.activity` records last activity caller,
counter and pointer coordinates. `scripts/validate-idle-followup.cjs` adds
longer stationary/reopen samples, live writer invalidation, populated menus,
plugin Single/Multiple panels, Inspector sections, native Watcher stimuli,
export error/recovery and diagnostic controls. Finite hover/input transitions
are retained and followed by bounded quiet rechecks; sustained work still fails.

## Native-window and input diagnostics (2026-10-15)

`get_ui_state` adds `input_activity` counters (move events, changed positions, buttons, keys, last pointer) and `window_status` (valid, owned, visible, minimized, hung). These are passive and never notify/log. Mutation-guarded `control_ui {action:"window_restore"}` validates ownership before restoring only this child process's minimized HWND with no activation. Test suites record window interruptions and compare input counters; they cannot call an unrendered/minimized interval a quiet pass. No OS mouse/key injection.

## Debug repaint display filter (2026-10-15)

`get_debug_info` accepts optional `hide_ui_refresh: bool`: omitted/false keeps raw
queries compatible with idle audits, true applies the actual drawer's routine
DEBUG/TRACE accessibility-repaint display rule. `ui.filter.hide_ui_refresh`
reports the drawer default (true). Raw capture/retention are unchanged. The
same-process scenario scripts/validate-debug-hover.cjs verifies real GPUI hover,
raw-vs-displayed queries, normal DB records, search and paging.
## Preview cache policy diagnostics (2026-10-15)

`open_media` now hands ordered path/ID/thumbnail triples through FileManager's
`preview_items_by_ids` and Preview's `set_indexed_filmstrip_items`, matching the
FileManager opening route. `get_view_state.preview.image_load.cache_policy` exposes
the progressive policy, nearby indexed/display thumbnail paths and pending job
count without file/database reads. `get_debug_info` retains reuse/cancellation
details in original messages. No new tools or mutation permissions are added.


## Browser runtime update — 2026-10-15

Browser diagnostics in get_ui_state.browser: filesystem, show_descendants, worker_active, status tuple (generation,revision,loading,scanned,error). control_file_manager adds navigate(value), show_descendants(enabled), thumbnails(enabled), cancel_directory, missing_records, record_candidate(media_id,value), relink_record(media_id), delete_missing_records (two calls confirm), locate_file(media_id), close_dialogs. All UI mutations require the existing opt-in.

`manage_tag_records`: list_missing is read-only; relink(record_id,path) and delete_missing(record_ids,confirm=true) require mutation opt-in. `manage_favorite`: rename(folder_id,name), move(folder_id,delta), mutation opt-in. Mutations use production services and publish normal reload events. Scripts validate-browser-mcp.cjs / validate-browser-extended.cjs / validate-browser-restart.cjs run via fileatlas-mcp-debug.cjs with isolated profiles. No OS input automation or external process attachment is used.

## Window bounds in `get_ui_state.window_status` (2026-10-15)

`window_status` now also returns `bounds` (`x`, `y`, `width`, `height`) from
`GetWindowRect` for the MCP child's own HWND, or `null` when the handle is
invalid or the rect call fails. Values are physical device pixels, so a
scaled display reports width/height larger than the logical GPUI window size;
use `panel_instances` client bounds for logical measurements. Read-only: this
adds no mutation and no new tool.

## Sidebar section debugging (2026-10-16)

- `get_ui_state.sidebar_folders` returns the snapshot the mounted Navigation
  sidebar published (`NavigationService::sidebar_state`), or `null` before the
  first publish. It carries drives with usage bytes, system and user quick
  access rows, favorite groups with their folders, library folder paths,
  collapsed-section flags, `show_network_drives`, `browse_mode` and the open
  inline editor target/text. Screenshots stay a visual check; section state is
  asserted from this field.
- `control_ui { action: "sidebar_request", request: <SidebarUiRequest JSON> }`
  queues one semantic sidebar edit through `NavigationService::request_ui`, the
  same path the rendered rows and popup menus use. Payload kinds:
  `toggle_section{key}`, `set_network_drives{show}`, `toggle_drive{root}`,
  `activate_path{path}`, `toggle_expanded{path}`, `add_to_library{path}`,
  `add_quick_access{path}`, `remove_quick_access{path}`,
  `add_favorite_folder{group_id,path}`,
  `remove_favorite_folder{group_id,folder_id}`, `remove_favorite_group{group_id}`,
  `open_editor{editor,text}`, `confirm_editor`, `cancel_editor`. Editor targets:
  `add_quick_access`, `rename_quick_access{path}`, `create_group`,
  `rename_group{group_id}`, `add_favorite{group_id}`,
  `rename_favorite{group_id,folder_id}`.
- The action returns `{"queued":true,"ui_state":...}`. Requests are applied on
  the next view render, so poll `get_ui_state` (the sidebar timer re-notifies
  while a request is pending) before asserting. Navigation validates targets and
  paths; unknown groups/folders/sections or missing directories return an error
  and never reach the view.
- Like every other control action this is gated by Standard or Full access. It injects no OS pointer/keyboard events and
  does not open popup menus: `control_ui` `pointer_move`/`pointer_down`/
  `pointer_up` do, and their result is checked on a screenshot.
- `window_status.bounds` (added with the startup window sizing work) reports the
  native `GetWindowRect` in physical pixels, which is what explains a
  screenshot's pixel size; section state itself is asserted from
  `sidebar_folders`, not from screenshots.

## Pointer geometry in `get_ui_state.window_status` (2026-10-16)

`screenshot_window` captures the whole window rect (`PrintWindow` +
`GetWindowRect`), while `control_ui` pointer actions dispatch GPUI logical
client points. `window_status.geometry` closes that gap so a click target is
derived instead of guessed:

- `scale_factor` — `GetDpiForWindow(hwnd)/96`, the same value GPUI uses.
- `client_size` — `GetClientRect` divided by `scale_factor`, i.e. the logical
  client area the app lays out in.
- `screenshot_to_pointer.{offset_x,offset_y,divide_by}` — the client origin
  inside the window rect (`ClientToScreen(0,0)` minus `GetWindowRect` left/top)
  and the divisor. Pointer coordinate = `(screenshot_px - offset) / divide_by`.

Read `geometry` *after* `resize_window`: the startup window and a resized window
report different sizes, and offsets are per-HWND-state. On this host the probe
returned `scale_factor 1.5`, `offset (11, 45)` and a 960×600 logical client for
the 1462×956 physical window rect. Read-only: adds no mutation and no new tool.



## Review corrections — 2026-10-16

`control_ui.sidebar_request` additionally accepts refresh_roots, retry_save, toggle_group (group_id). `get_ui_state.sidebar_folders` includes roots_revision, system_folders_pending, save_error, directory child_count, drive usage_error and editor.error. These ordinary controls require Standard or Full access. Test scenarios are checked into scripts/validate-sidebar-review*.cjs and validate-browser-selfwatch.cjs; pointer coordinates use get_ui_state.window_status.geometry.


## Native activation regression tools — 2026-10-16

Mutation-guarded control_ui actions window_minimize and window_activate target only the published HWND owned by the current MCP child (ShowWindowAsync, and SetForegroundWindow on activation). They exercise real window lifecycle events, without OS input injection or controlling another process. window_status.active reports actual foreground ownership, so tests do not equate a queued request with activation. get_ui_state.file_manager.snapshot_revision reports FileManager query scheduling independently of directory generation. Reproduction: scripts/validate-browser-refresh.cjs.


## Address bar and key events — 2026-10-16

Mutation-guarded control_ui action address_bar accepts request:{kind:back/forward/up/edit/cancel/submit/set_text/crumb}, with text/path payloads as defined by Navigation::AddressBarRequest. It queues the same mounted-view handlers as controls. Read-only diagnostics are get_ui_state.sidebar_folders.address_bar (path/editing/text/error/breadcrumbs and can_*).

control_ui key_press accepts only enter, escape, ctrl-l, alt-left, alt-right, alt-up in value. CaptureWindowHandle::{queue_key,key_pending,take_key} carries a single pending String to Shell; the existing MCP input poll/dispatch lease delivers PlatformInput::KeyDown through the same GPUI window, not OS injection. The field prefer_character_input is false. Input text in scenarios uses the real InputState via set_text; actual keyboard/paste/IME entry is not claimed by that setup. These UI-control tools require Standard or Full access.

Checked-in scripts: validate-address-bar.cjs (semantic routes + actual Enter/Escape), validate-address-pointer.cjs (real pointer controls), validate-address-restart.cjs and prepare-address-tag-restart.cjs (scope stability). See docs/validation-ui/2026-10-16-address/README.md.


## Region separation probe and control (2026-10-16)

`get_ui_state.region_separation` reports the applied theme token (`border`/`shadow`)
read straight from `theme::region_depth_name()`, so tests observe what the paint-time
helpers actually use instead of re-reading the JSON file. Mutation-guarded
`control_ui {action:"region_separation", value:"border"|"shadow"}` resolves
`fileatlas_plugin_settings::SettingsService` from the Cordis context and calls the same
`update` path the Settings view uses; an unknown value is rejected before any write.
It does not touch GPUI directly, so the live repaint still comes from the Settings
view's `cx.refresh_windows()`; pointer evidence for the real dropdown is recorded in
docs/validation-ui/2026-10-16-region-separation/README.md.

## Settings software check (2026-10-17)

`get_ui_state.settings_software_check` publishes the passive snapshot shown by
Settings > Software: `state` (`loading`/`ready`/`failed`), `revision`, and
`software` containing display identity, installation state, detected version/path,
purpose, and project/download/GitHub URLs. `control_ui {action:"settings_tab",
value:"software"}` selects the view for same-process validation. The detector
reads Windows uninstall/App Paths registry entries and conventional executable
locations in a background ReadModel; it does not launch, install, or query the
listed applications. `scripts/validate-software-check.cjs` exercises the page
and snapshot with an isolated profile. Everything's app download is voidtools'
website; its linked GitHub repository is the official ES command-line tool,
not the Everything application.

Search location (2026-10-17): mutation-guarded `control_file_manager` action
`search_entire_computer` accepts `enabled` (default true) and calls the same
FileManager action as the top-bar toggle. `get_ui_state.file_manager.search_entire_computer`
reports the rendered selection. Scope semantics are verified against real SDK
results and a same-process GPUI click; no OS input injection is used.

### Launcher interaction diagnostics (2026-10-17)

`get_launcher_state` adds nullable `installed_on` (YYYY-MM-DD) and `size_bytes` to entries, plus a nullable `ui` projection while its panel exists: selected_entry, draft_name/category/hidden, sort (name/installed/size), descending, visible_ids/count, icons_loaded/failed/pending and render_count. Disposing the view clears this projection.

Mutation-guarded `add_launcher_program {path}` calls the launcher's public manual-add service; it never starts the target. Mutation-guarded `control_ui input_text` accepts at most 1000 printable characters and sends text to the currently focused GPUI input handler in the same process. `key_press` additionally allows ctrl-a and backspace. This uses the existing window input queue, without OS injection or clipboard changes. Same-process launcher validation compiles a no-window executable into the isolated profile; double-clicking its tile writes only a marker beside that executable. See `scripts/validate-program-launcher.cjs`.

Mutation-guarded `set_launcher_pin {id, scope}` sets `scope` to `none`, `category`, or `all` through `LauncherService`. `get_launcher_state.entries[].pin_scope` returns the same scope string, and `ui.visible_ids` reports the view's current pinned ordering. `scripts/validate-program-launcher.cjs` verifies multiple pinned entries across name/date/size sort directions and panel reopen.

Clipboard favorites: `get_clipboard_state` exposes `favorite_count` and each item’s `favorite` flag; `pinned_count`/`pinned` remain compatibility aliases. `add_clipboard_text.favorite` accepts legacy `pinned`. UI metadata includes `confirm_favorite_delete` without exposing content.
# Theme diagnostics (2026-10-18)

Read-only `get_ui_state.theme` exposes `id` (`workbench-navy`), current `accent`,
`selection`, `base`, `panel`, `card`, `border` (packed RGB integers) and `body_size`
(logical px). These values read the public UI Bridge contract without extra IO or
notification; they prove selected tokens, while `screenshot_window` from the same
process proves their actual visible application. No new mutation tool is added;
accent changes are exercised through real Settings controls.

## Program workspace and columns diagnostics (2026-10-19)

`get_ui_state.program_workspace` is a read-only snapshot when the optional
`fileatlas.columns` and `fileatlas.workspaces` services are active. It reports the
runtime layout, zero-based active column, layout revision, active workspace
definition, default workspace ID, all saved workspace/column definitions, and
whether the workspace manager panel is open. It is `null` when either plugin
is unavailable. Workspace addresses and filter definitions are configuration
data; this diagnostic does not enumerate media or touch the FileManager database.

For isolated same-process UI verification, mutation-guarded `control_ui` actions
are available at Standard or Full access: `columns_layout` with `value` set to
`single`, `dual`, or `quad`; `activate_column` with a zero-based numeric `value`
(service validation rejects indexes outside the active layout); `workspace_select`
with an existing workspace ID; and `workspace_manager` with `open` or `close`.
Workspace selection uses the public WorkspaceService and updates the active
workspace; layout and column activation use ColumnLayoutService. These actions
cannot delete, rename, or rewrite workspace definitions. `workspace_manager`
only changes the panel open state. Re-read `get_ui_state.program_workspace`
after each action to verify the resulting service state.
# Unified favorite groups (2026-10-18)

`control_ui {action:"sidebar_request", request:{kind:"add_current_favorite",group_id}}`
uses Navigation's real group-plus handler with action-time committed-directory
validation. Mutation guard remains required. Read-only `sidebar_folders` adds
`favorite_current_path`, `favorite_action_error`, `favorite_imported_paths`; each
`favorite_groups[]` has measured `header_bounds` in logical client coordinates,
so real GPUI pointer clicks can target the visible plus/menu/chevron after layout.
Bounds are diagnostics, never instructions to control another process. Scenes:
`scripts/validate-favorite-groups.cjs` and `validate-favorite-groups-restart.cjs`.
# Declarative settings providers — 2026-10-18

`control_ui {action:"settings_plugin_page",value:<owner>}` accepts owners with a
live `UiViewContribution.settings_page` as well as the existing runtime-managed
tools. Disabled runtime tool pages remain accessible for saved settings. Unknown
or removed provider-only owners are rejected. The settings validation scenario
is `scripts/validate-settings-minimal.cjs`; it uses same-process pointer/key events.


## Native window testing — 2026-10-18

`get_ui_state` includes `selected_window`, `main_window_status`, and `native_panels`
(ID/HWND/owned-window status/render_count). `control_ui` action `window_select` accepts
`main` or a live native instance ID and selects the target for screenshots, resize and
same-process pointer/key dispatch. Mutation permission is required. Selection rejects
unregistered or non-owned HWNDs; closing the selected native instance falls back to main.
`window_minimize`/`window_activate` operate on that selected window. No external app input.

`control_ui.window_close` posts WM_CLOSE only for a selected, registered native plugin HWND; main is rejected. It exercises the same OS close callback used by system chrome.

## Windows drag-out diagnostics — 2026-10-19

`get_ui_state.drag_drop` reports the last `DragDropService` native lifecycle:
`idle`, `queued`, `active`, `dropped`, `cancelled`, or `failed`. This lets a
same-process `control_ui` pointer gesture verify that the FileManager callback
queued and entered the OLE drag path, and that the app returned to a usable
state. `control_ui` pointer input is synthetic GPUI input; it does not simulate
a held OS mouse button or verify a drop into Explorer/another process. Those
system-level delivery paths still require a real Windows drag.

## Live MCP mutation access — staged permissions

MCP publishes the `mcp/settings` settings-page view through `fileatlas.mcp`; it uses
the existing SettingsService and owns no duplicate persistence. The segmented
setting is Off, Standard or Full and is checked for every host and contributed
mutating route. Standard covers UI control and ordinary mutations; Full also
permits plugin-declared high-impact tools, which retain per-call `confirm=true`.
Changes apply immediately without restarting MCP. The old boolean-key validation
below is historical; current same-process Windows evidence is documented at
`docs/validation-ui/2026-10-19-mcp-access-stages/`.

## Actual content paint pairs — 2026-10-19

`get_ui_state.theme.content_colors` is read-only: `formats` lists extension,
background and foreground from the same `file_format_colors` used by FileManager;
`tags` lists tint/selected/background/foreground/icon from `tag_colors` (neutral,
black, white, purple, blue and yellow in both selection states). `selection_text`,
`media_background`, `media_text` and `media_star` resolve actual navigation and
thumbnail roles. This projection performs no IO and adds no timers or mutation
permissions. `scripts/validate-thumbnail-contrast.cjs` uses these roles plus real
isolated local files, assigned tags, rating, and captured same-process windows.
