# FileManager 插件端口 / 文件管理窗口数据对接

## 文件变更刷新保留视口（2026-10-20）

共享的 `start_file_operation` 完成路径在 `refresh=true` 时使用 `refresh_index()`，
包括分组状态下的文件拖放。一次完成的文件变更会停留在当前浏览范围内：在读取新快照
期间保留已渲染的行/富媒体，保留当前的逻辑滚动偏移和有效选区，并让现有的完成/索引
事件路径去协调已变更记录。它不得调用导航/过滤的 `reload()` 路径，该路径会清空行和
ListState 并要求富媒体窗口偏移归零。现有的逐项结果反馈与选区修复规则保持不变。
没有持久化/架构或文件系统操作层面的变更。当完整目录快照已经可见时，monitor 在同目录
watcher 替换结果仍在加载期间会保留该快照。初始导航/不同目录仍会发布它们的批次；
完成的替换和错误仍然可以送达。这可以防止重新扫描过程中瞬时的短清单缩小 ListState
并把滚动钳制为零。回归检查：`scripts/validate-drop-scroll.cjs`；
同进程证据：`docs/validation-ui/2026-10-20-drop-scroll/README.md`。

## 子文件夹分组拖放目标（2026-10-20）

已被下方的分组边框更新取代：原先仅标题栏可用的目标已移动到一个包裹标题和媒体内容的
通用边框内。单张拖拽卡片携带其媒体 ID；当拖拽多张已选卡片中的一张时，携带整个当前
选区。标题路径被用作目标目录，FileManager 调用
`FileOperationsService::bulk_move_into_folder`，保留冲突处理、逐项结果、索引事件、
刷新和选区修复。其他分组模式会忽略标题上的文件拖放。MCP 的 `control_file_manager`
动作 `move_files_to_subfolder` 在隔离的实时进程中驱动同一个 FileManager 操作，
需要唯一已索引 ID 加上一个当前可见的目标分组。它是一条测试/控制路径；常规使用
仍然是指针拖放。服务路径的初步证据见
`docs/validation-ui/2026-10-20-subfolder-drop/README.md`。

媒体卡片从 `fileatlas-contracts` 发布共享的 `FileManagerFileDragPayload`，
其中包含路径、已选 ID，以及基于实时修饰键状态的拖放资格。File Staging 接受该
payload 并添加这些路径而不移动源文件。FileManager 到 staging 的入站指针拖拽已通过
同进程 MCP 验证，见
`docs/validation-ui/2026-10-20-file-staging/README.md`。

## 通用分组边框与指针拖放验证（2026-10-20）

`view/group_frame.rs` 负责分组边界、共享拖拽悬停和文件放置。
`month_header` 只负责标题/折叠控件。每个虚拟 FileManager 行都保留 `group_label`
和 `group_end`，包括没有标题的行。相邻片段共享同一目录标识、连续边框边缘和悬停
状态，同时保留行虚拟化和 200 条记录的富媒体窗口。Subfolder 边框在标题、卡片子元素、
行间隙、尾部空白和延续行上都接受文件；折叠的分组保留其边框目标。其他分组模式有边框
但没有文件移动处理器。原生主题角色提供边框和拖拽反馈。没有新的存储、数据库架构
或文件操作语义。

Subfolder 模式让整个 FileManager 列表视口拥有一个父文件夹的拖放兜底目标。位于已绘制
子边框内部的指针解析到该子项；空白网格区域解析到 `browser.directory`，包括父目录
没有直属媒体分组的情况。父级目标悬停使用原生选中角色。仅在 Subfolder 模式下抑制
卡片的平台 OLE 交接，以便 GPUI 能够路由内部子项到父项、子项到子项的拖拽；其他布局
保留原生文件拖出。

当 FileManager 视图获得焦点时，Ctrl+C 将当前选中的媒体 ID 存入该列的内存复制缓冲区。
Ctrl+V 通过 `FileOperationsService::bulk_copy_to_folder` 把它们复制到鼠标所在的子分组
下，或当指针位于其他空白网格区域时复制到当前目录。该缓冲区按视图/列独立存在，不是
操作系统剪贴板集成。MCP UI 快照暴露 `file_copy_buffer_count` 以供同进程交互检查。
运行时证据：`docs/validation-ui/2026-10-20-file-keyboard-parent-drop/README.md`。

MCP 的 `get_ui_state.file_manager.group_frames` 诊断把当前绘制的虚拟行索引映射为
逻辑客户端坐标下的 `{path,first,bounds:[x,y,width,height]}`。这是瞬时的绘制几何信息，
在 render 与 prepaint 之间可能短暂为空，且不会触发读取或通知。bounds 在 render 时
清除并由边框画布重新发布。消费者必须等待匹配的可见几何信息，而不是跨快照变更复用
坐标。

`control_file_manager {action:"debug_internal_file_drag",enabled:true|false}` 会排队
`FileManagerUiRequest::DebugInternalFileDrag(bool)`。此调试入口需要 Standard MCP 访问级别
以及显式设置的 `FILEATLAS_DEV_TEST_DB` 隔离配置档。它仅抑制卡片的平台拖出交接，
使 MCP 合成的 GPUI 指针事件能够演练真实的类型化卡片拖拽和边框放置处理器。状态仅存在于
内存中，按列独立，默认为 false，并在主面板上以 `file_manager.debug_internal_file_drag`
报告。它不是保存的产品设置。常规原生 OLE 交接仍需物理 Windows 输入覆盖。实时指针证据
及剩余限制：`docs/validation-ui/2026-10-20-group-frame-drop/README.md`。

同一个子文件夹边框还接受来自 staging 面板的共享 `FileStagingDragPayload`。
这遵循普通的 FileManager 放置契约：释放时立即移动可用的已暂存路径；不显示
复制/移动选择框。FileManager 调用 `FileOperationsService::move_paths_to_folder`，
刷新当前视图，报告逐路径失败，并发出 `FileStagingPathsMoved` 以便 staging 功能
移除成功的路径。其他分组模式忽略此 payload。Staging 拖拽 payload 是内部 GPUI 数据，
不会启动原生 OLE 拖出。

## 独立的 FileManager 列（2026-10-19）

FileManager 发布 `file_manager.column.0..3.content` 和匹配的
`file_manager.column.0..3.address` UI 视图插槽。每列惰性拥有自己的
`FileManagerView`、`FileManagerService` 浏览 worker、过滤/搜索输入、选区、
滚动状态和瞬时 UI 请求。服务 fork 共享数据库、索引、watcher、分组和 Everything
提供者，同时让浏览器快照和导航版本保持本地。每列的地址控件导航自己的路径/文件夹/
标签状态。Navigation 插件的文件夹/标签选择事件由 Shell 路由到活动列。只有列 0 写入
遗留的单视图 `SessionService` 记录，避免产生相互竞争的会话写入者。列状态由宿主通过
`ProgramWorkspace` 持久化；FileManager crate 不拥有 workspace 存储。

