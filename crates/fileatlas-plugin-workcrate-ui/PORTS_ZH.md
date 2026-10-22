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
document 与 message，外加 `canvas`：busy、zoom、`state`
（官方 `CanvasSnapshot`：session_id/live/tool/revision/dirty/selection/selection_bounds/
artboard/preview/preview_size/last_step，会话未开时为 null）、`object`
（官方节点详情：id/kind/name/bounds/text/linked/status/link_path）
以及指针手势进行中的 `drag`（kind 为 `move` 或 `handle<N>`、origin、start、current）。
视图丢弃时会清空它。页面编辑器是单一界面：显示页面即自动打开官方会话
（`ensure_canvas_session`），原先的预览/编辑双模式已移除；回到页面列表或工程
列表仍照旧提交并关闭会话。已有的埋点在
`workcrate.control.<id>` 下度量控件，包括画板、检视器、页面预览、
工程操作、材料字段、`field-edit-input`、`field-edit-save`、
`settings-vectorcraft-executable`、`settings-vectorcraft-port`、
`settings-vectorcraft-save` 以及 `settings-vectorcraft-browse`，还有编辑态的
`workcrate-editor-rail`、`workcrate-canvas-zoom-in`、
`workcrate-canvas-zoom-out`、`workcrate-canvas-undo`、`workcrate-canvas-redo`、
`workcrate-canvas-handoff`、`workcrate-canvas-viewport`、
`workcrate-canvas-artboard`、`workcrate-canvas-object`、`canvas-x`、`canvas-y`、
`canvas-width`、`canvas-height`、`workcrate-canvas-apply-bounds`、
`canvas-text-input`、`workcrate-canvas-set-text`、`workcrate-canvas-create-text`、
`workcrate-canvas-links-update`、`workcrate-canvas-links-relink`、
`workcrate-canvas-place-link`（链接图片）、`workcrate-canvas-place-embed`（添加图片），
以及编辑态页面栏每一页的 `rail-<board_id>`。

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

## 2026-10-22 预览与编辑优化

编辑结果发布到现有版本 1 预览缓存，无存储迁移。显式打开编辑后，预览/编辑
切换复用官方会话，左侧缩略图同步当前帧；离开页面/项目提交关闭。初次浏览
仍只读本地。项目无自选主图时显示首个缓存页面；同项目后台加载不清空旧图。
右侧支持多行文字、字体/字号、左/中/右对齐，全部转发官方 text.setStyle。
新增 Full + confirm 的 canvas_text_style（id、可选 font/size/justify）；格式输入
为空表示保留属性，不代表混合字体的当前值。几何拒绝非有限数，异步选择文件
后复核页面/选区。公开签名、控件与限制以同目录 PORTS.md 最新节为准。

## 2026-10-22 单字段保存

检查器保存走 `save_field(id, key, value, label)`，它刻意不复用 `perform`：不置
`busy`、不重载工程、也不清空左侧页面栏。每条在途草稿记录在 `field_saves` 的
`(页面 ID, 字段键)` 里，只渲染成该行自己的“保存中”标记。保存被串行化：只要
`field_saves` 非空，新的保存、切页、画布模式切换与画布任务都会被拒绝。面板状态把
它导出为 `field_saves: ["<boardId>#<key>"]`，与 `editing: "<boardId>#<key>"` 并
列——所有编辑行共用 `field-edit-input`、`field-edit-save`、`field-edit-cancel`
三块几何，因此只有这两个键能证明当前打开的是哪一行。

任务从 Boards 重读该页记录，用 `management::apply_field` 改一个字段后调用
`update_info`。在此之前先复用该页已持有的官方会话（`canvas_holds`），没有会话时
用 `canvas_open_quiet` 静默预热，所以首次保存自己打开静默 headless 会话、后续保存
继续复用，页面始终停留在预览态。官方保存帧的解码与选中对象查询都在同一条后台线程
完成（`canvas_frame`），UI 线程不再打开 PNG。`apply_field_save` 只补这一张卡片：
带回的官方帧直接进覆盖层（此时 `boards[].canvas_revision` 等于会话 revision），
否则只把该页放进预览队列。内容未变（`changed:false`）什么都不排队，并只关闭本任务
自己预热的会话。被拒绝的写入保留草稿行并显示 `保存失败：…`，文案取自 Boards 的中
文提示，内部角色名不会出现在提示里；`已保存：<字段>` 的字段名由
`management::field_label` 提供。预热保存应用帧后会启动画布探针，官方程序侧之后的
改动仍能回到该页。

