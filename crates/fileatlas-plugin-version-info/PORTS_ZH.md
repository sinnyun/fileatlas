# Version Info plugin ports / 历史版本信息对接

## 生命周期与归属

- Cordis 插件：`fileatlas.version-info`；服务：`fileatlas/version-info`（`VersionInfoService`）。
- 在 `fileatlas.database` 之后、`fileatlas.versions` 之前启动；获取共享的 `DatabaseService` 并发布类型化的 `VersionInfoService` 门面。
- 存储展示元数据、当前修订标记、分支祖先投影以及待合并的冲突状态。Lore 拥有文件内容与修订对象。
- 版本行与分支快照存放在由 `fileatlas-plugin-database` 管理的类型化表中，其中包括浏览器模式的持久存储。启动时，遗留的同级文件 `version-info.sqlite3` 会被导入一次、以幂等方式执行，并作为回滚副本保留；它永远不会被以写入方式打开。

## 服务契约

- `list(entity_id)` 按从新到旧返回修订，包含修订 ID/编号、父级 ID、标题、时间戳、大小、MIME、预览路径与当前状态。
- 这份完整的持久化目录拥有已保存版本的可见性。Lore 当前 HEAD 的祖先链不是它的过滤条件；`set_current` 只更改标记，不删除更旧或更新的记录。
- `save(record)` 以事务方式 upsert 元数据，并为每个实体维护唯一一条当前修订。
- `set_current(entity_id, version_id)` 会拒绝未知的修订，并在单个事务中更新当前状态。
- `put_graph(entity_id, graph)` 存储最新的 Lore 分支投影与可恢复的待合并状态；当不存在投影时 `graph(entity_id)` 返回一个空投影。
- 实体 ID 由 Lore 插件从路径派生。当前限制：重命名或移动文件会创建一个新的历史身份；路径重新绑定支持尚未实现。

## 错误与排查

数据库错误与 JSON 错误以 `VersionInfoError` 返回。请通过 DatabaseService 诊断查看类型化的服务方法以及 `versions` / `version_graph` 表。该门面不暴露任意 SQL，也不重写 FileAtlas 的标注记录。
