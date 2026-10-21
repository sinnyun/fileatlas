# Shell composition ports / Shell 对接

## 主工具栏视图组合 — 2026-10-19

宿主将活动栏格的 `command.column.{i}.view-modes`、
Columns 自有的 `command.columns` 下拉框，然后是活动栏格的
`command.column.{i}.view-options`，挂载到可滚动过滤器旁边的
固定尺寸水平组中。FileManager 保留视图行为；Columns 保留布局与
活动栏格动作。可见顺序为网格、列表、布局、子文件夹、缩放。

## QuickLook 替换 — 2026-10-16

Shell 在 FileManager/MCP 之前启动 `fileatlas.quicklook`，并在两个
消费者之后停止它。Preview 与 MediaPlayback crate、依赖以及启动和
关闭路径已被移除。其历史依赖顺序在下方已被取代：QuickLook → FileManager → MCP。AI 建议由 FileManager
通过其现有的 AI/Settings/Tagging/History 依赖注册。通用浮动
面板仍供 Settings 及其他特性贡献使用。Shell
不包含任何文件查看或安装实现。

`fileatlas-shell` 是宿主组合根。它发布 `UiBridgeService`，按依赖顺序启动 Cordis 插件，将 GPUI 视图工厂挂载到宿主区域（包括浮动的 `workspace.modal` 视图），并把命令 ID 作为 Cordis 事件分发。Settings 命令由 Settings 插件贡献；其模态打开状态通过 `UiSurfaceState` 共享，其浮动层挂载在应用内容之上。

同一份 UI Bridge 命令目录通过 MCP 的
`command_list` 与
`command_run` 暴露；因此 Shell 渲染与 MCP 分发使用生命周期拥有的
命令 ID 与策略元数据。处理函数保留动作时的校验。

宿主框架与贡献回退视图使用 `fileatlas_ui_bridge::theme` 的表面与文本 token；原生底层表面使用源码的 Mica/Acrylic alpha。GPUI Kit 的排版预设仍被 shell 使用；与源码 Segoe UI 字体加载及 Windows 字体可用性的对齐尚未验证。

当 `SessionState.sidebar_width` 在 200 到 400 逻辑像素之间时 shell 恢复该值（否则为 264 px），并在侧边栏分隔条拖动结束时保存宽度。updater 在 Settings 之前启动。当 Settings 确认更新后，它会持久化当前会话快照、将暂存路径加入队列并退出 GPUI；应用循环返回后，shell 在销毁 updater 服务之前安装并重新启动暂存的构建。此更新的关闭路径仍需交互式 Windows 验证。

显式的 `dev-test` 配置会在 FileManager 及其
提供方之后启动 `fileatlas-plugin-dev-test`。它将数据库、Settings、Session 与缩略图缓存
重定向到 `target/dev-test/`（或 `FILEATLAS_DEV_TEST_DIR`）；`FILEATLAS_DEV_TEST_DB` 只能
覆盖数据库文件。该配置让本地媒体加载检查
不触及用户正常的 FileAtlas 数据库与设置。

## 当前运行时依赖顺序

Database → FileOperations → DragDrop → FileManager；Database → Preview → FileManager；Database → MediaPlayback → FileManager；Database → Thumbnails → Indexing → Watcher；Navigation 依赖 Database/Indexing 并可选依赖 Watcher；Settings 读取 Updater 和 Session 并可选读取 Navigation；Tagging 依赖 Database；History 在 Database、FileOperations 与 Tagging 之后启动。Session 与 Updater 在 Settings 之前启动；当 FileManager、MCP、所有插件或显式的 `ai` 开发插件启用时，AI 在 Settings 之后启动。Shell 在所有必需提供方以及可选的 Settings/Tagging/History 提供方之后启动 FileManager。Preview 通过 Cordis 消费 AI 建议。在 `--mcp` 模式下，它还会启动所有必需的特性提供方，发布 Shell 自有的 `CaptureWindowHandle`，并在 FileManager、Navigation、Tagging、Preview、MediaPlayback 与 FileOperations 之后启动 `fileatlas.mcp`。GPUI 退出后，Shell 在销毁 Updater 之前消费任何已入队的更新安装。关闭时它在 FileManager 之前停止 MCP，然后在 History、AI、Settings、Tagging 与 MediaPlayback 之前停止 Preview；Navigation 在 Watcher、Indexing 与 Thumbnails 之前停止。在依赖插件启动前保持服务可用，并在提供方之前停止消费者。FileManager 在 Preview 中打开图像，把视频路由到 MediaPlayback，从卡片拖动手势调用 DragDrop，并可选消费 Settings、Tagging 与 History 服务。

