# MCP plugin ports / MCP 对接

Status: local stdio MCP server and typed PhoTag tools are implemented. The MCP client launches `photag-shell --mcp` as a child process; that process owns both the GPUI window and MCP stdio server, so MCP controls and reads the exact PhoTag instance it launched. It does not attach to a separately running PhoTag process. Windows `PrintWindow` capture and live MCP round trips have been exercised with the isolated dev-test profile; see `docs/validation-report.md` for the checked interactions and remaining limits.

## Cordis lifecycle and dependencies

- Cordis plugin: `photag.mcp`; lifecycle guard service:
  `photag/mcp-task-guard`; host window-handle service:
  `photag/mcp-window-handle` (`CaptureWindowHandle`).
- Requires `GalleryService`, `NavigationService`, `TaggingService`, `PreviewService`, `MediaPlaybackService`, `MetadataExportService`, `FileOperationsService`, and `CaptureWindowHandle`.
- The Shell publishes `CaptureWindowHandle` before providers start, fills it with the actual PhoTag HWND when GPUI creates the window, and stops the MCP plugin before stopping its providers.
- MCP uses the official Rust `rmcp` server SDK and stdio transport. Protocol messages use stdin/stdout; diagnostics use stderr.
- Cordis disposal aborts the MCP task. If Cordis cannot publish the lifecycle
  guard during startup, the transport task is aborted before the plugin returns
  an error. No localhost/network listener is started.

## Tool contracts

- `screenshot_window`: captures only the PhoTag top-level HWND on Windows with `PrintWindow(PW_RENDERFULLCONTENT)`, encodes the result as PNG, and returns MCP image content plus pixel dimensions. A missing, minimized, invalid, or unsupported window returns an error. Visual output has been inspected for Gallery and Preview on the isolated validation profile; other screens and DPI/window-size configurations remain to be checked.
- `resize_window`: changes the current PhoTag HWND outer size for isolated layout checks. Width is limited to 640–4096 and height to 480–2160; the result includes requested and actual window bounds because Windows DPI and monitor work-area limits can clamp the request. It does not move, activate, or reorder the window; capture afterward with `screenshot_window` to check the rendered layout.
- `list_library`: reads registered folder IDs, paths, last-scan times/counts, and the hierarchical tag tree.
- `search_media`: supports a phrase across filename/path/tag, folder ID, tag IDs, media type, offset, a maximum page size of 100, and the source sort orders.
- `get_media`: returns a single indexed media row and its current tags.
- `get_view_state`, `open_media`, `control_preview`, and `control_playback`: inspect/change preview and playback state using the existing preview/playback services. Preview controls include `close`; live image rendering was exercised with the generated PNG. Video/audio codecs and device output still need Windows runtime validation.
- `cancel_scan`: requests cancellation for a folder's active scan.
- `dev_test_status`, `dev_test_scan_fixture`, `dev_test_inject_virtual`, and
  `dev_test_clear_virtual`: control the optional `DevTestService` in the same
  PhoTag process. These tools are available when `PHOTAG_DEV_PLUGINS` contains
  `dev-test`; virtual injection accepts only 1,000, 10,000, or 100,000 rows.
  Start operations return immediately; poll `dev_test_status` until `running`
  is false, then use `search_media` / `list_library` and `screenshot_window` to
  inspect the same running application. The isolated `target/dev-test` data
  directory is selected by the Shell profile, not by MCP tool arguments.
- `set_rating`, `assign_tags`, `remove_tags`, `create_tag`, `rename_tag`, `delete_tag`, `add_folder`, `remove_folder`, `rescan_folder`, `rename_media`, `copy_media`, `move_media`, `delete_media`, and `export_metadata` are mutating tools. They require `PHOTAG_MCP_ALLOW_MUTATIONS=1`; file/export, folder, tag-delete, and rescan tools also require `confirm: true`. Batch limits are 100 media IDs and 50 tag IDs. Tag assignment follows the PhoTag best-effort XMP/IPTC writeback behavior.
- `delete_tag` removes the requested tag and its direct associations; it passes `clean_parents: false` because the MCP request only confirms deleting that tag and does not separately request ancestor-assignment cleanup. The Navigation service's affected media IDs are intentionally omitted from the existing `{ "ok": true }` response.

