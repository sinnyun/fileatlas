# WorkCrate UI 插件对接

## Cordis 集成

插件 `fileatlas.workcrate-ui` 保留命令 `workcrate/open`、面板
`workcrate/overview`、侧边栏启动器以及 `workcrate/settings` 中的 WorkCrate 页面。
依赖项为 Projects、Boards、Export、Thumbnails、FileOperations、Settings
和 UiBridge。Cordis 拥有注册与监听器生命周期。Projects 拥有
工程/引用/预设存储；Boards 拥有官方应用配置、
原生文档绑定与预览缓存。UI 不拥有任何平行的文档格式。

## 原生工作流

Projects 显示标题/封面卡片、搜索、按名称/创建时间排序以及已归档过滤。
创建时首先询问用户拥有的原生文档文件夹，然后创建
内部工程记录以及带有初始画板的官方 `.vectorcraft` 文档。如果原生创建失败，UI 会移除不完整的工程记录，
并且不对用户文件夹做任何改动。克隆操作同样会先询问目标文件夹，
然后复制原生文档与内部工程元数据，之后再刷新
克隆体的画板/预览缓存。重命名、封面、归档/恢复以及经确认的删除
都通过 Projects 服务完成。工程列表仅解码显式封面，
绝不打开原生文档。

工程与画板浏览读取本地工程注册表、缓存的画板
目录以及缓存的预览文件。打开某个工程或页面绝不启动
VectorCraft、它的 CLI 或端口连接。缺失的本地预览会显示一个
同步提示。显式的“同步工程与预览”操作会在后台
工作线程中调用 `refresh_project`，并重新加载由此生成的本地缓存。关联一个已有的
`.vectorcraft` 文件或创建一个 A3 横向画板。已有的画板
尺寸保持不变。多选、全选/清除/退出、有序图片导出
以及全画板 PDF 都使用 Export。原生文件选择器用于选择已有文档、
放置输入文件以及导出目标。

第三层遵循 `old/src/components/AssemblyStudioView.tsx`：左侧页面轨道、
中部画布、右侧材料信息。同时也检视了 October20 的
projects/pages/editor 概念图。GPUI Kit/主题角色拥有控件与颜色。
中部画布分两种模式：预览态只画缓存的官方 PNG，不启动任何东西；
编辑态（`view/canvas.rs`、`view/canvas_surface.rs`、`view/object.rs`）
通过 Boards 打开一个官方画布会话，只叠加官方程序报告的内容——
渲染出的页面位图、官方 `selectionBounds` 界框与 8 个句柄，
以及指针按下期间的半透明预测框。指针输入从窗口像素映射为文档点，
并作为官方 `pointer_gesture` 转发；数值框调用 `object.setBounds`
（`reference:0`）；撤销/重做与缩放都作用于官方会话。
面板自身不实现命中测试、变换计算、裁剪、文本排版或私有历史，
完整编辑仍然可以通过 `open_editor` 交给官方程序。

实际宽/高为只读。标题、尺寸标注、工程级覆盖（含
继承/重置）、材料、尺寸、数量与附加项都使用原生 PageInfo/header
更新。稳定的输入实体保存草稿；只有显式保存才会提交。已有的
字段预设设置与原生下拉菜单仍然可用。

WorkCrate 设置页在已有字段预设之外，包含 VectorCraft 可执行文件路径与本地
控制端口。“保存程序设置”通过 `Boards.configure`
持久化；它不会连接端口，也不会启动任何一个程序。
后台 CLI 操作使用配置的可执行文件以及同目录 CLI
发现机制。只有“在 VectorCraft 中编辑”才会启动官方 GUI 窗口；编辑态只会通过
已配置的控制端口连上本来就在运行的 GUI，否则保持完全静默。

## 后台工作与 UI 检视

目录/封面读取使用带键的 ReadModel 工作线程、完成 Wake/listen 以及
缓存快照。预览解码使用一条串行 ReadModel 队列；代次
键会在导航之后拒绝过期的结果。任务使用 background_executor 与忙碌门控；
Boards 串行化原生事务。渲染仅投射缓存快照。
ChangeGate 仅在其序列化状态改变时才发布 `workcrate.ui`。

