# Program launcher plugin / 程序启动台插件

Status: native icon-grid launchpad integrated. Same-process discovery, single selection, right-click menu, harmless-fixture double-click launch, inspector edits, search, sorting and opening the asynchronous native file picker have runtime evidence. Selecting/canceling a real file and installed-app/elevated uninstall flows remain unverified.

## Cordis identity and startup

- Plugin: `photag.launcher`; service: `photag/launcher` (`LauncherService`).
- Requires shared `photag/database` and `photag/ui-bridge` services.
- Also reads `photag/settings` for `Settings.plugin_launch_modes["photag.launcher"]`. Settings can disable this standalone tool by disposing its Cordis Fiber and re-enable it by spawning a fresh generation.
- Shell starts the database and Navigation before this plugin. Navigation renders the lifecycle-owned `navigation.launcher-shortcuts` view contribution as the Apps button in its left rail.
- Registers `program-launcher/panel`, the `program-launcher/open` command, and a sidebar shortcut. Cordis stop removes the command, panel definition/instances, and view contribution.

## Data and operations

- Storage is `%LOCALAPPDATA%\\PhoTag\\launcher\\launcher.json`; icon assets are under `launcher/icons/`.
  First start imports the shared SQLite projection and copies legacy `program-icons/` assets. Thereafter
  JSON is authoritative; legacy rows/files remain untouched as a recovery source.

The launcher uses a versioned JSON snapshot after one-time import. It does not directly access SQLite;
persisted edits and discovery merges are saved before the in-memory snapshot is refreshed.

- On first start only, `DatabaseService::launcher_snapshot()` supplies the legacy projection for JSON import; after import the service never reads launcher tables. Passive rendering reads the in-memory JSON snapshot.
- `refresh_discovery()` scans supported Windows sources on a worker, then merges discovered entries. It preserves custom names, manual category assignments and hidden status.
- New discoveries receive one of the Chinese categories using normalized program name/path heuristics. Subsequent scans can update an automatically assigned category (`custom_category=0`); user-moved entries are retained.
- CRUD operations cover categories, custom display names, category assignment, hidden status, removal, and manual executable/shortcut addition.
- Each program can be unpinned, pinned at its category top, or pinned at the top of All Programs. Pin choices persist in JSON; several programs may share either scope. The star badge distinguishes global (filled) and category (outline) pins. Pin priority precedes the selected name/date/size sort.
- `launch(id)` starts the selected executable/script or opens its shortcut via Explorer. `uninstall(id)` uses a registered uninstall command and requests explicit confirmation first.

## Inputs, outputs, and events

- UI command `program-launcher/open` opens/focuses the existing generic floating panel.
- `PanelOnly` hides the main workspace under the shared panel; the default keeps the PhoTag workspace visible.
- Sidebar view slot: `navigation.launcher-shortcuts`.
- Panel: searchable desktop-style icon tiles in the middle, categories/counts on the left and an information editor on the right. Right-click menus retain launch, rename, move, hide/show, uninstall and remove.
- Visible UI strings use the shared i18n catalog. Category icons are glyphs from the existing text/icon rendering path.
- No new Cordis events are emitted. Panel and contribution lifetimes use shared UI Bridge contracts.

## Limits and diagnostics

- Discovery covers per-user and machine uninstall registry entries plus per-user and common Start Menu `.lnk` files. It does not scan all executables on every disk; entries without a usable executable/shortcut are omitted.
- Name matching preserves Unicode letters and digits. `DisplayIcon` is accepted as a launch target only for supported executable/script/shortcut paths; icon-only `.ico` files are not programs.
- Categories are heuristic and may need manual correction. Automatic rescans preserve manual category changes.
- Uninstall commands come from registry command lines. Review the confirmation and Windows behavior; elevated uninstallers may launch separately and complete asynchronously.
- Inspect `LauncherService` JSON persistence errors and `photag-program-discovery` tracing for failures. Live checks must use same-process PhoTag MCP.

## Initial same-process verification — 2026-10-17

- `cargo +1.97.1 build --locked --offline -p photag-shell -j 2` passed (existing LNK4098 warning).
- Isolated profile MCP run: `node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-launcher-v3 --plugins dev-test,launcher --interactive --allow-mutations`.
- `get_launcher_state`: scanning finished, 398 entries, 10 categories, no `.ico` launch targets. Counts and UI evidence are in `docs/validation-ui/2026-10-17-program-launcher/README.md`; screenshot: `sidebar-launchpad.png`.
- No application was launched and no uninstall command was executed. Manual add/rename/move/hide/remove and native uninstall confirmation remain unverified.

## Desktop icon layout and metadata — 2026-10-17

User reference: desktop shortcuts with icon/name only. Current concept is `docs/reference-ui/concepts/program-launcher-2026-10-17-v3.png` and its companion notes; it supersedes the wide action-card and v2 grid concepts.

