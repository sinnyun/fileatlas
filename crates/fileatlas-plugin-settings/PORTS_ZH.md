# 原生 GPUI 主题偏好 — 当前 2026-10-19

Settings 保留 `color_mode` 并新增 `theme_file: Option<PathBuf>`（serde 默认 None）。
None 选择 GPUI 的默认主题对。启动/操作时的持久化会读取所选的原生
ThemeSet JSON，经由 UI Bridge 排队应用，并在文件缺失或无效时回退到原生默认值，
同时保留原始数据。文件读取不在渲染中执行。
旧的 accent_name 仍会被序列化以兼容，但不再驱动任何私有调色板。
外观页面选择框架默认值与当前生成的主题；Material
插件注册自己的设置页面以进行种子/样式/对比度生成与激活。
现有侧栏模式操作仍通过 Settings 持久化；所有框架/自定义视图都从
原生主题网关获取更新。下方较早的手动主题笔记仅是历史实现记录。

## 全局布局密度与间距 — 2026-10-23

Settings 的 `ui_density` 是可选 JSON 字符串，旧文件缺失时默认为 `standard`；有效值为
`compact`、`standard`、`comfortable`。外观页以同一设置项统一缩放 UI Bridge 三档间距、
共享按钮/输入控件/导航行高度。所有插件读取 UI Bridge 运行时令牌，不提供插件级覆盖。
颜色继续由 `color_mode` 与 GPUI Kit 原生 ThemeSet/Material 主题页控制；不增加第二份颜色状态。
该字段与既有 Settings JSON 同文件保存，没有迁移步骤；未知值安全回退到标准密度。

# Settings 插件对接 / 设置面板对接

## QuickLook 替换 — 2026-10-16

内嵌的文件查看器及其 Toolbar 设置分区已被移除。
Settings 现在从外观页开始，提供外观、快捷键、标签、AI、
更新与软件这几个分区。空格与回车始终调用 QuickLook；旧的可配置打开
键仍被接受。旧的 toolbar/filmstrip JSON 字段作为惰性
兼容数据保留，因此现有设置不会丢失已持久化的值。它们
不再有任何活动的查看器控件或处理器。后文关于
Toolbar 选项卡的历史描述均以本节为准。

状态：与源码兼容的设置模型和 JSON 持久化处于活动状态。本插件贡献了一个 Cordis 拥有的浮动 Settings 窗口，包含六个源自源码的分区、一个切换命令、关闭按钮、Escape 处理以及可拖拽标题栏。控件通过 `SettingsService::update` 立即写入，因此 UI 状态与 `%LOCALAPPDATA%/FileAtlas/settings.json` 保持同步。强调色选择在启动时和每次编辑后更新共享的 UI bridge 主题状态。Windows 视觉与交互验证仍未完成。

## Cordis 与 UI 生命周期

- 插件：`fileatlas.settings`；服务：`fileatlas/settings`（`SettingsService`）。
- 需要宿主 `fileatlas/ui-bridge` 服务。它发布 settings 服务，贡献 `workspace.modal` Settings 视图与 `settings/toggle` 命令/窗口菜单项，并在同一插件所有者名下注册它们。
- 插件停止时，Cordis 效果清理会移除 UI 贡献元数据、命令监听器和视图工厂。该服务与 `UiSurfaceState` 由其宿主生命周期持有。
- `settings.language` 扩展槽位在 Settings 渲染期间按贡献 ID 和代号（generation）从活动的 UiBridge 目录进行调和（reconcile）；注销会丢弃旧的 `AnyView`，重新注册则构造全新的实例。
- Shell 将 `workspace.modal` 挂载在应用内容之上。Settings 面板初始为关闭状态，首次打开时居中，宽度根据视口在 680–980 px 之间。其标题栏可拖拽；Close 与 Escape 会关闭它。在捕获按键处于活动状态时，Escape 会取消捕获而不关闭 Settings。共享的 bridge 状态使宿主命令可以打开窗口、面板也可以自行关闭。

## 已迁移的设置控件

面板包含外观、快捷键、标签、AI、更新、软件、存储和插件分区。存储页汇总运行时插件及已注册设置页的存储路径，按插件分组显示路径、内容用途和占用大小，并提供刷新。目录路径可能重叠，因此不显示简单相加的总量。具体统计语义见 PORTS.md 的存储章节。程序设置 → 软件是面向 QuickLook 与 Everything 的只读探测器；截图工具插件页面单独报告 Shotoshot。

