# 运行时调试插件

- Cordis 身份：`fileatlas.debug`；公开服务：`fileatlas/debug`（`Arc<DebugService>`）。
- 启动依赖：`UiBridgeService`。Shell 在运行时/插件启动之前创建缓冲区并调用 `attach`，然后在发布 bridge 之后启动 DebugPlugin。所有其他插件只发出 `tracing`/`log`；它们不依赖这个 UI 插件。
- `DebugPlugin::Config/Input = Arc<DebugService>`。默认容量 10,000 条记录，单条 target/action/message 上限为 4096 UTF-8 字节并附加截断标记。最旧的记录会被淘汰；清空后序号仍保持单调递增。状态仅存在于内存中，重启即清除。
- `attach(&Arc<DebugService>) -> Result<(), String>` 一次性安装进程 tracing 订阅器、log 适配器与 panic 钩子。捕获 sink 为弱引用，并在插件销毁时分离；支持重新附加。panic 钩子会保留先前的处理器。预先安装了不同的订阅器/logger 时会显式报启动错误，而不是静默缺失捕获。
- `Record`：id、带毫秒与 UTC 偏移的本地 ISO 时间戳、单调 elapsed_ms、级别、类别、target、action、message、可选 duration_ms、span_id、parent_span_id、线程。类别取自模块路径的第一段。所有应用发出的 tracing/log 级别都被保留；rmcp 传输层的 DEBUG/TRACE 载荷被排除，以防递归记录被查询的日志响应。MCP 操作与被拒绝的请求有各自独立的应用记录。span 会产生开始与结束记录；结束记录使用单调挂钟时长（包含等待），若该 span 收到 ERROR 则带错误状态。嵌套 span 通过 ID 关联。worker 显式提供的 duration_ms 字段保留在时长列中。
- `query(&Filter) -> Page`：类别精确匹配、级别大小写不敏感的精确匹配；对 message/action/target/ID 做大小写不敏感的子串匹配，最新在前，页码从零开始，默认 50 行，页大小限制在 1..200，页码按可用结果数限制。返回总数/保留数/淘汰数、类别以及最新 ID。`through_id` 在继续收集的同时冻结一个检视视图；容量淘汰仍然适用。`record_by_id` 精确取回被保留的详情。
- 收集版本号与视图失效是两个独立的机制。`view_revision(panel: bool) -> (u64,u64)` 返回 UI/控件与实时内容两个时钟；已关闭的抽屉、状态按钮和暂停的视图不会因新日志而重绘。GPUI 的 DEBUG/TRACE `Sending a11y tree update` 记录仍保留在完整的 buffer/filter/paging API 中，但不推进内容时钟，因为抽屉自身的绘制就会发出它们。其他应用记录以及框架的警告/错误仍会驱动实时更新。这在不丢弃无障碍诊断的前提下打破了观察者反馈循环。
- `control(action, value) -> Result<(), String>` 同时驱动真实 UI 与 MCP：open/close/toggle、category/level/search、page、page_size（25/50/100/200）、pause/follow、select 和 clear。历史翻页在当前序号处自动冻结。UI 控件/过滤状态是临时的，无会话/schema 迁移。`clear` 重置视图与淘汰计数，而不是序号。`ui_state` 返回共享的抽屉状态。
- UI 贡献：`workspace.debug-status` 中的底部状态按钮、`workspace.bottom` 中可展开的停靠区、元数据 `debug/panel`。Cordis stop 时会移除视图、目录元数据、公开服务、sink 与事件观察者。抽屉为客户端高度的 46%，限制在 220..380 逻辑像素之间，包含可滚动的工具栏/列表/详情、精确的级别/类别菜单（类别列表在 320 逻辑像素内滚动）、关键字输入框、暂停/恢复、清空、上一页/下一页、页大小和关闭。点击某行会显示保留的详情、线程与关联 ID。
- 观察的事件：UiCommandInvoked、FolderSelectionChanged、DirectoryPathSelected、TagSelectionChanged、TagFilterSelectionChanged 与 MediaIndexChanged。调试插件不发出任何产品级变更。
- 生产者：所有应用启动 apply span 与此前的 stderr 诊断路径、带调用方位置与 SQL 错误的 DB 序列化读/写 span、FileManager 的 load/page/reload/filter/group/selection/preview/actions、Navigation 的请求/选择、文件与标签操作、索引流的阶段/错误/结束与 worker 耗时、缩略图生成、媒体播放命令、历史与元数据导出。文件/数据行为与原始错误返回均保持不变。
- 限制：这是应用日志，不是指令级追踪或原生崩溃转储。不虚构非输出的第三方原生组件/OS 动作。进程中止/终止可能妨碍 UI 检视，而被取消/丢弃的 span 报告的是其存活期，并不能证明业务成功。没有持久日志导出、SQL 语句/正文捕获、CPU 分析器或持久的逐文件审计历史。worker span 不会在每个 spawn_blocking/线程边界自动传播父上下文；线程 ID 与显式扫描 ID 有助于关联。
- 失败排查：target/debug-plugin-*.log 下的编译/测试日志；隔离同进程 MCP 日志/截图与验证证据记录在 docs/validation-ui/2026-10-14-debug-panel/README.md。`debug.query` 读取缓冲区时不会记录自身；`debug.control` 使用常规的变更保护。

