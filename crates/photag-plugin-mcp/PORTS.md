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
- Batch scan receivers are drained by background workers so the indexing worker can finish; the current tools do not expose scan progress snapshots.

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
`control_navigation` accepts create/edit/delete/merge/close with optional
required-on-target tag_id; these open dialogs, never commit deletion/merge.
Both tools require the existing mutation flag and asynchronous view polling.

MCP `open_media` currently still rejects audio, despite Playback's new
underlying audio-only implementation. New Playback media_id/open_media API
and audio routing are planned, not implemented. Tool text JSON may carry an
error without CallToolResult.isError; clients must check both. The new
scripts/photag-mcp-debug.cjs does so, uses an isolated target profile, captures
via screenshot_window and keeps one child window alive in interactive mode.
It cannot attach to another instance. Current-source compilation is blocked
by Gallery's 19 errors; the last successful exe predates final layout edits.
