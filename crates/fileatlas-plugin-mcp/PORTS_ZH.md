# MCP 插件端口 / MCP 对接

## WorkCrate — 2026-10-20

三级修订版新增受 Standard 门控的 `workcrate-ui.navigate`、
`manage_project`（创建/重命名/封面/复制/归档/恢复）、`create_page`
（A3 横向）以及 `update_page`（名称、物理尺寸与内容）。
`state.ui` 发布实际挂载的界面、忙碌状态以及来自共享 UI 表面的选择项。
`delete_project` 与 `export_pages` 需要 Full 加上每次调用的确认；删除操作会把功能数据保留在回收站中。有序 PDF 与
PNG/JPEG 导出使用原始图像的 DesignCraft 渲染。确切载荷与剩余限制参见 WorkCrate UI PORTS。

`workcrate-ui.state` 从 JSON 清单返回项目名称、引用计数与画板元数据。
它不读取链接图像的字节。`workcrate-ui.open_board`
在当前 FileAtlas UI 中打开已存在的画板；`workcrate-ui.place_image`
将一个路径添加为项目引用，校验并放置实际的 JPEG/JPG/PNG
字节，然后打开该画板。两种变更都需要 Standard MCP 访问权限，且
只更改 WorkCrate 记录/画板文件；它们不写入源图像。
`workcrate-ui.export_pdf` 向请求的路径写入 PDF，需要 Full
访问权限并加上 `confirm=true`。UI 插件拥有这些工具并负责注销它们。这些模式与错误处理由
`fileatlas-plugin-workcrate-ui/PORTS.md` 维护。

## FileManager 组框指针诊断（2026-10-20）

`get_ui_state.file_manager.group_frames` 报告主面板绘制的框线段：
行索引键映射到逻辑客户端坐标中的 `{path,first,bounds:[x,y,width,height]}`。多行组在每个线段上保留相同路径。几何
信息仅存在于内存中，在绘制时重新发布，且在布局期间可能短暂为空。

受 Standard 门控的 `control_file_manager {action:"debug_internal_file_drag",enabled:true}`
除非存在显式的 `FILEATLAS_DEV_TEST_DB` 隔离 dev-test 配置，否则会被拒绝。
随后活动状态的 FileManager 会保留其类型化 GPUI 拖放路径，同时跳过
原生 OLE 拖出交接，而该交接原本会阻塞合成指针投递。
使用 `enabled:false` 禁用它；默认为 false，且永不持久化。
`file_manager.debug_internal_file_drag` 报告主面板的状态。指针测试
使用现有的 `control_ui` 事件与真实文件系统结果，而不是直接的 `move_files_to_subfolder` 控制路由。参见 FileManager PORTS 与
`docs/validation-ui/2026-10-20-group-frame-drop/README.md`。

`get_ui_state.file_manager.file_copy_buffer_count` 字段报告活动主视图内存中的
Ctrl+C 选择数量。`control_ui {action:"key_press",
value:"ctrl-c"|"ctrl-v"}` 将组合键派发进同一 GPUI 进程；指针
位置决定 Ctrl+V 的文件夹目标。参见
`docs/validation-ui/2026-10-20-file-keyboard-parent-drop/README.md`。

## 实时列工具栏诊断 — 2026-10-19

`get_ui_state.theme` 现在报告 `mode`（`dark`/`light`）、作为 `id` 的原生主题名称、
`framework: "GPUI Kit ThemeRegistry"`
以及当前 base/panel/card/border/text 语义颜色，使实时模式检查不会
把原先固定的深蓝色值误当作所选调色板。

Material 工具由 `fileatlas.material-theme` 进行生命周期管理，并非在此硬编码。
`material-theme.state` 将生成器领域错误作为 `last_error` 返回；失败的
生成调用使用普通的 MCP 失败结果。参见该插件的 PORTS。

`get_ui_state.program_workspace.current_snapshot` 将现有公开的
`workspace.current_snapshot` 表面读取为类型化的 `ProgramWorkspace`，或在发布之前返回 null。
其 `columns[i].list_layout` 与 `show_subfolders` 描述
每个实时面板的视图控件。`active_workspace` 仍是已保存的定义；
旧的顶层 `file_manager`/`browser` 诊断描述主面板。
这个只读投影不增加轮询、IO 或状态变更。

## 访问级别 — 2026-10-19

MCP 插件贡献 `fileatlas.mcp` 设置页面。`Settings.mcp_access_level`
有 Off、Standard 与 Full 三个级别。Standard 允许 UI 控制与常规
变更；Full 额外允许那些插件自有 MCP 元数据标记为
`confirmation_required` 的工具，且这些调用仍需要 `confirm=true`。调用
读取当前 SettingsService 快照，因此级别变更立即生效。
之前的 `mcp_allow_mutations` 布尔值被接受为旧版 JSON 别名
（`false` → Off、`true` → Full）。参见
`docs/validation-ui/2026-10-19-mcp-access-stages/` 获取运行时证据。
下面较早的验证条目保留了那些历史构建使用的环境变量值；
当前授权始终由本文所描述的 SettingsService 访问级别控制。

## 剪贴板工作台 — 2026-10-18

- `get_clipboard_state` 是只读的，报告数量、类别名称、条目 ID、
  长度、类别分配、内容 `kind`（text/link/code）、修订号、收藏状态（旧版 pinned 别名）与时间戳。`ui` 通过 `UiSurfaceState` 报告当前过滤器、可见 ID、选择项以及编辑器/组名的长度。它有意省略
  剪贴板文本。
- `add_clipboard_text {content, category_id?, favorite?}` 是受变更保护的操作，
  直接通过 `ClipboardService` 添加文本行；它从不读取或写入
  操作系统剪贴板。仅使用隔离配置存放 UI fixture 数据。
- 操作系统剪贴板捕获与系统级按键未作为 MCP 动作暴露；
  那些 Windows 路径仍需专门的运行时证据。
- `scripts/fileatlas-mcp-debug.cjs` 始终启用 `dev-test` 并提供显式的配置本地 `FILEATLAS_DEV_TEST_DB`，即使请求的插件列表为 `core`。Windows 已知文件夹解析可能忽略 `LOCALAPPDATA` 覆盖；因此旧客户端没有为仅 `core` 建立隔离。报告现在包含有效插件、子进程 PID 与隔离状态。显式的配置路径在运行中的 UI 里可见。
- `control_ui input_text` 使用其实际按键/字符值派发可打印 Unicode 字符，避免了此前无法可靠到达聚焦输入框的合成 `text` 键。它仍是同进程 GPUI 输入，带有现有变更守卫与 1000 字符上限。`scripts/validate-clipboard-workbench.cjs` 演练中文组输入、即时分配、过滤器、搜索、重命名/删除与捕获暂停；`scripts/validate-clipboard-restart.cjs` 检查在不访问操作系统剪贴板的情况下的独立子进程重启。

## 快捷键设置验证 — 2026-10-18

`command_list` 包含每个已注册快捷键的 `id`、默认 `key`、
`effective_key`、启用状态与 `scope`（`application` 或 `system`）。设置面板从同一目录发现这些行，
并按贡献 ID 应用持久化的覆盖。
`control_ui {action:"key_press", value:"ctrl-alt-q"}` 支持 GPUI 的
连字符分隔修饰键组合语法，用于同进程捕获测试。MCP
按键是窗口内的合成事件，并不模拟另一应用处于前台时的全系统按键。

## 程序启动器诊断 — 2026-10-17

