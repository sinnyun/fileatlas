# 程序设置存储汇总页（2026-10-27）

## 页面内容

程序设置导航新增“存储”。页面按运行时插件及已注册设置页贡献分组，列出每个已登记路径、内容说明和占用大小；为适应窄窗口，长路径中间会省略，旁边的复制按钮可复制完整地址。插件未声明路径时明确显示未登记状态。页内“刷新”会重新统计，路径清单随插件运行时目录变化重新生成。

统计在 UI Bridge `ReadModel` 后台执行，设置页渲染仅读取缓存。目录递归统计、不跟随符号链接；共享 SQLite 文件计入 WAL/SHM。缺失路径显示 0 B，不可完整读取时显示部分大小。每行按路径独立统计，重叠路径/共享文件可能重复出现，页面不将这些值冒充为可直接相加的总量。

## 同进程 MCP 验证

命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --plugins whiteboard --profile .local/validation/profiles/plugin-storage-summary-20261027 --access-level standard --scenario scripts/validate-plugin-storage-summary.cjs`。场景打开设置并调用 `control_ui { action: "settings_tab", value: "storage" }`，断言 `get_ui_state.settings_open=true`、`settings_tab=storage`，随后调用 `settings_storage_refresh` 并等待异步统计后截图。场景报告记录了 10 个运行时插件；Windows 同进程截图为 3286×1831 像素，DPI 未由截图接口提供。报告位于 `.local/validation/profiles/plugin-storage-summary-20261027/report.json`；截图在同目录 `screenshots/plugin-storage-summary.png`。截图中的彩色矩形是 MCP 几何诊断叠层，不是产品控件。

## 检查边界

覆盖聚合页入口、刷新请求、插件分组和路径/占用值可见性。截图确认完整路径复制按钮存在；场景未执行剪贴板写入。执行 `cargo +1.97.1 fmt --all -- --check`，锁定检查 Settings/MCP/i18n，构建 Shell，以及 UI theme self-test/guard、UI refresh、architecture、feature-ledger 和场景脚本语法检查。目录内容依赖该隔离 profile 中实际创建的文件；权限拒绝、超大用户目录的扫描耗时及小窗布局单独需要实际环境验证。完整项目检查结果追加在 `docs/redesign-validation.md`。