## 2026-10-22 检查器卡片与稳定行高

检查器分为两个 `section_card`：「页面信息 · 画板规格」与「页面内容 · 同步物料栏」。
每个字段行固定 32 px；草稿只把该行的值替换为共享 `field-edit-input` 加“保存/取消”，
因此打开、提交或取消草稿都不会推动下方字段或改变面板高度。`begin_edit` 会聚焦输入
框，其 `PressEnter` 订阅负责提交。面板页脚固定 24 px 状态行（忙碌/保存中/最近消
息），保存反馈不再改变内容高度。新增参数动作是整行的 `page-add-field`，文案为
“＋ 新增物料参数行”。

## 2026-10-22 预览自动获取

`view/autosync.rs` 维持一个空闲观察器（`refresh::poll`，1500 ms）：只有在面板不
忙、没有画布会话（也不在进入画布）、没有字段保存在途且没有未完成的状态读取时，
才向 Boards 查询 `preview_state`。戳未变化时既不发布状态也不启动进程。戳过期时
在后台执行器启动一次 `auto_refresh_project`；失败保留已有缓存预览，记录
`auto_sync.error`，只显示一次“自动同步失败：…”，30 秒后重试。成功后只补丁
`boards_view`，不整项目重载：已解码图像继续作为占位，`load_generation` 使旧解码
失效，选中页优先排队，页面仍存在时保留选择与编辑态，不打开官方编辑会话，底部
显示“已自动同步 VectorCraft 的更改”。状态 JSON 新增
`auto_sync { syncs, pending, error }`。画布会话打开期间观察器保持空闲，因为该页
由既有画布探针负责；离开画布时再刷新目录。这条路径绝不驱动、聚焦或激活用户的
GUI，并且只读取已保存文件。

## 2026-10-22 编辑区内切页

页面编辑区持有官方会话时，切换页面不再重开连接：`switch_canvas_page` 对选中页调用
`canvas_open`，由 Boards 把同一个官方进程移到该画板；`session_id` 不变，编辑/预览
模式保持，落地的帧只补丁该页卡片（`boards[].canvas_revision`）。左侧页面栏与
`workcrate-ui.open_board` 都走这条路径。只有选择另一个工程、返回页面列表/工程列表
或关闭面板才提交并关闭。若切页请求到达时渲染或字段保存正占用会话，会在其结束时补
做（`apply_canvas_frame`、字段保存完成回调）；进入编辑模式也会向当前选中页收敛。
新增原生页面仍属于目录事务，会先保存并关闭预览会话，再由编辑器重新打开。页面栏条
目导出 `workcrate.control.rail-<board_id>` 几何供指针实测。证据见
`scripts/validate-workcrate-page-switch.cjs` 与
`docs/validation-ui/2026-10-22-workcrate-page-switch/README.md`。

## 2026-10-22 编辑页局部刷新

编辑区打开时，普通动作不再整目录重载。状态监听器比较
`(selected_project, selected_board)` 与两个刷新修订：持有会话时的页面选择走
`switch_canvas_page`，目录修订清空画布，仅页面修订变化时调用
`refresh_visible_board`（复用或移动同一会话，只重绘该页卡片），只有选择或目录
变化才 `reload`。`projects.wake` 由 `deliver_project_change` 处理：重读
`projects(true)` 与 `field_presets()`，无变化返回 `false` 不发布，有变化只补丁
`items`/`preset_fields`，不碰卡片、预览与会话；所以画布内放置引用与重新链接不再
触发重载。`CanvasFrame` 自带页面记录（`board: Option<WorkCrateBoard>`），
`apply_canvas_frame` 直接补对应卡片，不再为取页面记录多读一次目录。项目改名原地
更新该行；导出只提交/关闭必须关闭的画布，不重载；放置只重读被放置页。
`perform` 只保留给确实改变目录或必须关闭会话的动作：新建工程/页面、关联文档、
同步、工程归档/复制/封面/删除、离开画布与画布错误。