- Tiles show 64px native Windows shell icons and a two-line display name. Single click selects; double click calls `launch(id)`; right click opens the existing GPUI Kit PopupMenu with launch/rename/pin/move/hide/uninstall/remove. Rename focuses the right-hand editor; move and pin use submenus.
- Pin management is available from the tile context menu and the selected-program information area. The three states are mutually exclusive per program; multiple programs may be pinned. Global pins sort first in All Programs and every category view; category pins sort above unpinned entries in All Programs and within their own category. The current sort field orders programs within each pin level. A filled star marks global pins and an outline star marks category pins.
- Right-hand information area reserves its width so selection cannot move the double-click target. Name, category and hidden status are a draft; `LauncherService::update_entry(id, name, category_id, hidden)` validates and saves them in the JSON snapshot. Close clears the selection.
- `LauncherEntry` / database entry projection now include `installed_on: Option<String>` (ISO calendar date) and `size_bytes: Option<i64>`. Registry `InstallDate` is parsed as YYYYMMDD; positive DWORD `EstimatedSize` is converted from KiB to bytes. Missing/invalid values remain unknown; filesystem modification times and executable file length are not installation information.
- Installation metadata is retained in the JSON snapshot. Former launcher tables remain as migration/recovery data and are not updated by the runtime.
- Name, installation date and registered size have ascending/descending order. Missing metadata sorts last in both directions; ties use normalized name then ID.
- `LauncherService::wake()` returns the shared refresh Wake. Views use lifecycle-owned `refresh::listen`; no idle timer is needed. Icon reads use shared `ReadModel<Vec<PathBuf>, IconSet>` and a single background COM worker. Successful shell icons are cached as PNGs in `launcher/icons/`; identity includes normalized Windows path, file size and modification time. The service also retains decoded images for the plugin lifetime. Canonical `\\?\` and mixed-separator paths are normalized before Shell extraction so manually added executables and shortcuts resolve their native icons. Executables that `IShellItemImageFactory` cannot render fall back to `ExtractIconExW`/`DrawIconEx`. Failed icons are not persisted, allowing a later retry; the UI uses its category glyph for the remaining failures. `get_launcher_state.ui.icon_cache` reports disk hits, shell extractions, failures, manual-program status and failed program paths. GDI bitmaps/DCs and COM apartments are released in the worker.
- `get_launcher_state` exposes metadata and a read-only `ui` projection (selection/draft, sort direction, visible IDs, icon counts and render count). `add_launcher_program` uses the public service and requires mutation permission; it does not launch anything. Runtime validation is `scripts/validate-program-launcher.cjs` through the same-process MCP client.
- The Add Program button uses `rfd::AsyncFileDialog` in a tracked GPUI task. Only one picker can be active at a time; the button is disabled until it is dismissed. On success it calls `LauncherService::add_manual`, returns to All Programs, clears hidden/search filters, and selects the added or already-discovered path; cancel leaves state unchanged. This prevents newly added programs from being hidden by a previously selected category or search. Do not replace this with synchronous `FileDialog::pick_file` on the GPUI thread, which can freeze or terminate the host while opening the Windows picker.


Current validation: `scripts/validate-program-launcher.cjs` verifies the Chinese input fields, single-click selection without launch, a no-window fixture executable launched on double click, right-click menus, name/category/hidden saves, six sort directions, scan real-change and settled quiet state. The plugin caches native icons across panel reopen using its feature-owned in-memory result cache; all extraction stays behind shared ReadModel. Evidence and exact counts: `docs/validation-ui/2026-10-17-program-launcher/README.md`.

2026-10-18 icon revalidation: the manually added fixture resolves its Shell icon after Windows path normalization. Native resource extraction is used when `IShellItemImageFactory` cannot render an executable. A cold same-process run extracted and persisted icons for all 399 programs; after restart, the saved rows appeared before discovery refresh and all 399 PNGs loaded from disk in 1.8s with zero Shell re-extractions or failures. Details and screenshot are recorded in the validation README.
# Shared theme (2026-10-18)

Launcher cards and category/hidden navigation reuse UI Bridge `components`.
Selection/hover/fallback icons consume `theme` and follow the persisted accent;
native program icons remain unmodified content. Geometry, actions and lifecycle
stay feature-owned within the existing generic panel. See `docs/ui-theme.md`.

PanelOnly presentation uses `PanelCatalog::open_presented(PANEL_ID,true)` for an independent native tool window; WorkspaceAndPanel keeps the embedded public panel. Data/services are unchanged.
# MCP contributions

The launcher contributes namespaced tools through the host-published
`photag_contracts::mcp::McpToolRegistry`: `launcher.snapshot`, `launcher.add_program`,
`launcher.set_pin`, `launcher.create_category`, `launcher.rename_category`,
`launcher.delete_category`, `launcher.update_program`, `launcher.refresh`,
`launcher.launch`, `launcher.remove_program`, and `launcher.uninstall_program`.
The plugin owns their schemas and handlers and routes them through
`LauncherService`. Program launch, category deletion, list removal, and software
uninstall require explicit `confirm:true`; other writes are mutation-gated.
Cordis removes the contributions when `photag.launcher` stops. Former
unnamespaced MCP tool names are retired; clients should use these namespaced
routes.