当 `fileatlas.launcher` 启用时，`get_launcher_state` 从同一 FileAtlas 进程中的 `LauncherService` 读取其内存
快照。它返回扫描
状态、类别与应用条目（显示名称、启动路径、类别、
hidden/available 标志、发现来源、pin 范围、安装日期/大小，以及
是否注册了卸载命令）。受变更保护的工具涵盖
添加与刷新发现；类别创建/重命名/删除；程序
显示名称/类别/hidden 编辑；固定；启动；仅列表移除；以及
卸载。启动、移除、类别删除与卸载需要
`confirm=true`；卸载在显式 MCP 确认后运行已注册命令，不再出现另一个原生
提示。仅列表移除从不删除文件。
变更需要在"设置 → MCP → AI 控制"中设为 Standard 或 Full。高影响
启动器操作额外需要 Full 与每次调用的 `confirm=true`。缺失的启动器服务返回显式
错误。使用 `control_ui` 指针动作与 `screenshot_window` 进行实时面板
交互。

`scripts/validate-program-launcher.cjs` 演练类别 CRUD、部分
程序编辑、发现刷新、确认门控、仅启动器移除
与重新添加，以及实时面板/图标/排序/pin 流程。它不调用真实
卸载程序；已验证 `confirm=false` 会使状态保持不变。卸载
针对已安装软件的实际执行仍是手动系统集成
检查。

## Everything 搜索诊断 — 2026-10-17

`get_ui_state.file_manager.search_provider` 仅在一次成功的 SDK IPC 查询之后报告 `everything`；`sqlite-fallback` 表示 FileManager 使用了其原始数据库搜索，
因为可选适配器/查询不可用。`search_media` 经由
FileManagerService 路由，并演练与 FileManager 查询相同的提供方。
`search_media.search_provider` 与 `get_ui_state.file_manager.search_provider` 保留
该特定结果/快照的提供方。`file_manager.everything` 包含
`state`、实际 EXE 的 `version`、`path` 与 `index_loading`。安装检查
是只读的；只有真实的文件名/路径搜索才能准备后台客户端。
`control_file_manager {action:"everything_recheck"}` 刷新安装
ReadModel 并重试当前文本搜索。状态转换场景是
`scripts/validate-everything-status.cjs`；它在隔离的可移植路径覆盖中读取实际 1.4/1.5 EXE 资源，
而不安装或执行这些 fixture。
隔离集成场景是 `scripts/validate-everything-search.cjs`。
`control_file_manager {action:"search_scopes_open",enabled}` 打开/关闭实时
顶栏范围面板；`search_scope` 接受 `filenames|paths|tags|time`，并
使用 FileManager 持久化的范围开关。`get_ui_state.file_manager.search_scopes`
与 `search_scopes_open` 暴露渲染出的选项，供同进程检查使用。

## 未保存版本状态 — 2026-10-17

`get_file_version_state {media_id}` 是只读的，返回路径以及
`state.status`、`currentVersionId/currentTitle`、`matchingVersionId/matchingTitle`。
它比较不可变内容，从不自动创建或切换版本。
`get_ui_state.file_manager.version_states` 暴露可见窗口的读取结果、
每文件错误与 `badge_visible`；`version_states_loading` 报告有界的
ReadModel 工作。只有确认的 `modified` 才显示琥珀色卡片圆点。

## 历史可见性诊断 — 2026-10-17

`get_ui_state.inspector_history.visible` 仅在 Inspector 打开、
History 标签被选中且 FileManager 当前 Inspector 上下文为文件时为 true。
其它历史表面字段在隐藏时保留其最后渲染的值；
版本切换后使用 `visible` 与 `file_manager.selected_media_ids` 验证实时页面，
而不是把过时的 count/current 值当作可见性。

## QuickLook 替换 — 2026-10-16

本节取代下面旧的 Preview/Playback 工具契约。
MCP 现在依赖 `QuickLookService`；PreviewService 与 MediaPlaybackService
及其 crate 已移除。`control_preview` 与 `control_playback` 及其请求载荷
已从 tools/list 移除。`open_media {media_id}` 是一个异步、
受变更保护的调用，用于向已安装的 QuickLook 打开任意普通
文件类型。`get_view_state` 返回 `{quicklook: LaunchSnapshot}`；派发
成功不是外部渲染/可见性的证据。`get_ui_state` 包含
相同诊断与 `quicklook_notice`（最后一次 FileManager 失败消息）。
`control_file_manager {action:"preview",media_id}` 演练真实的 FileManager 提示
流程；`control_ui {action:"key_press",value:"space"}` 派发真实的
同进程 GPUI 按键。Settings Toolbar 不再是可接受的标签，且
固定面板诊断仅列出 Settings。其它预览术语，例如
批量匹配预览与 inspector 字段，保留其自身含义。

## 上下文检查器验证新增 — 2026-10-16

- `get_ui_state.file_manager.inspector_context` 读取实时渲染的上下文键
  `program|tag|folder|file`，无需另一次数据库查询。
- `control_file_manager {action:"inspector_tag_name",value:string}` 入队
  `FileManagerUiRequest::InspectorTagName(String)`（1–1024 字节）。它只更改
  实时输入框。通过同进程指针事件点击 Save 提交。
- `control_navigation {action:"tag_appearance",tag_id,color?,icon?}` 是变更操作，
  并使用 `NavigationService::set_tag_appearance`。它检查 ID 与十六进制
  颜色，设置两个字段，并清除省略的值。它用于填充隔离 fixture。
- `get_ui_state.inspector_demo` 通过现有 UiSurfaceState 暴露显式视觉演示的
  enabled/selected/active/count/filter。正常启动
  返回 enabled=false。实际的 GPUI 指针事件演练节点选择、
  菜单、分支过滤与内存中的 Create/Switch；这些字段并不暗示
  存在持久化的文件版本后端。

## Lore 支撑的历史版本 — 2026-10-17

`list_file_versions {media_id}` 读取 Lore 针对已索引
文件的修订历史，返回带标题的修订、时间戳、大小、缩略图、父修订与
活动修订。`create_file_version {media_id,title,confirm:true}` 将
当前文件字节保存为带标题的修订。`activate_file_version
{media_id,version_id,confirm:true}` 恢复一个修订，并刷新该
媒体行的派生文件元数据与缩略图。Create/activate 需要在"设置 → MCP → AI 控制"中的 Full
访问权限以及显式 `confirm:true`。同进程
FileManager History 标签调用相同的 Versions 服务。插件生命周期
依赖为 `fileatlas/versions` → `fileatlas/version-info` 与
`fileatlas/thumbnails`；持久的版本与分支元数据使用 FileAtlas 浏览器数据库中类型化的
`DatabaseService` 表。现有同级
`version-info.sqlite3` 数据被幂等地导入，并作为回滚副本
保留。MCP 服务操作已通过
同进程验证。History 标签、修订选择与版本切换
是通过该 MCP 子进程的 GPUI 窗口点击完成的；待处理冲突被读取，
并在以相同配置重启该子进程后被中止。参见
`docs/validation-ui/2026-10-17-lore-version-history/README.md`。

`list_file_branches {media_id,include_archived?}` 读取分支头、祖先、
每分支修订与待处理的合并冲突。`manage_file_branch` 接受
`action`=`create|switch|archive|merge|resolve|abort` 以及相关的 `name`、
`source_branch`、`strategy` 与 `overwrite` 字段。所有写入都需要
变更守卫与 `confirm:true`。会改变字节的 switch/merge 操作刷新
FileManager 行并发出 `MediaIndexChanged`；resolve 接受 `ours` 或 `theirs`。

变更守卫与同进程子/窗口边界仍然有效。
版本/分支往返、FileManager History 指针交互以及跨重启的待处理
合并恢复，由
`scripts/validate-lore-*` 场景通过 `fileatlas-mcp-debug.cjs` 在隔离
配置中演练。证据：
`docs/validation-ui/2026-10-17-lore-version-history/README.md` 与
`docs/validation-ui/2026-10-16-inspector-complete/README.md`。