`file_manager.column.{i}.address` 把面板路径渲染为一条连续的路径链：祖先面包屑位于
`file_manager-column-{i}-address-trail` 行中，该行按内容定尺寸，并在面板过窄时收缩/
横向滚动；当前文件夹面包屑（`file_manager-column-{i}-crumb-current`）保持固定在其右侧
且永不收缩，尾部间隔块把编辑按钮保持在行的最右端。这里刻意不使用 gpui-component 的
`overflow_x_scrollbar` 包装器，因为它会强制容器 `size_full()`，导致当前文件夹与祖先
脱离。点击任何面包屑都通过 `open_column_path` 路由；同一处理器也服务于当前面包屑。见
`docs/validation-ui/2026-10-19-column-address-breadcrumb/README.md`。

每个面板分别发布 `command.column.{i}.filters`、`.view-modes`、
`.view-options`、`.search`、`.size`、
`.tools`、`.status`、`.modal` 和 `.inspector`；宿主应显示活动索引对应的
控件/状态/模态/inspector 插槽。当前限制：全局 Navigation 侧栏仍向列 0 发送事件；
需要宿主侧的活动面板路由，侧栏选择才能跟随活动面板。部分 inspector/文件操作仍使用
全局 feature 服务。

宿主组合活动面板的 `.view-modes`（grid/list）、Columns 插件的 `command.columns`
布局下拉框，然后是 `.view-options`（带状态的子文件夹开关/缩略图缩放）。过滤器
保持为独立可滚动区域。两个投影复用同一面板实体和订阅；不复制任何浏览状态。
子文件夹图标关闭时使用 ToggleLeft，开启时使用 ToggleRight。
启用状态的子文件夹按钮使用主题拥有的 primary 变体以获得持久的强调色背景；关闭时
使用普通 ghost 呈现。

## MCP 贡献（2026-10-19）

FileManager 通过宿主发布的 `McpToolRegistry` 贡献 `file_manager.search_media` 和
`file_manager.get_media`。插件拥有它们的 JSON Schema，使用 `deny_unknown_fields`
解析请求，应用分页/标签/类型/排序限制，并调用自身 `FileManagerService` 的方法。
`get_media` 返回 FileManager 媒体行及其分配的 FileAtlas 标签。Cordis 在
`fileatlas.file_manager` 停止时移除这两个工具。

## Everything 文件名/路径搜索（2026-10-17）

FileManager 可选读取 `EverythingSearchService`（`fileatlas/everything-search`）
用于文件名/路径文本搜索。适配器把当前目录查询限定到该目录，把集合搜索限定到已注册
根目录，并在没有根目录时搜索整个 Everything 索引。`FileManagerService::prepare_everything_search`
在快照和分页读取中被调用，这些读取本就运行在 GPUI render 路径之外。候选路径只是预过滤；
SQLite 会重新应用勾选的文件名、路径和标签范围，并保留库、类型、扩展名、日期、评分、
分组、排序和分页行为。浏览器模式通过 `cache_search_files` 物化真实的 SDK 命中；
因此未访问过的嵌套文件可以在没有递归扫描的情况下出现。搜索清单仅存在于会话中。
普通/扩展 Windows 路径复用现有行标识并保留标注。适配器缺失、IPC 不可用、平台不支持
或结果超限均使用 SQLite 回退。搜索可以准备一个已安装的兼容后台客户端。索引加载完成后，
一个就绪版本变更会让现有 monitor 重试当前搜索；空闲检查不会发通知。搜索栏使用 ReadModel
进行 EXE 安装/版本读取，显示安装/兼容版本提示，只打开官方下载页面，并提供显式重新检查。
见 `../fileatlas-plugin-everything-search/PORTS.md` 和
`docs/validation-ui/2026-10-17-everything-search/README.md`。

## 原始文件查看器替换 — 2026-10-16

本节取代下文所有历史性的 Preview/MediaPlayback 引用。
FileManager 现在要求 `QuickLookService`（`fileatlas/quicklook`）而不是被移除的
PreviewService 和 MediaPlaybackService。Space、Enter、保留的自定义打开键、文件双击、
上下文菜单 Preview 和 inspector 缩略图点击，会把所有普通文件类型发送到已安装的
QuickLook。调用在阻塞 worker 上运行，一次只有一个待处理的 FileManager 启动；
按住/修改的 Space 不会反复切换。失败时打开共享的 GPUI 告警，包括在 QuickLook
缺失时给出本地化的安装指引。没有内嵌查看器、filmstrip、解码器、播放传输、
安装流程或默认播放器回退。

`FileManagerUiRequest::Preview(i64)` 保留其公开名称，现在调用同一个处理器。
`preview_items_by_ids` 连同其 filmstrip 消费者一起被移除；
`media_paths_by_ids` 和 `media_path_pairs_by_ids` 为其他操作保留。
FileManager 文件变更保留选区修复，不再推进/重新打开已删除的查看器。缩略图生成、
标签预览快照、inspector 预览、评分、标签、EXIF、文件历史、拖放和文件操作保留其
归属权。

AI 建议从被移除的 Preview crate 迁移到 FileManager 独立的
`AiTarget` 和 `file_manager/ai-suggestions-view` 贡献。显式的 Suggest Tags
选择一个目标并请求推理，而不打开 QuickLook。文件浏览时的自动分析遵循现有的
`settings.ai.auto_analyze` 标志。轮询仍使用共享刷新，并且仅在实际变更时通知。
被动的模型文件可用性使用带版本键的 ReadModel，在目标变化和下载完成时刷新；
render 和 timer tick 不会反复 stat 模型文件。显式的 Analyze 保留全新的操作时校验。

运行时限制与命令：`../fileatlas-plugin-quicklook/PORTS.md` 和
`../../docs/validation-ui/2026-10-16-quicklook/README.md`。

## 上下文 inspector 与隔离视觉夹具 — 2026-10-16

inspector 根据实际选区和当前 FileManager 快照投影 Program / Tag / Folder / File。
`FileManagerUiSnapshot::inspector_context` 发布
`program|tag|folder|file`。清空选区时显示 Program，同时保留活动路径和计数。
内部 Navigation 动作版本变更即使选择了相同目标也能恢复文件夹或标签上下文；
后台数据变更会保留显式的取消选择。不引入平行的导航模型。

标签重命名/外观/显示操作使用 NavigationService 并保留其现有的后代行为。
`FileManagerUiRequest::InspectorTagName(String)` 为 MCP 验证填充活动名称 InputState，
但不保存。展开的编辑器按顺序排列名称、颜色、图标、显示和一个 Confirm 按钮。
颜色/图标/显示控件更新视图拥有的草稿；Confirm 重新校验目标，并且仅对已变更字段
调用现有的重命名/外观/显示服务。这些是依次执行的现有命令，不是新的原子 DB 事务；
失败使用操作状态并重新加载模型。仅修改名称/颜色/图标时，未变更的显示标志不会
重新应用后代可见性。图标选择使用 14 个现有主题标识符；未知的已持久化图标/颜色
会被保留直到该字段被修改。文件夹统计来自当前浏览投影；未打开的子文件夹显示
未知计数而不是虚构的 0。

历史与全局撤销/重做是分开的。早期视觉夹具展示于
`docs/validation-ui/2026-10-16-inspector-complete/README.md`，它是有界的
内存内演示，现在仅作为历史证据。

