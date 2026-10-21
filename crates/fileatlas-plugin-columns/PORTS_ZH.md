# 主窗口分列插件对接

## 身份与生命周期

- Cordis 插件：`fileatlas.columns`；服务：`fileatlas/columns`（`ColumnLayoutService`）。
- 依赖 `WorkspaceService`；Shell 在 `fileatlas.workspaces` 之后、FileManager 之前启动它。
- Cordis 拥有服务发布与清理。状态变更会触发共享的 `fileatlas_ui_bridge::refresh::Wake`。

## 输入 / 输出

- `snapshot()` 返回当前的 `ColumnState { layout, active_index, revision }`。
- `set_layout(Single|Dual|Quad)` 更改布局并钳制激活索引。
- `activate(index)` 选中一个已存在的列，并拒绝越界的索引。
- 启动布局从所选的程序工作区读取。
- 生命周期所属的视图贡献：槽位 `command.columns` 中的 `columns/toolbar`。
  一个紧凑的布局图标会打开一个下拉菜单，其中带有勾选状态的 Single/Dual/Quad 布局
  选项，并且在显示多列时还带有勾选状态的激活列选项。
  这两种操作都调用本服务，工具栏通过它的 `Wake` 刷新；
  移除 `fileatlas.columns` 会注销该视图。
  Shell 把该贡献放在当前激活 FileManager 窗格的视图模式
  与视图选项投影之间，紧邻网格/列表。它使用共享的幽灵图标
  按钮并为下拉箭头留出空间；Square/Columns2/Grid2x2 分别表示单列、
  并排双列和 2×2 四列布局。

## 当前限制

- Shell 把该贡献挂载在现有的顶部控件行中，并把 FileManager 窗格布局为一个、
  两个并排，或四个组成 2×2 网格。共享的 FileManager
  过滤器/搜索/视图操作从激活窗格挂载；每个窗格保留自己的
  地址、导航历史、内容、选择、滚动和查询状态。
- `fileatlas.file_manager` 把文件夹/标签/路径选择事件路由到激活窗格。
  每个窗格都有最小宽度，当窗口窄于各窗格宽度之和时可以
  横向滚动。
- 同进程 FileAtlas MCP 的 `get_ui_state.program_workspace` 报告当前布局
  与激活窗格。`control_ui` 支持 `columns_layout` 和 `activate_column`。
- Windows 指针行为、极小窗口下的滚动，以及大目录
  内容仍有待进一步的平台特定验证。