当前浏览器运行时取代下面旧的已索引库描述；参见末尾的 2026-10-15 更新与 `docs/validation-ui/2026-10-15-browser/README.md`。

状态（更新于 2026-10-19）：正常的 FileAtlas 启动在 GUI 进程中以 `http://127.0.0.1:38473/mcp` 承载 Streamable HTTP MCP（`FILEATLAS_MCP_HTTP_PORT` 覆盖端口）。旧版 `--mcp` stdio 模式仍保留用于隔离调试。绑定仅限回环；rmcp 根据其回环允许列表验证 Host。修改访问由"设置 → 插件设置 → MCP → AI 控制"实时控制，并作为 `Settings.mcp_access_level` 持久化到现有的 Settings JSON。Off 只允许读取；Standard 允许 UI 控制与常规变更；Full 额外允许插件元数据将其标记为高影响的工具。缺失的值默认为 Off。每个宿主与贡献的变更调用都在调用时读取当前设置；没有重启也没有环境变量覆盖。高影响操作仍需要其每工具的 `confirm:true` 检查。旧版 JSON 键 `mcp_allow_mutations` 将 false 映射到 Off、true 映射到 Full。

MCP 特性工具通过 `fileatlas_contracts::mcp::McpToolRegistry` 贡献。宿主在特性启动之前发布该注册表；每个 Cordis 特性插件贡献其自己的带命名空间的工具定义、JSON 输入模式、处理器以及变更/确认元数据，然后将返回的注册守卫保存在插件专属的 Cordis 生命周期服务中。这些包装服务具有不同的 Cordis 标识，因此多个插件可以在一个 realm 中发布守卫。MCP 在 `tools/list`、`tools/call` 与 `get_tool` 时解析贡献，因此停止一个插件会将其工具从活动服务器移除。MCP crate 不拥有贡献的特性参数定义。Launcher、FileManager、Navigation、Tagging、Clipboard、File Operations、Metadata Export、QuickLook、Versions、Debug 以及可选启用的 DevTest 贡献其工具；每个插件的 `PORTS.md` 记录其名称、模式与行为。旧的无命名空间别名不再由 `tools/list` 公布；窗口捕获/缩放、跨插件 UI 检查/控制与全局命令目录仍是宿主工具，因为它们依赖于运行中的窗口或共享的 UI Bridge 注册表。

2026-10-19 的运行时功能验证使用了 `start-dev.bat all` 及其
`127.0.0.1:38473/mcp` 的同进程 HTTP 端点。该配置将其
数据库/设置/会话/缩略图存储在 `target/dev-test` 下。本次运行创建了
`test-data/mcp-live-2026-10-19` 并包含复制的 JPEG 与文本 fixture。`tools/list`
返回 62 个工具，其中包含 52 个带命名空间的插件贡献；未列出
已退役的无命名空间别名。`file_manager.get_media`、`file_manager.search_media`、
`navigation.list_library`、`tagging.list_tags`、`clipboard.snapshot`、
`launcher.snapshot`、`quicklook.snapshot`、`quicklook.inspect`、`debug.query`、
`get_ui_state`、`command_list` 与 `screenshot_window` 均已完成；截图
返回了真实的 MCP 图像块。

仅在隔离测试进程中设置 `FILEATLAS_MCP_ALLOW_MUTATIONS=1` 时，本次运行
注册并重新扫描了 fixture 目录，索引了其 JPEG/text/README；
设置并恢复了图像评分；创建、分配、读取、移除并删除
了一个临时标签；复制了测试 JPEG、重命名并移动了该副本，随后仅
将该测试副本删除到回收站；向测试目录导出了 10 条元数据记录；
并为文本 fixture 创建/列出/激活了一个 Lore 版本。激活后其
SHA-256 保持不变。启动器类别创建/重命名/
删除成功。DevTest 扫描了全部 12 个已签入的 fixture 文件，通过 FileManager 搜索找到了
嵌套的 JPEG，注入了 1,000 个虚拟行并将其清空。
注入期间 DevTest 状态报告 1,000 行，而
`file_manager.search_media` 即使使用 fixture 文件夹 ID 查询仍返回 12 条结果；这被记录为观察到的 FileManager/基准
可见性缺口，而非 FileManager 规模显示的成功。临时标签/类别与
测试文件夹注册已被移除。fixture 扫描保留在隔离的
DevTest 配置中；生成的 `metadata-export.json` 与物理测试 fixture
保留在测试目录中。应用在未设置变更环境
变量的情况下重启，最后一次 `file_manager.set_rating` 尝试正确地以
`FILEATLAS_MCP_ALLOW_MUTATIONS=0` 被拒绝。

当前 API 名称与完整参数模式位于其提供方旁边：
`clipboard.*`、`file-operations.*`、`file_manager.*`、`launcher.*`、
`metadata-export.*`、`navigation.*`、`quicklook.*`、`tagging.*`、
`versions.*`、`debug.*` 以及（启用时）`dev-test.*`。其余宿主
工具是窗口截图/缩放、跨插件 `control_ui`、
`control_navigation`/`control_file_manager` UI 驱动操作，以及全局
`command_list`/`command_run` 桥接。

## Cordis 生命周期与依赖

- Cordis 插件：`fileatlas.mcp`；生命周期守卫服务：
  `fileatlas/mcp-task-guard`；宿主窗口句柄服务：
  `fileatlas/mcp-window-handle`（`CaptureWindowHandle`）。
- 需要 `FileManagerService`、`NavigationService`、`TaggingService`、`QuickLookService`、`MetadataExportService`、`CaptureWindowHandle`、UI Bridge 目录/表面、HistoryService 以及共享的 `McpToolRegistry`。特性数据操作通过每个特性的注册表贡献派发；宿主不会为 MCP 处理器获取 Clipboard、Launcher、File Operations、Versions、Debug 或 DevTest 服务。
- Shell 在提供方启动之前发布 `CaptureWindowHandle`，在 GPUI 创建窗口时用实际的 FileAtlas HWND 填充它，并在停止其提供方之前停止 MCP 插件。
- MCP 使用官方 Rust `rmcp` SDK。正常启动在 GUI 进程中以 `http://127.0.0.1:38473/mcp` 提供 Streamable HTTP；`FILEATLAS_MCP_HTTP_PORT` 修改端口。`--mcp` 为诊断保留 stdio。
- Cordis 释放时中止 MCP 任务。若 Cordis 在启动期间无法发布生命周期
  守卫，传输任务会在插件返回错误之前被
  中止。不会启动任何 localhost/网络监听器。

## 工具契约

确切当前的特性 API 列在所属插件的 `PORTS.md` 中。
下面历史验证备注中的 `tools_read.rs`/`tools_write.rs` 名称指的是已退役的无命名空间入口点；宿主拒绝那些调用，
客户端应使用 `tools/list` 显示的带命名空间插件工具。

- `command_list` 读取实时的 UI Bridge 命令注册表，并报告命令
  ID、所有者、标签、菜单/快捷键放置、`CommandNeeds` 与当前可用性。`command_run { command_id, confirm? }` 派发与 UI 命令贡献使用的相同
  `UiCommandInvoked` Cordis 事件。Undo/redo
  需要可用的历史条目与 MCP 变更选择；已确认的变更额外需要 `confirm: true`。响应意味着事件
  已入队，而非异步操作已完成。当前元数据
  导出 UI 命令打开其面板；独立的 `export_metadata` 工具使用其目标与确认契约执行
  一次导出。