Mutations are disabled by default. `confirm: true` is an explicit call-site acknowledgement, not proof of a human interaction; configure the MCP host to ask the user before invoking mutating tools. Copy/move destinations and media paths use the existing FileOperations service. Do not run this MCP server for an untrusted client.

## Current limits and debugging

- The MCP server is a child-process stdio endpoint: the client must launch `photag-shell --mcp`; the child process itself is the GUI application instance MCP controls. It does not attach to an already-open PhoTag process.
- MCP tool availability does not complete native UI behavior listed in `docs/source-feature-audit.md`; tools call the same Cordis services and their existing limits apply.
- Check `PHOTAG_MCP_ALLOW_MUTATIONS`, whether `--mcp` was passed, Shell plugin dependency order, and stderr when tools are missing or startup fails.
- If a screenshot fails, check HWND publication and window state first. If it returns a blank or stale image, inspect PrintWindow/GPUI Windows rendering behavior; do not treat protocol compilation as screenshot parity.
- Batch scan receivers are drained by background workers so the indexing worker can finish; scan progress is exposed through `get_ui_state.scan_progress` when a scan is active.

## UI validation tools (2026-10-13)

`get_ui_state` reads settings_open/settings_tab, language, translation_revision
and preview_chrome_alpha. These supplement get_view_state; they do not describe
OS pointer position or all GPUI widget state. `control_ui { action, value? }`
requires PHOTAG_MCP_ALLOW_MUTATIONS=1 and supports settings_open/settings_close,
settings_tab, language (en/zh-CN), statistics, bulk_actions, gallery_dialogs_close
select_media (value is an indexed media ID string), and deselect. Settings uses public
UiSurfaceState; language uses I18nService; Gallery uses GalleryUiRequest.
Settings and Gallery consume requests asynchronously. Wait for rendering before
screenshot_window. No OS mouse/keyboard injection is used.

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
Gallery close button. These service-level requests and screenshots verify
state/rendering; they do not prove pointer click propagation inside filter
popovers, native dialogs, or every designed state.

## v5 live Gallery/Navigation controls (2026-10-14 handoff)

`get_ui_state` adds `gallery_popover`, `tools_open` and the live Gallery
snapshot (filters, selected IDs, matched count, month grouping and inspector
sections). `control_gallery` controls the same Gallery actions and controlled
Popover state as the UI. Actions: popover, search, media_type, sort, rating,
extension, tag, tag_mode, extension_mode, date_from, date_to, clear_filters,
inspector_section, tag_picker, month_grouping, tools_open and tools_close.
Read tools/list for exact optional fields and accepted string values.
`control_gallery` additionally accepts bulk_open, bulk_close, and
bulk_configure; configuration uses `value` for delete/copy/move and
`bulk_source` for all/folder/tag/selection. `get_ui_state` reports the live
bulk action/source. `open_media_menu` with `media_id` and `close_media_menu`
control a live Gallery-owned media action surface; its state and indexed ID are
reported by `get_ui_state`. The native right-click ContextMenu remains
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
scripts/photag-mcp-debug.cjs does so, uses an isolated target profile, captures
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

control_gallery actions select_many (1–1000 unique existing media_ids) and scroll (zero-based current-result index in value) are mutation-guarded UI requests, including unloaded records. get_ui_state gallery includes window_offset/loaded_count and folder_id/path_prefix. 100012-row runtime observed window_offset99812/loaded_count200 and bulk2 across pages. Detailed profile/plans/reports and current limits: docs/redesign-validation.md latest section.

## v2 panel and GPUI input validation (2026-10-14)

control_ui adds navigation_tab(folders/tags/extensions), inspector_tab(information/history/extensions; info alias), sidebar_width/inspector_width(value:logical pixels100–1000 then hostclamp), pointer_down/pointer_drag/pointer_up(value:x,y logicalclient coordinates0–4096). All are guarded by PHOTAG_MCP_ALLOW_MUTATIONS. CaptureWindowHandle now holds the nativehandle and bounded single pending McpPointerInput{kind,x,y}; queue_pointer rejects busy/poisoned and shell take_pointer drains/deferred GPUI event dispatch. No OS pointer/keyboard injection and no external-process attach. get_ui_state exposes active pages and applied widths. control_navigation adds expand_folder(folder_id), expand_path(value:directory), select_path(value:directory), folder_search(value:query); requests validate registeredroot containment and perform canonical path selection/lazy loading. Live event drags/clicks and read-only rejection verified; see docs/redesign-validation.md.

