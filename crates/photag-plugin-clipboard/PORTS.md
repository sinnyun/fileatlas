# Clipboard workbench / 剪贴板工作台

## Cordis identity

- Plugin `photag.clipboard`; service `photag/clipboard` (`ClipboardService`).
- Requires shared `photag/database` and `photag/ui-bridge` services.
- Also reads `photag/settings` for `Settings.plugin_launch_modes["photag.clipboard"]`. Settings can dispose/re-spawn this standalone tool Fiber; Cordis cleanup signals the clipboard monitor and removes its command, panel, shortcut, and sidebar contributions.
- The shell enables this plugin in `core`, `all`, MCP and explicit `clipboard` profiles. Navigation provides its left rail slot; the existing shortcuts plugin registers Ctrl+Shift+V.
- Clipboard change monitoring uses the open-source `clipboard-master` crate (MIT); text and image read/write uses `arboard` (MIT OR Apache-2.0, `image-data` enabled). Type-detection inspiration was reviewed in [ZClipboardManager](https://github.com/TheHolyOneZ/ZClipboardManager) (GPL-3.0); its Tauri app is not a reusable library and no upstream code was copied.

## Data and operations

- User data is stored in `%LOCALAPPDATA%\\PhoTag\\clipboard\\clipboard.json`; image assets live in
  `clipboard/images/<sha256>.png`. In isolated MCP profiles both are under the selected profile.
  JSON is versioned and written to a sibling temp file before replacement. First start imports existing
  clipboard rows/categories from shared SQLite and copies available image assets. Once JSON exists it
  is authoritative; legacy SQLite rows remain untouched for recovery and are no longer read or updated.

- The monitor independently attempts non-empty text (up to 1 MB) and image capture on each clipboard change; text and images are separate history entries. Images are saved as full-resolution PNG files named by SHA-256. Temporary entries are deduplicated and limited to 500 rows; favorites are durable and never pruned. Orphan generated PNGs are removed when the history snapshot reloads.
- Capture only reads clipboard formats and writes PhoTag-owned JSON/image files. It does not call `set_text`, `set_image`, `EmptyClipboard`, or a clipboard-clear API. Selecting “copy to system clipboard” is the only history action that replaces the current OS clipboard.
- `ClipboardService::use_item` restores an entry to the system clipboard so the user can paste it in the target application. The UI also supports search, category creation/filtering/assignment, text editing, local favorites, temporary-item deletion, and clearing temporary history.
- `assign_category(id, Option<i64>)` saves membership immediately and independently of the content editor. `rename_category(id, name)` and `delete_category(id)` maintain groups; deleting a group moves its records to unclassified without deleting text. Empty/duplicate/over-40-character names are rejected.
- Public `content::classify(&str) -> ContentKind` derives `Text`, `Link`, `Code`, or detectable `Html` without modifying content or user groups. Links require a single HTTP(S)/FTP URL with a host; code recognizes valid JSON containers, fenced snippets, and selected multiline declarations. This is a heuristic, not a language parser. Image entries use the explicit image type and render cached preview/detail images.
- Existing SQLite clipboard tables are retained as migration/recovery data and are no longer in the clipboard runtime write path. No photo schema changes are introduced.

## UI and limits

- `clipboard-workbench/panel` is a single generic floating panel; the left navigation button and Ctrl+Shift+V open it. `PanelOnly` hides the main workspace while the panel is open. Ctrl+F remains assigned to the program launchpad.
- Default size is 1160×760, minimum 1000×660. Visual reference: `docs/reference-ui/concepts/clipboard-workbench-layout-2026-10-18-v2.png`. Navy surfaces, Lucide icons, type/group navigation, card badges, multiline editor and immediate group controls are implemented. Group editing is outside navigation scrolling, so save controls remain visible. Clear-history and delete-group actions require UI confirmation.
- The supported capture representations are arboard text and decoded bitmap/image data. Windows file-drop lists and arbitrary app-specific clipboard formats are not yet stored. The panel can pause capture in-memory for the current plugin session; private-app exclusions and a persisted privacy mode still need design.
- OS clipboard access can fail while another process owns/locks the clipboard; inspect `photag-clipboard-listener` logs. Global hotkey registration can fail on conflict; inspect `photag.shortcuts` logs.
- The listener uses a dedicated OS thread and is shut down by Cordis cleanup. Actual OS capture/write and cross-application paste remain unverified; UI evidence does not establish these paths.
- MCP `get_clipboard_state` omits content and exposes kind, revision, group IDs, and `clipboard.ui` metadata (selection/filter/visible IDs/input lengths). Metadata uses `UiSurfaceState` behind `ChangeGate`; `refresh::listen` delivers service changes without polling IO. Mutation-guarded `add_clipboard_text` writes fixture text to the isolated profile's JSON file and does not touch the OS clipboard.

## Local favorites (2026-10-18)

- `add_favorite(id)` saves an existing history item as a durable local favorite. `remove_favorite(id)` permanently deletes favorite content only; the UI explicitly names this operation and requires confirmation. There is no unstar-to-temporary operation.
- `delete(id)` rejects favorites. `clear_unpinned()` clears temporary rows only; automatic retention also excludes favorites. Saving edits or adding duplicate text cannot demote a favorite.
- Existing `ClipboardHistoryRecord::pinned` and legacy SQLite `pinned` values mean favorite. Previously pinned records import as favorites. JSON preserves both temporary history and favorites; only temporary history is subject to retention/clear.
- Sidebar uses 收藏 / 本地收藏; starred cards and inspector expose explicit removal instead of pin toggling. Generic inspector trash is hidden for favorites. `clipboard.ui.confirm_favorite_delete` contains the pending ID; changing selection/filter cancels it.
# Shared theme (2026-10-18)

The view no longer owns a private palette. Colors, runtime accent and selection
come from UI Bridge `theme`; sections, cards, navigation and action/icon buttons
reuse `components`. Content-kind icons and date formatting remain feature-owned.
Capture, persistence, categories and favorites behavior is unchanged by this
styling change. See `docs/ui-theme.md` for future UI contributions.