- `screenshot_window`：在 Windows 上仅使用 `PrintWindow(PW_RENDERFULLCONTENT)` 捕获 FileAtlas 顶层 HWND，将结果编码为 PNG，并返回 MCP 图像内容与像素尺寸。窗口缺失、最小化、无效或不支持时返回错误。视觉输出已在隔离验证配置中就 FileManager 与 Preview 进行了检查；其它屏幕与 DPI/窗口大小配置仍待检查。
- `resize_window`：为隔离布局检查更改当前 FileAtlas HWND 的外部尺寸。宽度限制为 640–4096，高度限制为 480–2160；结果包含请求的与实际的窗口边界，因为 Windows DPI 与显示器工作区限制会钳制请求。它不移动、激活或重排窗口；之后使用 `screenshot_window` 捕获以检查渲染布局。
- `list_library`：读取已注册文件夹 ID、路径、最后扫描时间/次数与层次化标签树。
- `search_media`：支持跨文件名/路径/标签的短语、文件夹 ID、标签 ID、媒体类型、偏移、最大页大小 100，以及源的排序顺序。
- `get_media`：返回单条已索引的媒体行及其当前标签。
- `get_view_state`、`open_media`、`control_preview` 与 `control_playback`：使用现有预览/播放服务检查/更改预览与播放状态。预览控件包括 `close`；实时图像渲染已用生成的 PNG 演练。视频/音频编解码器与设备输出仍需 Windows 运行时验证。
- `cancel_scan`：为某文件夹的活动扫描请求取消。
- `dev_test_status`、`dev_test_scan_fixture`、`dev_test_inject_virtual` 与
  `dev_test_clear_virtual`：在同一 FileAtlas
  进程中控制可选的 `DevTestService`。当 `FILEATLAS_DEV_PLUGINS` 包含
  `dev-test` 时这些工具可用；虚拟注入仅接受 1,000、10,000 或 100,000 行。
  启动操作立即返回；轮询 `dev_test_status` 直到 `running`
  为 false，然后使用 `search_media` / `list_library` 与 `screenshot_window`
  检查同一个运行中的应用。隔离的 `target/dev-test` 数据
  目录由 Shell 配置选择，而非 MCP 工具参数。
- `set_rating`、`assign_tags`、`remove_tags`、`create_tag`、`rename_tag`、`delete_tag`、`add_folder`、`remove_folder`、`rescan_folder`、`rename_media`、`copy_media`、`move_media`、`delete_media` 与 `export_metadata` 是变更工具。常规变更需要 Standard 或 Full；插件元数据标记了高影响的文件/导出、文件夹、标签删除与重新扫描操作，这些需要 Full 加上 `confirm: true`。批量限制为 100 个媒体 ID 与 50 个标签 ID。标签分配/移除只更新 SQLite，不写入 IPTC，也不创建 XMP sidecar。
- `delete_tag` 移除请求的标签及其直接关联；它传递 `clean_parents: false`，因为 MCP 请求只确认删除该标签，而不单独请求祖先分配清理。Navigation 服务返回的受影响媒体 ID 有意从现有的 `{ "ok": true }` 响应中省略。

MCP 访问默认为 Off。Standard 允许 UI 控制与常规变更；Full 额外允许插件声明的高影响变更。`confirm: true` 是显式的调用点确认，而非人类交互的证据；请配置 MCP 主机在调用变更工具前询问用户。复制/移动目标与媒体路径使用现有的 FileOperations 服务。不要为不受信任的客户端运行此 MCP 服务器。

## 当前限制与调试

- `get_view_state.preview.image_load` 暴露 Preview 实际的初次/完整绘制时间、质量、尺寸、错误、渲染次数与轮询唤醒原因。`get_debug_info.view_revision` 暴露抽屉独立的 UI/内容时钟。这些只读诊断不生成日志，也不使视图失效。
- `control_ui { action: "accessibility_activate", confirm: true }` 需要 Full 访问与每次调用确认，并仅请求属于本进程的、由 Shell 发布的 HWND 提供其原生 Windows UI Automation 提供方（`WM_GETOBJECT`、UIA 根 ID）。这会激活真实的 GPUI 无障碍更新，用于反馈环回归测试；不使用外部窗口附加或操作系统指针/键盘注入。超时限制为 2 秒；非 Windows 返回不支持错误。

- 正常 HTTP MCP 调用针对已在运行的 GUI 进程。`--mcp` stdio 端点仍是旧的隔离调试路径。
- MCP 工具可用性不会完成 `docs/source-feature-audit.md` 中列出的原生 UI 行为；工具调用相同的 Cordis 服务，并受其现有限制约束。
- 当工具缺失或启动失败时，检查"设置 → MCP → AI 控制"访问级别、是否传递了 `--mcp`、Shell 插件依赖顺序以及 stderr。
- 如果截图失败，先检查 HWND 发布与窗口状态。如果返回空白或过时图像，检查 PrintWindow/GPUI Windows 渲染行为；不要把协议编译当作截图一致性的证据。
- 批量扫描接收器由后台 worker 排空，以便索引 worker 可以完成；扫描进度在有扫描活动时通过 `get_ui_state.scan_progress` 暴露。

## UI 验证工具（2026-10-13）

`get_ui_state` 读取 settings_open/settings_tab、language、translation_revision
与 preview_chrome_alpha。这些是对 get_view_state 的补充；它们不描述
操作系统指针位置或全部 GPUI 控件状态。`control_ui { action, value? }`
需要 Standard 或 Full 访问，并支持 settings_open/settings_close、
settings_tab、language (en/zh-CN)、statistics、bulk_actions、file_manager_dialogs_close、
select_media（value 是已索引媒体 ID 字符串）与 deselect。Settings 使用公开
UiSurfaceState；language 使用 I18nService；FileManager 使用 FileManagerUiRequest。
Settings 与 FileManager 异步消费请求。在
screenshot_window 之前等待渲染完成。不使用操作系统鼠标/键盘注入。

快捷键录制诊断（2026-10-18）：`get_ui_state.settings_shortcuts`
包含 `capturing_key: bool`、`binding_id: String` 与 `error: String`。
空闲时 `binding_id`/`error` 为空；无效/冲突的组合键使绑定保持
捕获模式并暴露可见的内联错误而不持久化
它。值来自 Settings 的捕获转换；读取不会变更
UI。`capturing_key` 标志也涵盖现有的打开媒体键捕获。
使用真实的 `control_ui` 指针点击/按键
（`scripts/validate-shortcuts-layout.cjs`）进入这些状态。

`control_ui {action:"pointer_scroll", value:"x,y,delta_y"}` 通过现有有界的同进程输入队列
入队一个 GPUI
`ScrollWheelEvent`。
位置为 0–4096 内的逻辑客户端像素；delta 为
-4096–4096 内的有限逻辑像素（负值向下滚动）。需要启用变更，不发送
操作系统输入，并返回与其它指针动作相同的待处理队列错误。

`control_ui {action:"canvas_drop_files", request:{"paths":["…"]}}` 把文件路径按
“拖放到 WorkCrate 页面编辑器画布”的同等语义交给该视图；视图过滤图片、按需
打开官方会话并把每个文件以链接方式放置。GPUI 封闭了操作系统的
`ExternalPaths` 拖拽值，进程内无法合成真实 `FileDropEvent` —— 因此经
`workcrate.canvas.drop-request` surfaces 键走语义等价通道，由面板的预览
轮询消费。需要启用变更。

在一次 locked 的 Shell 构建之后运行 `node scripts/validate-ui-mcp.cjs`，以扫描
隔离 fixture 配置、捕获 23 个 UI 状态，并采样真实的 Preview
渐隐曲线。输出位于 target/mcp-ui-screenshots；该脚本从不
执行 delete/copy/move/update 操作。悬停/工具提示激活、原生
对话框与指针驱动动画仍未被此接口测试。

