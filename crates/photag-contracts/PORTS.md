# Shared contracts ports / 插件公共契约

This crate contains UI-framework-neutral Cordis events. Keep names and argument types stable once consumed; add versioned or new event contracts for incompatible semantics.

| Event | Cordis name | Args | Producer → consumer |
|---|---|---|---|
| `FolderSelectionChanged` | `photag/navigation/folder-selection-changed` | `Option<i64>` | Navigation → Gallery |
| `TagSelectionChanged` | `photag/navigation/tag-selection-changed` | `Option<i64>` | Navigation → Gallery |
| `DirectoryPathSelected` | `photag/navigation/directory-path-selected` | `Option<String>` | Navigation → Gallery |
| `MediaIndexChanged` | `photag/indexing/media-index-changed` | `()` | Watcher/Indexing → Gallery |

Each event currently returns `()`. Keep payloads independent of GPUI types so plugin services remain usable without rendering.