Snapshot 包含 screen（`projects`、`pages`、`editor`）、project_id、board_id、
busy、page_count、boards（id/native_id/index/title/dimensions/info/preview_ready/
error）、previews_ready、preview_pending、selection、connection、executable、port、
document 与 message，外加 `canvas`：mode（`preview`/`edit`）、busy、zoom、`state`
（官方 `CanvasSnapshot`：live/tool/revision/dirty/selection/selection_bounds/
artboard/preview/preview_size/last_step，预览态为 null）、`object`
（官方节点详情：id/kind/name/bounds/text/linked/status/link_path）
以及指针手势进行中的 `drag`（kind 为 `move` 或 `handle<N>`、origin、start、current）。
视图丢弃时会清空它。已有的埋点在
`workcrate.control.<id>` 下度量控件，包括画板、检视器、页面预览、
工程操作、材料字段、`field-edit-input`、`field-edit-save`、
`settings-vectorcraft-executable`、`settings-vectorcraft-port`、
`settings-vectorcraft-save` 以及 `settings-vectorcraft-browse`，还有编辑态的
`workcrate-canvas-preview`、`workcrate-canvas-edit`、`workcrate-canvas-zoom-in`、
`workcrate-canvas-zoom-out`、`workcrate-canvas-undo`、`workcrate-canvas-redo`、
`workcrate-canvas-handoff`、`workcrate-canvas-viewport`、
`workcrate-canvas-artboard`、`workcrate-canvas-object`、`canvas-x`、`canvas-y`、
`canvas-width`、`canvas-height`、`workcrate-canvas-apply-bounds`、
`canvas-text-input`、`workcrate-canvas-set-text`、`workcrate-canvas-create-text`、
`workcrate-canvas-links-update`、`workcrate-canvas-links-relink`、
`workcrate-canvas-place-link`（链接图片）与 `workcrate-canvas-place-embed`（添加图片）。

## MCP

工具由全局 McpToolRegistry 拥有生命周期，名称一律带 `workcrate-ui.` 前缀。
标准控制工具：`navigate`、`open_board`、`manage_project`、`create_page`、
`update_page`、`place_image`（任意官方受支持的设计文件）、`field_presets`、
`configure_vectorcraft`、`refresh_project`、`attach_document`、`open_editor`、`state`。
工程创建与克隆操作需要一个用户选择的原生文档文件夹。
配置仅持久化 executable/CLI/port 值；浏览不会连接或启动 VectorCraft。
`navigate`、`open_board` 与 `state` 读取缓存的本地工程、画板与画布数据——
它们绝不启动官方程序。Full/逐次确认工具包含 `delete_project`、`export_pdf`、
`export_pages`。

画布工具驱动与编辑态同一个官方会话，因此通过 MCP 发起的调用与面板里的点击
落到同一个 `.vectorcraft` 文件和同一个运行中的程序：`canvas_open`（画板 id）、
`canvas_state`（读取官方快照）、`canvas_gesture`（官方指针事件）、
`canvas_set_bounds`（`object.setBounds`，`reference:0`）、`canvas_set_text`、
`canvas_create_text`、`canvas_place`（`path`、`at`、`link`——必要时扩大会话被授予的文件夹）、
`canvas_links`（`update`/`relink`）、`canvas_step`（`undo`/`redo`）与
`canvas_close`（提交、关闭，然后重读目录）。每个会变更的画布工具都要求
`confirm=true` 并返回官方快照；它们不接受原始官方命令 id 或任意参数。
确切 JSON schema 见 src/mcp.rs 与 src/mcp/{operations,editing}.rs。
原生服务调用通过阻塞工作线程运行。
过时的合成文本/排印/链接/替换工具已移除。

## 限制与验证

原生创建、同步、画布编辑与
导出都必须有可用的官方 CLI；只有 `open_editor` 会启动 GUI 窗口，编辑态只在控制端口
已经应答时才会连上正在运行的 GUI。当官方程序不可用时，
缓存的工程与预览仍可浏览。多工程 PDF 受官方导出限制并被 Export 拒绝。
画布编辑会即时写回原生文档，所以 `refresh_project` 只需重读目录与预览；
依旧不存在内嵌的第二个编辑器。`type_text` 就地输入只存在于实时通道，
因此无头通道改为用 `text.create`/`text.setText` 新建或替换文本。
原生对话框与布局验收需要同进程 FileAtlas MCP 验证；编译并非
运行时证据。本轮实测覆盖与未覆盖项见
`docs/validation-ui/2026-10-21-workcrate-canvas-editing/README.md`。
