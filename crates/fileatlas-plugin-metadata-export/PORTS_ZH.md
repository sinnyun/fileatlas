# 元数据导出插件对接 / Metadata export plugin ports

当前浏览器运行时已取代下文中较旧的索引库描述；请参阅末尾的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：原有覆盖全部库的数据库查询与 JSON 序列化现已通过 Cordis 服务
提供。一个由生命周期管理的 GPUI 模态框现在提供源兼容的原生保存文件
对话框与覆盖确认，调用已注册的 Cordis 命令，在命名的 worker 线程上
执行导出，并显示记录数量/路径或失败信息。

## 生命周期与依赖

- Cordis 插件：`fileatlas.metadata-export`；服务：`fileatlas/metadata-export`
  （`MetadataExportService`）。
- 需要 `fileatlas/database`，并在数据库插件之后启动。
- 还需要宿主发布的 `fileatlas/ui-bridge` 服务，用于面板与命令的生命周期
  注册。
- 使用源项目的 `rfd` 0.15 原生文件对话框库进行保存文件选择。该库已包含在
  workspace lockfile 中并禁用了默认 portal 特性；Windows 使用原生 Windows
  对话框后端。
- 插件 ID：`fileatlas.metadata-export`；命令 ID：
  `metadata-export/export`；面板视图 ID：`metadata-export/panel-view`。
- 当 `fileatlas.metadata-export` 的 Cordis 上下文被释放时，它会移除自己的
  目录条目、视图和命令监听器。

## 输入 / 输出

- `records()` 通过数据库 reader 连接调用源兼容的 `export_all_metadata`
  查询。它按完整路径排序导出每个媒体行，包含 `filename`、`full_path`、
  `rating` 以及层级化 `tags`。
- `to_pretty_json()` 返回源格式的美化顶层 JSON 数组。
- `write_to(path)` 在所选路径同步写入这些字节。GPUI 命令在专用 worker
  线程上执行等效步骤。
- `set_destination(path)` 保存命令使用的路径，且
  `status()` 向面板暴露 `Idle`、`Exporting`、`Succeeded { path, records }` 或
  `Failed(message)`。
- GPUI 对话框贡献到 workspace.modal，并通过 UiBridge surface
  metadata-export 打开。命令在准备阶段打开；“选择目标”会以 JSON 过滤器/默认
  fileatlas_backup.json 打开 rfd，并在返回目标路径后调用 request_export。
  它显示真实的 Idle/Exporting/Succeeded/Failed 状态、记录数量/路径与错误，
  并带有不确定进度和受限高度的页脚。
  原生取消操作不会改变目标路径；覆盖确认由操作系统负责。
- 结果负载：成功状态包含确切的输出路径与导出的行数；错误保留数据库、
  JSON、文件系统和状态锁的消息。面板渲染空闲引导、进行中的消息以及
  持久的终态成功或失败消息（成功时包含路径与记录数）；服务失败与
  状态不可用时使用 danger 颜色，而输入与命令分发错误有单独可见的错误行。
  Ready 与 active 状态包含关于当前操作的简明引导。
- Cordis 命令为单飞（single-flight）：导出运行期间再次调用会被拒绝，且
  不会替换活动状态。线程启动错误会将状态转为 `Failed`，因此视图不会
  卡在 `Exporting`；worker 崩溃也会被捕获并报告为失败。
  完成情况仅由与该次活动运行关联的 worker 写入；面板通过弱实体更新
  观察它，每次调用最多启动一个本地状态轮询，并在任务结束或视图被释放时
  停止轮询。数据库与文件 I/O 在不持有状态 mutex 的情况下运行。
- 选择或提交新的目标会清除之前的终态结果（除非有运行在进行中），因此
  成功文本始终标识实际写入的文件，而不会残留挂在新选的目标路径上。

## 调试

将输出字段、记录顺序、零评分默认值以及 ` > ` 标签路径格式与
`vendor/photag-source/src/db/queries.rs::export_all_metadata` 进行对照。
针对数据库、序列化和文件系统失败，检查面板结果与 `MetadataExportError`
文本。保存文件对话框通过 `rfd` 使用平台原生实现；Windows 的覆盖行为
遵循原生对话框。其他桌面平台行为取决于对应的 `rfd` 后端，且尚未验证。
worker 在对话框返回后直接写入所选路径。

## 共享视觉令牌

导出面板使用 fileatlas-ui-bridge::theme 获取源的次要/danger 文本及其 subtle 边框色。布局间距与排版仍使用 GPUI Kit 预设；Windows 视觉一致性未经验证。


## 本地化集成（2026-10-13）

可见控件标签使用由 fileatlas.i18n 提供的公开 UI Bridge 翻译器。
稳定的命令、源设置/schema 值、媒体路径与用户创建的标签数据保持不变。
缺失的翻译回退为英文；操作系统/后端错误与部分生成的诊断信息尚未完全
本地化。
07 配套设计图展示了百分比进度，但公开的导出状态只有 `Exporting` 且
不携带进度计数，因此面板报告实际的活动状态而不编造百分比。实时 MCP
视觉审阅保留给父级重设计任务；此处 Windows 原生对话框交互仍未验证。

## v5 设计 07 命令与可见状态（2026-10-14）

`metadata-export/export` 命令通过 UiBridgeService surface `metadata-export`
打开准备对话框。“选择目标”调用源兼容的原生 JSON 保存对话框
（`fileatlas_backup.json`，操作系统覆盖确认），然后设置目标并调用
`MetadataExportService::request_export`。模态框轮询真实的共享状态并显示
idle/exporting/succeeded/failed、记录数与目标。由于服务没有按记录的进度契约，
进度为不确定进度。MCP `export_metadata` 也驱动此状态模态框。关闭不会取消
worker，并且在导出期间禁用关闭。Schema/查询/写入器、服务身份与 worker
生命周期保持不变；UI 贡献现在拥有 workspace.modal。

## 2026-10-14 当前实现与 MCP 证据

通过 FileAtlas MCP 渲染了 Reference07 的真实成功 12 条记录以及文件系统目录失败（os5）。查询/schema/顺序/写入器未变更。原生保存/覆盖点击仍未验证；参见 docs/redesign-validation.md。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动以及现有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；已插桩的公开操作会上报挂钟时间的 span 时长与错误。领域服务契约、事件负载与源操作顺序保持不变。捕获范围与限制参见 fileatlas-plugin-debug/PORTS.md。

## 共享刷新所有权（2026-10-15）

导出面板向 UI Bridge refresh::poll 报告状态/可见性变化，保留现有的加载/成功/错误与打开行为。状态未变化时不进行每 tick 的 notify。


## 浏览器运行时更新 — 2026-10-15

运行时通过 `queries::export_metadata_scoped(conn,true)` 导出带标签/评分的文件记录；临时目录缓存行被排除。遗留的 `export_metadata` 为源兼容测试而保留。收藏夹与标签保持其既有导出表示形式。

## MCP 功能贡献

本插件拥有 `metadata-export.export {path, confirm:true}`。它使用同一个
`MetadataExportService` 状态机并等待成功/失败结果；在 handler 写入目标
文件之前，由 MCP 宿主强制要求确认。之前不带命名空间的 `export_metadata`
工具在迁移期间继续保留。