`ui.reloads` 统计整目录重载次数，MCP 可断言局部动作前后计数、`session_id` 与
页面 ID 列表不变。`WorkCratePanel` 仍是单个 GPUI 视图，任何 notify 都会重建元素
树；本轮去掉的是目录数据重载及其重新解码/排队，不是元素级增量绘制。自动同步仍是
整目录换卡片（保留已解码图像、计数不变），因为原生文档没有逐页内容修订号。
证据见 `scripts/validate-workcrate-local-refresh.cjs` 与
`docs/validation-ui/2026-10-22-workcrate-local-refresh/README.md`。

## 2026-10-22 字段保存期间不再整体置灰按钮

`WorkCratePanel::action` 的 disabled 不再包含 `field_saves`；保存进行中点击
按钮只设置提示"正在保存页面内容，请稍候…"并拒绝执行。所有动作处理器本就在
`field_saves` 非空时拒绝新动作，未新增门控。`workcrate-ui.state` 中的
`renders` 计数统计整视图渲染次数；截图逐行对比显示打字仅改约 58/1831 像素行，
修复后保存中一帧约 74/1831 行（修复前 840/1831——所有按钮同时置灰）。把输入框
包进子视图宿主无法限制 GPUI 的 dirty 向祖先传播，该尝试已回退。

## 2026-10-22 画布命令执行期间不再整体置灰

`canvas_action`、位置/尺寸数字输入框与工具栏动作不再因 `canvas_busy`
禁用。结构性禁用条件保留（无画布、"更改文字"需要文字对象、"链接图片"需要已
链接对象）；`busy || canvas_busy` 时点击提示"画布操作进行中，请稍候…"。
`WorkCratePanel::action`（画布工具栏缩放/撤销/重做与顶部工具栏）同样去掉
`canvas_busy` 禁用并使用相同点击提示。数值草稿由应用按钮的拒绝逻辑兜底，
不再靠输入框禁用。

## 2026-10-22 页面编辑器合并为单一界面

预览/编辑模式切换已移除：`WorkCratePanel` 不再有 `canvas_mode`，
`toggle_canvas_mode` 与 `canvas_mode_button` 删除，
`workcrate-canvas-preview` / `workcrate-canvas-edit` 控件不复存在。
显示页面（页面栏点击、恢复、远端选择或快照落地）即运行 `ensure_canvas_session`，
以与原"编辑"按钮相同的方式打开官方会话；自动同步、字段保存与预览监视器
仍只在 `canvas.is_some()` 时让位。编辑器工具栏常显缩放/撤销/重做，并把原
预览专属的"放置设计文件""导出本页图片"与 `workcrate-canvas-handoff`
并列；右侧栏（`workcrate-editor-rail`，310px）纵向堆叠画布对象面板与页面
物料信息面板，`inspector` 与 `canvas_object_panel` 只渲染内部内容。
`render_editor` 中原本地位图预览分支已删除——画板始终是官方画面。
`workcrate.ui` 状态不再包含 `canvas.mode`。

## 2026-10-22 画布角点拖拽等比缩放

`CanvasDrag` 把角点手柄（0/2/5/7）的采样点引导到过对角锚点的等比路径上
（`constrain_corner`：主导轴决定伸缩比，双轴同时越过锚点整体翻转；边中点手柄
保持单轴自由）。官方引擎仍执行命中测试与缩放——只有采样点被改变。之所以在
本侧实现：安装的 vectorcraft-cli 0.14.0 在 headless 会话中忽略
`pointer_gesture` 的 `mods` 参数（四种写法实测均为自由缩放）。视觉预测与转发
手势共用同一约束点。

## 2026-10-22 画布接受拖入的图片

画布视口（预览与编辑模式）对外部 `ExternalPaths`、`FileManagerFileDragPayload`
（仅 `allows_drop()`）与 `FileStagingDragPayload` 注册 `on_drop`/`drag_over`，
三者都汇入 `canvas_drop_files`：按扩展名白名单过滤图片，非图片提示
“拖入的内容里没有可放置的图片。”，忙碌时提示“画布操作进行中，请稍候…”，
每张图片走
`canvas_place_dropped(board, path, centre+24·i)`——会话未开则先静默开启；官方
单读根无法覆盖的来源先复制进 `<文档文件夹>/materials`（见 boards PORTS）
再链接放置副本。拖拽悬停在视口上时边框以 `accent_hover` 高亮。

