# Shell composition ports / Shell 对接

`photag-shell` is the host composition root. It publishes `UiBridgeService`, starts Cordis plugins in dependency order, mounts GPUI view factories into host regions (including floating `workspace.modal` views), and dispatches command IDs as Cordis events. The Settings command is contributed by the Settings plugin; its modal open state is shared through `UiSurfaceState` and its floating layer is mounted above application content.

The host chrome and contribution fallback view use `photag_ui_bridge::theme` surface and text tokens; the native base surface consumes source Mica/Acrylic alpha. GPUI Kit typography presets are still used by the shell; parity with source Segoe UI font loading and Windows font availability has not been validated.

The shell restores `SessionState.sidebar_width` when it is between 160 and 500 pixels (otherwise 288 px) and saves the width when the sidebar divider drag ends. The updater is started before Settings. When Settings confirms an update it persists the current session snapshot, queues the staged path, and quits GPUI; after the application loop returns, the shell installs and relaunches the staged build before disposing the updater service. This update shutdown path still needs interactive Windows validation.

The explicit `dev-test` profile starts `photag-plugin-dev-test` after Gallery and
its providers. It redirects the database, Settings, Session and thumbnail cache
to `target/dev-test/` (or `PHOTAG_DEV_TEST_DIR`); `PHOTAG_DEV_TEST_DB` can
override only the database file. This profile keeps local media-load checks out
of the user's normal PhoTag database and settings.

## Current runtime dependency order

Database → FileOperations → DragDrop → Gallery; Database → Preview → Gallery; Database → MediaPlayback → Gallery; Database → Thumbnails → Indexing → Watcher; Navigation depends on Database/Indexing and optionally Watcher; Settings reads Updater and Session and optionally reads Navigation; Tagging depends on Database; History starts after Database, FileOperations, and Tagging. Session and Updater start before Settings; AI starts after Settings when Gallery, MCP, all plugins, or the explicit `ai` development plugin is enabled. Shell starts Gallery after all required providers and the optional Settings/Tagging/History providers. Preview consumes AI suggestions through Cordis. In `--mcp` mode it also starts all required feature providers, publishes the Shell-owned `CaptureWindowHandle`, and starts `photag.mcp` after Gallery, Navigation, Tagging, Preview, MediaPlayback, and FileOperations. After GPUI exits, Shell consumes any queued updater install before disposing Updater. On shutdown it stops MCP before Gallery, then Preview before History, AI, Settings, Tagging, and MediaPlayback; Navigation stops before Watcher, Indexing, and Thumbnails. Preserve service availability before dependent plugin startup and stop consumers before providers. Gallery opens images in Preview, routes videos to MediaPlayback, calls DragDrop from card drag gestures, and optionally consumes Settings, Tagging, and History services.

## Boundaries

The shell must not absorb scanning, SQL, filtering, tagging, or file operations. It publishes the actual GPUI HWND through `photag-plugin-mcp::CaptureWindowHandle`; the MCP adapter captures only this PhoTag window on Windows. Stop plugin fibers before dropping shared runtime resources so Cordis cleanup can remove published services and UI contributions. Source-backed behavior is mapped in `docs/source-feature-audit.md`; Windows runtime and visual parity still require native validation.
