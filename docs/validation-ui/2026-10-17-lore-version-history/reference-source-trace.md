# 参考项目原始源码对照：历史版本切换和显示

2026-10-17 直接读取 `F:\AI\geren\newworkspace` 的磁盘源码。

| 参考源码 | 实际职责 | PhoTag 对应 |
| --- | --- | --- |
| `src-tauri/src/plugins/versions/operations.rs::create_version` | Lore snapshot/stage/commit/flush 后，通过 VersionInfoService 保存版本 ID、显示信息和当前标记 | `photag-plugin-versions::create_version` → `VersionInfoService::save` |
| 同文件 `activate_version` | 校验 revision，Lore sync/flush，替换原路径内容，再更新当前版本标记 | `VersionsService::activate` → `VersionInfoService::set_current` |
| `src-tauri/src/plugins/version_info/graph.rs::graph_view` | SQLite 事务读取全部版本记录及持久化分支投影，显示查询不打开 Lore | `VersionInfoService::list/graph`；本轮修复 `VersionsService::list` 以完整持久化记录为列表主体 |
| `src/plugins/versions/service.ts::call` | 写操作开始及结束均失效该实体的显示缓存 | Gallery 操作完成后增加 `history_generation`，按新键请求 ReadModel，丢弃旧键结果 |
| `src/plugins/inspector/InspectorPanel.vue::versionGraph` | 按 revision ID 合并分支图与版本信息；第 93 行开始明确保留 Lore 当前链没有返回的元数据版本；稳定排序 | 完整保留 VersionInfo 的所有版本，按 revision number 降序，Lore 仅补充缺元数据的当前链 revision |
| 同文件 `activateVersion/loadVersions` | activate 后刷新当前文件及版本视图；同实体刷新保留旧内容，异常显示错误 | Gallery 异步 activation、历史代次刷新及错误信息 |
| `src/composables/useWorkspace.ts::refreshEntry` | 稳定实体 ID 原位更新文件属性与缩略图 revision，保留选择和标签等状态 | Indexing 原位更新媒体行；本轮修复 Gallery 不再用重扫的临时空/部分列表清除选择 |
| `src-tauri/src/plugins/versions/projection.rs` | 操作前持久化 dirty/generation；操作后保存图，失败保留旧图并提示修复 | 当前 PhoTag 采用独立后台 Lore 分支读取和 `put_graph`，尚未完整移植该 dirty/generation 投影协议 |

## 本轮定位

保存版本会写入 VersionInfo，激活只修改当前标记，不删除其他版本记录。
消费链有两处问题：

1. 列表曾只遍历 Lore 当前 HEAD 的祖先，回退后较新的已存版本被过滤掉。
2. 原路径恢复会触发目录重扫；Gallery 用初始空结果验证选择，清除了文件选择，
   右侧从历史页变为文件夹信息。截图确认这会让历史看起来消失。

修复以持久化完整版本列表作为显示主体，给 BrowserSnapshot 增加 `loading`
快照标记；只有完整结果才能据缺席记录清理选择。文件真的移除或过滤后，完整
结果仍会正常修复选择。

参考项目还有 metadata pending/reconcile、投影 dirty/generation 和显式修复路径。
不能把本轮修复称为全部故障恢复协议已经逐项移植。