## S2 与启动更新诊断（2026-10-13）

`get_ui_state` 额外从共享
UiSurfaceState 返回布尔值 `inspector_open`、
`update_checking`、`update_available` 与 `update_offer_open`。`control_ui` 额外接受 `inspector_open`、
`inspector_close` 与 `settings_check_updates`（无需 value）。
现有变更守卫仍然适用。Settings 在其 100 ms GPUI 轮询器上消费手动检查
请求；更新完成是异步的。
Inspector 控件与 Shell 切换及
FileManager 关闭按钮操作同一表面状态。这些服务级请求与截图验证
状态/渲染；它们不证明指针点击在过滤
弹出层内部、原生对话框或每个设计状态中的传播。

## v5 实时 FileManager/Navigation 控件（2026-10-14 交接）

`get_ui_state` 添加 `file_manager_popover`、`tools_open` 与实时 FileManager
快照（过滤器、选中的 ID、匹配数量、月份分组与 inspector
分区）。`control_file_manager` 控制与 UI 相同的 FileManager 动作与受控
Popover 状态。动作：popover、search、media_type、sort、rating、
extension、tag、tag_mode、extension_mode、date_from、date_to、clear_filters、
inspector_section、tag_picker、month_grouping、tools_open 与 tools_close。
读取 tools/list 获取确切的可选字段与接受的字符串值。
`control_file_manager` 还接受 bulk_open、bulk_close 与
bulk_configure；配置使用 `value` 表示 delete/copy/move，
`bulk_source` 表示 all/folder/tag/selection。`get_ui_state` 报告实时
批量动作/来源。`open_media_menu`（带 `media_id`）与 `close_media_menu`
控制由 FileManager 拥有的实时媒体操作表面；其状态与已索引 ID 由
`get_ui_state` 报告。带 `media_id` 的 `preview` 动作经由
FileManager 的选中卡片预览处理器路由，可用于其 filmstrip 交接的同进程
验证。原生右键 ContextMenu 仍由
指针触发。`control_navigation` 接受 create/edit/delete/merge/move/close 及可选的目标必需 tag_id；这些打开对话框，从不提交
删除/合并。两个工具都需要现有变更标志与异步
视图轮询。
`get_ui_state.scan_progress.error` 可在 Indexing 进度观察者仍有该扫描活动时
报告终止性失败。Navigation 消费
同一终止事件并独立保留其 Retry 卡片；MCP 进度
快照可能在完成 sentinel 之后消失。

MCP `open_media` 使用已索引的 `media_id`
将图像 ID 路由到 Preview，将视频/音频 ID 路由到 Playback；
`get_view_state` 包含播放 ID。
`export_metadata` 设置目标并运行共享的
MetadataExportService 状态机；`get_ui_state.metadata_export` 报告
可见面板读取的 idle/exporting/succeeded/failed 状态。工具文本
JSON 可能在不带 CallToolResult.isError 的情况下携带错误；客户端必须两者都检查。新的
scripts/fileatlas-mcp-debug.cjs 这样做了，使用隔离目标配置、通过 screenshot_window 捕获
并在交互模式下保持一个子窗口存活。
它无法附加到另一个实例。当前继续中的更改未经验证；较早的成功 exe 早于最终布局编辑。
## v5 重设计验证接口（2026-10-14）

`control_ui` 额外接受 `sidebar_open/sidebar_close`、`export_open/export_close`、
`writeback_retry/writeback_dismiss`（value 是确切记录的失败路径）。Sidebar 控件
使用相同的 UI Bridge 状态与 Shell 切换；writeback 动作使用生产重试
载荷或失败消除，从不使用合成的成功结果。`get_ui_state` 暴露
sidebar/export 可见性以及带路径/错误的已记录 writeback 失败。

`control_preview` 额外接受 `next/previous`、`tags_open/tags_close`、
`more_open/more_close` 与 `filmstrip_show/filmstrip_hide`。请求影响已挂载 Preview
视图受控的 GPUI Kit 弹出层与现有 SettingsService filmstrip 标志。这些不
注入操作系统输入。`control_playback` 添加 `set_volume`（`volume` 为 [0,1.5] 内的有限值）、
`tags_open/tags_close` 与 `more_open/more_close`；`get_view_state.playback` 返回实际音量
与受控表面。UI 请求是异步的；在 GPUI 轮询器之后读取状态。
现有的隔离配置变更策略仍然适用。

## 2026-10-14 当前实现与 MCP 证据

control_file_manager 的动作 select_many（1–1000 个唯一存在的 media_ids）与 scroll（value 为从 0 开始的当前结果索引）是受变更守卫的 UI 请求，包括未加载的记录。get_ui_state file_manager 包含 window_offset/loaded_count 与 folder_id/path_prefix。100012 行的运行时观察到 window_offset99812/loaded_count200 以及跨页的 bulk2。详细的配置/计划/报告与当前限制：docs/redesign-validation.md 最新章节。

## v2 面板与 GPUI 输入验证（2026-10-14）

control_ui 添加 navigation_tab(folders/tags/extensions)、inspector_tab(information/history/extensions；别名 info)、sidebar_width/inspector_width（value：逻辑像素 100–1000，之后由宿主钳制）、pointer_down/pointer_drag/pointer_up（value：x,y 逻辑客户端坐标 0–4096）。这些 UI 控制动作需要 Standard 或 Full 访问。CaptureWindowHandle 现在持有原生句柄与有界的单个待处理 McpPointerInput{kind,x,y}；queue_pointer 拒绝 busy/poisoned，shell 的 take_pointer 排空/延迟派发 GPUI 事件。没有操作系统指针/键盘注入，也没有外部进程附加。get_ui_state 暴露活动页面与已应用的宽度。control_navigation 添加 expand_folder(folder_id)、expand_path(value：目录)、select_path(value：目录)、folder_search(value：查询)；请求校验已注册根包含并执行规范路径选择/惰性加载。已验证实时事件拖拽/点击与只读拒绝；参见 docs/redesign-validation.md。

## 导航页签标识（2026-10-23）

control_ui `navigation_tab` 接受当前 Navigation 页签标识 `folders`、`tags`、`project-tree`、`extensions`，因此项目树侧栏页可以按语义打开，不再依赖 rail 坐标点击。`get_ui_state.navigation_tab` 返回同一标识。项目树页本身没有 MCP 状态：根目录来自 `fileatlas.project-tree` JSON，每个根目录的扫描状态只在页面内渲染。

## 浮动面板与列表验证（2026-10-14）

`control_file_manager layout` 通过 FileManagerUiRequest::Layout 接受值 grid/list。`get_ui_state.file_manager.layout` 是实时渲染的选择。`get_ui_state` 添加 panels（内置可见性/框架/客户端边界）、panel_instances（当前所有打开的框架/客户端边界）、open_panel_ids 与 active_panel。边界是以逗号分隔的逻辑像素字符串；client_bounds 由 GPUI 测量，独立于截图 DPI/非客户端偏移。

受变更守卫的 `control_ui panel_open` 在 value 中接受一个已注册的定义 ID 并返回 instance_id；panel_close 接受该返回的实例 ID。两者都调用 UI Bridge PanelCatalog；它们无法制造未注册的内容。dev-test 为隔离的单/多实例检查提供实际的任务状态定义。pointer_move/pointer_right_down/pointer_right_up 额外使用 x,y 入队同进程 GPUI 事件；现有的 pointer_down/drag/up 保持不变。没有操作系统注入或独立进程附加。在读取截图/快照之前等待宿主/特性轮询器。

## 原生文件夹选择器验证（2026-10-14）

