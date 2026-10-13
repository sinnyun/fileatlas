# Settings plugin ports / 设置面板对接

Status: the source-compatible settings model and JSON persistence are active. This plugin now contributes a GPUI Settings panel owned by Cordis. Controls write immediately through `SettingsService::update`, so UI state and the canonical `%LOCALAPPDATA%/PhoTag/settings.json` stay synchronized. Accent selection updates the shared UI bridge theme state at startup and after edits. The shell dependency graph compiles; plugin tests were deferred and local Windows UI verification is pending.

## Cordis and UI lifecycle

- Plugin: `photag.settings`; service: `photag/settings` (`SettingsService`).
- Requires the host `photag/ui-bridge` service. It publishes the settings service, contributes `workspace.settings` layout/slot/panel/widget entries and registers `settings/panel-view` under the same plugin owner.
- Cordis effect cleanup removes the UI contribution metadata and view factory when the plugin stops. The service is provided within the plugin generation.
- The current Shell mounts non-sidebar/non-gallery slots as auxiliary views. To make this panel a modal opened from a Settings command, the shell must add a visibility/command host contract; current rendering displays the contributed panel as an auxiliary surface.

## Migrated settings controls

The panel has Appearance, Toolbar, Shortcuts, Tags, AI and Updates sections. Direct toggles/options cover every `ToolbarSettings` boolean; accent preset name (System plus all seven pinned original names); date dividers; sidebar exclusion modifier; search scopes; tag search match and persistence; tag merge child behavior; AI automatic analysis; mandatory group add/remove; updater auto-check; filmstrip startup visibility; floating toolbar lock and stored offsets. Values are saved through the same serde model, including fields that are currently displayed read-only.

## Remaining UI ports

- Appearance: saved accent selection is applied to the shared `photag-ui-bridge::theme` accent state on plugin startup and every settings edit. Existing feature views still need to consume shared tokens, and original egui swatch styling has not been visually replicated.
- Shortcuts: `open_key` has the source-style capture interaction: click Change key, press a supported single key to persist it, or press Escape to cancel. Gallery reads the persisted binding when focused and opens the selected item; if the Settings provider is unavailable, Enter remains the fallback. F1–F9 use the source choices: Settings reads a fresh `NavigationService::snapshot()` when rendering the Shortcuts tab, recursively flattens tags, and labels each option with its icon (falling back to `🏷`) plus `full_path` with `>` spaced as ` > `. Choosing `None` clears only that key; choosing a tag replaces that key's assignment and saves immediately. Settings holds a weak navigation-service reference so stopping navigation releases its service normally. The settings plugin has a compile-time dependency on navigation and starts after it in the shell. With navigation stopped, existing assignments remain visible as tag IDs and editing is unavailable. Gallery F1–F9 dispatch toggles the assigned tag for selected media through `TaggingService`; tag changes are recorded in `HistoryService` when available. Preview-specific tag shortcuts and a general-purpose tag picker remain incomplete. Rating shortcuts are handled by Gallery; preview rating parity is incomplete.
- AI mandatory groups can be added/removed. Editing group names, picking/removing individual tags and wiring those groups into AI requests need the tag picker and AI provider contracts.
- Updates: the preference is editable. Live check, status, staged download, checksum verification and install controls need updater service integration.
- `toolbar_pos.offset_x/offset_y` remain displayed read-only; drag persistence belongs to the preview toolbar view.
- The original six-tab dialog dimensions, draggable modal behavior, ESC handling and exact visual styling are not represented by the current shell's auxiliary slot host.

## Persistence/debugging

- `snapshot()` returns a settings clone; `update` persists each interaction and reports errors in-panel. Missing/invalid JSON loads original PhoTag defaults.
- Compare the JSON schema/defaults against pinned `vendor/photag-source/src/settings.rs`. For save errors check the LocalAppData path and permissions. UI wiring errors usually indicate the bridge service or slot host was not initialized before this plugin.