## 边界

应用启动在
`run/init/open_window` 之前用 `with_assets` 注册 `gpui_kit::assets::Assets`。这提供原生组件
内部使用的 SVG（例如 DatePicker 日历以及菜单勾选/箭头图标）；它不替代
FileAtlas 源码的 MDL2 字形，也不改变窗口布局或特性归属。

shell 不得吸收扫描、SQL、过滤、打标或文件操作。它通过 `fileatlas-plugin-mcp::CaptureWindowHandle` 发布真实的 GPUI HWND；MCP 适配器在 Windows 上只捕获这个 FileAtlas 窗口。在释放共享运行时资源之前先停止插件 fiber，以便 Cordis 清理能移除已发布的服务与 UI 贡献。有源码依据的行为映射记录在 `docs/source-feature-audit.md`；Windows 运行时与视觉一致性仍需原生验证。

## 源窗口条带与本地化（2026-10-13）

宿主组合 `command.search`、`command.size`、`command.filters`、
`workspace.sidebar`、`file_manager.content`、`workspace.status`、`command.tools`
与 `workspace.modal`。FileManager 拥有全部过滤/选择控件；Shell 只
放置全宽的命令/状态条带和单个抬高的 FileManager 卡片。
`settings.language` 由 Settings 挂载，而不是作为工作区面板。i18n
插件在视图之前启动；翻译版本号会刷新原生窗口。
参考几何：源码 main.rs:1007-1367。实时 MCP 截图确认
这些区域已渲染；细节控件、扫描状态与悬停一致性仍
不完整。参见当前的验证记录。

## v2 分页面板分隔条（2026-10-14）

Shell 拥有两侧宽度、可见的 8pt 分隔条命中区域与 24pt 抓握区；特性页面仍归 Navigation/FileManager 自有。侧边栏默认 264pt、范围 200–400；检查器默认 300pt、范围 240–480。拖动使用来自初始宽度的指针增量，根层的 move/up/out-up 处理使其在越过相邻视图时保持活动，释放时保存 SessionState.sidebar_width/inspector_width。调整大小会为中心区域保留 320pt 空间。MCP 宽度请求使用相同的 setter/保存路径；shell 发布 shell.sidebar-width/shell.inspector-width。CaptureWindowHandle 指针邮箱通过渲染租约后的 Window.defer 排空，在本子窗口内分发原生 GPUI PlatformInput（不使用 OS 输入）。实时 MCP down/drag/up 将其改变 264→304 与 300→340；重启后两者均恢复。Settings 入口位于 48pt Navigation 导轨下方。源码/schema/后端不受影响。参见 docs/redesign-validation.md 最新 v2 证据。

## 分组组合与 Tools 滚动（2026-10-14）

只要 FileManager 启用，Shell 就启动 `fileatlas-plugin-grouping::GroupingPlugin`，并在 FileManager 之后销毁它；分组算法/状态仍归特性自有。FileManager 现在要求该公开服务。Tools 覆盖层使用一个绝对外层容器（300pt 宽，可用高度有界最大 600pt）与一个内部 GPUI Kit 滚动包装器：该包装器不保留绝对外层定位，因此以前直接把它应用于定位面板会扰乱 shell 布局。现有工具命令/视图生命周期注册不变。隔离的同进程 MCP 验证了 1700×1100 的 Tools 布局以及真实的分组选项点击；参见 docs/validation-ui/2026-10-14-grouping/README.md。

