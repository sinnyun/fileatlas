# Program launcher plugin / 程序启动台插件

状态：原生图标网格启动台已集成。同进程发现、单击选中、右键菜单、无害 fixture 的双击启动、检查器编辑、搜索、排序以及打开异步原生文件选择器都有运行时证据。选择/取消真实文件，以及已安装应用/提权卸载流程仍未验证。

## Cordis 身份与启动

- 插件：`fileatlas.launcher`；服务：`fileatlas/launcher`（`LauncherService`）。
- 依赖共享的 `fileatlas/database` 与 `fileatlas/ui-bridge` 服务。
- 还读取 `fileatlas/settings` 以获取 `Settings.plugin_launch_modes["fileatlas.launcher"]`。Settings 可以通过销毁该工具的 Cordis Fiber 来禁用这个独立工具，并通过生成新代次来重新启用它。
- Shell 在本插件之前启动数据库与 Navigation。Navigation 将其生命周期拥有的 `navigation.launcher-shortcuts` 视图贡献渲染为其左侧导轨中的 Apps 按钮。
- 注册 `program-launcher/panel`、`program-launcher/open` 命令以及一个侧边栏快捷入口。Cordis stop 会移除命令、面板定义/实例和视图贡献。

## 数据与操作

- 存储为 `%LOCALAPPDATA%\\FileAtlas\\launcher\\launcher.json`；图标资源位于 `launcher/icons/` 下。
  首次启动会导入共享 SQLite 投影并复制 legacy 的 `program-icons/` 资源。此后
  以 JSON 为权威；legacy 行/文件保持原样，作为恢复来源。

启动器在一次性导入之后使用带版本的 JSON 快照。它不直接访问 SQLite；
持久化的编辑与发现合并会在刷新内存快照之前保存。

- 仅在首次启动时，`DatabaseService::launcher_snapshot()` 为 JSON 导入提供 legacy 投影；导入之后该服务绝不再读取启动器表。被动渲染读取内存中的 JSON 快照。
- `refresh_discovery()` 在 worker 线程上扫描受支持的 Windows 来源，然后合并发现的条目。它保留自定义名称、手动类别指派和隐藏状态。
- 新发现的条目会根据规范化的程序名/路径启发式规则归入某个中文类别，包括 AI 工具、文件同步、远程访问、Windows/系统管理、开发、创意、办公、通讯、浏览器、媒体与游戏。后续扫描可以更新自动指派的类别（`custom_category=0`）；用户移动过的条目会被保留。
- 启动与重新扫描会清理那些名称/路径表明为安装程序、卸载程序、卸载快捷方式或仅有文档的开始菜单项的过期发现记录。扫描器在发现与手动添加时都会拒绝安装/卸载可执行目标；它不会运行或删除它们。
- `LauncherEntry.description` 是导出的一句式描述。它优先使用 Windows 注册的 `Comments` 元数据，其次是精心整理的产品名映射，最后是类别级句子。它通过 serde default 在 `launcher.json` 中保持向后兼容；重新扫描会重新生成描述，用户对类别/名称的编辑会刷新回退文本。
- CRUD 操作涵盖类别、自定义显示名、类别指派、隐藏状态、移除以及手动添加可执行文件/快捷方式。
- 每个程序可以取消固定、固定在其类别顶部、或固定在全部程序顶部。固定选择持久化在 JSON 中；多个程序可以共享同一固定范围。星形徽标区分全局（实心）与类别（描边）固定。固定优先级先于所选的名称/日期/大小排序。
- `launch(id)` 启动所选的可执行文件/脚本，或通过 Explorer 打开其快捷方式。`uninstall(id)` 使用注册的卸载命令，并先请求显式确认。

## 输入、输出与事件

- UI 命令 `program-launcher/open` 打开/聚焦现有的通用浮动面板。
- `PanelOnly` 会在共享面板下隐藏主工作区；默认保持 FileAtlas 工作区可见。
- 侧边栏视图槽位：`navigation.launcher-shortcuts`。
- 面板：中部为可搜索的桌面风格图标磁贴，左侧为类别/计数，右侧为信息编辑器，其中包含只读的程序描述行。右键菜单保留启动、重命名、移动、显示/隐藏、卸载与移除；只有当 Windows 注册了卸载命令时才提供卸载，且执行前需确认。
- 可见 UI 文本使用共享 i18n 目录。类别图标来自现有文本/图标渲染路径的字形。
- 不发出新的 Cordis 事件。面板与贡献的生命周期使用共享的 UI Bridge 契约。