## 共享刷新归属（2026-10-15）

抽屉与底部视图都基于现有视图版本号使用 UI Bridge 的 refresh::poll 与 ChangeGate。记录行为保持独立：所有捕获的记录仍由现有存储保留；只有实际与 UI 相关的版本号变化才请求重绘。不引入新的过滤或抑制。

## 抽屉重绘噪声显示规则（2026-10-15）

`Filter.hide_ui_refresh: bool` 对原始服务查询默认为 false。真实的
UiState 将其默认设为 true，因此抽屉在搜索与分页之前只排除包含
`Sending a11y tree update:` 的 DEBUG/TRACE
`gpui::window` 记录。类别菜单同样排除仅由这些
被隐藏记录构成的类别。WARN/ERROR 及所有其他操作/DB 记录仍然可见。
原始保留、淘汰、序号时钟与 record_by_id 不变；
retained 仍统计完整缓冲区。分类器与现有的
内容时钟反馈保护逻辑共享。不新增控件或持久化设置。
上文最初"无抑制"的表述描述的是收集行为，收集
保持不变；用户在之后显式要求了此展示过滤器。
证据：docs/validation-ui/2026-10-15-debug-hover/README.md。

## 图像计时详情记录（2026-10-15）

Preview 现在为每个任务发出源/缓存路径、尺寸/维度、角色、解码器、
缓存状态、阶段耗时与失败信息。现有的 message/详情抽屉在
选中某行时显示这些字段；duration_ms 仍填充时长列。没有新的渲染时查询、展示过滤器或持久化状态。
例行的 GPUI 重绘隐藏不会隐藏这些应用 DEBUG/ERROR 记录。

## 统一主题（2026-10-18）

视图绘制使用 UI Bridge 的语义主题 token 而不是本地颜色字面量。
FileManager 检查器/统计卡片与标题使用共享组件；紧凑
导航在保留其布局的同时使用通用的选中/悬停绘制。状态、分支
与媒体覆盖层颜色在中央主题中仍保持语义上的区分。
不改动产品服务/数据/事件契约。参见 `docs/ui-theme.md`。

## 插件自有的 MCP 工具

该插件通过共享 MCP 注册表注册 `debug.query`、`debug.control` 与
`debug.selected_logs`。查询参数与
响应直接来自 `DebugService::query`、`ui_state` 与
`view_revision`；抽屉变更调用 `DebugService::control`。变更类
工具遵循宿主 MCP 的变更策略。停止 Debug 时，这些工具会随
该插件的 Cordis 注册保护一并移除。

- `debug.query` 接受可选的 `category`、`level`、`search`、从零开始的
  `page`、`page_size`（1–200）以及 `hide_ui_refresh`。
- `debug.control` 接受 `action` 及可选的 `value`；action 为
  服务的 open/close/toggle、过滤、翻页、pause/follow、选择、
  复制/导出与清空操作。它受变更门控。
- `debug.selected_logs` 不接受参数，返回选中的日志文本与
  ID。
