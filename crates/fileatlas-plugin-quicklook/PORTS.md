# QuickLook viewer integration

## Identity and ownership

- Cordis plugin `fileatlas.quicklook`, service `fileatlas/quicklook` (`QuickLookService`).
- `Config=()`, `Input=()`. No startup dependencies, views, workers or events.
- Shell starts it before FileManager/MCP and stops it after those consumers.
- Cordis owns service publication/cleanup. FileAtlas never starts, terminates,
  installs, downloads, updates or uninstalls QuickLook; it only sends requests to an
  instance the user started.

## Contract

- `open(path: impl AsRef<Path>) -> Result<(), QuickLookError>` is a blocking,
  action-time operation. It requires an existing ordinary file. FileManager and MCP
  call it on a blocking worker. It never reads/decode/transforms media itself.
- **Request only.** FileAtlas never starts, restarts, terminates or installs
  QuickLook; the user owns that process. The sole dispatch target is the running
  instance's `\\.\pipe\QuickLook.App.Pipe.<user SID>` pipe, written as one
  `QuickLook.App.PipeMessages.Toggle|<path>|\r\n` line. When nothing answers that
  pipe the press fails with `NotRunning` and FileManager shows a translated alert
  telling the user to start QuickLook. The earlier `QuickLook.exe <path>` entry was
  removed on 2026-10-16 at the user's request: it is QuickLook's second-instance
  launcher, which boots the .NET/WPF runtime (1.06-1.12 s) before forwarding, and
  an instance FileAtlas started kept its window until the user closed it and then
  stopped answering previews altogether.
- Measured through the app over the pipe (2026-10-16, `target/mcp-ql-pipe-measure` and
  `target/mcp-ql-pipe-close`): the request leaves FileAtlas in 15-33 ms including the MCP
  round trip, and the viewer showing that same file appears 37-254 ms after the press
  once QuickLook and that file type's plugin are warm. The first press that uses a
  plugin type cost 600-755 ms, which is QuickLook loading that plugin, not dispatch.
- The request line is always assembled before the pipe is opened. QuickLook's
  reader loop dies on a connection that carries no line, so a half-open client
  would leave the user's QuickLook alive but permanently deaf to every
  cross-process request.
- The pipe path is derived from the current token and cached after the first
  success. Win32 requires the computer component in the name passed to
  `WaitNamedPipeW`, so the bare form always fails with `ERROR_BAD_PATHNAME`.
  QuickLook owns exactly one server instance, so `WaitNamedPipeW` (250 ms) gates
  the open; opening a busy pipe would block this worker. A name that nobody
  created fails instantly, and the wait/open pair retries three times so a rapid
  second press or a race for the single instance is not reported as unreachable.
  A diagnostic probe must only wait, never open.
- `PrivilegeMismatch` needs a Windows integrity comparison, and that process
  snapshot runs only when QuickLook's pipe rejects this client (`ERROR_ACCESS_DENIED`);
  a successful or absent-instance dispatch performs no snapshot.
- Errors: `NotRunning` (no instance answers, or it refuses this process),
  `MissingFile` (target removed), `PrivilegeMismatch` (elevated QuickLook),
  `NotInstalled` (non-Windows builds; there is no Windows install probe).
- `snapshot() -> LaunchSnapshot` returns the last attempted path and the dispatch
  outcome/error. `pipe_toggle_sent` only means dispatch; it does not assert
  external rendering, codecs, current visibility or focus.
- Windows MCP diagnostics use `QuickLookService::inspect()` and
  `visible_windows()`; the Win32 window/process/pipe implementation stays private
  to this crate. `QuickLookInspection` is read-only diagnostic state and does
  not claim that QuickLook rendered or focused a file.

## Consumers and failure inspection

- FileManager routes Space, Enter, its retained custom open key, file double-click,
  context-menu Preview and the inspector thumbnail click to this service for all
  file types. Holding Space or modified Space does not repeatedly toggle it.
- FileManager shows a GPUI alert on failure, translated: `NotRunning` tells the user to
  start QuickLook themselves (and to install it if it is missing). The app neither
  opens an installer, starts the viewer, nor falls back to another player.
- MCP contributes `quicklook.open_media {media_id}`, `quicklook.snapshot`, and
  `quicklook.inspect` from this feature crate through `McpToolRegistry`; open
  requires mutation permission and calls the same service. The MCP host retains
  its image-content `screenshot_quicklook` diagnostic because the feature tool
  contract currently returns structured JSON values.
  `control_file_manager {action:"preview",media_id}` exercises the FileManager alert flow.
- Inspect `quicklook.snapshot`, `get_ui_state.quicklook` and
  `get_ui_state.quicklook_notice`, plus `fileatlas_plugin_quicklook` tracing spans.
  Popup rendering and UI quiet-state checks use same-process FileAtlas MCP.

## Upstream references

- https://github.com/QL-Win/QuickLook/wiki/Develop%2C-build-and-integrate
- https://github.com/QL-Win/QuickLook/blob/master/QuickLook/App.xaml.cs
- https://github.com/QL-Win/QuickLook/blob/master/QuickLook/PipeServerManager.cs

No QuickLook binaries, libraries, plugins or source are bundled. Existing
FileManager thumbnails, inspector previews and indexing's FFmpeg support remain
independent of this service.
