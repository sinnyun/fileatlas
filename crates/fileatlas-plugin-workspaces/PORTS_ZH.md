# 程序工作区插件对接

## 身份与生命周期

- Cordis 插件：`fileatlas.workspaces`；服务：`fileatlas/workspaces`（`WorkspaceService`）。
- 在 `fileatlas.columns` 与 FileManager 之前启动。Shell 为 dev-test 提供隔离的 JSON 路径；普通启动使用 `%LOCALAPPDATA%\\FileAtlas\\workspaces\\workspaces.json`。
- Cordis 负责服务发布，并随插件纤程一起移除该服务。

## 存储与数据

- 一个带版本号的 JSON 文档保存命名启动布局、默认工作区 ID、
  单/双/四列模式，以及每列的地址/历史、搜索范围、
  媒体/标签/扩展名/日期/评分过滤器、排序、分组、网格/列表模式、
  子文件夹/缩略图偏好、选中的文件路径和滚动偏移。
- 它不保存任何媒体行。FileManager 的查询依旧通过 `fileatlas-plugin-database`。
- 写入时先序列化到同目录下的临时文件，同步该文件，然后原子地持久化覆盖目标文件。未知的未来格式版本会被拒绝，且不会重写该文件。文件缺失时加载内存中的单列默认配置，并在第一次变更时创建它。
- `ProgramWorkspace::validate` 检查 ID/名称、精确的列数和评分上下界。`WorkspaceService::upsert`、`set_default`、`remove`、`snapshot` 和 `active` 是公开操作。

## 当前行为

- JSON 模型、持久化服务和管理器 UI 支持创建、激活、
  重命名、设为默认、保存当前，以及删除非默认定义。删除
  处于激活状态的非默认工作区会返回到当前默认工作区。
- 启动时恢复默认工作区。激活某个工作区会把每一列的定义应用到相互独立的
  FileManager 窗格实例；FileManager 查询
  仍然通过共享的数据库/本地浏览器服务运行。
- “保存当前”消费 `workspace.current_snapshot`，这是宿主发布的临时配置
  快照。每个窗格合并自身当前状态，因此其他窗格
  所做的编辑会被保留。
- 同进程 FileAtlas MCP 的 `get_ui_state.program_workspace` 报告激活与
  默认工作区定义；`control_ui` 可以激活已存在的工作区，
  并打开/关闭管理器。
- 无效/缺失的地址会作为配置保留，并在打开时显示
  错误；它们不会改变 FileManager 的数据库数据。

## UI 集成

- Cordis 通过 `fileatlas-ui-bridge` 在 `workspace.modal` 中注册
  `workspace/manager-panel`；插件停止时 Cordis 会移除该贡献。
- 主工具栏使用 `UiSurfaceState::set_open("workspace-manager", true)` 打开它。
- 选择或创建工作区会把自己的 ID 写入 `workspace.selection`，
  并把布局键（`single`、`dual`、`quad`）写入 `workspace.layout`。公开的
  `WorkspaceService::active()` 返回这一运行时选择项；设置默认值
  仍是独立操作，用于控制下一次启动。
- 在保存实时布局之前，宿主可以把当前的
  `ProgramWorkspace` 序列化到 surface 值 `workspace.current_snapshot`。“保存
  当前”操作会持久化该快照；如果宿主没有发布过快照，它就
  保存服务当前持有的工作区定义。
- `workspace.selection` 只是一个 UI 诊断提示；`WorkspaceService::active()`
  才是所选定义的权威来源。
