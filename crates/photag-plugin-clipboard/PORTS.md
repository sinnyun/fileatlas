# Clipboard workbench / 剪贴板工作台

## Cordis identity

- Plugin `photag.clipboard`; service `photag/clipboard` (`ClipboardService`).
- Requires shared `photag/database` and `photag/ui-bridge` services.
- The shell enables this plugin in `core`, `all`, MCP and explicit `clipboard` profiles. Navigation provides its left rail slot; the existing shortcuts plugin registers Ctrl+Shift+V.
- Clipboard change monitoring uses the open-source `clipboard-master` crate (MIT); text read/write uses `arboard` (MIT OR Apache-2.0). Source attribution and licenses remain in Cargo.lock/package metadata.

## Data and operations

- Text history is stored in additive `clipboard_categories` and `clipboard_history` tables in PhoTag's existing SQLite database through typed `DatabaseService::clipboard_*` methods.
- The monitor captures non-empty text up to 1 MB, deduplicates identical content, and limits temporary history to 500 rows. Favorites are durable and never pruned.
- `ClipboardService::use_item` restores an entry to the system clipboard so the user can paste it in the target application. The UI also supports search, category creation/filtering/assignment, text editing, local favorites, temporary-item deletion, and clearing temporary history.
- `assign_category(id, Option<i64>)` saves membership immediately and independently of the content editor. `rename_category(id, name)` and `delete_category(id)` maintain groups; deleting a group moves its records to unclassified without deleting text. Empty/duplicate/over-40-character names are rejected.
- Public `content::classify(&str) -> ContentKind` derives `Text`, `Link`, or `Code` without modifying content or user groups. Links require a single HTTP(S)/FTP URL with a host; code recognizes valid JSON containers, fenced snippets, and selected multiline declarations. This is a heuristic, not a language parser.
- Browser mode includes both clipboard tables in its existing durable-store whitelist and deletion reconciliation. Earlier builds omitted these tables, leaving new clipboard data only in the browser session. No separate clipboard database or photo schema changes were introduced.

## UI and limits

- `clipboard-workbench/panel` is a single generic floating panel; the left navigation button and Ctrl+Shift+V open it. Ctrl+F remains assigned to the program launchpad.
- Default size is 1160×760, minimum 1000×660. Visual reference: `docs/reference-ui/concepts/clipboard-workbench-layout-2026-10-18-v2.png`. Navy surfaces, Lucide icons, type/group navigation, card badges, multiline editor and immediate group controls are implemented. Group editing is outside navigation scrolling, so save controls remain visible. Clear-history and delete-group actions require UI confirmation.
- Image/file formats are not stored. The panel can pause automatic text capture in-memory for the current plugin session; private-app exclusions and a persisted privacy mode still need design.
- OS clipboard access can fail while another process owns/locks the clipboard; inspect `photag-clipboard-listener` logs. Global hotkey registration can fail on conflict; inspect `photag.shortcuts` logs.
- The listener uses a dedicated OS thread and is shut down by Cordis cleanup. Actual OS capture/write and cross-application paste remain unverified; UI evidence does not establish these paths.
- MCP `get_clipboard_state` omits content and exposes kind, revision, group IDs, and `clipboard.ui` metadata (selection/filter/visible IDs/input lengths). Metadata uses `UiSurfaceState` behind `ChangeGate`; `refresh::listen` delivers service changes without polling IO. Mutation-guarded `add_clipboard_text` injects database-only fixture text and does not touch the OS clipboard.

## Local favorites (2026-10-18)

- `add_favorite(id)` saves an existing history item as a durable local favorite. `remove_favorite(id)` permanently deletes favorite content only; the UI explicitly names this operation and requires confirmation. There is no unstar-to-temporary operation.
- `delete(id)` rejects favorites. `clear_unpinned()` clears temporary rows only; automatic retention also excludes favorites. Saving edits or adding duplicate text cannot demote a favorite.
- Existing `ClipboardHistoryRecord::pinned` and SQLite `pinned` remain compatibility storage names, now meaning favorite. Previously pinned records become favorites without schema changes. Both temporary history and favorites use the existing shared durable store; only temporary history is subject to retention/clear.
- Sidebar uses 收藏 / 本地收藏; starred cards and inspector expose explicit removal instead of pin toggling. Generic inspector trash is hidden for favorites. `clipboard.ui.confirm_favorite_delete` contains the pending ID; changing selection/filter cancels it.
# Shared theme (2026-10-18)

The view no longer owns a private palette. Colors, runtime accent and selection
come from UI Bridge `theme`; sections, cards, navigation and action/icon buttons
reuse `components`. Content-kind icons and date formatting remain feature-owned.
Capture, persistence, categories and favorites behavior is unchanged by this
styling change. See `docs/ui-theme.md` for future UI contributions.
