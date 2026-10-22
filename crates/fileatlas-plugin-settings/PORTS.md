# Native GPUI theme preferences — current 2026-10-19

Settings retains `color_mode` and adds `theme_file: Option<PathBuf>` (serde default None).
None selects GPUI's default theme pair. Startup/action persistence reads the selected native
ThemeSet JSON, queues it through UI Bridge, and falls back to native defaults on missing or
invalid files while preserving the original data. File reads do not run in render.
Old accent_name is still serialized for compatibility but no longer drives a private palette.
The Appearance page selects framework defaults and the active generated theme; the Material
plugin registers its own settings page for seed/style/contrast generation and activation.
Existing rail mode action persists through Settings; all framework/custom views update from
the native theme gateway. Older manual-theme notes below are historical implementation only.

# Settings plugin ports / 设置面板对接

## QuickLook replacement — 2026-10-16

The embedded file viewers and their Toolbar settings section are removed.
Settings now starts on Appearance and offers Appearance, Shortcuts, Tags, AI,
Updates and Software. Space and Enter always call QuickLook; the legacy configurable open
key is still accepted. Old toolbar/filmstrip JSON fields remain as inert
compatibility data, so existing settings do not lose persisted values. They
have no active viewer controls or handlers. Later historical descriptions of
the Toolbar tab are superseded by this section.

Status: the source-compatible settings model and JSON persistence are active. This plugin contributes a Cordis-owned floating Settings window with six source sections, a toggle command, close button, Escape handling, and draggable title bar. Controls write immediately through `SettingsService::update`, so UI state and `%LOCALAPPDATA%/FileAtlas/settings.json` stay synchronized. Accent selection updates the shared UI bridge theme state at startup and after edits. Windows visual and interaction validation remains pending.

## Cordis and UI lifecycle

- Plugin: `fileatlas.settings`; service: `fileatlas/settings` (`SettingsService`).
- Requires the host `fileatlas/ui-bridge` service. It publishes the settings service, contributes the `workspace.modal` Settings view and `settings/toggle` command/Window menu entry, and registers them under the same plugin owner.
- Cordis effect cleanup removes the UI contribution metadata, command listener, and view factory when the plugin stops. The service and `UiSurfaceState` are held by their host lifetime.
- The `settings.language` extension slot is reconciled from the live UiBridge catalog by contribution ID and generation while Settings renders; unregister drops the old `AnyView`, and re-register constructs a fresh one.
- The Shell mounts `workspace.modal` above application content. The Settings surface starts closed and centers on first open, with a 680–980 px width based on the viewport. Its title bar can be dragged; Close and Escape dismiss it. While open-key capture is active, Escape cancels capture without closing Settings. The shared bridge state lets the host command open the window and the panel close itself.

## Migrated settings controls

The panel has Appearance, Shortcuts, Tags, AI, Updates, Software and Plugins sections. Direct controls cover accent selection, date dividers, sidebar exclusion modifier, search scopes, tag search match and persistence, tag merge child behavior, AI automatic analysis, mandatory group editing, updater auto-check and open-key/tag shortcuts. Program Settings → Software is a read-only detector for QuickLook and Everything; the Screenshot Tool plugin page reports Shotoshot separately.

## Plugin settings and runtime controls — 2026-10-18

The Settings rail is grouped into Program settings and Plugin settings. Plugin overview cards are lifecycle-aware for `fileatlas.launcher`, `fileatlas.clipboard`, and `fileatlas.screenshot`; their independent pages expose launch presentation settings. `Settings.plugin_enabled` persists enabled state and defaults missing entries to enabled. `PluginRuntimeRegistry` holds Cordis `FiberHandle`s and shell-provided spawn recipes; disabling awaits `FiberHandle::dispose`, and enabling spawns a fresh prepared plugin through the host `CordisContext`. This registry is limited to those three standalone workbench tools. They depend on core Database/Settings services that are not user-toggleable here; no general dependency-cascade manager is introduced.

`Settings.plugin_launch_modes` is keyed by Cordis owner ID and defaults to `WorkspaceAndPanel`. Launcher/Clipboard support `PanelOnly`; Screenshot also supports `Direct`, which invokes Shotoshot capture without opening the FileAtlas panel. `PanelCatalog::open_presented(id, true)` now creates an independent native window through the host, with no main-window restore or workspace suppression. The old global `panel.presentation` rendering path is removed. The setting changes apply on the next plugin open. Runtime evidence for the corrected native presentation is recorded in docs/validation-ui/2026-10-18-native-panels/.