## Lore 支撑的文件历史 — 2026-10-17

File > History 页面现在通过 `VersionsService` 读取和写入实际的 Lore 修订版本；
版本标题、父修订、时间戳、大小、预览路径和活动修订状态由独立的
`VersionInfoService` 持有。创建时需要输入标题。切换时会拒绝丢弃未提交的源文件或
Lore worktree 变更。成功后，FileManager 刷新由文件系统派生的媒体元数据和图片缩略图，
同时保留评分/标签/拍摄元数据。全局撤销/重做仍是独立的动作历史。

MCP 暴露 `list_file_versions`、`create_file_version` 和
`activate_file_version`。写入保持在禁用状态，除非在 MCP 进程中显式启用。
服务标识、存储和限制见 `../fileatlas-plugin-versions/PORTS.md` 和
`../fileatlas-plugin-version-info/PORTS.md`。在对同进程 FileAtlas MCP server
实际执行之前，Windows 运行时/MCP 演练仍未验证。

失败使用现有的操作状态、Navigation 错误、缩略图回退和 read-model 错误。
实现位于 `src/view/inspector/`。设计与证据：
`docs/reference-ui/concepts/right-inspector-complete-2026-10-16-v2.md`
和 `docs/validation-ui/2026-10-16-inspector-complete/README.md`。

用户后续要求取代了紧凑的历史标识：所有 File 标签页现在共享同一
`inspector_preview`（180 逻辑像素图片、文件名、评分）、头部和固定标签行。
只有可滚动正文变化。最新设计/证据：
`docs/reference-ui/concepts/right-inspector-unified-edit-2026-10-16.md` 和
`docs/validation-ui/2026-10-16-inspector-unified-edit/README.md`。

当前浏览器运行时取代了下文较旧的已索引库描述；见末尾的 2026-10-15 更新和
`docs/validation-ui/2026-10-15-browser/README.md`。

状态：查询/窗口服务以及有源码依据的 FileManager 交互已实现，包括虚拟化渲染、
过滤器、选区、预览路由、文件操作和批量操作。C0–C4 编码延续变更已写入但未验证；
Windows 行为和完整视觉一致性仍未验证。

## 生命周期与依赖

- Cordis 插件名：`fileatlas.file_manager`；服务：`fileatlas/file_manager`（`FileManagerService`）。
- 需要 `DatabaseService`、`FileOperationsService`、`DragDropService`、`MediaPlaybackService`、`PreviewService` 和 `SessionService`；在 Cordis 拥有的 UI bridge 中注册其视图/贡献。
- 可选消费 `HistoryService`、`SettingsService` 和 `TaggingService`。FileManager 在没有这些提供者时也能启动；当提供者缺失时，历史快捷键/标签热键/可配置打开方式不启用。
- 观察 `FolderSelectionChanged`、`TagSelectionChanged`、`DirectoryPathSelected` 和 `MediaIndexChanged`。

## 输入 / 输出