get_ui_state.folder_picker 返回 pending 与 native_open（Navigation 提供方实际找到了其当前原生窗口）。受变更守卫的 control_navigation folder_picker_cancel 调用 NavigationService::cancel_folder_picker；它只取消一个已打开的选择器。只读的 screenshot_folder_picker 通过现有 PrintWindow 编码捕获已识别的同进程原生选择器，不进行外部附加。调试客户端接受 capture_tool 用于计划的捕获；默认仍为 screenshot_window。实际的 Sidebar 加指针点击、原生截图、两个取消/重开周期与未变的库均已记录。这些控件不模拟文件夹接受，也不声称其已覆盖。

## 全部文件查询契约（2026-10-14）

`search_media.media_type` 现在除 `image`、`video` 与 `audio` 之外还接受 `file`；省略时查询所有类型。`file` 标识已索引的不支持/无扩展名文件，而非目录。同进程截图、FileManager 控件、原生 GPUI 指针派发与 Navigation 添加/重新扫描工具在隔离配置中验证这些卡片。

## 启动刷新诊断（2026-10-14）

`get_ui_state.library_refresh` 从 Watcher 拥有的启动扫描返回 `running`、`total_folders`、`completed_folders` 以及 `[path, error]` 失败项。这个只读字段让同进程 MCP 客户端等待协调并检查失败，而不发起重新扫描变更。`list_library` 与 `search_media` 随后显示当前持久化的行总数。

## 分组控件（2026-10-14）

`control_file_manager` 接受动作 `grouping`，值 `none|month|file_type|modified|created|folder|subfolder`；`toggle_group` 使用与 get_ui_state 完全一致的组标签；`popover` 接受 `grouping`。现有 `month_grouping` 布尔值仍是 Month/None 的别名。这些动作需要 Standard 或 Full 访问，并入队真实的 FileManager 动作。get_ui_state.file_manager 暴露 grouping_mode、有序组（格式为 [label,count]）、collapsed_groups、window_offset 与 loaded_count。search_media 仍是按查询排序的数据库查询；它不假装返回分组后的 UI 顺序。运行时命令/重启/截图记录在 docs/validation-ui/2026-10-14-grouping 下。

## 目录浏览控件（2026-10-14）

control_file_manager 添加 open_directory（value 为当前目录卡片列表中的条目）、directory_up 与 refresh。它拒绝不可见路径与无父级的视图；这些入队 FileManager 的真实动作并需要 Standard 或 Full 访问。get_ui_state.file_manager 发布 browsing、directories[{name,path}]、current_directory、parent_directory、directory_error 与 selected_directory；matched_count 仍是显示的已索引文件数量。目录卡片没有合成的媒体 ID。control_ui 添加 pointer_double_down/pointer_double_up（相同的 x,y 逻辑客户端坐标），由 Shell 作为同进程 GPUI MouseDown/MouseUp（click_count2）派发。单击/拖拽/右键行为不变。参见 docs/validation-ui/2026-10-14-folder-browser 的 84 步证据，包括鼠标命中测试与只读重启。

## 运行时诊断（2026-10-14）

运行时诊断工具：get_debug_info(category?,level?,search?,page?,page_size?) 从可选的 fileatlas/debug 服务返回抽屉真实的 ui 与 logs Page。只读，且有意不记录其自身的查询。control_debug(action,value?) 与原生控件共享 DebugService::control，并需要 Standard 或 Full 访问。未知服务/动作/无效值返回普通错误 JSON。被 MCP 拒绝的请求记录为 ERROR；stdout 仍保留给协议流量。

## 空闲回归矩阵（2026-10-15）

`get_ui_state` 添加 `settings_tag_choices` 与 `file_manager.dialog_reads`，来自
实际视图缓存，而不是新的数据库查询。对话框读取报告 statistics
状态/错误/photos/videos/audio/rating_counts 以及 bulk 状态/错误/数量/preview_ids。
受变更守卫的 `control_file_manager action=bulk_field` 需要 `field_index`
0..7 与 `value` 中的文本：older/newer 天数、min/max 评分、名称、扩展名、
min/max MB。它更新真实的 InputState 并重置确认；它无法
执行操作。可复用的同进程运行器支持 `--scenario
scripts/validate-idle-mcp.cjs`：隔离 fixture、启用原生无障碍、
Debug 跟随实时、稳定与配对的空闲观察、过时数据/错误
检查以及独立的活跃播放观察。持续/阻塞的情况导致
测试套件失败。阅读报告的覆盖/限制；这不是对
所有编解码器/操作系统集成行为的断言。

`get_view_state.preview.image_load.activity` 记录最后活动的调用方、
计数器与指针坐标。`scripts/validate-idle-followup.cjs` 添加
更长的静止/重开采样、实时写入器失效、已填充菜单、
插件 Single/Multiple 面板、Inspector 分区、原生 Watcher 刺激、
导出错误/恢复与诊断控件。有限的悬停/输入转换
会被保留并跟随有界的静默复查；持续工作仍然失败。

## 原生窗口与输入诊断（2026-10-15）

`get_ui_state` 添加 `input_activity` 计数器（移动事件、变化位置、按钮、按键、最后指针）与 `window_status`（valid、owned、visible、minimized、hung）。这些是被动式的，从不通知/记录日志。受变更守卫的 `control_ui {action:"window_restore"}` 在恢复之前校验所有权，仅恢复本子进程最小化的 HWND 且不激活。测试套件记录窗口中断并比较输入计数器；它们不能把未渲染/最小化的间隔称为安静通过。没有操作系统鼠标/按键注入。

## Debug 重绘显示过滤器（2026-10-15）

`get_debug_info` 接受可选 `hide_ui_refresh: bool`：省略/false 时保持与空闲
审计兼容的原始查询，true 时应用实际抽屉的常规
DEBUG/TRACE 无障碍重绘显示规则。`ui.filter.hide_ui_refresh`
报告抽屉默认值（true）。原始捕获/保留不变。
同进程场景 scripts/validate-debug-hover.cjs 验证真实的 GPUI 悬停、
原始与已显示查询、正常数据库记录、搜索与分页。
## Preview 缓存策略诊断（2026-10-15）

`open_media` 现在经由 FileManager 的
`preview_items_by_ids` 与 Preview 的 `set_indexed_filmstrip_items` 传递有序的 路径/ID/缩略图 三元组，与
FileManager 的打开路由一致。`get_view_state.preview.image_load.cache_policy` 暴露
渐进策略、邻近的已索引/显示缩略图路径与待处理任务
数量，且无需文件/数据库读取。`get_debug_info` 在原始消息中保留复用/取消
细节。不添加新工具或变更权限。


## 浏览器运行时更新 — 2026-10-15

get_ui_state.browser 中的浏览器诊断：filesystem、show_descendants、worker_active、status 元组（generation,revision,loading,scanned,error）。control_file_manager 添加 navigate(value)、show_descendants(enabled)、thumbnails(enabled)、cancel_directory、missing_records、record_candidate(media_id,value)、relink_record(media_id)、delete_missing_records（两次调用确认）、locate_file(media_id)、close_dialogs。所有 UI 变更都需要现有的选择加入机制。

`manage_tag_records`：list_missing 是只读的；relink(record_id,path) 与 delete_missing(record_ids,confirm=true) 需要变更选择加入。`manage_favorite`：rename(folder_id,name)、move(folder_id,delta)、变更选择加入。变更使用生产服务并发布常规重载事件。脚本 validate-browser-mcp.cjs / validate-browser-extended.cjs / validate-browser-restart.cjs 通过 fileatlas-mcp-debug.cjs 在隔离配置中运行。不使用操作系统输入自动化或外部进程附加。

## `get_ui_state.window_status` 中的窗口边界（2026-10-15）

