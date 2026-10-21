# 文件暂存篮插件对接（File Staging Basket plugin ports）

状态：持久化的路径篮使用由图片缩略图与标题优先卡片组成的响应式网格。
用户可以通过 OS 选择器添加多个任意文件，选中一张或多张卡片，移除一张
或全部清除，刷新缺失文件状态，移除不可用条目，以及将路径复制或移动到
所选目录。将卡片拖到 FileManager 的子文件夹分组上时，应使用现有的默认
文件拖放行为立即移动它。传输操作委托给
`FileOperationsService`；暂存插件不拥有任何文件传输引擎。

## Cordis 与 UI 生命周期

- 插件：`fileatlas.file-staging`；服务：`fileatlas/file-staging`
  （`FileStagingService`）。
- 需要 `FileOperationsService`、`SettingsService` 和 `UiBridgeService`。
- 注册单个原生浮动面板（`file-staging/panel`）、一个打开命令，以及一个由
  生命周期管理的启动栏控件。插件停止时，Cordis/UI Bridge 会移除注册与
  面板定义。
- FileManager 卡片上下文菜单通过共享 contracts crate 为当前选中的文件 ID
  发出 `FileStagingAddRequested`。暂存插件在阻塞 worker 上解析路径列表，
  只有当其内存列表发生变化时才唤醒已打开的篮视图。FileManager 保持与
  此功能 crate 相互独立。
- 暂存卡片使用框架的拖放系统与共享的
  `FileStagingDragPayload`。暂存面板也接受
  `FileManagerFileDragPayload`，在放置时把被拖动的路径加入其清单，
  并保留源文件。这种共享负载让各功能 crate 保持独立。FileManager 的子文件夹
  分组框接受暂存负载；放置时调用 `move_paths_to_folder`，在
  FileManager 操作状态中报告逐路径结果，并发出
  `FileStagingPathsMoved`，因此只有成功移动的路径会从篮中移除。不显示
  复制/移动提示；面板中仍提供显式的复制操作。
- 当共享的、由生命周期管理的 MCP registry 存在时，注册
  `file-staging.list`、`file-staging.add`、`file-staging.remove`、
  `file-staging.clear`、`file-staging.refresh_status`、
  `file-staging.remove_missing`、`file-staging.copy` 和 `file-staging.move`。
  Add/remove/clear
  是普通的受门控的变更操作；copy/move 在 Full 访问级别下需要 `confirm=true`。
- Shell 在 normal/core 模式下默认启动该功能，并在 Plugin settings 中列出它
  以供运行时启用/禁用。开发环境可用
  `FILEATLAS_DEV_PLUGINS=file-staging` 将其隔离（调试客户端也会启用 dev-test）。

## 数据与行为

- 在
  `%LOCALAPPDATA%\FileAtlas\file-staging\staging.json` 存储一个版本 1 的 JSON
  文档；隔离的 profile 使用 `<profile>\file-staging\staging.json`。它包含
  规范化的文件路径、添加时间戳，以及缓存的最后已知可用性。它绝不把
  文件字节复制到篮中。
- JSON 适合这种有界的、整体文档式的列表。写入使用同目录临时文件与
  备份替换；启动时若主文件缺失则恢复 `.json.bak`。未知版本会以明确的
  错误阻止插件启动。不存在需要迁移的更早格式。
- 重复路径按 Windows 大小写不敏感的路径标识忽略。目录不会被暂存。
  缺失条目会保留直到被移除或清理。
- 图片文件从源路径在裁剪的缩略图视口中渲染；其他文件类型与不可用文件
  使用共享主题的文件图标。没有添加任何暂存专属的缩略图缓存或文件解码
  服务。
- 目标冲突按路径报告，面向路径的复制 API 绝不替换已存在的目标
  文件。成功的条目会从篮中移除；失败或被跳过的条目保留以便重试。UI 结果
  反馈显示数量以及最多三条失败路径/原因。
- File Operations 中的路径操作会在执行时再次校验源。
  已索引的移动使用现有的数据库路径更新/移除与索引事件；
  未索引的文件在磁盘上移动，已注册文件夹的 Watcher 可以
  发现它们。复制到被监视根目录下时由 Watcher 发现。

## 公开操作与诊断

- `snapshot()` 返回篮的克隆。
- `add_paths(paths)` 对已存在的文件进行规范化并去重。
- `remove(path)`、`clear()`、`refresh_status()` 和 `remove_missing()` 在发布
  内存快照之前先持久化每一项成功的变更。
- File Operations 提供 `copy_paths_to_folder` 和 `move_paths_to_folder`，
  并为每个输入路径返回一个 `PathOperationOutcome`。这些使用现有的
  File Operations Cordis 服务；没有添加第二个操作插件或存储层。
- 当路径缺失、被锁定或发生冲突时，检查 `fileatlas/file-staging` 的启动
  诊断、`staging.json` 以及 File Operations 的逐路径结果。

## 限制

- 添加路径使用原生多文件选择器、FileManager 卡片菜单
  （暂存当前选中项）或 MCP 工具。未提供目录暂存和键盘快捷键。
- 预期的内部放置目标是 FileManager 的子文件夹分组；其他
  分组模式忽略篮负载。拖动已选中的卡片应立即移动所有可用的选中路径；
  否则应只移动被拖动的那张卡片。同进程 MCP 显示了拖动幽灵与目标高亮，
  但释放并未触发移动，因此该交互未经运行时验证。显式的“移动到…”选择器
  仍然可用。
- 篮的变更立即持久化。没有多阶段撤销、队列、执行中批次的
  暂停/取消，也没有中断传输的重启恢复。传输由现有服务调用完成/报告。
- 保留 File Operations 现有的已索引移动语义。篮不提供
  回收站删除。
- Windows 对话框、跨卷移动、路径权限失败与冲突竞争仍需要同进程
  MCP/Windows 运行时验证。

## 参考

当前实现遵循
`docs/reference-ui/concepts/file-staging-grid-drag-2026-10-20.png`：响应式
网格卡片优先展示图片预览与标题；预期的即时
子文件夹放置使用已建立的 FileManager 流程。生成的概念图是
布局指南；解析后的颜色与组件状态由原生 Theme 和 UI Bridge 控件负责。
