# FileAtlas 0.2 开发基线（2026-10-19）

状态：当前项目定义与后续开发基线。项目基线标识为
`dev-baseline-0.2-2026-10-19`。它记录当前已接受的产品范围、界面结构、插件边界和验证状态；
不是正式发布声明，也不变更 Cargo 中仍为 `0.1.0` 的应用/Crate 版本号。

## 项目定义

FileAtlas 是面向个人使用的 Windows 桌面文件浏览器与工作台管理器。文件和目录浏览、多栏导航及程序工作区管理构成产品主干；媒体索引、缩略图、标签、筛选和检查器是文件浏览中的集成能力。用户可以按需增加用途独立的功能插件。新增功能通过 Rust 编译期 feature crate 和 Cordis 生命周期集成，不引入动态 DLL 加载或外部插件商店。

主界面继续使用已接受的深蓝工作台主题、现有组件样式和全局导航/检查器。文件视图区域支持单栏、双栏和 2×2 四栏。每栏都是完整且独立的浏览器视图，拥有独立地址导航、目录内容、筛选/搜索、排序/分组、网格/列表、选择及滚动状态。公共筛选与视图操作作用于当前激活栏；搜索与功能按钮保持独立分组。左侧导航和右侧检查器各保留一份。

程序工作区保存启动布局和各栏浏览配置，不是文件或媒体数据仓库。工作区可以创建、打开、重命名、设为默认、保存当前状态和删除非默认项；启动时恢复默认工作区。工作区使用带版本字段的 JSON 配置，不存文件记录；媒体元数据查询仍由共享数据库服务处理。Everything 的配置归设置界面，工作区只记录搜索/浏览状态。

## 当前能力与证据

- `fileatlas-plugin-columns` 提供布局及激活栏服务和工具栏入口；Shell 负责把各栏内容放进主窗口，并将公共视图操作挂接到激活栏。
- `fileatlas-plugin-workspaces` 管理工作区清单和 JSON 持久化；`fileatlas-plugin-file-manager` 提供独立的多栏文件浏览状态。
- 同进程 FileAtlas MCP 已验证双栏、四栏显示、各栏独立地址历史、保存双栏工作区和启动恢复。截图、命令和未覆盖边界见[运行验证记录](validation-ui/2026-10-19-columns-foundation/README.md)。
- `cargo +1.97.1 fmt --all -- --check`、相关 crate 的 locked offline check、Shell locked offline build，以及 UI theme、UI refresh、architecture、feature-ledger、layering 检查通过。Windows 极窄窗口、长列表状态恢复和工作区管理器所有逐项点击仍不视为通过；以验证记录中的边界为准。

## 架构与数据约束

- `fileatlas-shell` 只负责启动、运行时构造和顶层组合；功能行为归各 Cordis feature crate。
- Cordis 管理插件服务、依赖、事件、Fiber 和生命周期清理；UI 注册走 `fileatlas-ui-bridge`。
- GPUI/GPUI Kit 负责原生 GUI。全局主题和组件沿用 UI Bridge 公共设施，不创建功能插件私有主题。
- 媒体数据继续经 `fileatlas-plugin-database` 的 SQLite 服务访问；工作区等适合整体读写的清单使用各自的版本化 JSON 文件，并由所属插件记录路径、兼容及恢复策略。
- 保留现有用户数据库、设置、会话和源数据语义；任何格式迁移须带兼容策略并验证，不能仅因本基线版本改变而改写用户数据。
- `vendor/photag-source/` 用于追溯后端行为和数据语义；其旧版 UI 不作为当前布局要求。

## 后续工作入口

泛化“继续”从[功能操作优化与扩展计划](functional-development-plan.md)中首个未完成的 F0–F3 项推进。新功能请求优先，需记录用途、插件边界、数据存储决定和验证范围。所有 Windows 界面/交互验证继续使用项目同进程 FileAtlas MCP 与隔离 profile；静态编译、检查或截图不能代替未实际覆盖的系统行为验证。

## 版本说明

FileAtlas 0.2 是本仓库采用的项目定义/开发基线版本，用于区分 2026-10-14 的布局基线与当前增加了完整多栏浏览和程序工作区能力的产品状态。它不表示所有 F0–F3 操作项目已完成，也不表示已发布 0.2 安装包。应用构建版本仍由 Cargo workspace 的版本字段独立管理。

## 原生主题与 Material 生成器（2026-10-19）

用户要求所有插件及自绘控件统一消费 GPUI Kit 原生主题。原手写 RGB palette、
mode_color 和组件覆盖表退役；UI Bridge 只适配原生角色与请求入口。
Material 以独立 Cordis 插件生成标准 ThemeSet JSON，主题系统负责应用；配置和存储
见 docs/ui-theme.md 与 fileatlas-plugin-material-theme/PORTS.md。布局和几何沿用本基线。
