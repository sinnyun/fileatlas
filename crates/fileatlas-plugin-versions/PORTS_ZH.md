# Lore 历史版本插件对接 / Lore Versions plugin ports

## 生命周期与依赖

- Cordis 插件：`fileatlas.versions`；服务：`fileatlas/versions`（`VersionsService`）。
- 需要 `VersionInfoService`、FileAtlas `ThumbnailService`、`DatabaseService`
  （MCP 媒体 ID 路由用它解析已索引的文件路径）以及
  `IndexingService`（在 MCP 恢复或改变内容的分支操作之后刷新文件系统元数据
  与缩略图）。Shell 在 Versions 之前启动 Indexing。
- Lore GUI `lore-vm` 锁定到提交 `35eaf6ed1273397a5b18a283dda560949bb94ca4`，与 `F:\AI\geren\newworkspace` 一致。
- 数据位于当前活动数据库目录之下：`versions/worktrees/<entity>/`、`versions/stores/<entity>/shared-store/`，以及 `versions/previews/<entity>/` 中不可变的预览副本。
- `VersionsService::storage_path() -> &Path` 为状态与诊断暴露当前活动的 Lore 存储根；修订版本元数据本身存储在共享的 FileAtlas 数据库服务中。
- 每个普通文件都有一个 Lore 仓库与私有共享对象存储。Lore 永远不会收到远程 URL，也不会把用户的库根作为仓库根。

## 操作

- `file_state(absolute_path) -> FileVersionState` 将源内容与不可变的已保存修订版本进行比较。状态：`unversioned`、`saved`（所选修订版本）、`matches_saved_version`（另一个已存储的修订版本）、`modified`（都不匹配）。它从不移动检出（checkout）或当前标记。不稳定/缺失/不可读的内容会返回错误。没有任何版本时不会打开/哈希 Lore。

- `list(absolute_path) -> Vec<VersionEntry>` 使用 VersionInfo 完整的已保存版本目录，按修订号降序排列。激活较旧的修订版本绝不会从该列表中移除后续已保存的版本。在部分写入后，Lore 提供活动的 HEAD，并补充当前祖先链中缺失元数据的修订版本。
- `create_version(absolute_path, title) -> VersionEntry` 校验普通文件与标题，将文件快照到 Lore worktree，用该标题 stage/commit，flush Lore，然后写入元数据与稳定的缩略图副本。
- 在激活较旧修订版本后保存编辑时，先读取当前分支的本地 `latest`。如果检出落后于它，`prepare_snapshot_commit` 只撤销被拒绝的私有 `content` 负载的暂存，用 `reset=true` 将私有 worktree 同步到那个显式的本地 tip，然后对原始源进行快照/暂存。这样会按时间顺序追加一个版本，而不删除 01/02 或覆盖已编辑的源。对于干净且未变更的历史源，仍然返回无变更。冲突、待处理的合并以及无关的已暂存路径不会被丢弃。由于这个离线存储没有远程，因此不会使用空的同步目标。
- `activate(absolute_path, version_id) -> ()` 校验请求的修订版本，并要求已索引的源与 Lore worktree 都匹配当前保存的状态。它同步请求的 Lore 修订版本，在安装失败时执行带回滚的原子文件替换，然后更新 VersionInfo 的当前标记。
- `branch_graph(absolute_path, include_archived) -> VersionGraph` 将 Lore 分支、祖先堆栈、修订历史以及任何已保存的待处理合并投影到 VersionInfo。
- `create_branch`、`switch_branch` 和 `archive_branch` 使用 Lore 的分支操作。切换会恢复所选分支的快照并更新活动修订版本元数据；归档当前分支会被拒绝。
- `merge_branch` 合并到当前分支。冲突会持久化到 VersionInfo 并且不改动库中的源文件。`resolve_merge(path, "ours"|"theirs")` 解析单文件冲突集并恢复合并结果；`abort_merge` 调用 Lore 的合并中止并清除待处理状态。
- 不支持网络同步、通过 shell 调用 Git 或目录快照。本插件只接受普通文件，与 FileAtlas History 的媒体文件范围一致。
- 如果 Lore 提交后 VersionInfo 写入失败，列表与 FileManager 状态会从仓库恢复 Lore HEAD，因此缺失的元数据行不会让已保存的文件显示为未版本化。下一次成功的快照会写入正常的目录元数据。
- 当没有操作或等待者保留某个按文件的异步操作锁时，该锁会被移除；服务不会为每个曾查看过的路径累积一个锁条目。
- FileManager 以最多八个并发 Lore 读取请求可见的文件状态。工作量与可见文件及已保存修订版本成正比；它在 render 之外运行，不会在整个视口上形成一条串行链。

## 消费者与事件流

- FileManager 通过带键（keyed）的 `ReadModel` 读取；显式的创建/切换操作在 render 之外异步执行。
- 在激活或改变内容的分支操作之后，`IndexingService`
  刷新由文件系统派生的大小/时间/类型以及图片缩略图，同时
  保留评分/标签/拍摄日期。MCP 发出 `MediaIndexChanged` 和
  `MediaVersionHistoryChanged(media_id)`；FileManager 重新加载当前查询
  并使所选文件的 History 面板缓存失效。
- MCP `list_file_versions` 为只读。`create_file_version` 和 `activate_file_version` 属于高影响操作：它们要求在 Settings → MCP → AI control 中设置为 Full 并且每次调用带 `confirm=true`。

## 插件属有的 MCP 工具

`VersionsPlugin` 通过宿主的 `McpToolRegistry`
贡献带命名空间的工具；插件保留 Cordis 注册守卫，因此停止 Versions 会
将这些工具从 `tools/list` 和 `tools/call` 中移除。

- `versions.list {media_id}` 和 `versions.file_state {media_id}` 先读取
  已索引的媒体行，然后用其规范化路径调用 `VersionsService`。
  `versions.list_branches {media_id, include_archived?}` 读取
  Lore/VersionInfo 的分支图。
- `versions.create {media_id, title}` 和 `versions.activate
  {media_id, version_id}` 是需要 `confirm=true` 的变更型工具。
- `versions.manage_branch {media_id, action, ...}` 支持 `create`、
  `switch`、`archive`、`merge`、`resolve` 和 `abort`；它要求
  `confirm=true`。成功且改变文件内容的操作会发出
  `MediaIndexChanged`，以便 FileManager 与数据库派生状态刷新。
- 旧的无命名空间 MCP 别名已退役；功能属有的带命名空间路由是当前的
  客户端 API。

## 限制与诊断

- 常规 FileManager 浏览可以针对可见的未保存变更徽标调用 `file_state`，
  而无需打开 History。Lore 存储初始化会发出 DEBUG 级
  `MeterProvider.ExistingMeterReturned`；这是 meter 复用，不是恢复/提交。
  初始阶段可能出现有限数量的连续输出。FileManager 按文件/快照/
  历史标识缓存被动读取；无关的共享数据库写入不得触发它们。回归证据参见
  `docs/validation-ui/2026-10-18-version-idle/README.md`。

- 身份标识目前对规范化的绝对路径做哈希。文件重命名/移动会开始一段独立的历史；外部编辑必须在切换之前保存为版本。符号链接/特殊文件会被拒绝。
- 内容快照通过 Lore 的共享存储去重。历史预览是独立的 JPEG 副本；只能与对应的 Lore entity 一起清除它们。
- 错误以 `VersionsError` 暴露并显示在 Inspector 中。请检查 `versions/worktrees/<entity>/.lore`、对应的共享存储、version-info SQLite 元数据以及 `fileatlas.debug` span。不要通过删除 worktree/存储来修复展示元数据。
