# Shared contracts ports / 插件公共契约

This crate contains UI-framework-neutral Cordis events. Keep names and argument types stable once consumed; add versioned or new event contracts for incompatible semantics.

| Event | Cordis name | Args | Producer → consumer |
|---|---|---|---|
| `FolderSelectionChanged` | `fileatlas/navigation/folder-selection-changed` | `Option<i64>` | Navigation → FileManager |
| `TagSelectionChanged` | `fileatlas/navigation/tag-selection-changed` | `Option<i64>` | Legacy single-tag Navigation → FileManager |
| `TagFilterSelectionChanged` | `fileatlas/navigation/tag-filter-selection-changed` | `TagFilterSelection { included, excluded }` | Navigation → FileManager |
| `TagInteractionPreferencesChanged` | `fileatlas/settings/tag-interaction-preferences-changed` | `TagInteractionPreferences { exclude_modifier, search_match, search_persist, merge_reparent_children }` | Settings → Navigation |
| `DirectoryPathSelected` | `fileatlas/navigation/directory-path-selected` | `Option<String>` | Navigation → FileManager |
| `TagDragPayload` | GPUI typed drag payload | `source_id: i64` | Navigation → FileManager card assignment |
| `MediaIndexChanged` | `fileatlas/indexing/media-index-changed` | `()` | Watcher/Indexing → FileManager |

Each event currently returns `()`. Keep payloads independent of GPUI types so plugin services remain usable without rendering.


## Review corrections — 2026-10-16

`SidebarFoldersChanged::Args = SidebarFoldersUpdate { revision:u64, config:SidebarFoldersConfig }`. Navigation produces versioned sidebar edits; Settings serializes them and rejects stale delivery. SidebarFoldersConfig retains quick_access, favorite_groups, show_network_drives (default true). This is compile-time service/event coordination, not a second plugin manager.
# Unified sidebar bookmark import markers (2026-10-18)

`SidebarFoldersConfig.imported_library_paths: Vec<String>` is serde-default and
omitted when empty. Navigation imports existing registered-library bookmarks into
favorite groups once per normalized Windows path; Settings persists the resulting
model through the existing `SidebarFoldersUpdate` event. Markers retain user
removals across restarts and avoid SQLite ID-reuse ambiguity. Group/entry IDs and
aliases remain in the existing `FavoriteGroup`/`FavoriteFolder` payloads.