The Shortcuts page also discovers lifecycle-owned command keybindings from
`UiCatalog.commands()`. Each plugin registers a stable keybinding contribution
that points to its command and declares `Application` or `System` scope. The
page shows owner and scope, captures a replacement chord, resets to the plugin
default, or stores an empty override to disable the binding. Overrides are
persisted in `Settings.keybindings` by contribution ID; missing entries use
the plugin default. The host dispatches application-scope overrides through
`UiCommandInvoked`. The shortcut service applies system-scope overrides on its
GPUI polling thread and suspends global registrations during key capture.

## External software check — 2026-10-17

The Program Settings → Software tab detects QuickLook and Everything from registered uninstall and
App Paths entries plus common executable locations. It reads registry values
and checks file existence only; it never launches either application or invokes
an installer. Refresh requests a new background `ReadModel` snapshot. Results
include installation state, detected version/path, project/download URLs and
the official GitHub URL where one exists. Everything's GitHub URL points to
voidtools/ES (the official command-line utility), while Everything itself is
downloaded from voidtools.com. The current tab only reports Everything's
presence; it does not query Everything or integrate its search.

## Remaining UI ports

- Appearance: saved accent selection is applied to the shared `fileatlas-ui-bridge::theme` accent state on plugin startup and every settings edit. The `System` option follows the Windows accent color, and feature views using shared palette tokens inherit that selected accent. FileManager, navigation, settings, preview, metadata-export, playback, and Shell consume shared palette tokens; original egui swatch styling, font resolution, and exact visual rendering still require Windows validation.
- Shortcuts: `open_key` has the source-style capture interaction: click Change key, press a supported single key to persist it, or press Escape to cancel. FileManager reads the persisted binding when focused and opens the selected item; if the Settings provider is unavailable, Enter remains the fallback. F1–F9 use the source choices: Settings reads a fresh `NavigationService::snapshot()` when rendering the Shortcuts tab, recursively flattens tags, and labels each option with its icon (falling back to `🏷`) plus `full_path` with `>` spaced as ` > `. Choosing `None` clears only that key; choosing a tag replaces that key's assignment and saves immediately. Settings holds a weak navigation-service reference so stopping navigation releases its service normally. The settings plugin has a compile-time dependency on navigation and starts after it in the shell. With navigation stopped, existing assignments remain visible as tag IDs and editing is unavailable. FileManager F1–F9 toggle the assigned tag for selected media; Preview F1–F9 toggle it for the current item. Both paths use `TaggingService`, record tag changes through `HistoryService` when available, and publish `MediaIndexChanged`. The Preview surface also has a searchable single-item tag picker. Number-row 0–5 rates the current Preview item and records `UndoEntry::RatingChanged` when its old rating was available; FileManager number-row shortcuts rate the selected media.
- Tag sidebar preferences: after publishing `SettingsService`, startup publishes the loaded exclude modifier, tag-search match mode, search-persistence flag, and merge-child policy as `TagInteractionPreferencesChanged`. Settings emits the updated value after any of those settings changes, with enum values mapped through `fileatlas-contracts` to avoid a dependency cycle. Navigation consumes the event and retains the latest preference snapshot; tag merge starts with the saved reparent/delete policy.
- AI mandatory groups support add/remove, inline rename, selection of existing navigation tags, and per-tag removal. The list is sourced from a fresh `NavigationService::snapshot()`; when navigation is stopped, groups can still be renamed and tags removed, while adding tags is unavailable. Preview reads the current `SettingsService::snapshot().ai` for each analysis request, including both mandatory groups and `auto_analyze`.
- Updates: `SettingsPlugin` reads the shell's `UpdaterService` and `SessionService`. When the settings view is created and auto-check is enabled it checks GitHub in the background. A newly found release opens the source-style floating offer with release notes, Update now, Not now, and GitHub actions; Not now suppresses future offers for this app session, while Settings → Updates can reopen it. The tab supports manual check, release notes, byte progress, checksum-verified staging, a two-step install confirmation, visible failures, and a durable install error log. Before installation it rewrites the latest persisted session snapshot, schedules the staged executable, and quits GPUI. After the app loop returns the shell replaces the executable and relaunches. Live Windows restart behavior remains unverified.
- `search_scopes` defaults all three source fields to enabled, seeds the FileManager's initial query scopes, and is updated by both the Settings panel and FileManager scope controls. Updates change only that property in the shared snapshot and persist through the source-compatible settings JSON.
- `toolbar_pos` exposes a lock toggle, 10 px directional offset controls, and reset to the default anchor. The Preview drag handle applies pixel offsets to the center-bottom anchor and persists them through `SettingsService` on release; save failures are shown in the Preview surface.