## 限制与诊断

- 发现范围包括每用户和机器的卸载注册表项，以及每用户和公共开始菜单的 `.lnk` 文件。它不会扫描每个磁盘上的全部可执行文件；没有可用可执行文件/快捷方式的条目会被省略。
- 名称匹配保留 Unicode 字母与数字。`DisplayIcon` 只有在指向受支持的可执行文件/脚本/快捷方式路径时才被接受为启动目标；纯图标 `.ico` 文件不是程序。
- 类别是启发式的，可能需要手动修正。自动重新扫描会保留手动的类别改动。当没有产品专属描述时，通用回退描述会说明其类别层面的用途。
- 卸载命令来自注册表命令行。请审阅确认过程与 Windows 行为；提权的卸载程序可能单独启动并异步完成。
- 排查失败时检查 `LauncherService` 的 JSON 持久化错误以及 `fileatlas-program-discovery` 的 tracing。实时检查必须使用同进程 FileAtlas MCP。

## 初次同进程验证 — 2026-10-17

- `cargo +1.97.1 build --locked --offline -p fileatlas-shell -j 2` 通过（存在既有的 LNK4098 警告）。
- 隔离配置的 MCP 运行：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-launcher-v3 --plugins dev-test,launcher --interactive --allow-mutations`。
- `get_launcher_state`：扫描已完成，398 个条目、10 个类别、没有 `.ico` 启动目标。计数与 UI 证据位于 `docs/validation-ui/2026-10-17-program-launcher/README.md`；截图：`sidebar-launchpad.png`。
- 未启动任何应用，也未执行任何卸载命令。手动添加/重命名/移动/隐藏/移除以及原生卸载确认仍未验证。

## 桌面图标布局与元数据 — 2026-10-17

用户参考：仅含图标/名称的桌面快捷方式。当前概念图为 `docs/reference-ui/concepts/program-launcher-2026-10-17-v3.png` 及其配套说明；它取代了宽幅动作卡片与 v2 网格概念图。

- 磁贴显示 64px 原生 Windows shell 图标和两行显示名。单击选中；双击调用 `launch(id)`；右键打开现有的 GPUI Kit PopupMenu，包含启动/重命名/固定/移动/隐藏/卸载/移除。重命名会聚焦右侧编辑器；移动与固定使用子菜单。
- 固定管理可从磁贴上下文菜单和所选程序信息区进行。就每个程序而言，三种状态互斥；多个程序可同时被固定。全局固定在全部程序和每个类别视图中排最先；类别固定在全部程序中以及其自身类别内排在未固定条目之上。当前排序字段在每个固定级别内部对程序排序。实心星标记全局固定，描边星标记类别固定。
- 右侧信息区预留其宽度，因此选中不会移动双击目标。名称、类别与隐藏状态是一份草稿；`LauncherService::update_entry(id, name, category_id, hidden)` 校验它们并将其保存到 JSON 快照中。关闭会清除选中。
- `LauncherEntry` / 数据库条目投影现在包含 `installed_on: Option<String>`（ISO 日历日期）和 `size_bytes: Option<i64>`。注册表 `InstallDate` 按 YYYYMMDD 解析；正的 DWORD `EstimatedSize` 从 KiB 转换为字节。缺失/无效值保持为未知；文件系统修改时间与可执行文件长度都不是安装信息。
- 安装元数据保存在 JSON 快照中。原先的启动器表仍作为迁移/恢复数据保留，运行时不再更新它们。
- 名称、安装日期与注册大小都支持升序/降序。缺失元数据在两个方向都排在最后；并列时按规范化名称再按 ID。
- `LauncherService::wake()` 返回共享的 refresh Wake。视图使用生命周期拥有的 `refresh::listen`；不需要空闲定时器。图标读取使用共享的 `ReadModel<Vec<PathBuf>, IconSet>` 与单个后台 COM worker。成功的 shell 图标以 PNG 缓存在 `launcher/icons/` 中；标识包含规范化的 Windows 路径、文件大小与修改时间。该服务还在插件生命周期内保留解码后的图像。规范的 `\\?\` 与混合分隔符路径会在 Shell 提取之前被规范化，从而让手动添加的可执行文件与快捷方式解析出各自的原生图标。无法由 `IShellItemImageFactory` 渲染的可执行文件回退到 `ExtractIconExW`/`DrawIconEx`。失败的图标不会被持久化，以便稍后重试；对剩余失败项 UI 使用其类别字形。`get_launcher_state.ui.icon_cache` 报告磁盘命中、shell 提取、失败、手动程序状态以及失败程序的路径。GDI 位图/DC 与 COM apartment 在 worker 中释放。
- `get_launcher_state` 暴露元数据和一个只读的 `ui` 投影（选中/草稿、排序方向、可见 ID、图标计数与渲染次数）。`add_launcher_program` 使用公开服务并需要变更权限；它不启动任何东西。运行时验证通过同进程 MCP 客户端执行 `scripts/validate-program-launcher.cjs`。
- 添加程序按钮在受跟踪的 GPUI task 中使用 `rfd::AsyncFileDialog`。同一时间只能有一个选择器处于活动状态；在关闭之前按钮保持禁用。成功后它调用 `LauncherService::add_manual`，返回全部程序，清除隐藏/搜索过滤，并选中刚添加或已发现的路径；取消则保持状态不变。这可以防止新添加的程序被之前所选的类别或搜索隐藏。不要将其替换为在 GPUI 线程上调用同步 `FileDialog::pick_file`，那可能在打开 Windows 选择器时冻结或终止宿主。


当前验证：`scripts/validate-program-launcher.cjs` 校验中文输入框、单击选中而不启动、双击启动无窗口 fixture 可执行文件、右键菜单、名称/类别/隐藏的保存、六种排序方向、扫描的真实变化与稳定后的静默状态。该插件使用其特性自有的内存结果缓存，在面板重开之间缓存原生图标；所有提取仍位于共享 ReadModel 之后。证据与精确计数：`docs/validation-ui/2026-10-17-program-launcher/README.md`。

2026-10-18 图标复验：手动添加的 fixture 在经过 Windows 路径规范化后解析出其 Shell 图标。当 `IShellItemImageFactory` 无法渲染某个可执行文件时使用原生资源提取。一次冷启动同进程运行提取并持久化了全部 399 个程序的图标；重启后，已保存的行在发现刷新之前出现，且全部 399 个 PNG 在 1.8 秒内从磁盘加载完成，零 Shell 重新提取、零失败。细节与截图记录在验证 README 中。
# 共享主题（2026-10-18）

启动器卡片与类别/隐藏导航复用 UI Bridge 的 `components`。
选中/悬停/回退图标使用 `theme` 并遵循持久化的强调色；
原生程序图标保持为未修改的内容。几何、动作与生命周期
在现有通用面板内仍归特性自有。参见 `docs/ui-theme.md`。

PanelOnly 展示方式使用 `PanelCatalog::open_presented(PANEL_ID,true)` 打开独立的原生工具窗口；WorkspaceAndPanel 保持内嵌的公开面板。数据/服务不变。
# MCP 贡献

启动器通过宿主发布的
`fileatlas_contracts::mcp::McpToolRegistry` 贡献带命名空间的工具：`launcher.snapshot`、`launcher.add_program`、
`launcher.set_pin`、`launcher.create_category`、`launcher.rename_category`、
`launcher.delete_category`、`launcher.update_program`、`launcher.refresh`、
`launcher.launch`、`launcher.remove_program` 与 `launcher.uninstall_program`。
该插件拥有它们的 schema 与处理函数，并通过
`LauncherService` 路由。程序启动、类别删除、列表移除与软件
卸载需要显式的 `confirm:true`；其他写入受变更门控。
Cordis 会在 `fileatlas.launcher` 停止时移除这些贡献。原先
不带命名空间的 MCP 工具名已废弃；客户端应使用这些带命名空间的
路由。
