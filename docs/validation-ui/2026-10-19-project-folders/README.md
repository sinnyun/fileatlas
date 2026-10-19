# 项目文件夹插件

状态：插件实现、静态构建和同进程 MCP 功能流程已检查；完整视觉验收仍部分待查。

## 实现

- 新增独立 Cordis 插件 `photag-plugin-projects`，通过 Settings 插件运行时支持启停。
- Navigation 左 rail 的“项目”页签与文件夹/标签使用同一侧栏内容列。插件停用时由 Cordis 注销项目视图，导航 rail 同步隐藏该页签。
- 页面顶部加号会把导航地址栏当前打开的目录直接添加为项目根目录；无需手动填写地址。项目根目录可移除；目录清单使用插件拥有的版本 1 JSON，并通过临时文件和备份替换保存。
- 项目树递归读取放在 UI Bridge `ReadModel` worker，根目录以下的全部子目录按层级缩进展示。文件夹点击发出 Navigation 公共 `SelectPathPreservingPage` 请求，主窗活动栏跳转而项目页保持打开。
- 参考：导航结构取自[sidebar-pages-preview v2](../../reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png)及[配套说明](../../reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.md)；树行密度取自[收藏分组概念图](../../reference-ui/concepts/sidebar-favorite-groups-2026-10-18.png)，项目树具体清单见[配套说明](../../reference-ui/concepts/sidebar-favorite-groups-2026-10-18.md)。

## 静态检查

- `cargo +1.97.1 fmt --all -- --check`：通过。
- `cargo +1.97.1 check --locked --offline -p photag-plugin-projects -p photag-shell -j 2`：通过。
- `node scripts/check-architecture.cjs`、`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-feature-ledger.cjs`、`node scripts/check-layering.cjs`：通过。架构基线包含当时工作区中已存在的 Material Theme 与 Shell Controls 插件改动。

## 同进程 MCP 功能验证

- 命令：`cargo +1.97.1 build --locked --offline -p photag-shell -j 2`；`node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile .local/validation/profiles/projects-current-folder-20261019 --plugins core --access-level standard --scenario scripts/validate-projects-sidebar.cjs`。
- 结果：通过 MCP 打开隔离 profile 下的真实目录，再切到“项目”并点击加号；该当前目录被加入项目列表，版本 1 项目 JSON 写入 `<profile>/projects/projects.json`；截图可见根目录及 `docs/notes`、`src/deep` 全层级行；点击 `deep` 后 `gallery.current_directory` 到达对应路径，`navigation_tab` 仍为 `projects`。
- 系统/窗口：Windows，同进程 `photag-shell --mcp`；窗口 3286×1831 物理像素，GPUI scale factor 1.5；所有 fixture、清单、报告、日志及截图均在 `.local/validation/profiles/projects-sidebar-20261019/`。
- 截图：`.local/validation/profiles/projects-current-folder-20261019/screenshots/projects-sidebar-restored.png`（新 MCP 子进程从已有清单恢复项目树）、`projects-sidebar-added.png`、`projects-sidebar-navigated.png`。它们是功能流程证据；未对小窗口、无当前目录提示、移除后重启、失效根目录及超大目录树做视觉验收。
- 复现场景入口：[`scripts/validate-projects-sidebar.cjs`](../../../scripts/validate-projects-sidebar.cjs)。

## 结论与未覆盖状态

- 功能：通过侧栏切换、打开当前目录并点击加号加入项目、JSON 保存、递归展开、点击子目录主窗导航且保留项目页。测试期间根目录重复添加保持幂等。
- 视觉：项目树与文件夹/标签共用导航 rail 和内容列；MCP 截图已打开检查。普通窗口下层级缩进、选中态及主窗跳转清楚。没有宣称小窗、超长树和失效根目录视觉通过。
- 后续覆盖：无当前目录提示、项目移除及重启恢复、失效/不可读目录提示、窄侧栏与小窗口、超大树的耗时/内存。

## 待运行验证与限制

- 未通过 PhoTag MCP 点击项目按钮、添加真实隔离目录、确认递归展开/跨栏目录跳转、移除与重启恢复，也未检查窄窗口和截图；这些状态不能标记为视觉或功能验收通过。
- 目录选择使用 Navigation 的文件系统浏览路由，生产 Shell 以浏览模式启动；已有 Navigation 逻辑允许任意存在的目录，并交给当前活动栏。静态代码路径已核对，实际点击仍待 PhoTag MCP 验证。完整说明见 [`photag-plugin-projects/PORTS.md`](../../../crates/photag-plugin-projects/PORTS.md)。
- 所有子目录一次性枚举，目录符号链接不跟随；超大树的耗时和内存尚未实测。