## Persistence/debugging

- `snapshot()` returns a settings clone; `update` persists each interaction and reports errors in-panel. Missing/invalid JSON loads original FileAtlas defaults.
- Compare the JSON schema/defaults against pinned `vendor/photag-source/src/settings.rs`. For save errors check the LocalAppData path and permissions. UI wiring errors usually indicate the bridge service or slot host was not initialized before this plugin.

## Theme modes and shared visual tokens

`Settings.color_mode` persists `dark` / `light` in the existing
source-compatible settings JSON (missing field defaults to dark). The Appearance
page edits this field through `SettingsService`; successful writes publish it to
`fileatlas-ui-bridge::theme::ColorMode` and refresh Kit controls and semantic-color
consumers. `settings/rail-entry` is a Cordis-owned `workspace.sidebar-footer`
view containing the theme toggle directly above Settings. A failed write leaves
the prior mode active and logs the persistence error.

The floating Settings panel uses the shared backdrop-aware base and modal surface helpers, primary/danger text, and hairline tokens. Layout spacing and typography still use GPUI Kit presets; Windows visual parity is unverified.

## Code map

- `src/view.rs` owns Settings view state, preference edits, key capture, modal lifecycle, and grouped Program/Plugin navigation.
- `src/view/tabs.rs` renders Appearance, Shortcuts, Tags and AI; `src/view/software.rs` owns the external-software snapshot and Software page; `src/view/ai_groups.rs` owns mandatory-group editing and `tag_hotkeys.rs` owns F1–F9 assignment menus. `src/view/updates_page.rs` renders release/check/download/install states; `updates.rs` owns update lifecycle and the floating offer.
- `src/settings.rs` owns the source-compatible preference schema/defaults, plugin presentation/enabled maps and JSON service persistence; `src/runtime.rs` coordinates optional tool plugin Fiber stop/spawn through Cordis APIs.

## Localization and live Settings validation (2026-10-13)

Successful preference edits call UI Bridge `set_accent_name` followed by
`apply_accent_components(&mut gpui_kit::App)` so self-painted highlights and
GPUI Kit controls share the runtime accent. The existing source preset resolution,
settings persistence, control placement and event payloads are preserved.

Settings mounts lifecycle-owned `settings.language` view contributions in
Appearance. The i18n provider owns the language preference and controls.
`UiSurfaceState` settings.tab accepts appearance/shortcuts/tags/ai/updates/software;
the existing GPUI poller observes it as well as open/close requests for MCP tests.
Visible labels and input placeholders update through UI Bridge translation.

The redesign uses six vertically stacked navigation entries beside the active
page, a bounded scrolling content area, and a footer that says changes save
automatically. All existing page controls remain in the page renderer.
Appearance keeps the seven source accent choices plus System; Shortcuts retains
key capture and F1–F9 tag assignments; Tags, AI, and Updates retain their
source-backed settings. Software runs the read-only external-software check.
The Updates page separates current version, startup-check preference, live check
status, available release notes/download verification, and the explicit staged
restart/install action. Automatic checks continue to populate status quietly;
manual checks may open the existing floating update offer. This layout has not
yet received the parent task's live MCP screenshot review.

## Quiet startup update checks (2026-10-13)

Per user direction, automatic startup checks still honor the persisted
`Settings.auto_check_updates`, but never open the update offer. Check results
remain available in Settings. `begin_update_check(manual)` tracks request
origin; explicit manual checks may open the existing offer, and download,
verification and explicit install behavior are unchanged. The existing
`settings.check-updates` surface request invokes that same manual action on
the GPUI poller. Read-only diagnostic surfaces publish update-checking,
update-available and update-offer state for the plugin MCP tools. Live MCP
verification found a release without opening a startup offer, then opened
that offer after a manual request. No update was downloaded or installed.

## 2026-10-14 current implementation and MCP evidence

Reference03 six Settings pages and07 update offer were rendered via FileAtlas MCP. Native checkbox/accent/header/navigation and compact rows use shared theme. F1–F9 triggers have bounded360pt width. Update offer now uses viewport-bounded solid two-column notes/status with fixedfooter; actual download/staged/verify/error handlers and download close restrictions remain. Startup available/offerclosed and manualcheck offeropen were observed; download/install untested.

## Shared floating frame (2026-10-14)