## Floating panels and list validation (2026-10-14)

`control_gallery layout` accepts value grid/list through GalleryUiRequest::Layout. `get_ui_state.gallery.layout` is the live rendered choice. `get_ui_state` adds panels(builtin visibility/frame/client bounds), panel_instances(all currently open frame/client bounds), open_panel_ids and active_panel. Bounds are comma-separated logical pixel strings; client_bounds are GPUI-measured, independent of screenshot DPI/nonclient offsets.

Mutation-guarded `control_ui panel_open` accepts a registered definition ID in value and returns instance_id; panel_close accepts that returned instance ID. Both call UI Bridge PanelCatalog; they cannot manufacture unregistered content. dev-test supplies actual task-status definitions for isolated single/multiple checks. pointer_move/pointer_right_down/pointer_right_up additionally queue same-process GPUI events using x,y; existing pointer_down/drag/up remain. No OS injection or separate process attachment. Wait for the host/feature poller before reading screenshots/snapshots.

## Native folder picker validation (2026-10-14)

get_ui_state.folder_picker returns pending and native_open(the Navigation provider actually found its current native window). Mutation-guarded control_navigation folder_picker_cancel calls NavigationService::cancel_folder_picker; it only cancels an open picker. Read-only screenshot_folder_picker captures the identified same-process native picker through existing PrintWindow encoding, with no external attachment. The debug client accepts capture_tool for a planned capture; default remains screenshot_window. Actual Sidebar + pointer clicks, native screenshots, two cancel/reopen cycles and unchanged library were recorded. These controls do not simulate folder acceptance or claim its coverage.

## All-file query contract (2026-10-14)

`search_media.media_type` now accepts `file` in addition to `image`, `video`, and `audio`; omission queries all types. `file` identifies indexed unsupported/extensionless files, not directories. Same-process screenshots, Gallery controls, native GPUI pointer dispatch and Navigation add/rescan tools validate these cards with an isolated profile.

## Startup refresh diagnostics (2026-10-14)

`get_ui_state.library_refresh` returns `running`, `total_folders`, `completed_folders`, and `[path, error]` failures from the Watcher-owned startup scan. The read-only field lets the same-process MCP client wait for reconciliation and inspect failures without invoking a rescan mutation. `list_library` and `search_media` then show current persisted row totals.

## Grouping controls (2026-10-14)

`control_gallery` accepts action `grouping`, value `none|month|file_type|modified|created|folder|subfolder`; `toggle_group` uses an exact get_ui_state group label; `popover` accepts `grouping`. The existing `month_grouping` boolean remains an alias for Month/None. All require PHOTAG_MCP_ALLOW_MUTATIONS=1 and queue the real Gallery actions. get_ui_state.gallery exposes grouping_mode, ordered groups as [label,count], collapsed_groups, window_offset and loaded_count. search_media remains a database query in query sort order; it does not pretend to return grouped UI order. Runtime commands/restart/screenshots are recorded under docs/validation-ui/2026-10-14-grouping.

## Directory-browser controls (2026-10-14)

control_gallery adds open_directory(value=an entry in the current directory-card list), directory_up and refresh. It rejects invisible paths and no-parent views; these queue Gallery's real actions and keep PHOTAG_MCP_ALLOW_MUTATIONS gating. get_ui_state.gallery publishes browsing, directories[{name,path}], current_directory, parent_directory, directory_error and selected_directory; matched_count remains the displayed indexed-file count. Directory cards have no synthetic media IDs. control_ui adds pointer_double_down/pointer_double_up (same x,y logical client coordinates), dispatched by Shell as same-process GPUI MouseDown/MouseUp with click_count2. Single/drag/right-click behavior is unchanged. See docs/validation-ui/2026-10-14-folder-browser for84-step evidence including mouse hit tests and read-only restarts.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics tools: get_debug_info(category?,level?,search?,page?,page_size?) returns actual drawer ui and logs Page from optional photag/debug service. Read-only and deliberately does not log its own query. control_debug(action,value?) shares DebugService::control with native controls and requires PHOTAG_MCP_ALLOW_MUTATIONS=1. Unknown service/action/invalid values return normal error JSON. MCP rejected requests are logged as ERROR; stdout remains reserved for protocol traffic.
