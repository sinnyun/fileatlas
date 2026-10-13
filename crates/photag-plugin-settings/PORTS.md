# Settings plugin ports / 设置面板对接

Status: the source-compatible settings model and JSON persistence are active. This plugin contributes a Cordis-owned floating Settings window with six source sections, a toggle command, close button, Escape handling, and draggable title bar. Controls write immediately through `SettingsService::update`, so UI state and `%LOCALAPPDATA%/PhoTag/settings.json` stay synchronized. Accent selection updates the shared UI bridge theme state at startup and after edits. Windows visual and interaction validation remains pending.

## Cordis and UI lifecycle

- Plugin: `photag.settings`; service: `photag/settings` (`SettingsService`).
- Requires the host `photag/ui-bridge` service. It publishes the settings service, contributes the `workspace.modal` Settings view and `settings/toggle` command/Window menu entry, and registers them under the same plugin owner.
- Cordis effect cleanup removes the UI contribution metadata, command listener, and view factory when the plugin stops. The service and `UiSurfaceState` are held by their host lifetime.
- The Shell mounts `workspace.modal` above application content. The 360 px Settings surface starts closed and centers on first open. Its title bar can be dragged; Close and Escape dismiss it. While open-key capture is active, Escape cancels capture without closing Settings. The shared bridge state lets the host command open the window and the panel close itself.

## Migrated settings controls

The panel has Appearance, Toolbar, Shortcuts, Tags, AI and Updates sections. Direct toggles/options cover every `ToolbarSettings` boolean; accent preset name (System plus all seven pinned original names); date dividers; sidebar exclusion modifier; search scopes; tag search match and persistence; tag merge child behavior; AI automatic analysis; mandatory group editing; updater auto-check; filmstrip startup visibility; floating toolbar lock and position offsets. Every control persists through the same serde model.

## Remaining UI ports

- Appearance: saved accent selection is applied to the shared `photag-ui-bridge::theme` accent state on plugin startup and every settings edit. The `System` option follows the Windows accent color, and feature views using shared palette tokens inherit that selected accent. Gallery, navigation, settings, preview, metadata-export, playback, and Shell consume shared palette tokens; original egui swatch styling, font resolution, and exact visual rendering still require Windows validation.
- Shortcuts: `open_key` has the source-style capture interaction: click Change key, press a supported single key to persist it, or press Escape to cancel. Gallery reads the persisted binding when focused and opens the selected item; if the Settings provider is unavailable, Enter remains the fallback. F1–F9 use the source choices: Settings reads a fresh `NavigationService::snapshot()` when rendering the Shortcuts tab, recursively flattens tags, and labels each option with its icon (falling back to `🏷`) plus `full_path` with `>` spaced as ` > `. Choosing `None` clears only that key; choosing a tag replaces that key's assignment and saves immediately. Settings holds a weak navigation-service reference so stopping navigation releases its service normally. The settings plugin has a compile-time dependency on navigation and starts after it in the shell. With navigation stopped, existing assignments remain visible as tag IDs and editing is unavailable. Gallery F1–F9 toggle the assigned tag for selected media; Preview F1–F9 toggle it for the current item. Both paths use `TaggingService`, record tag changes through `HistoryService` when available, and publish `MediaIndexChanged`. The Preview surface also has a searchable single-item tag picker. Number-row 0–5 rates the current Preview item and records `UndoEntry::RatingChanged` when its old rating was available; Gallery number-row shortcuts rate the selected media.
- Tag sidebar preferences: after publishing `SettingsService`, startup publishes the loaded exclude modifier, tag-search match mode, search-persistence flag, and merge-child policy as `TagInteractionPreferencesChanged`. Settings emits the updated value after any of those settings changes, with enum values mapped through `photag-contracts` to avoid a dependency cycle. Navigation consumes the event and retains the latest preference snapshot; tag merge starts with the saved reparent/delete policy.
- AI mandatory groups support add/remove, inline rename, selection of existing navigation tags, and per-tag removal. The list is sourced from a fresh `NavigationService::snapshot()`; when navigation is stopped, groups can still be renamed and tags removed, while adding tags is unavailable. Preview reads the current `SettingsService::snapshot().ai` for each analysis request, including both mandatory groups and `auto_analyze`.
- Updates: `SettingsPlugin` reads the shell's `UpdaterService` and `SessionService`. When the settings view is created and auto-check is enabled it checks GitHub in the background. A newly found release opens the source-style floating offer with release notes, Update now, Not now, and GitHub actions; Not now suppresses future offers for this app session, while Settings → Updates can reopen it. The tab supports manual check, release notes, byte progress, checksum-verified staging, a two-step install confirmation, visible failures, and a durable install error log. Before installation it rewrites the latest persisted session snapshot, schedules the staged executable, and quits GPUI. After the app loop returns the shell replaces the executable and relaunches. Live Windows restart behavior remains unverified.
- `search_scopes` defaults all three source fields to enabled, seeds the Gallery's initial query scopes, and is updated by both the Settings panel and Gallery scope controls. Updates change only that property in the shared snapshot and persist through the source-compatible settings JSON.
- `toolbar_pos` exposes a lock toggle, 10 px directional offset controls, and reset to the default anchor. The Preview drag handle applies pixel offsets to the center-bottom anchor and persists them through `SettingsService` on release; save failures are shown in the Preview surface.

## Persistence/debugging

- `snapshot()` returns a settings clone; `update` persists each interaction and reports errors in-panel. Missing/invalid JSON loads original PhoTag defaults.
- Compare the JSON schema/defaults against pinned `vendor/photag-source/src/settings.rs`. For save errors check the LocalAppData path and permissions. UI wiring errors usually indicate the bridge service or slot host was not initialized before this plugin.

## Shared visual tokens

The floating Settings panel uses the shared backdrop-aware base and modal surface helpers, primary/danger text, and hairline tokens. Layout spacing and typography still use GPUI Kit presets; Windows visual parity is unverified.

## Code map

- `src/view.rs` owns Settings view state, preference edits, key capture, modal lifecycle, and the six-tab host.
- `src/view/tabs.rs` renders the six tab contents; `src/view/ai_groups.rs`, `tag_hotkeys.rs`, and `updates.rs` implement their focused editing flows and update offer.
- `src/settings.rs` owns the source-compatible preference schema/defaults and JSON service persistence.