The existing settings modal content now uses UI Bridge `panels::mount`, ID settings, default900×680/minimum680×360 logical pixels. Header dragging, eight resize handles, maximize/restore and close are shared. Tab rail, automatic settings persistence and native update behavior stay in Settings. Body width/height use actual GPUI content bounds. The frame and host backdrop occlude underlying mouse targets; Settings key capture retains its original surface flags. Runtime pointer tab clicks, blank-area/backdrop isolation and frame drag/resize are recorded in the current validation report.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by fileatlas.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See fileatlas-plugin-debug/PORTS.md for capture scope and limits.

## Idle read models (2026-10-15)

Shortcuts and AI group choices consume one shared background tag-choice model,
loaded when a relevant Settings page is open, keyed by Navigation availability
and `tag_revision()`. Reopening reloads; tag mutations/events invalidate it
while open. Render never calls Navigation::snapshot. Missing/error choices are
cached as well. `settings.tag-choices` is a read-only published UI snapshot:
state loading/ready/failed, revision and choices[{id,name,label}] or error,
exposed by MCP `get_ui_state.settings_tag_choices`. Empty worker polls do not
redraw. Update monitoring now notifies only for byte progress or task results;
disconnected check/download channels become errors and end monitoring rather
than leaving an endless 120 ms repaint loop. No update is downloaded or
installed by idle validation.

## Shared refresh ownership (2026-10-15)

Settings surface and updater timers use UI Bridge refresh::poll. Shared gates track visibility/tab and update progress; tag choices use ReadModel. Terminal worker errors notify once and stop; empty channels do not paint.

## Sidebar folders block (2026-10-15)

`Settings.sidebar_folders` (`fileatlas_contracts::SidebarFoldersConfig`:
`quick_access`, `favorite_groups`, `show_network_drives`) is a new serde-default
JSON block; older documents deserialize unchanged. The plugin observes
`SidebarFoldersChanged` and accepts `SidebarFoldersUpdate { revision, config }` and writes the received document through
`SettingsService::update`. At startup it pushes the persisted block into
`NavigationService::sidebar_folders().restore(...)` when the Navigation service
is already published (shell starts Navigation before Settings). No SQLite
change; navigation never reads settings directly (cycle-safe event flow).

## Sidebar persistence review — 2026-10-16

`SettingsService::replace/update` serialize disk publication under the state write lock and publish the new memory snapshot only after success. The SidebarFoldersChanged observer rejects stale revisions; a failed revision can retry. It propagates save errors to Navigation for visible retry feedback. Normal JSON fields and paths are unchanged. Tests cover concurrent edits, stale delivery and failure/retry; this is not a crash-safe filesystem transaction guarantee.


## Region separation token (2026-10-16)

`Settings.region_separation` (`"border"` default, `"shadow"`) joins the existing
source-compatible JSON schema; older files keep working through `serde(default)`.
`SettingsService::persist` is the single write funnel and now republishes the
paint-time tokens it owns (`set_accent_name`, `set_region_depth_name`) after a
successful disk write, so the Settings view, `control_ui {action:"region_separation"}`
and any future writer apply the same state; a failed write leaves both disk and
theme untouched. The Settings view keeps `apply_accent_components(cx)` (it needs
`&mut App`) and adds `cx.refresh_windows()`, the same full-window repaint the
language switch uses, because region edges are painted by every region.
Startup still applies the loaded values in `SettingsPlugin::apply`.

## External software check (2026-10-17)

The `software` settings tab uses a revision-keyed background `ReadModel` to read
QuickLook/Everything installation records, executable paths and versions. It
publishes `settings.software-check` with `state`, `revision`, and `software`
entries (`name`, `purpose`, `state`, `version`, `path`, `project_url`,
`download_url`, `github_url`, `github_note`). Opening the tab and explicit
Refresh request a new read; settled renders do not repeat filesystem IO.
Links have software-specific GPUI IDs. Link actions open project/download pages;
this page does not install, execute or upgrade external software. Everything
requires 1.5 for the separately owned search adapter. The ES GitHub link is
identified as the command-line tool, not the Everything application installer.
See the Everything-search validation evidence for the same-process software
page and missing/incompatible/ready inline FileManager states.

Search location (2026-10-17): `Settings.search_entire_computer: bool` defaults to
false when absent. FileManager saves it through `SettingsService::update` only on
explicit location changes and restores it at view construction. This changes no
Everything settings and does not replace the persisted navigation path.

## Shortcut layout and capture states (2026-10-18)

