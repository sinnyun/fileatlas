# Shell composition ports / Shell 对接

`photag-shell` is the host composition root. It publishes `UiBridgeService`, starts Cordis plugins in dependency order, mounts GPUI view factories into host regions (including floating `workspace.modal` views), and dispatches command IDs as Cordis events. The Settings command is contributed by the Settings plugin; its modal open state is shared through `UiSurfaceState` and its floating layer is mounted above application content.

The host chrome and contribution fallback view use `photag_ui_bridge::theme` surface and text tokens; the native base surface consumes source Mica/Acrylic alpha. GPUI Kit typography presets are still used by the shell; parity with source Segoe UI font loading and Windows font availability has not been validated.

The shell restores `SessionState.sidebar_width` when it is between 200 and 400 logical pixels (otherwise 264 px) and saves the width when the sidebar divider drag ends. The updater is started before Settings. When Settings confirms an update it persists the current session snapshot, queues the staged path, and quits GPUI; after the application loop returns, the shell installs and relaunches the staged build before disposing the updater service. This update shutdown path still needs interactive Windows validation.

The explicit `dev-test` profile starts `photag-plugin-dev-test` after Gallery and
its providers. It redirects the database, Settings, Session and thumbnail cache
to `target/dev-test/` (or `PHOTAG_DEV_TEST_DIR`); `PHOTAG_DEV_TEST_DB` can
override only the database file. This profile keeps local media-load checks out
of the user's normal PhoTag database and settings.

## Current runtime dependency order

Database → FileOperations → DragDrop → Gallery; Database → Preview → Gallery; Database → MediaPlayback → Gallery; Database → Thumbnails → Indexing → Watcher; Navigation depends on Database/Indexing and optionally Watcher; Settings reads Updater and Session and optionally reads Navigation; Tagging depends on Database; History starts after Database, FileOperations, and Tagging. Session and Updater start before Settings; AI starts after Settings when Gallery, MCP, all plugins, or the explicit `ai` development plugin is enabled. Shell starts Gallery after all required providers and the optional Settings/Tagging/History providers. Preview consumes AI suggestions through Cordis. In `--mcp` mode it also starts all required feature providers, publishes the Shell-owned `CaptureWindowHandle`, and starts `photag.mcp` after Gallery, Navigation, Tagging, Preview, MediaPlayback, and FileOperations. After GPUI exits, Shell consumes any queued updater install before disposing Updater. On shutdown it stops MCP before Gallery, then Preview before History, AI, Settings, Tagging, and MediaPlayback; Navigation stops before Watcher, Indexing, and Thumbnails. Preserve service availability before dependent plugin startup and stop consumers before providers. Gallery opens images in Preview, routes videos to MediaPlayback, calls DragDrop from card drag gestures, and optionally consumes Settings, Tagging, and History services.

## Boundaries

Application startup registers `gpui_kit::assets::Assets` with `with_assets` before
`run/init/open_window`. This supplies SVGs used internally by native components
(for example DatePicker calendar and menu check/chevron icons); it does not replace
PhoTag's source MDL2 glyphs or change window layout or feature ownership.

The shell must not absorb scanning, SQL, filtering, tagging, or file operations. It publishes the actual GPUI HWND through `photag-plugin-mcp::CaptureWindowHandle`; the MCP adapter captures only this PhoTag window on Windows. Stop plugin fibers before dropping shared runtime resources so Cordis cleanup can remove published services and UI contributions. Source-backed behavior is mapped in `docs/source-feature-audit.md`; Windows runtime and visual parity still require native validation.

## Source window bands and localization (2026-10-13)

The host composes `command.search`, `command.size`, `command.filters`,
`workspace.sidebar`, `gallery.content`, `workspace.status`, `command.tools`,
and `workspace.modal`. Gallery owns all filter/selection controls; Shell only
places the full-width command/status bands and single elevated Gallery card.
`settings.language` is mounted by Settings, not as a workspace panel. The i18n
plugin starts before views; translation revisions refresh native windows.
Reference geometry: source main.rs:1007-1367. Live MCP screenshots confirm
these regions render; detailed control, scan-status and hover parity remain
incomplete. See the current validation record.

## v2 paged-panel splitters (2026-10-14)

Shell owns both widths, visible8pt splitter hit regions and24pt grips; feature pages remain Navigation/Gallery-owned. Sidebar defaults264pt, bounds200–400; Inspector defaults300pt, bounds240–480. Drag uses pointer delta from the initial width, root move/up/out-up handling keeps it active across neighboring views, and release saves SessionState.sidebar_width/inspector_width. Resizing reserves320pt center space. MCP width requests use the same setter/save path; shell publishes shell.sidebar-width/shell.inspector-width. CaptureWindowHandle pointer mailbox is drained via Window.defer after the render lease, dispatching native GPUI PlatformInput within this child window (no OS input). Live MCP down/drag/up changed264→304 and300→340; restart restored both. Settings entry sits under the48pt Navigation rail. Source/schema/backend unaffected. See docs/redesign-validation.md latestv2 evidence.

## Grouping composition and Tools scrolling (2026-10-14)

Shell starts `photag-plugin-grouping::GroupingPlugin` whenever Gallery is enabled and disposes it after Gallery; grouping algorithms/state remain feature-owned. Gallery now requires that public service. Tools overlay uses an absolute outer container (300pt wide, bounded available height up to600pt) and an inner GPUI Kit scroll wrapper: the wrapper does not preserve absolute outer positioning, so applying it directly to the positioned panel previously disrupted the shell layout. Existing tool command/view lifecycle registration is unchanged. Isolated same-process MCP verified1700×1100 Tools layout and actual grouping-choice clicks; see docs/validation-ui/2026-10-14-grouping/README.md.

## Directory-browser composition (2026-10-14)

Enabling Gallery also enables Navigation and its Indexing/Watcher dependencies, so ungrouped folder cards route through their public services. Product listing/query/rendering stays in feature crates. The MCP pointer dispatcher recognizes double_down/up with click_count2 alongside the original single/drag/right inputs; it remains a deferred same-process GPUI event. No OS input injection or separately launched debug app.

## Runtime diagnostics (2026-10-14)

Shell now composes the debug plugin before other feature startup, disposes it last, and hosts lifecycle-owned workspace.bottom/workspace.debug-status slots. Diagnostic policy, storage and UI remain in photag-plugin-debug. See its PORTS.md.

## Shared refresh and passive input diagnostics (2026-10-15)

Shell lifecycle/surface/locale polling uses UI Bridge refresh::poll; locale changes still refresh component windows. A transparent canvas registers capture-phase GPUI move/down/key observers during paint solely to update MCP's bounded in-memory input counters. It does not intercept input, notify or log.


## Address interaction verification — 2026-10-16

The title bar continues to host Navigation's command.breadcrumb contribution; address/history behavior remains in the Navigation feature. Shell's same-process MCP dispatcher additionally consumes CaptureWindowHandle::take_key and dispatches parsed GPUI Keystroke as PlatformInput::KeyDown after the render lease. poll_panel_requests only wakes for an actually pending key/pointer. No keyboard input is injected into the OS or other applications.
