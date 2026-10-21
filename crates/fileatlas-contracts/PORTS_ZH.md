# Shared contracts ports / 插件公共契约

本 crate 只包含与 UI 框架无关的 Cordis 事件。一旦被消费，就应保持名称与参数类型稳定；对于不兼容的语义，请添加带版本或新的事件契约。

| 事件 | Cordis 名称 | 参数 | 生产者 → 消费者 |
|---|---|---|---|
| `FolderSelectionChanged` | `fileatlas/navigation/folder-selection-changed` | `Option<i64>` | Navigation → FileManager |
| `TagSelectionChanged` | `fileatlas/navigation/tag-selection-changed` | `Option<i64>` | 遗留的单标签 Navigation → FileManager |
| `TagFilterSelectionChanged` | `fileatlas/navigation/tag-filter-selection-changed` | `TagFilterSelection { included, excluded }` | Navigation → FileManager |
| `TagInteractionPreferencesChanged` | `fileatlas/settings/tag-interaction-preferences-changed` | `TagInteractionPreferences { exclude_modifier, search_match, search_persist, merge_reparent_children }` | Settings → Navigation |
| `DirectoryPathSelected` | `fileatlas/navigation/directory-path-selected` | `Option<String>` | Navigation → FileManager |
| `TagDragPayload` | GPUI 类型化拖拽载荷 | `source_id: i64` | Navigation → FileManager 卡片打标 |
| `MediaIndexChanged` | `fileatlas/indexing/media-index-changed` | `()` | Watcher/Indexing → FileManager |

每个事件目前都返回 `()`。请让载荷独立于 GPUI 类型，这样插件服务在没有渲染的情况下仍然可用。


## 评审修正 — 2026-10-16

`SidebarFoldersChanged::Args = SidebarFoldersUpdate { revision:u64, config:SidebarFoldersConfig }`。Navigation 产生带版本号的侧栏编辑；Settings 负责序列化它们，并拒绝过期的投递。SidebarFoldersConfig 保留 quick_access、favorite_groups、show_network_drives（默认为 true）。这是编译期的服务/事件协作，不是第二个插件管理器。
# 统一侧栏书签导入标记（2026-10-18）

`SidebarFoldersConfig.imported_library_paths: Vec<String>` 使用 serde 默认值，
并在为空时被省略。Navigation 会把已存在且已注册库的书签导入到收藏分组中，
每个规范化后的 Windows 路径只导入一次；Settings 通过现有的
`SidebarFoldersUpdate` 事件持久化由此得到的模型。这些标记会在多次重启之间保留
用户的移除操作，并避免 SQLite ID 复用带来的歧义。分组/条目 ID 与
别名仍然保存在现有的 `FavoriteGroup`/`FavoriteFolder` 载荷中。