## 目录浏览组合（2026-10-14）

启用 FileManager 也会启用 Navigation 及其 Indexing/Watcher 依赖，因此未分组的文件夹卡片经由其公开服务路由。产品级列表/查询/渲染保留在特性 crate 中。MCP 指针分发器在原有单击/拖动/右键输入之外识别 click_count 为 2 的 double_down/up；它仍是延迟的同进程 GPUI 事件。不使用 OS 输入注入或单独启动的调试应用。

## 运行时诊断（2026-10-14）

Shell 现在在其他特性启动之前组合 debug 插件，最后销毁它，并承载生命周期拥有的 workspace.bottom/workspace.debug-status 槽位。诊断策略、存储与 UI 仍位于 fileatlas-plugin-debug。参见其 PORTS.md。

## 共享刷新与被动输入诊断（2026-10-15）

Shell 的生命周期/界面/语言环境轮询使用 UI Bridge 的 refresh::poll；语言环境变化仍会刷新组件窗口。一个透明画布在绘制期间注册捕获阶段的 GPUI move/down/key 观察者，仅为更新 MCP 有界的内存输入计数器。它不拦截输入，也不通知或记录日志。


## 地址交互验证 — 2026-10-16

标题栏继续承载 Navigation 的 command.breadcrumb 贡献；地址/历史行为仍位于 Navigation 特性中。Shell 的同进程 MCP 分发器另外消费 CaptureWindowHandle::take_key，并在渲染租约之后将解析出的 GPUI Keystroke 作为 PlatformInput::KeyDown 分发。poll_panel_requests 只在确实有待处理按键/指针时唤醒。不向 OS 或其他应用注入键盘输入。

## 原生工具窗口组合 — 2026-10-18

前台纠正：新建与复用的原生面板现在共享 `activate_panel`，
并延迟到其原生窗口与 Root 存在之后执行。它只显示/恢复那个
HWND，并调用 GPUI 平台激活来处理前台/焦点。不使用
永久置顶样式或抢占焦点的轮询。主窗口保持最小化。
证据：docs/validation-ui/2026-10-18-native-foreground/README.md。

`shell/native_panels.rs` 在独立的 GPUI Kit Root 窗口中承载 UI Bridge 原生面板工厂，
使用各自定义的标题/默认/最小尺寸以及共享主题。
系统框架负责移动/调整大小/关闭。关闭/Escape 委托给 PanelCatalog；
Cordis 属主清理会移除原生记录，宿主在下一次
共享轮询时关闭其窗口。主工作区渲染从不被抑制。只有内嵌面板的启动
会请求恢复主 HWND。原生重开只激活/恢复其自身的 HWND。
MCP 捕获/输入可以选择 `main` 或某个活动的原生面板 ID；两者都留在这个子进程中。
# Shell 控件归属（2026-10-19）

Shell 按命名槽位组合插件视图。Settings 插件拥有
`workspace.sidebar-footer`；`fileatlas-plugin-shell-controls` 拥有侧边栏、
检查器、工作区管理器以及 Tools 动作视图。Shell 保留框架、
品牌标识、区域布局、状态组合、调整大小表面以及插件命令贡献的通用
渲染。新的应用动作属于生命周期拥有的
视图贡献，而不是硬编码的 Shell 按钮。


## 原生主题 / Material 生成器（2026-10-19）

Shell 在 Settings 之后启动编译期 MaterialThemePlugin，并在 Settings 之前
销毁它。该插件拥有其服务/设置/MCP 贡献以及带版本的生成
文件。Shell 在 GPUI 启动时初始化 UI Bridge 的原生主题监听器；主题 JSON 由
Settings 在启动/动作时加载，绝不由 Shell 渲染加载。自定义宿主表面与
插件一样消费同样的原生 ThemeColor 角色。原先的组件覆盖调色板已移除。