The active visual reference and layout/state checklist are
`docs/reference-ui/concepts/shortcuts-settings-2026-10-18.{png,md}`. The
`view/shortcuts.rs` page projects all UI Bridge command keybindings into aligned
horizontal rows; tag selectors fill the remaining row width. Capture replaces
the row controls with a blue recording field. Validation errors stay inline and
keep capture active for retry; no invalid/conflicting value is persisted.
Escape, inline Cancel, switching tabs, closing the panel, and successful capture
release the shared capture state (allowing global shortcuts to resume).
`settings.shortcut-capture` and `settings.shortcut-error` publish the current
binding ID/error only on capture transitions; MCP reads these without UI IO.
The same-process UI scenario is `scripts/validate-shortcuts-layout.cjs`.
# Workbench theme (2026-10-18)

Missing `accent_name` now defaults to UI Bridge `DEFAULT_ACCENT_NAME` (Workbench
Blue). Explicit stored System/legacy values remain supported; no configuration
rewrite or schema migration. Appearance enumerates `ACCENT_PRESETS` plus System
rather than duplicating the preset list. Settings applies all native component
states and refreshes consuming windows after an edit. Shared headings use
`components::section_heading`; theme rules are in `docs/ui-theme.md`.
# Sidebar group compatibility (2026-10-18)

The shared sidebar JSON model now has serde-default `imported_library_paths`;
Settings persists this field through the existing revisioned sidebar update
handler. Old JSON remains readable; custom groups/aliases are retained. No new
file/path/store is introduced. Navigation owns the import policy and live plus
action, Settings still owns durable writes and failure handling.
# Minimal settings center and schema integration — 2026-10-18

The active references are the ten `settings-*-2026-10-18.png` concepts and
`docs/reference-ui/concepts/settings-complete-2026-10-18.md`. This supersedes old
boxed settings layouts. Shared floating geometry remains 1120x840/min 920x580;
navigation is 224 logical pixels, footer fixed, content scrolls independently.

Appearance, tag match/scopes/merge, standard toggles and built-in plugin launch
presentation use UI Bridge's declarative `settings_page` renderer. Shortcut
capture, AI groups, software detection and update state machines retain their
existing actions inside common sections. IDs and SettingsService persistence
remain source-compatible; no new data files or migration.

Lifecycle-owned `UiViewContribution.settings_page` providers are discovered from
the existing view catalog, generate plugin navigation automatically and are
reconciled by owner/ID/generation. Optional built-in runtime pages append provider
content; provider-only pages work without a runtime toggle registration. Stopping
an owner removes its settings contribution. ChangeGates observe catalog/runtime
changes without notifying on quiet polls. Shortcut capture stays active on plugin
pages and is canceled by tab/owner/visibility changes.


## MCP access setting — 2026-10-19

`fileatlas.mcp` owns the MCP settings-page contribution (`mcp/settings`) and edits
`Settings.mcp_access_level` through the existing `SettingsService`; Settings
does not duplicate MCP policy or storage. The segmented levels are `off`,
`standard` (UI control and ordinary additions/edits/deletions), and `full` (also
high-impact operations). MCP reads the current settings snapshot for every
mutating host/contributed call, so changes persist immediately and apply without
restarting the endpoint. Plugin tool metadata identifies mutation and
confirmation requirements; high-impact calls still require per-call
`confirm=true`. Missing levels default to Off. The legacy JSON boolean
`mcp_allow_mutations` remains accepted (`false` → Off, `true` → Full). Current
same-process Windows evidence is at
`docs/validation-ui/2026-10-19-mcp-access-stages/`; the earlier two-state record
at `docs/validation-ui/2026-10-19-mcp-settings-toggle/` is historical.


## 2026-10-23 WorkCrate defaults

Shared Settings JSON gains serde-default `workcrate: WorkCrateSettings { sort_by_name: bool, image_export_jpeg: bool }`. Missing legacy values preserve recently-created ordering and PNG defaults (both false). WorkCrate UI edits these through SettingsService.update; the settings plugin adds no WorkCrate behavior. Existing paths, failure behavior and other settings remain unchanged.

## Plugin data locations — 2026-10-23

Plugin settings pages now include a read-only “数据存储位置” section. Known data paths are rooted beside `SettingsService.path()` so isolated profiles resolve to their profile directory. Screenshot output uses its saved output directory, falling back to `<data-root>/Screenshots`. Shared media features point to `fileatlas.db`; WorkCrate and other entries point to their registered feature files/directories. Entries without a registered path say so rather than fabricate one. This is display-only and performs no filesystem reads or writes during rendering. Live same-process MCP/UI verification is pending; static crate compilation and governance checks pass.