## 插件设置与运行时控件 — 2026-10-18

Settings 侧栏分为程序设置和插件设置两组。插件概览卡片对 `fileatlas.launcher`、`fileatlas.clipboard` 和 `fileatlas.screenshot` 具有生命周期感知；它们各自的独立页面暴露启动呈现设置。`Settings.plugin_enabled` 持久化启用状态，并将缺失条目默认为启用。`PluginRuntimeRegistry` 持有 Cordis `FiberHandle` 和 shell 提供的 spawn 方案（recipe）；停用时等待 `FiberHandle::dispose`，启用时通过宿主 `CordisContext` spawn 一个全新的预备插件。该注册表仅限于这三个独立工作台工具。它们依赖核心的 Database/Settings 服务，这些服务在此不由用户切换；不引入通用的依赖级联管理器。

`Settings.plugin_launch_modes` 以 Cordis 所有者 ID 为键，默认为 `WorkspaceAndPanel`。Launcher/Clipboard 支持 `PanelOnly`；Screenshot 还支持 `Direct`，它直接调用 Shotoshot 截图而不打开 FileAtlas 面板。`PanelCatalog::open_presented(id, true)` 现在通过宿主创建一个独立的原生窗口，不再恢复主窗口或抑制工作区。旧的全局 `panel.presentation` 渲染路径已被移除。设置变更在下一次打开插件时生效。修正后原生呈现方式的运行时证据记录在 docs/validation-ui/2026-10-18-native-panels/。

`fileatlas.whiteboard` 插件页的“数据存储位置”列出 `whiteboards/library.json`、
`whiteboards/previews/` 和 `whiteboards/pasted-images/`。JSON 文件保存画板/场景、
资源与 Eagle 链接、模板、颜色和预览引用；该画布不使用共享 SQLite 媒体数据库。
插件设置页对每个已登记存储位置异步显示文件/目录占用大小，结果由 `ReadModel`
缓存，刷新按钮显式重新统计。目录递归统计且不跟随符号链接；共享媒体数据库的
大小包含 SQLite `-wal`/`-shm`。缺失路径计为 0 B，不可完整读取时显示部分大小提示；
重叠目录的行不可直接相加。文件系统 IO 不在渲染阶段执行。

快捷键页面还会从 `UiCatalog.commands()` 发现生命周期拥有的命令键绑定。
每个插件注册一个稳定的键绑定贡献，指向其命令并声明 `Application`
或 `System` 作用域。页面显示所有者与作用域，可捕获替换按键组合、重置为插件
默认值，或存储一个空覆盖以禁用该绑定。覆盖按贡献 ID 持久化
在 `Settings.keybindings` 中；缺失条目使用插件默认值。宿主通过
`UiCommandInvoked` 分发应用作用域的覆盖。快捷键服务在其
GPUI 轮询线程上应用系统作用域的覆盖，并在按键捕获期间挂起全局注册。

## 外部软件检查 — 2026-10-17

程序设置 → 软件选项卡通过已注册的卸载项和
App Paths 注册表项以及常见可执行文件位置来检测 QuickLook 与 Everything。它只读取注册表值
并检查文件是否存在；从不启动这两个应用或调用
安装程序。刷新请求一份新的后台 `ReadModel` 快照。
结果包括安装状态、检测到的版本/路径、项目/下载 URL，以及
存在时的官方 GitHub URL。Everything 的 GitHub URL 指向
voidtools/ES（官方命令行工具），而 Everything 本身
从 voidtools.com 下载。当前选项卡仅报告 Everything
是否存在；不查询 Everything，也不集成其搜索。

## 剩余 UI 对接项

