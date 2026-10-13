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
