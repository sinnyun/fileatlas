# Shared contracts ports / 插件公共契约

This crate contains UI-framework-neutral Cordis events. Keep names and argument types stable once consumed; add versioned or new event contracts for incompatible semantics.

| Event | Cordis name | Args | Producer → consumer |
|---|---|---|---|
| `FolderSelectionChanged` | `photag/navigation/folder-selection-changed` | `Option<i64>` | Navigation → Gallery |
| `TagSelectionChanged` | `photag/navigation/tag-selection-changed` | `Option<i64>` | Legacy single-tag Navigation → Gallery |
| `TagFilterSelectionChanged` | `photag/navigation/tag-filter-selection-changed` | `TagFilterSelection { included, excluded }` | Navigation → Gallery |
| `TagInteractionPreferencesChanged` | `photag/settings/tag-interaction-preferences-changed` | `TagInteractionPreferences { exclude_modifier, search_match, search_persist, merge_reparent_children }` | Settings → Navigation |
| `DirectoryPathSelected` | `photag/navigation/directory-path-selected` | `Option<String>` | Navigation → Gallery |
| `TagDragPayload` | GPUI typed drag payload | `source_id: i64` | Navigation → Gallery card assignment |
| `MediaIndexChanged` | `photag/indexing/media-index-changed` | `()` | Watcher/Indexing → Gallery |

Each event currently returns `()`. Keep payloads independent of GPUI types so plugin services remain usable without rendering.
