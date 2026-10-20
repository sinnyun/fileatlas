# Shell composition ports / Shell 对接

## Main toolbar view composition — 2026-10-19

The host mounts the active pane's `command.column.{i}.view-modes`, the
Columns-owned `command.columns` dropdown, then the active pane's
`command.column.{i}.view-options` in a fixed-size horizontal group beside the
scrollable filters. FileManager retains view behavior; Columns retains layout and
active-pane actions. The visible order is grid, list, layout, subfolders, zoom.

## QuickLook replacement — 2026-10-16

Shell starts `fileatlas.quicklook` before FileManager/MCP and stops it after both
consumers. The Preview and MediaPlayback crates, dependencies, startup and
shutdown paths are removed. Their historical dependency order below is
superseded: QuickLook → FileManager → MCP. AI suggestions are registered by FileManager
through its existing AI/Settings/Tagging/History dependencies. Generic floating
panels remain available for Settings and other feature contributions. Shell
has no file-viewing or installation implementation.

`fileatlas-shell` is the host composition root. It publishes `UiBridgeService`, starts Cordis plugins in dependency order, mounts GPUI view factories into host regions (including floating `workspace.modal` views), and dispatches command IDs as Cordis events. The Settings command is contributed by the Settings plugin; its modal open state is shared through `UiSurfaceState` and its floating layer is mounted above application content.

The same UI Bridge command catalog is exposed by MCP `command_list` and
`command_run`; Shell rendering and MCP dispatch therefore use lifecycle-owned
command IDs and policy metadata. Handlers retain action-time validation.

The host chrome and contribution fallback view use `fileatlas_ui_bridge::theme` surface and text tokens; the native base surface consumes source Mica/Acrylic alpha. GPUI Kit typography presets are still used by the shell; parity with source Segoe UI font loading and Windows font availability has not been validated.

The shell restores `SessionState.sidebar_width` when it is between 200 and 400 logical pixels (otherwise 264 px) and saves the width when the sidebar divider drag ends. The updater is started before Settings. When Settings confirms an update it persists the current session snapshot, queues the staged path, and quits GPUI; after the application loop returns, the shell installs and relaunches the staged build before disposing the updater service. This update shutdown path still needs interactive Windows validation.

The explicit `dev-test` profile starts `fileatlas-plugin-dev-test` after FileManager and
its providers. It redirects the database, Settings, Session and thumbnail cache
to `target/dev-test/` (or `FILEATLAS_DEV_TEST_DIR`); `FILEATLAS_DEV_TEST_DB` can
override only the database file. This profile keeps local media-load checks out
of the user's normal FileAtlas database and settings.

## Current runtime dependency order

Database → FileOperations → DragDrop → FileManager; Database → Preview → FileManager; Database → MediaPlayback → FileManager; Database → Thumbnails → Indexing → Watcher; Navigation depends on Database/Indexing and optionally Watcher; Settings reads Updater and Session and optionally reads Navigation; Tagging depends on Database; History starts after Database, FileOperations, and Tagging. Session and Updater start before Settings; AI starts after Settings when FileManager, MCP, all plugins, or the explicit `ai` development plugin is enabled. Shell starts FileManager after all required providers and the optional Settings/Tagging/History providers. Preview consumes AI suggestions through Cordis. In `--mcp` mode it also starts all required feature providers, publishes the Shell-owned `CaptureWindowHandle`, and starts `fileatlas.mcp` after FileManager, Navigation, Tagging, Preview, MediaPlayback, and FileOperations. After GPUI exits, Shell consumes any queued updater install before disposing Updater. On shutdown it stops MCP before FileManager, then Preview before History, AI, Settings, Tagging, and MediaPlayback; Navigation stops before Watcher, Indexing, and Thumbnails. Preserve service availability before dependent plugin startup and stop consumers before providers. FileManager opens images in Preview, routes videos to MediaPlayback, calls DragDrop from card drag gestures, and optionally consumes Settings, Tagging, and History services.

## Boundaries

Application startup registers `gpui_kit::assets::Assets` with `with_assets` before
`run/init/open_window`. This supplies SVGs used internally by native components
(for example DatePicker calendar and menu check/chevron icons); it does not replace
FileAtlas's source MDL2 glyphs or change window layout or feature ownership.

The shell must not absorb scanning, SQL, filtering, tagging, or file operations. It publishes the actual GPUI HWND through `fileatlas-plugin-mcp::CaptureWindowHandle`; the MCP adapter captures only this FileAtlas window on Windows. Stop plugin fibers before dropping shared runtime resources so Cordis cleanup can remove published services and UI contributions. Source-backed behavior is mapped in `docs/source-feature-audit.md`; Windows runtime and visual parity still require native validation.

## Source window bands and localization (2026-10-13)

The host composes `command.search`, `command.size`, `command.filters`,
`workspace.sidebar`, `file_manager.content`, `workspace.status`, `command.tools`,
and `workspace.modal`. FileManager owns all filter/selection controls; Shell only
places the full-width command/status bands and single elevated FileManager card.
`settings.language` is mounted by Settings, not as a workspace panel. The i18n
plugin starts before views; translation revisions refresh native windows.
Reference geometry: source main.rs:1007-1367. Live MCP screenshots confirm
these regions render; detailed control, scan-status and hover parity remain
incomplete. See the current validation record.