`window_status` 现在还针对 MCP 子进程自己的 HWND 从
`GetWindowRect` 返回 `bounds`（`x`、`y`、`width`、`height`），句柄无效
或 rect 调用失败时返回 `null`。值是物理设备像素，因此
缩放显示器报告的宽高大于逻辑 GPUI 窗口尺寸；
逻辑测量请使用 `panel_instances` 的客户端边界。只读：这不
添加变更，也不添加新工具。

## 侧栏分区调试（2026-10-16）

- `get_ui_state.sidebar_folders` 返回已挂载的 Navigation
  侧栏发布的快照（`NavigationService::sidebar_state`），或在首次
  发布之前返回 `null`。它携带带使用量字节的驱动器、系统与用户快速
  访问行、含其文件夹的收藏组、库文件夹路径、
  折叠分区标志、`show_network_drives`、`browse_mode` 与打开的
  内联编辑器目标/文本。截图仍是视觉检查；分区状态
  由该字段断言。
- `control_ui { action: "sidebar_request", request: <SidebarUiRequest JSON> }`
  经由 `NavigationService::request_ui` 入队一次语义侧栏编辑，与已渲染的行和弹出菜单使用相同路径。载荷类型：
  `toggle_section{key}`、`set_network_drives{show}`、`toggle_drive{root}`、
  `activate_path{path}`、`toggle_expanded{path}`、`add_to_library{path}`、
  `add_quick_access{path}`、`remove_quick_access{path}`、
  `add_favorite_folder{group_id,path}`、
  `remove_favorite_folder{group_id,folder_id}`、`remove_favorite_group{group_id}`、
  `open_editor{editor,text}`、`confirm_editor`、`cancel_editor`。编辑器目标：
  `add_quick_access`、`rename_quick_access{path}`、`create_group`、
  `rename_group{group_id}`、`add_favorite{group_id}`、
  `rename_favorite{group_id,folder_id}`。
- 该动作返回 `{"queued":true,"ui_state":...}`。请求在下
  一次视图渲染时应用，因此在断言之前轮询 `get_ui_state`（请求待处理期间
  侧栏计时器会持续重新通知）。Navigation 校验目标与
  路径；未知的组/文件夹/分区或缺失的目录返回错误，
  且永远不会到达视图。
- 与其它所有控制动作一样，这由 Standard 或 Full 访问门控。它不注入操作系统指针/键盘事件，
  也不打开弹出菜单：`control_ui` 的
  `pointer_move`/`pointer_down`/
  `pointer_up` 会，且其结果通过截图检查。
- `window_status.bounds`（与启动窗口尺寸调整工作一起添加）以物理像素报告
  原生 `GetWindowRect`，这解释了
  截图的像素大小；分区状态本身由
  `sidebar_folders` 断言，而非由截图。

## `get_ui_state.window_status` 中的指针几何（2026-10-16）

`screenshot_window` 捕获整个窗口矩形（`PrintWindow` +
`GetWindowRect`），而 `control_ui` 指针动作派发 GPUI 逻辑
客户端点。`window_status.geometry` 弥合了这一差距，使点击目标被
推导而非猜测：

- `scale_factor` — `GetDpiForWindow(hwnd)/96`，与 GPUI 使用的值相同。
- `client_size` — `GetClientRect` 除以 `scale_factor`，即应用
  布局所用的逻辑客户端区域。
- `screenshot_to_pointer.{offset_x,offset_y,divide_by}` — 窗口矩形内的客户端原点
  （`ClientToScreen(0,0)` 减去 `GetWindowRect` 的 left/top）
  与除数。指针坐标 = `(screenshot_px - offset) / divide_by`。

在 `resize_window` *之后* 读取 `geometry`：启动窗口与缩放后的窗口
报告不同的大小，且偏移是每个 HWND 状态各自相关的。在本主机上，探针
返回 `scale_factor 1.5`、`offset (11, 45)`，且 1462×956 物理窗口矩形对应
960×600 的逻辑客户端。只读：不添加变更，也不添加新工具。



## 评审更正 — 2026-10-16

`control_ui.sidebar_request` 额外接受 refresh_roots、retry_save、toggle_group (group_id)。`get_ui_state.sidebar_folders` 包含 roots_revision、system_folders_pending、save_error、directory 的 child_count、drive 的 usage_error 与 editor.error。这些常规控件需要 Standard 或 Full 访问。测试场景已签入 scripts/validate-sidebar-review*.cjs 与 validate-browser-selfwatch.cjs；指针坐标使用 get_ui_state.window_status.geometry。


## 原生激活回归工具 — 2026-10-16

受变更守卫的 control_ui 动作 window_minimize 与 window_activate 仅针对当前 MCP 子进程拥有的已发布 HWND（ShowWindowAsync，以及激活时的 SetForegroundWindow）。它们演练真实的窗口生命周期事件，不进行操作系统输入注入，也不控制另一个进程。window_status.active 报告实际的前台所有权，因此测试不会把已入队的请求等同于激活。get_ui_state.file_manager.snapshot_revision 独立于目录 generation 报告 FileManager 查询调度。复现：scripts/validate-browser-refresh.cjs。


## 地址栏与按键事件 — 2026-10-16

受变更守卫的 control_ui 动作 address_bar 接受 request:{kind:back/forward/up/edit/cancel/submit/set_text/crumb}，其 text/path 载荷由 Navigation::AddressBarRequest 定义。它入队与控件相同的已挂载视图处理器。只读诊断是 get_ui_state.sidebar_folders.address_bar（path/editing/text/error/breadcrumbs 与 can_*）。

control_ui key_press 在 value 中只接受 enter、escape、ctrl-l、alt-left、alt-right、alt-up。CaptureWindowHandle::{queue_key,key_pending,take_key} 向 Shell 传递单个待处理的 String；现有的 MCP 输入轮询/派发租约通过同一 GPUI 窗口投递 PlatformInput::KeyDown，而非操作系统注入。字段 prefer_character_input 为 false。场景中实际的文本输入通过 set_text 使用真实的 InputState；该设置不声称实现了实际的键盘/粘贴/IME 输入。这些 UI 控制工具需要 Standard 或 Full 访问。

已签入的脚本：validate-address-bar.cjs（语义路由 + 实际 Enter/Escape）、validate-address-pointer.cjs（真实指针控制）、validate-address-restart.cjs 与 prepare-address-tag-restart.cjs（作用域稳定性）。参见 docs/validation-ui/2026-10-16-address/README.md。


## 区域分离探针与控件（2026-10-16；2026-10-23 移除）

`get_ui_state.region_separation` 字段与 `control_ui` 的 `region_separation`
动作已随底层 `RegionDepth` 令牌一同移除：统一的 Fluent 轻质感浮起样式使
所有区块恒为带阴影的浮起卡片。历史验证证据保留在
docs/validation-ui/2026-10-16-region-separation/README.md。

## 设置软件检查（2026-10-17）

`get_ui_state.settings_software_check` 发布"设置 > 软件"显示的被动
快照：`state`（`loading`/`ready`/`failed`）、`revision`，以及包含显示标识、安装状态、检测到的版本/路径、
用途与项目/下载/GitHub URL 的 `software`。`control_ui {action:"settings_tab",
value:"software"}` 选择用于同进程验证的视图。检测器
在后台 ReadModel 中读取 Windows 卸载/App Paths 注册表项与常规可执行文件
位置；它不启动、安装或查询列出的应用。`scripts/validate-software-check.cjs` 使用隔离配置演练该页面
与快照。Everything 的应用下载是 voidtools 的
网站；其链接的 GitHub 仓库是官方 ES 命令行工具，
而非 Everything 应用程序。