- `load(MediaQuery)` 返回 `FileManagerSnapshot`：紧凑有序骨架、总数、初始富媒体窗口（最多 200）、批量媒体标签、用于命名活动标签过滤器的完整标签树、可用扩展名、文件夹计数和保留的查询。
- `ensure_window(snapshot, range)` 在当前窗口已覆盖 range 时返回 `None`，否则返回带有偏移、媒体行和批量标签的 `FileManagerWindow`。`load_page(query, offset)` 加载接下来的 200 行。
- `media_by_id(id)` 解析一个已索引行；`open_count(id)` 读取原始的提示性打开计数器；`apply_rating(ids, rating)` 持久化与源码兼容的钳制评分。过滤器映射位于 `filter.rs`；选区行为位于 `selection.rs`。
- `Preview` 按钮、Enter 和卡片双击通过 `PreviewService` 打开所选项。其 filmstrip 遵循完整的过滤后媒体序列：图片在 Preview 中渲染，音频显示源码风格的深色舞台及文件名/评分/标签/默认播放器操作，视频交接给 `MediaPlaybackService`。左/右导航按查询顺序跨越全部三种媒体类型。Escape 和 `Return to file_manager` 关闭视频播放。`Open` 继续启动系统默认应用。
- 卡片在拖拽回调上调度 `DragDropService::start_file_drag`。已选卡片按选区顺序提供选中路径；未选卡片只提供自身路径。Ctrl/Shift 修饰的拖拽被抑制，与源文件 file_manager 一致。该服务专用的 OLE worker 避免重入 GPUI 的窗口过程，但收不到 GPUI 持有的当前指针手势，因此对外部 Windows 的投递目前会失败。GPUI 后端限制和验证状态见 drag-drop 崩溃后续记录。
- 所选项详情区域读取文件系统的只读/修改/创建值，格式化文件大小和图片尺寸/宽高比，显示持久化的打开次数、评分、拍摄日期和层级标签。文件系统元数据失败被视为字段不可用。
- 对于选中的图片，`Show EXIF fields` 惰性读取源 `kamadak-exif` 0.6.1 解析器的输出，先显示原始精心挑选的相机/曝光/GPS 字段，随后是完整的 EXIF 字段列表，并可应请求再次隐藏这些字段。选区变化会关闭 EXIF 区块，使新的文件不会在每次普通 file_manager render 期间被读取。
- GPUI 视图通过 `SessionService` 恢复并持久化源会话的搜索、排序、媒体类型、扩展名、日期、评分和标签过滤器。过滤器持久化会保留无关的导航/会话字段。搜索使用输入框；日期边界使用从已保存日期初始化的 GPUI Kit 日历日期选择器。活动的包含/排除标签过滤器显示为可单独移除的、带标签名称（或已删除标签标记）的胶囊。
- 扩展名过滤器点击遵循 `SessionState.ext_click_mode` 中持久化的源 Select/Exclude 模式；控件允许用户切换模式，每次点击在该模式下切换成员资格。包含/排除过滤胶囊移除各自的值，Clear 移除两组。
- 搜索范围默认值从可选的 `SettingsService` 加载；在 FileManager 中切换文件名/路径/标签范围会通过该服务持久化完整的范围集合。Settings 写入失败显示在 FileManager 状态行中。
- GPUI 视图仅当已保存的 file_manager 选中 ID 仍处于活动查询中时才恢复它们，并恢复已保存的像素滚动偏移。它持久化选区/过滤器变化、已变更的滚动偏移（每 200 ms 采样一次）以及视图被 drop 时的当前状态。更改查询会清除选区并把滚动返回到顶部。在下一个保存间隔之前被终止的进程可能丢失最新的滚动移动。
- 缩略图大小（100–300 px）、按日期排序的月份分组和折叠的月份区块从 `SessionService` 恢复并保存回其中；月份分组还读取并更新 `SettingsService.show_date_dividers`，包括从 Settings 面板做出的修改。默认缩略图大小为 160 px，月份分组默认开启。
- GPUI `uniform_list` 渲染可见媒体行外加两行超扫。其列数由当前窗口宽度和缩略图大小推导；列表为可见全局行范围请求 `ensure_window`，并且只保留服务的 200 行富媒体缓存。紧凑骨架继续驱动键盘导航和月份边界。FileManager 卡片在异步缓存图片后面显示缓存的 8×8 RGB 微缩略图。其操作菜单提供 AI Suggest Tags、Preview、用系统默认应用打开、在文件资源管理器中显示、层级标签分配、复制、移动、回收站删除确认和 0–5 评分。打开/显示和 AI 建议针对被点击的卡片；标签、复制、移动、删除和评分在被点击卡片属于选区时针对当前选区，否则只针对该卡片。标签变更使用 `TaggingService`，并在可用时记录源码形状的历史条目。Preview 和 AI 建议请求包含已索引媒体 ID；AI 操作通过 `PreviewService` 打开被点击的图片，其 AI 建议 surface 观察活动图片并启动推理。
- `open_media_menu` FileManager UI 请求为已索引 ID 渲染一个应用拥有的操作面板，包含 Preview、open/reveal、评分、标签、AI 建议、重命名、复制、移动和删除。它复用 FileManager 处理器并报告其活动 ID/打开状态；原生 GPUI 上下文菜单仍由指针触发。
- `FileManagerUiRequest::Preview(media_id)` 执行与双击相同的选中卡片预览处理器。它复用已加载的 `MediaItem` 以便立即打开，把当前富媒体窗口设置为第一个 filmstrip，并在后台解析完整的有序结果。若另一个媒体项已变为活动，则迟到的路径结果会被忽略。
- 可选的选区详情弹窗在其选中媒体 ID 上于后台 worker 一次性读取打开次数和文件系统权限/时间戳；重绘使用缓存结果。
- Statistics 打开源数据库查询并显示图片/视频总数、总存储量、评分分布、前十名标签和最近十二个媒体月份。查询错误显示在面板中。
- 当 TaggingService 可用时，FileManager 采样 writeback 失败登记表，并把每个元数据错误连同 Retry 和 Dismiss 操作呈现出来；200 ms 的服务 monitor 在异步写入完成或失败时通知视图。Retry 调用 `writeback::retry_failure(path)` 以排队确切的失败 payload；dismissal 调用 `writeback::clear_failure(path)`。
- 会话滚动偏移通过 uniform list 底层滚动句柄以像素存储。橡皮筋选择从空白 file_manager 区域开始，普通拖拽替换选区，Ctrl+拖拽向选区追加，普通点击空白区域清除选区。它通过 uniform list 的实测行高、视口边界和滚动偏移映射拖拽矩形，只考虑与可见矩形相交的行。AI 推理和建议分配通过 Preview AI surface 执行；非图片卡片请求会报告建议需要图片。
- Preview 异步接收活动查询的完整有序 `(path, media_id)` 序列，从紧凑 file_manager 骨架 ID 解析而来，因此图片/音频/视频导航在已加载富媒体页面之外仍保留媒体标识。已加载窗口提供即时首绘；解析期间被删除的行被跳过而不会错位路径/ID 配对。
- 配置的 `Settings.open_key` 在 FileManager 根获得焦点且无批量/删除确认活动时打开所选项（默认 `Space`；缺少 settings provider 时保留当前 `Enter` 行为）。F1–F9 设置映射为选中媒体切换其标签，与源应用一样在第一个选中项上测试成员资格；分配/移除通过 `TaggingService`，保留关联/祖先和元数据写入语义。成功切换在历史可用时记录源码形状的 `AssignedTags`/`RemovedTags` 条目。
- 在有媒体被选中时，FileManager 暴露一个可搜索的层级标签选择器。搜索匹配标签名称和完整路径；所选选择可以从所有选中媒体分配或移除，或清除所有分配。成功操作会重新加载结果，并在历史可用时记录 `AssignedTags`、`RemovedTags` 或预清除的 `ClearedTags` 历史条目。
- 成功的 FileManager 评分变更记录 `RatingChanged`，携带源码的代表性先前评分和钳制后目标评分；成功的单项重命名仅在已索引文件名确认变更后记录 `Renamed`。Ctrl+Z/Ctrl+Y 调用可选的 `HistoryService`；完成状态在 FileManager 中可见，并且撤销/重做后重新加载查询。
- 当前 FileManager 历史生产者包括标签热键和标签选择器、评分以及单项重命名。删除、复制、移动和批量文件操作不是源 `UndoEntry` 的变体，仍不可撤销。Settings 打开键支持依赖于适配器暴露的 GPUI 键名；Windows 焦点/快捷键行为仍需原生验证。

## 消费者与调试

UI 请求应使用可见范围并保留与快照配对的查询。过滤器变更重置选区/窗口状态；`MediaIndexChanged` 在保留仍然有效的选中 ID、锚点和像素滚动偏移的前提下重新加载快照。启动时的文件夹协调在行提交时发出此事件，因此添加到较早索引中的文件会出现在打开的 FileManager 中。当行或标签看起来过期时检查查询参数、偏移和媒体 ID。微缩略图占位符恰好接受来自数据库行的 192 字节 RGB；格式错误或缺失的 blob 回退到中性媒体占位符。Preview 插件提供源码风格的显示/隐藏信息覆盖层和缓存的 ffprobe 视频详情。EXIF 值和 Windows 文件属性/时间戳需要本地真实文件验证；Windows 视觉一致性仍未验证。

## 过滤器驱动的批量操作

file_manager 的 “Bulk actions…” 面板移植了源 `BulkActionState` 工作流：选择 Delete（回收站）、Copy to 或 Move to；范围匹配整个库、所选文件夹/路径、所选标签或当前选区；然后应用以 AND 组合的日期范围、早于/晚于天数、闭区间评分范围、文件名子串、扩展名和文件大小边界。它从紧凑 file_manager 骨架显示实时匹配计数，并在执行时使用同样的 ID。当前选区模式刻意跳过额外过滤器。每次打开都会把动作、范围和过滤输入重置为源默认值。Ctrl+Shift+D 仅在 FileManager 拥有焦点、无预览/删除确认活动且面板未打开时打开面板；它不能重置进行中的批量配置。Escape 关闭面板。批量删除需要第二次显式确认点击，绑定到第一次点击时的匹配 ID 快照；如果目标 ID 变化，新集合必须再次确认。FileManager 的选区在其查询重新加载时被清除，两次确认点击之间的选区变化会被 ID 快照检查捕获。批量复制/移动使用原生文件夹选择器和 `FileOperationsService` 逐项结果；取消选择器会关闭面板。移动到已注册根之外会让文件留在磁盘上并从库中移除其行，与 FileAtlas 一致。移动/删除完成后刷新 file_manager。单项复制/移动使用与源码兼容的保存文件选择器。

批量日期边界使用独立的 GPUI Kit 日历选择器并采用源的可选日期行为：每个边界初始未设置，选择日历中的某天后应用一个包含性的本地日历边界，相邻的清除按钮移除该边界。更改任一日期都会清除待处理的删除确认，因此必须重新查看实时匹配集合。当前选区的批量预览通过 `FileManagerService::media_by_id` 解析每个选中 ID，因此它们覆盖活动 200 行媒体窗口之外的选区，并对已删除的行给出报告而不是显示部分集合。原生文件操作行为、文件夹选择器取消、回收站行为、键盘焦点和批量匹配在集中的 Windows 验证完成之前仍未验证。