- 外观：保存的强调色选择在插件启动时和每次设置编辑时应用到共享的 `fileatlas-ui-bridge::theme` 强调色状态。`System` 选项跟随 Windows 强调色，使用共享调色板令牌的功能视图会继承所选的强调色。FileManager、navigation、settings、preview、metadata-export、playback 和 Shell 均消费共享调色板令牌；原始的 egui 色板样式、字体解析以及精确的视觉渲染仍需 Windows 验证。
- 快捷键：`open_key` 具有源码风格的捕获交互：点击“更改按键”，按一个受支持的单键即可持久化，或按 Escape 取消。FileManager 在获得焦点时读取持久化的绑定并打开所选条目；若 Settings 提供者不可用，则回车仍为回退方案。F1–F9 使用源码选项：Settings 在渲染快捷键选项卡时读取一份新鲜的 `NavigationService::snapshot()`，递归展平标签，并为每个选项标注其图标（回退为 `🏷`）加上以 ` > ` 空格分隔的 `full_path`。选择 `None` 只清除该按键；选择某个标签会替换该键的指派并立即保存。Settings 持有导航服务的弱引用，因此停止导航能正常释放其服务。settings 插件对 navigation 有编译期依赖，并在 shell 中于其后启动。导航停止时，现有指派仍以标签 ID 形式可见，但无法编辑。FileManager 的 F1–F9 为所选媒体切换所指派标签；Preview 的 F1–F9 为当前条目切换。两条路径都使用 `TaggingService`，在可用时通过 `HistoryService` 记录标签变更，并发布 `MediaIndexChanged`。Preview 面板还有一个可搜索的单条目标签选择器。数字键 0–5 为当前 Preview 条目评分，并在旧评分可获得时记录 `UndoEntry::RatingChanged`；FileManager 数字键快捷评分所选择的媒体。
- 标签侧栏偏好：在发布 `SettingsService` 之后，启动时会把已加载的排除修饰键、标签搜索匹配模式、搜索持久化标志和合并子策略作为 `TagInteractionPreferencesChanged` 发布。Settings 在上述任一设置变化后发布更新值，枚举值通过 `fileatlas-contracts` 映射以避免依赖环。Navigation 消费该事件并保留最新的偏好快照；标签合并以保存的重父/删除策略开始。
- AI 必选组支持添加/移除、行内重命名、从现有导航标签中选择，以及按标签移除。列表来源于一份新鲜的 `NavigationService::snapshot()`；导航停止时，组仍可重命名并移除标签，但无法添加标签。Preview 在每次分析请求时读取当前 `SettingsService::snapshot().ai`，包括必选组和 `auto_analyze`。
- 更新：`SettingsPlugin` 读取 shell 的 `UpdaterService` 和 `SessionService`。创建设置视图且自动检查启用时，会在后台检查 GitHub。新发现的发行版会以源码风格的浮动提示打开，包含发行说明、立即更新、暂不更新和 GitHub 操作；“暂不更新”会在本次应用会话内抑制后续提示，而 Settings → 更新可重新打开它。该选项卡支持手动检查、发行说明、字节进度、校验和验证的暂存区、两步安装确认、可见的失败信息以及持久的安装错误日志。安装前它会重写最近持久化的会话快照、调度暂存的可执行文件并退出 GPUI。应用循环返回后，shell 替换可执行文件并重新启动。Windows 实际重启行为仍未验证。
- `search_scopes` 将三个源码字段全部默认启用，为 FileManager 的初始查询范围提供种子，并由 Settings 面板和 FileManager 范围控件共同更新。更新只修改共享快照中的该属性，并通过与源码兼容的设置 JSON 持久化。
- `toolbar_pos` 提供锁定开关、10 px 方向偏移控件以及重置到默认锚点。Preview 拖拽手柄将像素偏移应用于底部中央锚点，并在释放时通过 `SettingsService` 持久化；保存失败会显示在 Preview 面板中。

## 持久化/调试

- `snapshot()` 返回设置的克隆；`update` 持久化每次交互并在面板内报告错误。缺失/无效的 JSON 会加载原始 FileAtlas 默认值。
- 将 JSON schema/默认值与锁定的 `vendor/photag-source/src/settings.rs` 对比。保存错误时检查 LocalAppData 路径与权限。UI 接线错误通常表示在此插件之前 bridge 服务或槽位宿主尚未初始化。

## 主题模式与共享视觉令牌

`Settings.color_mode` 在现有的
与源码兼容的设置 JSON 中持久化 `dark` / `light`（字段缺失时默认为 dark）。外观
页面通过 `SettingsService` 编辑此字段；成功写入后会将其发布到
`fileatlas-ui-bridge::theme::ColorMode` 并刷新 Kit 控件和语义色
消费者。`settings/rail-entry` 是一个 Cordis 拥有的 `workspace.sidebar-footer`
视图，其中主题切换按钮直接位于 Settings 上方。写入失败会保留
之前的模式并记录持久化错误。