因 GPUI 0.2.2 封闭了 `ExternalPaths` 的构造，MCP 验证走语义等价通道
`control_ui {action:"canvas_drop_files"}` → surfaces 键
`workcrate.canvas.drop-request`，由面板预览轮询（1.5s）消费进同一个
`canvas_drop_files`；`ui.drops` 计数接受的拖放请求。`boards.canvas_holds`
在会话未开时现返回 `Ok(false)`（原先报错，与函数名相悖；原有两个调用点
都先判 `canvas_is_open()`，兼容）。验证记录：
`docs/validation-ui/2026-10-22-workcrate-canvas-drop/README.md`。

## 2026-10-22 项目管理面板重设计：共享三层入口带与统一图标按钮

Tier 1 项目管理界面按 `docs/reference-ui/concepts/
workcrate-projects-user-2026-10-21.png` 落地：居中三层入口带
（`WorkCratePanel::tier_band`，三屏共用）、hero 头部＋统一搜索框＋排序/归档
图标开关＋primary 新建工程；项目卡改为标题＋图标行（重命名/复制/删除/
更多菜单）＋“N 个画布 · 相对时间”＋页内信息条（取自 boards-cache 首页摘要）＋
预览说明＋“.vc 关联工程 / 进入页面矩阵”页脚；底部居中创建磁贴。全部颜色
来自原生主题与 `fileatlas_ui_bridge::components`，无本地调色板。

MCP 面：`workcrate-ui.state` 新增 `sort_by_name`/`show_archived`。页面/编辑器
的旧面包屑替换为同一入口带＋窄操作条（关联工程/新增 A3 页面/同步工程与预览，
带图标）；入口胶囊保留旧控制 id `workcrate-projects-home`/
`workcrate-pages-home`，旧场景仍可导航。页数与首页摘要在快照读取中从各项目
boards 缓存读取（不开官方进程）。相对时间为纯函数并带单测。验证记录：
`docs/validation-ui/2026-10-22-workcrate-projects-redesign/README.md`。

## 2026-10-22 页面管理与装配工作台重设计：统一图标按钮与分区卡片

Tier 2 页面管理按 `docs/reference-ui/concepts/workcrate-pages-user-2026-10-21.png`
落地：hero 头部（eyebrow、项目名标题、“共 N 个独立物料 · 已选中 M 个”统计）＋
多选图标开关（ListChecks）、全选/取消全选 ghost、关联工程/同步工程与预览
outline、primary 新增画布；页面卡为常显复选框图标（Square/SquareCheck，勾选
即开启多选）＋规格行＋物料信息条（无字段不显示）＋预览（打开/多选切换语义
不变）＋“第 N 页 · 编辑”页脚；导出页脚含 PNG/JPEG 切换、导出选中页为图片
（FileImage）、导出全部为 PDF。窄操作条仅在编辑器屏显示。

Tier 3 编辑器按 `workcrate-studio-user-2026-10-21.png` +
`workcrate-inspector-2026-10-21.png` 优化：画布工具条统一图标控件——添加图片
（ImagePlus）、新建文字（Type）、ZoomOut/ZoomIn、Undo2/Redo2（新
`WorkCratePanel::icon_action`，与 `screen_action` 相同的保存/画布忙碌守卫；
原纯文字 `action` 助手删除）、放置设计文件（FilePlus）、导出本页图片
（FileImage）、primary 在 VectorCraft 中编辑（ExternalLink）；左栏为编号迷你
页卡（accent 编号＋截断标题＋物理尺寸行，活动卡 accent 边框）；对象面板与
页面检查器统一为 `section_card` 分区卡片（画布对象·官方选区、位置与尺寸·
官方 bounds、文字/文字内容·官方文本、链接图片·外部文件、文字格式·字体与
字号、画板·添加图片、页面信息·画板规格、页面内容·同步物料栏）。画布编辑
语义（手势、映射、拖放、官方会话）零改动。

全部旧控制 id 保留（`pages-multi-select`、`pages-select-all`、
`pages-clear-selection`、`pages-exit-multi-select`、`select-page-{id}`、
`open-page-{id}`、`page-preview-{id}`、`export-*`、`rail-{id}`、`field-*`、
`preset-*`、`page-add-field`、`workcrate-canvas-*`、`place-design-file`、
`export-current-image`、`workcrate-canvas-handoff`），既有场景脚本无需修改。
验证记录：`docs/validation-ui/2026-10-22-workcrate-pages-studio-redesign/README.md`
（场景 `scripts/validate-workcrate-pages-redesign.cjs`）。