## 共享视觉 token

GPUI file_manager、卡片、统计和批量操作面板消费 fileatlas-ui-bridge::theme 以获取源码调色板表面、文本层级、危险提示、选中强调色以及感知背景的 base/modal/card alpha。Toolkit 文本大小预设和间距助手仍未映射到每一个源 token；Windows 视觉一致性未验证。

## 区域投影与 MCP UI 请求（2026-10-13）

一个 FileManager 实体被投影到 command.search/size/filters/tools、
file_manager.content、workspace.status 和 workspace.modal。投影观察同一个实体，
共享过滤器、选区、分页和会话写入。主内容不再包含重复的标题/搜索/选项/操作行。
统计和批量操作是浮动面板。选中详情/重命名/复制/移动/评分
仍可从底部状态栏的额外操作按钮获得。
缩略图大小为连续的 80–300；评分使用最近拇指范围的滑块。
源码位置：main.rs:1007-1277 和 filterbar.rs:600-821。

`FileManagerService::request_ui(FileManagerUiRequest)` 排队 Statistics、BulkActions、
CloseDialogs、Select(media_id) 或 Deselect。活动 GPUI 视图使用现有 UI 动作消费该请求；
这是异步的，不注入 OS 输入。MCP 在排队选择前验证媒体 ID。请求会替换一个未被消费的
请求，因此客户端必须在操作之间等待。这些 UI 请求不执行批量文件操作。Cordis 保留服务
所有权，且所有投影都由原有的 FileManager 注册清理逻辑移除。

延续工作添加了类型化的 `ConfigureBulk(action, source)` 请求。活动视图
使用与其批量按钮相同的重置和状态迁移；MCP 可以打开、
关闭并配置现有批量面板，并从 `get_ui_state` 读取其当前动作/来源。
卡片上下文菜单仍由指针触发，因为 GPUI Kit 的 `ContextMenu`
拥有自身状态且只暴露右键激活。为了确定性访问，
`open_media_actions(media_id)` 为已索引行排队可见的 FileManager 拥有的媒体操作面板；
`control_file_manager` 接受 `open_media_menu`/`close_media_menu`，`get_ui_state`
报告其活动打开状态和媒体 ID。其 Preview、open/reveal、评分、标签、复制、移动和删除
按钮调用现有的 FileManager 操作。

标签和核心参数化计数使用 UI Bridge translator 并提供英文回退。
月份折叠键保持源码英文；只有显示标签被本地化。MCP 以两种语言
捕获了确认的 9 个夹具卡片、底部选中操作、统计和批量面板。
悬停文件名/标签覆盖层、120ms 悬停/140ms 选中动画、精确的月份排版和扫描状态
优先级仍未验证或与源码不同；这不是完全一致。

## v5 S2 区域接线（2026-10-13）

`view::file_manager` 现在从 `UiBridgeService` 接收 `Arc<UiSurfaceState>`。
单个 FileManager 实体还通过现有的生命周期拥有的视图注册表，把 `file_manager/inspector`
贡献到 `file_manager.inspector`。
inspector 关闭 `surfaces["inspector"]`；Shell 每 100 ms 观察该状态。
基本信息和标签初始展开；EXIF/路径初始折叠。EXIF 按选中媒体 ID 缓存。
页外媒体/标签使用现有的 FileManager 和 Tagging 服务查找。
移除标签使用 `TaggingService::remove` 并记录
`UndoEntry::RemovedTags`，然后在重新加载后恢复选区。

`command.filters` 对类型、标签、扩展名、日期、
评分和排序使用 GPUI Kit popovers。单选 popover 通过自身实体关闭；
多选 popover 保持打开并读取当前包含/排除状态。
标签选择发出 `TagFilterSelectionChanged`（共享导航范围）；
其包含/排除点击模式是瞬时的。扩展名模式保留源的
`ext_click_mode` 持久化。`command.tools` 通过 `UiCommandInvoked` 分发现有的
`metadata-export/export` 命令。状态操作复用预览、标签和文件操作服务。运行时证据和
剩余的设计状态记录在 `docs/redesign-validation.md`。

## 当前交接 / v5 控制契约（2026-10-14）

`src/ui_requests.rs` 导出 `FileManagerUiRequest` 和 `FileManagerUiSnapshot`。
请求现在包含搜索、媒体类型/排序、评分范围、扩展名/标签
包含/排除、日期边界、清除过滤器、inspector 区块、标签选择器
和月份分组，以及原有的选择/对话框/悬停请求。
`FileManagerService::ui_snapshot()` 读取由活动 FileManager render 发布的状态；
它包含过滤器、选中 ID、匹配计数、分组和展开的 inspector 区块。它不是独立的
数据库查询。

GPUI `ListState/list` 现已适配不同的月份头部/卡片行高度；像素滚动持久化、
可见范围服务缓存、键盘 reveal 和实测行橡皮筋映射已作调整。服务仍然
保留 200 行富媒体缓存。真实的九项 MCP 截图显示
了间距修正；跨页选择、恢复和缩放行为仍待
验证。上文过时的 `uniform_list` 描述仅为历史。

Filter popovers 共享 `surfaces.value("file_manager.popover")`，具有原生触发
打开/关闭回调和插件 MCP 控件。Date Calendar 范围选择
和 04/06 批量/标签选择器布局是最近未验证的编辑。批量状态
新增 MediaTypeFilter、标签 ID 和扩展名排除；原有范围和
绑定确认的文件操作执行保持不变。当前 FileManager
编译在这些新面板中因 19 个缺失的类型/trait/可见性错误而失败。
确切诊断和待办的公开接口/运行时验证见 docs/development-handoff-2026-10-14.md；
不要声称这项工作已完成。

右侧 Inspector 保持选中项的缩略图预览、文件名和
可编辑评分固定在独立可滚动的 Information、History 和
Extensions 标签页上方。点击缩略图调用现有的 `PreviewService`
路径。Extensions 标签从 `UiViewCatalog::snapshot()` 读取可执行贡献，其插槽以
`file_manager.inspector` 开头；空目录显示显式的不可用状态。History 只显示实际的
`HistoryService` 撤销/重做描述并将其标记为全局，因为该服务
不暴露按媒体的历史。活动标签存储在
`UiSurfaceState` 的 `file_manager.inspector.tab` 键中，取值 `information`、
`history` 和 `extensions`；MCP 可通过共享 surface 状态设置此键。
inspector 布局和新文案仍需最终的 FileAtlas MCP
视觉检查。

## 2026-10-14 当前实现与 MCP 证据

UI 请求 SelectMany(Vec<i64>) 和 ScrollTo(usize) 支持显式已索引选择和从 0 开始的
当前结果滚动。FileManagerUiSnapshot 发布 window_offset/loaded_count。未测量的虚拟行
使用逻辑 ListOffset，已测量的可见行保留 reveal 行为。滚动页面变化通知发布；生产页面
大小仍为 200。Reference 00/01/04/06/07/08 布局以及真实的 100012 行最后页面 200、
跨页批量 2、紧凑 writeback os32/retry/dismiss 均通过 FileAtlas MCP 检查。
原生指针/拖拽/窗口恢复仍未验证。较早的 19 错误说明已成历史；当前 Shell 构建通过。