浮动 Settings 面板使用感知的背景（backdrop-aware）基础色与模态面板辅助函数、主要/危险文本以及发丝线（hairline）令牌。布局间距与排版仍使用 GPUI Kit 预设；Windows 视觉一致性未验证。

## 代码地图

- `src/view.rs` 拥有 Settings 视图状态、偏好编辑、按键捕获、模态生命周期，以及分组的程序/插件导航。
- `src/view/tabs.rs` 渲染外观、快捷键、标签和 AI；`src/view/software.rs` 拥有外部软件快照与软件页面；`src/view/ai_groups.rs` 拥有必选组编辑，`tag_hotkeys.rs` 拥有 F1–F9 指派菜单。`src/view/updates_page.rs` 渲染发行/检查/下载/安装状态；`updates.rs` 拥有更新生命周期与浮动提示。
- `src/settings.rs` 拥有与源码兼容的偏好 schema/默认值、插件呈现/启用映射以及 JSON 服务持久化；`src/runtime.rs` 通过 Cordis API 协调可选工具插件 Fiber 的停止/spawn。

## 本地化与实时 Settings 验证（2026-10-13）

成功的偏好编辑会调用 UI Bridge 的 `set_accent_name`，随后调用
`apply_accent_components(&mut gpui_kit::App)`，使自绘高亮与
GPUI Kit 控件共享运行时强调色。现有的源码预设解析、
设置持久化、控件摆放和事件载荷均保持不变。

Settings 在外观页挂载生命周期拥有的 `settings.language` 视图贡献。
i18n 提供者拥有语言偏好和控件。
`UiSurfaceState` 的 settings.tab 接受 appearance/shortcuts/tags/ai/updates/software；
现有的 GPUI 轮询器既观察它，也观察用于 MCP 测试的打开/关闭请求。
可见标签与输入占位符通过 UI Bridge 翻译更新。

本次重设计在活动内容页旁边使用六个垂直堆叠的导航条目、
一个有界滚动的内容区域，以及一个说明更改会自动保存的页脚。
所有现有页面控件仍保留在页面渲染器中。
外观保留七个源码强调色选项加 System；快捷键保留
按键捕获与 F1–F9 标签指派；标签、AI 和更新保留其
源码支撑的设置。软件运行只读的外部软件检查。
更新页面将当前版本、启动检查偏好、实时检查
状态、可用发行说明/下载验证，以及明确的暂存
重启/安装操作分开展示。自动检查继续安静地填充状态；
手动检查可能打开现有的浮动更新提示。此布局尚未
接受父任务的实时 MCP 截图审查。

## 安静的启动更新检查（2026-10-13）

按用户指示，自动启动检查仍然尊重持久化的
`Settings.auto_check_updates`，但绝不打开更新提示。检查结果
仍可在 Settings 中获得。`begin_update_check(manual)` 跟踪请求
来源；显式手动检查可以打开现有提示，下载、
验证与显式安装行为保持不变。现有的
`settings.check-updates` 面板请求会在
GPUI 轮询器上调用同一个手动操作。只读诊断面板会发布更新检查中、
有可用更新和更新提示状态，供插件 MCP 工具使用。实时 MCP
验证发现了一个发行版且未打开启动提示，然后在
手动请求后打开了该提示。未下载或安装任何更新。

## 2026-10-14 当前实现与 MCP 证据

Reference03 六个 Settings 页面与07 更新提示通过 FileAtlas MCP 渲染。原生复选框/强调色/标题/导航和紧凑行使用共享主题。F1–F9 触发器有界360pt 宽度。更新提示现在使用视口受限的实底双栏说明/状态与固定页脚；实际的下载/暂存/验证/错误处理器与下载关闭限制仍然保留。观察到启动时 available/offerclosed 和手动 check 后 offeropen；下载/安装未测试。

## 共享浮动框架（2026-10-14）

