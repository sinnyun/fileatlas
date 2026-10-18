# 插件设置中心验证记录（2026-10-18）

概念图：`docs/reference-ui/concepts/plugin-settings-hub-2026-10-18.png`。实现清单见同目录 Markdown。

## 覆盖范围

- 程序设置保留现有设置页，并新增插件总览；插件设置分组按运行时注册表为启动台、剪贴板、截图工具创建独立导航页。
- 卡片启停会实际 dispose/spawn Cordis Fiber；剪贴板停用期间 `clipboard-workbench/open` 命令被移除，启用后恢复。
- Navigation 侧栏按 UiBridge 槽位与注册代次同步插件视图；同进程 MCP 停用剪贴板时，剪贴板托槽按钮从侧栏消失，重新启用后恢复。截图 `target/mcp-plugin-settings-final/screenshots/clipboard-disabled-sidebar-final.png` 对应停用状态。
- 启动台/剪贴板可选“主界面和悬浮面板”或“仅显示悬浮面板”；截图另支持直接调用 Shotoshot，默认 Direct。
- 程序设置中的“软件”页保留 QuickLook 与 Everything 安装检测；截图插件页单独显示 Shotoshot 检测。
- 当前可停启插件只包括这三个独立工具。通用详细设置视图提供启动方式与打开面板入口；其他插件注册、复杂设置视图工厂及依赖级联启停仍未接入。

## 验证

命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 build --locked -p photag-shell -j 2`、`node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-plugin-settings-final --plugins core --allow-mutations --scenario scripts/validate-plugin-settings.cjs`。

2026-10-18 纠正：程序设置“软件”页保留 QuickLook 和 Everything 检查，截图插件单页仅显示 Shotoshot。验证命令：`node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-software-restored-20261018 --allow-mutations --scenario scripts/validate-software-check.cjs`。MCP 确认 QuickLook 4.5.0 与 Everything 1.5.0.1423 已安装，检查页显示版本、路径及官方链接；未启动或查询两项外部程序。截图：`target/mcp-software-restored-20261018/screenshots/settings-software-check.png`。构建命令：`cargo +1.97.1 build --locked -p photag-shell -j 2`；被运行中的程序占用时，按用户授权关闭对应 PhoTag 实例后，标准目标目录构建成功。

MCP 为独立 profile、同进程窗口，允许的 mutations 仅作用于 `target/mcp-plugin-settings`。场景验证三个注册项、独立页面状态、启动台 panel-only 状态、剪贴板停用/恢复及截图默认 Direct。概览和页面截图在 `target/mcp-plugin-settings/screenshots/`。panel-only 截图确认工作台内容被隐藏且只显示启动台共享面板。

Governance 检查和格式/编译结果记录于本次变更摘要；运行窗口截图仍为大窗口状态，未覆盖小窗口滚动、改变强调色和各交互态的完整视觉验收。截图页和启动台页布局已由同进程 MCP 截取。