Final v2：Inspector 页面由 file_manager.inspector.tab（information/history/extensions）
控制，每 200ms 监视。预览固定 220pt，基本信息/标签/exif/路径内容独立滚动；
preview/open 使用现有服务。新的 Add-ons 标签为 filenamefilter 保留 Extensions 的含义。
缩略图选中检查覆盖层已移除；蓝色边框现在打包真实的 RGBA alpha。运行时匹配 3/选中 7，
页面切换和最小窗口期间保留预览；历史是现有的全局 undo/redo 摘要，右侧扩展槽当前为空。

## Grid/list 与稳定卡片菜单（2026-10-14）

`FileManagerUiRequest::Layout(bool)` 选择 list(true)/grid(false)；`FileManagerUiSnapshot.list_layout` 发布活动选择。顶部 command.size 投影在现有大小控件旁提供 Grid/List 按钮。List 每个虚拟行一条媒体，带 56pt 缩略图、文件名、本地化的类型/尺寸/文件大小/评分；它共享查询/顺序、200 行富缓存、选区、拖拽/标签放置、上下文菜单和预览处理器。grid/list 切换使行布局失效但不改变过滤器或选中 ID。`SessionState.file_manager_list_layout` 保存/恢复该选择，并对既有会话以 serde 默认值 false 处理。橡皮筋边界使用整个列表行。

卡片操作按钮在悬停淡出时保持挂载（最小 opacity 0.08），使指针进入其菜单时 GPUI Kit 不会丢弃 dropdown owner。直接右键仍使用 ContextMenu 并采用相同的 card_menu 构建器。实际的菜单移动、Ratings 子菜单和 Preview 执行已通过同进程 MCP 指针事件演练。

## 全文件卡片（2026-10-14）

All 过滤器包含普通 `file` 行且无特殊排除。Grid 和 list 缩略图在没有可用缩略图时（包括图片加载失败）使用共享的扩展名/类型图标；inspector 使用同一图标解析器。图片/视频/音频图标和文档/压缩包/代码/字体/普通图标是 UI 类别，不是解码器支持的声明。每张卡片都有一个不可交互的大写扩展名徽标，无扩展名时为 `FILE`；稳定的扩展名哈希提供其背景色。右上角的省略号按钮已移除。指针触发的上下文菜单及其现有处理器仍然可用。不支持的内联预览继续报告现有错误；默认应用 Open 在上下文菜单中可用。现有的类型/扩展名/标签/搜索过滤器继续正常生效。

## 分组提供者（2026-10-14）

FileManager 现在在生产 Cordis 图中要求来自 `fileatlas.grouping` 的 `GroupingService`。分组规则、稳定顺序和虚拟区块计算位于 fileatlas-plugin-grouping。现有 filter/tools 月份切换是七选一的分组 popover。`load_grouped(query, mode)` 返回与骨架对齐的 FileManagerSnapshot.group_labels 以及分组 `[label,count]`；初始/可见富窗口解析这些排好序的 ID（200 行常规缓存），而不是把原始 SQL offset 应用于重排后的视图。`load(query)`/`load_page(query, offset)` 仍是不分组的公开查询 API。选择/键盘/Preview/filmstrip 使用排好序的骨架顺序。模式变更保留存活的选中 ID 并重置滚动；切换模式会清除折叠键。Session 保存新增的模式字符串加上当前折叠标签；旧月份键在内存中迁移。源日期分组的拍摄/修改回退仍存在；显式 Modified/Created 使用不同的、已记录的时间戳。遗留 Settings 的 show_date_dividers 控制 Month/None；专门的分组模式不被旧布尔值覆盖。`FileManagerUiRequest::Grouping(GroupingMode)`/`ToggleGroup(String)` 和 snapshot.grouping_mode/groups/collapsed_groups 扩展了 MCP 访问。窄过滤行可横向滚动，Tools 也列出分组选项。

## 分组标题带（2026-10-14）

共享分组头部现在为每种模式和 grid/list 布局绘制不透明的蓝灰色填充、细描边和悬停填充。40pt 高度/计数/折叠的公开行为不变。同进程 MCP 指针折叠和渲染状态记录在 docs/validation-ui/2026-10-14-group-header/README.md。

Subfolder 排序现在调用带 query.path_prefix 的 GroupingService::arrange_in_scope。当前目录的直接文件先于后代，包括所选目录本身是嵌套的情况；无范围查询把已注册根的直接分组放在前面。没有分组标签、快照/窗口/选区契约或区块几何的变更。见 docs/validation-ui/2026-10-14-root-first。

## 不分组的目录浏览器（2026-10-14）

`FileManagerSnapshot.browser: BrowserSnapshot` 描述活动/当前目录、有界的父级、直接的 FileManagerDirectory{name,path} 卡片以及可选的列表错误。`load_grouped(query, GroupingMode::None)` 现在通过 MediaQuery.direct_directory 投影所选路径/根的直接文件，并通过 IndexingService::try_list_subdirectories 读取实际子文件夹。没有目录时显示已注册根；全局文本搜索和标签视图保留递归文件结果。其他分组模式和公开 load/load_page 仍为递归。FileManager 行有独立的 directory_range/media_range，并在橡皮筋几何中考虑目录列；文件夹不获得 MediaSkeleton/MediaItem ID。200 行富缓存和 filmstrip 仍只针对文件。

文件夹选择是临时的；实际位置/布局使用现有 Session 字段。双击/Enter 排队公开的 NavigationService::request_ui(SelectPath)，Up/Alt+Up 选择父级或 AllFiles，Refresh/F5 重新读取。Shell 与 FileManager 一同启用 Navigation/Indexing；仅 FileManager 的服务测试仍可在没有 UI provider 的情况下使用递归 load。SelectedFolder 批量范围在浏览器模式下是即时的，并拒绝未选择的虚拟根；WholeLibrary 保持全局。新的 UI 请求 OpenDirectory(String)、DirectoryUp、Refresh 以及 snapshot browsing/directories/current_directory/parent_directory/directory_error/selected_directory 支持同进程 MCP。精确的运行时/测试限制见 docs/validation-ui/2026-10-14-folder-browser。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动和现有错误/诊断输出现在发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作报告挂钟 span 时长和错误。领域服务契约、事件 payload 和源操作顺序不变。捕获范围和限制见 fileatlas-plugin-debug/PORTS.md。

## 对话框读模型（2026-10-15）

统计和批量匹配预览通过 UI Bridge ReadModel 在现有的 FileManager 模型 poller 上加载，而不在 render 期间加载。统计按索引和打开对话框期间 Database writer 版本缓存。批量预览按来源、文件夹/路径/标签范围、选中 ID、所有原始过滤字段/日期、索引和 writer 版本缓存。相对日期为该预览请求固定。CurrentSelection 仍绕过过滤器，且 execute 在任何文件操作前重新校验当前目标。错误会被缓存；编辑字段会用新键重试。对话框关闭会丢弃读模型；没有数据库架构/文件操作顺序的变更。
`FileManagerUiRequest::BulkField(usize,String)` 更新真实输入并清除删除确认。`FileManagerUiSnapshot::dialog_reads` 暴露类型化的 FileManagerDialogReads（statistics 状态/错误/照片/视频/音频计数；bulk 状态/错误/匹配计数/已加载预览 ID），没有额外查询。Statistics 还发布 rating_counts。`FileManagerService::data_revision() -> u64` 转发 Database writer 时钟，因此直接的服务变更也会使读取失效。