现有设置模态内容现在使用 UI Bridge `panels::mount`，ID 为 settings，默认900×680/最小680×360 逻辑像素。标题栏拖拽、八个缩放把手、最大化/还原和关闭均为共享。标签栏、自动设置持久化和原生更新行为留在 Settings 中。主体宽/高使用实际 GPUI 内容边界。框架与宿主背景遮挡下层鼠标目标；Settings 按键捕获保留其原始面板标志。运行时的指针标签点击、空白区/背景隔离以及框架拖拽/缩放记录在当前验证报告中。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与现有错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；插桩的公开操作报告挂钟 span 时长与错误。领域服务契约、事件载荷与源码操作顺序保持不变。捕获范围与限制参见 fileatlas-plugin-debug/PORTS.md。

## 空闲读模型（2026-10-15）

快捷键与 AI 组的选项消费一个共享的后台标签选项模型，
在相关 Settings 页面打开时加载，以导航可用性与
`tag_revision()` 为键。重新打开会重新加载；打开期间的
标签变更/事件会使其失效。渲染从不调用 Navigation::snapshot。
缺失/错误的选项同样会被缓存。`settings.tag-choices` 是一个只读发布的 UI 快照：
状态 loading/ready/failed、revision 与 choices[{id,name,label}] 或 error，
由 MCP `get_ui_state.settings_tag_choices` 暴露。空的 worker 轮询不会
重绘。更新监控现在只在字节进度或任务结果时通知；
断开的检查/下载通道会变成错误并结束监控，
而不是留下无尽的 120 ms 重绘循环。空闲验证不会
下载或安装任何更新。

## 共享刷新所有权（2026-10-15）

Settings 面板与更新器定时器使用 UI Bridge refresh::poll。共享门（gate）跟踪可见性/标签页与更新进度；标签选项使用 ReadModel。终结性的 worker 错误只通知一次并停止；空通道不会绘制。

## 侧栏文件夹区块（2026-10-15）

`Settings.sidebar_folders`（`fileatlas_contracts::SidebarFoldersConfig`：
`quick_access`、`favorite_groups`、`show_network_drives`）是一个新的 serde 默认
JSON 区块；旧文档反序列化保持不变。插件观察
`SidebarFoldersChanged` 并接受 `SidebarFoldersUpdate { revision, config }`，通过
`SettingsService::update` 写入收到的文档。
启动时，若 Navigation 服务已经发布（shell 先启动 Navigation 再启动 Settings），
它会把持久化的区块推入
`NavigationService::sidebar_folders().restore(...)`。无 SQLite
变更；导航从不直接读取设置（事件流转避免环）。

## 侧栏持久化审查 — 2026-10-16

`SettingsService::replace/update` 在状态写锁下序列化磁盘发布，仅在成功后发布新的内存快照。
SidebarFoldersChanged 观察者拒绝过期 revision；失败的 revision 可以重试。它将保存错误传播给 Navigation 以获得可见的重试反馈。普通 JSON 字段与路径不变。测试覆盖并发编辑、过期投递与失败/重试；这不是崩溃安全的文件系统事务保证。


## 区块分隔令牌（2026-10-16；2026-10-23 移除）

原 `Settings.region_separation` 字段（"border"/"shadow"）及其外观设置行已随
统一的 Fluent 轻质感浮起样式移除：所有区块统一为带柔和阴影的浮起卡片，
该选项不再存在。仍含此键的旧 JSON 文件可正常加载（serde 忽略未知字段），
该值不再使用。成功写入后重发布的绘制期令牌仅剩 `set_accent_name`。

## 外部软件检查（2026-10-17）

`software` 设置选项卡使用以 revision 为键的后台 `ReadModel` 来读取
QuickLook/Everything 的安装记录、可执行文件路径与版本。它
发布 `settings.software-check`，包含 `state`、`revision` 与 `software`
条目（`name`、`purpose`、`state`、`version`、`path`、`project_url`、
`download_url`、`github_url`、`github_note`）。打开该选项卡和显式
刷新都会请求一次新读取；稳定后的渲染不会重复文件系统 IO。
链接拥有针对软件的 GPUI ID。链接操作打开项目/下载页面；
本页面不安装、执行或升级外部软件。Everything
需要 1.5 版本才能使用由独立所有者维护的搜索适配器。ES 的 GitHub 链接
被标注为命令行工具，而非 Everything 应用安装程序。
同一进程的软件页面以及缺失/不兼容/就绪的内联 FileManager 状态
参见 Everything-search 验证证据。