搜索位置（2026-10-17）：受变更守卫的 `control_file_manager` 动作
`search_entire_computer` 接受 `enabled`（默认 true），并调用与顶栏切换相同的
FileManager 动作。`get_ui_state.file_manager.search_entire_computer`
报告渲染出的选择。范围语义针对真实 SDK
结果与同进程 GPUI 点击验证；不使用操作系统输入注入。

### 启动器交互诊断（2026-10-17）

`get_launcher_state` 在条目中添加可空的 `installed_on`（YYYY-MM-DD）与 `size_bytes`，并在其面板存在时添加可空的 `ui` 投影：selected_entry、draft_name/category/hidden、sort（name/installed/size）、descending、visible_ids/count、icons_loaded/failed/pending 与 render_count。销毁视图会清除该投影。

受变更守卫的 `add_launcher_program {path}` 调用启动器的公开手动添加服务；它从不启动目标。受变更守卫的 `control_ui input_text` 最多接受 1000 个可打印字符，并将文本发送到同一进程中当前聚焦的 GPUI 输入处理器。`key_press` 另外允许 ctrl-a 与 backspace。这使用现有的窗口输入队列，没有操作系统注入或剪贴板改动。同进程启动器验证会把一个无窗口可执行文件编译进隔离配置；双击其图块仅在该可执行文件旁边写入一个标记文件。参见 `scripts/validate-program-launcher.cjs`。

受变更守卫的 `set_launcher_pin {id, scope}` 通过 `LauncherService` 将 `scope` 设为 `none`、`category` 或 `all`。`get_launcher_state.entries[].pin_scope` 返回相同的 scope 字符串，且 `ui.visible_ids` 报告视图当前的置顶排序。`scripts/validate-program-launcher.cjs` 验证跨名称/日期/大小排序方向与面板重开时的多个置顶条目。

剪贴板收藏：`get_clipboard_state` 暴露 `favorite_count` 与每个条目的 `favorite` 标志；`pinned_count`/`pinned` 仍是兼容别名。`add_clipboard_text.favorite` 接受旧版 `pinned`。UI 元数据包含 `confirm_favorite_delete` 且不暴露内容。
# 主题诊断（2026-10-18）

只读的 `get_ui_state.theme` 暴露 `id`（`workbench-navy`）、当前 `accent`、
`selection`、`base`、`panel`、`card`、`border`（打包的 RGB 整数）与 `body_size`
（逻辑 px）。这些值读取公开的 UI Bridge 契约，无需额外 IO 或
通知；它们证明所选令牌，而来自同一进程的 `screenshot_window` 证明其实际可见的应用。不添加新的变更工具；
强调色变化通过真实的 Settings 控件演练。

## 程序工作区与列诊断（2026-10-19）

当可选的
`fileatlas.columns` 与 `fileatlas.workspaces` 服务活动时，`get_ui_state.program_workspace` 是只读快照。它报告
运行时布局、从 0 开始的激活列、布局修订、活动工作区
定义、默认工作区 ID、所有已保存的工作区/列定义，以及
工作区管理面板是否打开。任一插件
不可用时它为 `null`。工作区地址与过滤定义是配置
数据；该诊断不枚举媒体，也不触及 FileManager 数据库。

用于隔离的同进程 UI 验证，受变更守卫的 `control_ui` 动作
在 Standard 或 Full 访问下可用：`columns_layout`（`value` 设为
`single`、`dual` 或 `quad`）；`activate_column`（从 0 开始的数值 `value`，
服务校验会拒绝超出活动布局的索引）；`workspace_select`
（带已存在的工作区 ID）；以及 `workspace_manager`（`open` 或 `close`）。
工作区选择使用公开的 WorkspaceService 并更新活动
工作区；布局与列激活使用 ColumnLayoutService。这些动作
不能删除、重命名或重写工作区定义。`workspace_manager`
只更改面板打开状态。每次动作后重新读取 `get_ui_state.program_workspace`
以验证产生的服务状态。
# 统一收藏组（2026-10-18）

`control_ui {action:"sidebar_request", request:{kind:"add_current_favorite",group_id}}`
使用 Navigation 真实的组加号处理器，并在动作时校验已提交目录。
仍需变更守卫。只读 `sidebar_folders` 添加
`favorite_current_path`、`favorite_action_error`、`favorite_imported_paths`；每个
`favorite_groups[]` 带有以逻辑客户端坐标测量的 `header_bounds`，
因此真实 GPUI 指针点击可以在布局完成后定位到可见的加号/菜单/折叠箭头。
边界只是诊断信息，绝不是控制另一进程的指令。场景：
`scripts/validate-favorite-groups.cjs` 与 `validate-favorite-groups-restart.cjs`。
# 声明式设置提供方 — 2026-10-18

`control_ui {action:"settings_plugin_page",value:<owner>}` 接受带有活动
`UiViewContribution.settings_page` 的所有者，以及现有的运行时管理
工具。已禁用的运行时工具页面对于已保存的设置仍可访问。未知
或已被移除的仅提供方所有者会被拒绝。设置验证场景
是 `scripts/validate-settings-minimal.cjs`；它使用同进程的指针/按键事件。


## 原生窗口测试 — 2026-10-18

`get_ui_state` 包含 `selected_window`、`main_window_status` 与 `native_panels`
（ID/HWND/所属窗口状态/render_count）。`control_ui` 动作 `window_select` 接受
`main` 或某个活动的原生实例 ID，并为截图、缩放与
同进程指针/按键派发选择目标。需要变更权限。选择会拒绝
未注册或非本进程所有的 HWND；关闭所选原生实例时回退到 main。
`window_minimize`/`window_activate` 作用于该选中窗口。没有外部应用输入。

`control_ui.window_close` 仅对所选、已注册的原生插件 HWND 投递 WM_CLOSE；main 被拒绝。它演练与系统窗口装饰使用的相同的 OS 关闭回调。

## Windows 拖出诊断 — 2026-10-19

`get_ui_state.drag_drop` 报告最近一次 `DragDropService` 的原生生命周期：
`idle`、`queued`、`active`、`dropped`、`cancelled` 或 `failed`。这让
同进程的 `control_ui` 指针手势可以验证 FileManager 回调
已入队并进入 OLE 拖拽路径，以及应用已回到可用
状态。`control_ui` 指针输入是合成的 GPUI 输入；它不模拟
按住的 OS 鼠标按钮，也不验证拖放到 Explorer/另一个进程。那些
系统级投递路径仍需一次真实的 Windows 拖拽。

## 实时 MCP 变更访问 — 分级权限

MCP 通过 `fileatlas.mcp` 发布 `mcp/settings` 设置页面视图；它使用
现有的 SettingsService，不拥有重复的持久化。该分段
设置为 Off、Standard 或 Full，并对每条宿主与贡献的变更
路由进行检查。Standard 涵盖 UI 控制与常规变更；Full 还
允许插件声明的高影响工具，这些工具保留每次调用的 `confirm=true`。
更改立即生效，无需重启 MCP。下面旧的布尔键验证
是历史性的；当前的同进程 Windows 证据记录于
`docs/validation-ui/2026-10-19-mcp-access-stages/`。

## 实际内容绘制配对 — 2026-10-19

`get_ui_state.theme.content_colors` 是只读的：`formats` 列出由 FileManager
使用的同一 `file_format_colors` 中的扩展名、
背景与前景；`tags` 列出 `tag_colors` 中的 tint/selected/background/foreground/icon（neutral、
black、white、purple、blue 与 yellow，两种选择状态）。`selection_text`、
`media_background`、`media_text` 与 `media_star` 解析实际的导航与
缩略图角色。该投影不执行 IO，也不添加计时器或变更
权限。`scripts/validate-thumbnail-contrast.cjs` 使用这些角色加上真实
隔离本地文件、已分配标签、评分，以及捕获的同进程窗口。