Inspector/选区详情 EXIF 使用同样的带键后台边界，键为选中 ID + 索引/writer 版本。已加载/回退的媒体路径会被复用；否则由 worker 一次性完成其查找。缺失来源/EXIF/错误均被缓存。Rendering 和展开操作从不查询路径或读取 EXIF；取消选择/多选/关闭 EXIF 会放弃读模型。迟到的结果不能填充下一个选区。`exif_cache` 是其仅绘制的投影。

## 统一被动 UI 模型（2026-10-15）

FileManager monitor 现在向 UI Bridge refresh::poll 返回单个聚合变更标志。Surface/progress 签名使用共享 ChangeGate。统计、批量、EXIF、窗口外 Inspector 回退、单选元数据和选中拖拽路径全部使用共享 ReadModel。Render 只声明非阻塞需求；完成投递检查当前选区并发布现有快照字段。错误/空结果被缓存，绝不在每次绘制时重读。现有的索引刷新清除由选区派生的缓存。
## 预览缩略图交接（2026-10-15）

`preview_items_by_ids(&[i64]) -> Result<Vec<fileatlas_plugin_preview::FilmstripMedia>, FileManagerError>`
按请求顺序返回现有的源路径、ID 和可选 thumb_path，
以现有查询语义跳过缺失的行。可见已加载窗口在无 IO 的情况下被投影；
完整过滤结果解析仍留在后台 executor 上，并且只在已打开身份仍处于活动时应用。
Preview 消费公开契约；FileManager 不拥有预览解码。
没有架构、过滤器、事件或源文件变更的改动。缩略图新鲜度、回退、
生成取消和诊断见 Preview PORTS。


## 浏览器运行时更新 — 2026-10-15

生产的浏览在所有分组方式下都由文件系统驱动。`live.rs` 适配 Yazi VfsEntries/Refresher（MIT，钉住的参考位于 vendor/yazi-reference）。一个目录 worker、一个最新待处理请求、协作式取消、代次拒绝、有界的四消息通道、首批 32 条目之后每批 2048 个文件。UI 在每次共享 poll 中最多排空四条消息。文件系统读取/stat 和会话缓存批次在线程外运行。目录包含空/隐藏目录；后代遍历是显式的且不跟随符号链接目录。子项读取错误产生部分结果加一个错误。

公开控件：`browser_enabled`、`show_descendants`、`set_show_descendants(bool)`、`thumbnails_enabled`、`set_thumbnails_enabled(bool)`、`invalidate_directory`、`cancel_directory`、`poll_directory`、`directory_worker_active`。`directory_status() -> (u64 generation,u64 revision,bool active,usize scanned,Option<String> error)`。Active 表示视图正在加载；worker_active 单独报告取消后仍在运行的 OS 调用/生产者。被阻塞的 OS 读取或已在运行的解码器不能被强制中断。

FileManager 消费 WatcherService 和 IndexingService。原生 watcher 变更修补会话缓存或使目录失效；显式刷新重新读取；激活使用持续运行的 watcher。快照使用 ReadModel；行渲染器声明需求，每次 poll 合并一次。200 行富窗口和少量可见超扫请求缩略图；过期代次/请求/文件版本不能发布解码结果。Preview 仍是现有的优先级解码器。tools 菜单包含缩略图启用/禁用和缺失记录维护。Session 按路径恢复选区，而不是按缓存行 ID。收藏主页、标签结果、定位并选择、grid/list、分组和现有操作面板共享已接受布局。

缺失记录 UI 选择确切文件，或从一个显式挑选的文件夹中提出同名候选，然后逐行应用或跳过。删除缺失记录需要两步确认并重新校验本地缺失状态。没有自动移动推断。已演练的 Windows 路径见浏览器验证证据；编解码器、不可访问驱动器和原生选择器边界覆盖仍有限。


## 复查修正 — 2026-10-16

目录块变更在快照读取进行中被合并。已完成的部分快照会在请求下一个目录版本之前投递，防止持续键失效使第一个结果饿死。显式导航/过滤器重新加载仍使旧结果失效。刷新保留选区/滚动而不持久化瞬时的空选区；未变更的 SessionService 保存不写入。通过同进程 MCP 检查了对隔离配置档自身的浏览和外部文件创建。


## 重复目录刷新修复 — 2026-10-16

窗口激活不再使浏览器失效或重试其订阅。当前原生/PollWatcher 在未聚焦时保持活动；实际文件事件和显式刷新仍会更新视图。LiveBrowser 在排队枚举之前注册所选 watch；其 worker 在 WatcherService::browse_registration_pending 上协作等待（线程外，可取消）。注册时旧的合成根变更已移除。初始快照等待第一个当前枚举批次，而不是短暂显示较早缓存的清单。

后台刷新在新结果到达前保留现有行、滚动、选区、对话框和当前 200 行富窗口；load_grouped_window 是一个保留其 offset 的 crate 私有查询变体。BrowserSnapshot.loading 标记部分目录结果；选区只针对完整交付的骨架做校验，因此版本还原期间的临时空结果不能用文件夹信息替换文件 History inspector。显式导航/过滤器重新加载保留其重置语义。FileManagerUiSnapshot.snapshot_revision:u64 是供查询调度的只读诊断，以 MCP file_manager.snapshot_revision 暴露。同进程生命周期和外部变更证据：docs/validation-ui/2026-10-16-refresh/README.md。

## 标签分配刷新范围 — 2026-10-17

Inspector 的标签分配/移除/清除操作只修补受影响媒体 ID 的 `FileManagerSnapshot.media_tags`
和选中文件回退缓存。它们不重新加载 file_manager 查询，也不重置其列表状态。
如果存在活动的包含/排除标签过滤器，分配可能改变结果成员资格，因此 FileManager 只通过
`refresh_index` 刷新当前查询窗口；该路径在替换窗口的同时保留当前逻辑滚动锚点和选区。
显式的过滤/导航重新加载保留其现有重置行为。同进程的
滚动中途分配证据见
`docs/validation-ui/2026-10-17-file-manager-tag-scroll/README.md`。


## 地址栏集成 — 2026-10-16

目录延迟更新：视图在 200ms 回退之外拥有一个共享的 Wake listener。
UI 请求、全部五个范围/索引观察者、有界生产者批次以及快照/视口 ReadModel 完成
都在发布状态后向其发信号。一个 sampler 消费两条路径。首批计数文件加目录
（32 条目）；后续批次在 2048 条目或 32ms（非空时）刷新。中间快照查询以最小
50ms 间隔合并；首个范围和最终完成绕过它。逻辑行几何即使
在 GPUI 测量重置行之前第二个结果到达也能保留滚动。没有额外持久化或
源文件写入。证据：docs/validation-ui/2026-10-16-navigation-latency。

