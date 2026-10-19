# 2026-10-19 程序启动台分类与说明

按当前活动参考 `docs/reference-ui/concepts/program-launcher-2026-10-17-v3.png` 和配套概念说明实施；保留左侧分类、中间大图标网格、右侧信息区及右键菜单结构。右侧信息区增加只读“程序说明”一行。

## 验证环境与命令

- Windows 同进程 PhoTag MCP 子进程；隔离 profile：`target/mcp-launcher-current-review`，启用 `launcher,dev-test`，访问级别 `full`。dev-test 隔离 PhoTag 自有数据，但发现扫描使用当前主机开始菜单和注册表。
- 命令：`node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-launcher-current-review --plugins launcher --access-level full --plan target/mcp-launcher-current-review/plan.json`
- 修正规则后的隔离 MCP 快照有 292 个程序、13 个分类（其中“其他”和常用/General 为 0）；分类覆盖 AI、文件同步、远程连接、开发、设计、办公、通讯、浏览器、影音、游戏与系统工具。没有卸载器、安装器目标或仅文档类入口。所有快照条目均有非空一句话说明。
- 随后以正常用户配置启动构建后的 PhoTag，`%LOCALAPPDATA%\PhoTag\launcher\launcher.json` 从 298 项清理到 295 项，未发现卸载/安装器目标、空说明或“其他”类条目。CADCraft、LightCraft、NeatReader 三条旧自动分类项现分别归入设计创作、设计创作、办公软件并生成产品说明。用户配置的扫描与合并已落盘；原有隐藏、自定义名称和手动分类继续由插件合并规则保留。
- Photoshop 2025 显示产品说明“用于图像处理、照片修饰、合成与视觉设计。”；右键菜单显示“卸载”，对应系统已登记命令。未点击卸载。
- 截图：[程序说明](../../../../target/mcp-launcher-current-review/screenshots/launcher-description-row.png)、[右键卸载菜单](../../../../target/mcp-launcher-current-review/screenshots/launcher-uninstall-menu.png)。完整 MCP 结果：`target/mcp-launcher-current-review/report.json`。

## 说明来源

为常见产品优先整理官方产品说明；其他已登记软件使用 Windows `Comments`，没有说明时按已分类的主要用途生成一句话。产品说明参考：[Qoder](https://docs.qoder.com/desktop/overview)、[DeepSeek](https://www.deepseek.com/en/download/)、[Anchorpoint](https://www.anchorpoint.app/)、[Clustta](https://docs.clustta.com/welcome/clustta-101.html)、[Reasonix](https://reasonix.io/)、[Skills Manager](https://github.com/S-zhi/skills-manager)、[腾讯 ima](https://ima.tencent.com/download)、[ZTools](https://www.ztools.cn/)、[T8 Penguin Canvas](https://github.com/T8mars/t8-penguin-canvas)、[CADCraft](https://cadcraft.org/)、[LightCraft](https://github.com/lightcraft-app/lightcraft)及[Neat Reader](https://www.neat-reader.com/download)。

## 静态与治理检查

- Rust 工具链：`1.97.1-x86_64-pc-windows-msvc`；格式检查及 `cargo +1.97.1 check --locked --offline -p photag-shell -j 2`、`cargo +1.97.1 build --locked --offline -p photag-shell -j 2`通过。
- `node scripts/check-ui-theme.cjs`、`check-ui-refresh.cjs`、`check-architecture.cjs`、`check-feature-ledger.cjs`、`check-layering.cjs`通过。架构守卫报告两个既有 orphan-directory 提示；layering有既有 launcher/clipboard→settings 分层问题，没有新增依赖边。
- 编译器报告既有 `LNK4098` 链接警告。

## 边界

隔离 MCP 确认的是主机扫描、信息区和卸载入口可见性；真实卸载执行、安装器提权流程以及产品名不在说明目录内时描述的精确性未验证。正常用户 JSON 已在重建后启动时完成自动清理与扫描合并。常用分类排序和自定义分类保持不变。