## v2 paged-panel splitters (2026-10-14)

Shell owns both widths, visible8pt splitter hit regions and24pt grips; feature pages remain Navigation/FileManager-owned. Sidebar defaults264pt, bounds200–400; Inspector defaults300pt, bounds240–480. Drag uses pointer delta from the initial width, root move/up/out-up handling keeps it active across neighboring views, and release saves SessionState.sidebar_width/inspector_width. Resizing reserves320pt center space. MCP width requests use the same setter/save path; shell publishes shell.sidebar-width/shell.inspector-width. CaptureWindowHandle pointer mailbox is drained via Window.defer after the render lease, dispatching native GPUI PlatformInput within this child window (no OS input). Live MCP down/drag/up changed264→304 and300→340; restart restored both. Settings entry sits under the48pt Navigation rail. Source/schema/backend unaffected. See docs/redesign-validation.md latestv2 evidence.

## Grouping composition and Tools scrolling (2026-10-14)

Shell starts `fileatlas-plugin-grouping::GroupingPlugin` whenever FileManager is enabled and disposes it after FileManager; grouping algorithms/state remain feature-owned. FileManager now requires that public service. Tools overlay uses an absolute outer container (300pt wide, bounded available height up to600pt) and an inner GPUI Kit scroll wrapper: the wrapper does not preserve absolute outer positioning, so applying it directly to the positioned panel previously disrupted the shell layout. Existing tool command/view lifecycle registration is unchanged. Isolated same-process MCP verified1700×1100 Tools layout and actual grouping-choice clicks; see docs/validation-ui/2026-10-14-grouping/README.md.

## Directory-browser composition (2026-10-14)

Enabling FileManager also enables Navigation and its Indexing/Watcher dependencies, so ungrouped folder cards route through their public services. Product listing/query/rendering stays in feature crates. The MCP pointer dispatcher recognizes double_down/up with click_count2 alongside the original single/drag/right inputs; it remains a deferred same-process GPUI event. No OS input injection or separately launched debug app.

## Runtime diagnostics (2026-10-14)

Shell now composes the debug plugin before other feature startup, disposes it last, and hosts lifecycle-owned workspace.bottom/workspace.debug-status slots. Diagnostic policy, storage and UI remain in fileatlas-plugin-debug. See its PORTS.md.

## Shared refresh and passive input diagnostics (2026-10-15)

Shell lifecycle/surface/locale polling uses UI Bridge refresh::poll; locale changes still refresh component windows. A transparent canvas registers capture-phase GPUI move/down/key observers during paint solely to update MCP's bounded in-memory input counters. It does not intercept input, notify or log.


## Address interaction verification — 2026-10-16

The title bar continues to host Navigation's command.breadcrumb contribution; address/history behavior remains in the Navigation feature. Shell's same-process MCP dispatcher additionally consumes CaptureWindowHandle::take_key and dispatches parsed GPUI Keystroke as PlatformInput::KeyDown after the render lease. poll_panel_requests only wakes for an actually pending key/pointer. No keyboard input is injected into the OS or other applications.

## Native tool-window composition — 2026-10-18

Foreground correction: new and reused native panels now share `activate_panel`,
deferred until their native window and Root exist. It shows/restores only that
HWND and invokes GPUI platform activation for foreground/focus handling. No
permanent topmost style or polling that steals focus. Main stays minimized.
Evidence: docs/validation-ui/2026-10-18-native-foreground/README.md.

`shell/native_panels.rs` hosts UI Bridge native panel factories in independent GPUI Kit
Root windows, using each definition title/default/minimum size and the shared theme.
System chrome owns moving/resizing/closing. Closing/Escape delegates to PanelCatalog;
Cordis owner cleanup removes the native record and the host closes its window on the next
shared poll. Main workspace rendering is never suppressed. Only an embedded panel launch
requests restoring the main HWND. Native reopen activates/restores only its own HWND.
MCP capture/input can select `main` or a live native panel ID; both stay in this child process.
# Shell control ownership (2026-10-19)

Shell composes plugin views by named slot. The Settings plugin owns
`workspace.sidebar-footer`; `fileatlas-plugin-shell-controls` owns sidebar,
inspector, workspace-manager and Tools action views. Shell retains framing,
branding, region layout, status composition, resize surfaces and generic rendering
of plugin command contributions. New application actions belong to lifecycle-owned
view contributions rather than hardcoded Shell buttons.


## Native themes / Material generator (2026-10-19)

Shell starts the compile-time MaterialThemePlugin after Settings and disposes it before
Settings. The plugin owns its service/settings/MCP contributions and versioned generation
files. Shell initializes UI Bridge's native-theme listener at GPUI startup; theme JSON is
loaded by Settings on startup/actions, never by Shell render. Custom host surfaces consume
the same native ThemeColor roles as plugins. The former component override palette is gone.