command.size 中重复的 Up 按钮已移除；Navigation 现在把 Up 放在完整面包屑/地址字段之前。
FileManager 的文件夹卡片、选择器、定位和现有 Alt+Up 仍通过 NavigationUiRequest::SelectPath 路由。
FileManager 获得焦点时的 Ctrl+L 和 Alt+Left/Right 排队公开的 AddressBarRequest 变体。
Navigation 拥有历史/输入；FileManager 不实现平行的历史。现有的选区/文件操作撤销历史
是独立的且保持不变。

## 未保存版本指示器 — 2026-10-17

FileManager 通过 `ReadModel` 为最多 200 个可见文件请求 `VersionsService::file_state`，
以快照/历史/数据版本和文件标识为键。IO 在
worker 上运行；渲染读取缓存结果，完成轮询在空闲时停止。
`Modified` 在 grid 布局中于缩略图左上角添加琥珀色圆点，在 list 布局中于文件名前添加，
避免与小缩略图格式标签重叠。其 tooltip 标识所选的已保存基线。点击会选中该文件并
打开 History；它从不创建或切换版本。未版本化的文件、未变更的旧选中修订，
以及与另一已保存修订匹配的内容没有圆点。
History 解释当前基线和任何匹配的其他修订。读取错误
可在 History 和 `FileManagerUiSnapshot.version_states` 中获得；它们不被
归类为已修改。快照字段还暴露 `version_states_loading`。
真实文件系统变更和 `HistoryChanged` 使缓存状态失效。
验证：`docs/validation-ui/2026-10-17-version-status/README.md`。

## 搜索位置切换 — 2026-10-17

`FileManagerFilter.search_entire_computer` 映射到 `MediaQuery.search_entire_computer`。
顶部搜索按钮通过 `FileManagerUiRequest::SearchEntireComputer(bool)` 切换当前文件夹/
全计算机模式，并保存 `Settings.search_entire_computer`（serde 默认 false）。活动的
文本搜索会把文件夹/路径规范化为递归的 `directory_tree`，与直接/递归浏览设置无关。
全计算机模式清除文件夹、路径和实时缓存限制，并不向 Everything 传递根；
因此结果遵循 Everything 配置的已索引卷、文件夹和排除项。它不编辑 Everything 设置。
其他 FileAtlas 过滤器保持活动。没有选定文件夹时，本地模式保留已注册集合根；
其 tooltip 解释这一回退。清除文本会恢复普通浏览和所选导航位置。
FileManager 的变更不替换 Navigation 的选区或历史。同进程证据：Everything 搜索验证 README。

## 统一主题（2026-10-18）

View 绘制使用 UI Bridge 语义主题 token 而不是本地颜色字面量。
FileManager inspector/statistics 卡片和标题使用共享组件；紧凑
导航以其通用选择/悬停绘制保留其布局。状态、分支
和媒体覆盖层颜色在中央主题中保持语义区分。
没有产品服务/数据/事件契约变更。见 `docs/ui-theme.md`。

## 主工具栏布局（2026-10-18）

`command.size` 只保留打开文件夹/刷新；`command.filters` 左侧包含
可滚动的过滤 chips，右侧包含 grid/list/后代/缩略图大小。`view_controls`
使用共享 UI Bridge 图标按钮，配有本地化的 tooltip/可访问性标签和现有的布局/后代/session
处理器。搜索位置、范围和 Everything 依赖反馈仅有图标；反馈
文本在 tooltip 中仍可获得，包括安装/版本状态。
`workspace.status` 保留计数/进度/取消/错误；按用户指示移除了选区的 Tags/More
按钮。上下文菜单和现有公开命令保留其功能。没有新的设置、服务或持久化契约。

## 被动版本状态失效（2026-10-18）

即使 History 关闭，可见卡片也会自动读取已保存内容状态，
以显示未保存变更徽标。`version_badges::StateKey` 包含 FileManager 快照
代次、显式历史代次以及可见媒体标识/路径。它必须
不包含 `DatabaseService::write_revision`：无关的剪贴板/启动器/缓存
写入此前会使此读取失效，并在下一次普通 paint 时重新打开 Lore。
文件系统/索引变更仍推进快照，版本操作仍推进历史代次。成功/错误仍由共享 ReadModel 缓存。空闲
回归检查是 `scripts/validate-version-idle.cjs`；它读取原始 debug 事件而不是
隐藏 OpenTelemetry 日志，并同时验证无关写入和真实变更。

## MCP 功能贡献

FileManager 通过 `McpToolRegistry` 拥有 `file_manager.search_media`、`file_manager.get_media` 和 `file_manager.set_rating`。Search/get 的 schema 和处理器使用 `FileManagerService`；rating 接受 1–100 的正数媒体 ID 和 0–5 的值并调用 `apply_rating`。这些写入受变更门控。旧的无命名空间别名已退役；`tools/list` 暴露插件拥有的名称。

## 缩略图对比度修正 — 2026-10-19

文件扩展名徽标消费 UI Bridge 的 `file_format_colors` 而不是 HSL
绘制和无关的强调按钮前景色。紧凑标签胶囊使用
共享的不透明 `tag_colors` 角色配对；评分、播放和溢出徽标使用
媒体专用的背景/文本角色。缺失文件横幅把 danger 填充与
`text_on_danger` 配对。没有数据架构、路径、媒体像素或后端行为的变更。
实时证据：`docs/validation-ui/2026-10-19-thumbnail-contrast/README.md`。

## 原生主题角色消费（2026-10-19；取代手工颜色）

所有 FileManager 背景/文本/状态、自定义文件格式/评分/标签徽标和标签颜色
建议都通过 UI Bridge 的原生 GPUI ThemeColor 角色投影解析。不存在
FileManager 调色板或按模式的 RGB 路径。拖出通过
DragImageStyle 向平台适配器提供当前原生绘制。媒体像素和持久化的用户标签颜色仍是
内容；标签表面/前景是原生角色。见 native-theme 验证记录。

## 原生交接之前的内部功能拖拽（2026-10-20 修正）

媒体卡片现在在每种分组模式下都把其共享 GPUI 拖拽保持在主窗口内。
在拖拽开始时就立即启动 OLE 会使插件的放置接收者看不到普通手势。
`card.rs` 使用 GPUI `on_drag_move`，仅当指针离开视口之后、
仅对发起拖拽的面板（共享 allowed-flag 标识），并且通过 payload 拥有的
`native_handoff_started` 原子标志每次手势仅一次，才排队现有的 DragDropService。原生
路径取自该活动 payload，而不是任意重新绘制的卡片。选区、修饰键、仅复制的原生
payload 和服务生命周期保持不变。现有的 Subfolder/debug 原生抑制仍然适用。
钉住的 GPUI 的 Windows 平台没有实现外部拖拽提升，因此
我们保留现有的 Windows 适配器，而不是依赖其不受支持的
`external_drag_payload` 路径。实际的同窗口 FileManager 到 WorkCrate 拖拽
以 `debug_internal_file_drag=false` 检查；边界变更后与
Explorer 的交接仍需原生桌面覆盖。