搜索位置（2026-10-17）：`Settings.search_entire_computer: bool` 在缺失时
默认为 false。FileManager 只在显式
位置变更时通过 `SettingsService::update` 保存它，并在视图构造时恢复。这不改动任何
Everything 设置，也不替换持久化的导航路径。

## 快捷键布局与捕获状态（2026-10-18）

当前视觉参考与布局/状态检查清单是
`docs/reference-ui/concepts/shortcuts-settings-2026-10-18.{png,md}`。
`view/shortcuts.rs` 页面将所有 UI Bridge 命令键绑定投射为对齐的
水平行；标签选择器填满剩余行宽。捕获状态会用蓝色录制字段
替换该行控件。校验错误保持行内显示并
让捕获继续活动以便重试；不会持久化任何无效/冲突的值。
Escape、行内取消、切换选项卡、关闭面板以及成功捕获
都会释放共享捕获状态（使全局快捷键得以恢复）。
`settings.shortcut-capture` 与 `settings.shortcut-error` 仅在捕获状态
转换时发布当前绑定 ID/错误；MCP 读取这些状态无需 UI IO。
同一进程的 UI 场景脚本是 `scripts/validate-shortcuts-layout.cjs`。
# 工作台主题（2026-10-18）

缺失的 `accent_name` 现在默认取 UI Bridge 的 `DEFAULT_ACCENT_NAME`（Workbench
Blue）。显式存储的 System/legacy 值仍受支持；不重写
配置，也不做 schema 迁移。外观枚举 `ACCENT_PRESETS` 加 System，
而不是复制预设列表。Settings 应用所有原生组件
状态，并在编辑后刷新消费窗口。共享标题使用
`components::section_heading`；主题规则见 `docs/ui-theme.md`。
# 侧栏分组兼容性（2026-10-18）

共享侧栏 JSON 模型现在带有 serde 默认的 `imported_library_paths`；
Settings 通过现有的带 revision 的侧栏更新处理器
持久化该字段。旧 JSON 仍可读；自定义分组/别名得到保留。不引入新的
文件/路径/存储。导入策略与 live plus 操作归 Navigation 所有，
Settings 仍负责持久写入与失败处理。
# 精简设置中心与 schema 集成 — 2026-10-18

当前参考是十张 `settings-*-2026-10-18.png` 概念图和
`docs/reference-ui/concepts/settings-complete-2026-10-18.md`。这取代了旧的
盒式设置布局。共享浮动几何仍为 1120x840/最小 920x580；
导航为 224 逻辑像素，页脚固定，内容独立滚动。

外观、标签匹配/范围/合并、标准开关和内置插件启动
呈现都使用 UI Bridge 的声明式 `settings_page` 渲染器。快捷键
捕获、AI 分组、软件检测与更新状态机在通用分区内保留其
现有操作。ID 与 SettingsService 持久化
保持与源码兼容；不新增数据文件或迁移。

生命周期拥有的 `UiViewContribution.settings_page` 提供者从
现有视图目录中被发现，自动生成插件导航，并按
所有者/ID/代（generation）调和。可选的内置运行时页面追加
提供者内容；仅提供者页面在没有运行时切换注册时也可工作。停止
一个所有者会移除其设置贡献。ChangeGate 观察目录/运行时
变化，在安静轮询时不发出通知。快捷键捕获在插件
页面保持活动，并因标签页/所有者/可见性变化而取消。


## MCP 访问设置 — 2026-10-19

`fileatlas.mcp` 拥有 MCP 设置页贡献（`mcp/settings`），并通过
现有 `SettingsService` 编辑 `Settings.mcp_access_level`；Settings
不重复实现 MCP 策略或存储。分段级别为 `off`、
`standard`（UI 控制与普通的增/改/删）和 `full`（还包括
高影响操作）。MCP 在每次变更宿主/贡献调用时读取当前设置
快照，因此变更立即持久化并无需重启
端点即可生效。插件工具元数据标识变更与
确认要求；高影响调用仍需每次调用
`confirm=true`。缺失的级别默认为 Off。旧的 JSON 布尔值
`mcp_allow_mutations` 仍被接受（`false` → Off，`true` → Full）。当前
同一进程的 Windows 证据位于
`docs/validation-ui/2026-10-19-mcp-access-stages/`；较早的双状态记录
`docs/validation-ui/2026-10-19-mcp-settings-toggle/` 已是历史。
