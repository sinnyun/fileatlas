# 程序启动台批量管理（2026-10-26）

## 实现

程序启动台新增批量管理模式：中间区域切换为程序列表，支持选择当前列表/逐行勾选；检查器可统一修改分类、名称前缀/后缀和隐藏/显示状态。已选程序以稳定 ID 保存，筛选或扫描移除条目时清理失效选择。服务对目标 ID 和分类预先校验，再一次更新版本化 `launcher.json`；MCP 暴露同一批量服务操作。原图标模式保留。

布局按 [批量管理概念图](../../reference-ui/concepts/program-launcher-batch-management-2026-10-26.png) 实现，保留 v3 左侧分类导航、外层面板和现有主题组件。

## 验证

- Toolchain：`1.97.1-x86_64-pc-windows-msvc`。
- 构建：`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`，通过；链接器提示已有 LNK4098 警告。
- 运行命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/program-launcher-batch --plugins launcher --access-level standard --scenario scripts/validate-launcher-batch.cjs`。
- 隔离 profile：`.local/validation/profiles/program-launcher-batch/`。同进程 MCP 用真实指针事件切入批量模式，点击首两行选择 2 个程序；在窗口里选择分类、输入名称前后缀、选择隐藏并点击应用。快照确认 `batch_mode=true`、选中 ID、草稿和最终记录均符合预期；之后恢复原名称、分类及可见状态。场景输出：`batch mode, row selection, category/name/visibility edits and restoration passed`。
- 已打开并核对 `screenshots/launcher-batch-list.png`、`launcher-batch-selection.png`、`launcher-batch-category-menu.png`、`launcher-batch-edit-draft.png` 和 `launcher-batch-applied.png`。实际列表保留名称/分类/显示状态/路径列；长路径被省略号截断；选中行、分类菜单、输入草稿和隐藏应用后的行移除均有对应截图。所有截图位于本机隔离 profile，未纳入 Git。
- `cargo +1.97.1 fmt --all -- --check`、Launcher/I18n Cargo check，以及 `check-ui-theme.cjs`、`check-ui-refresh.cjs`、`check-architecture.cjs`、`check-feature-ledger.cjs` 均通过。

## 尚未验证

本次完成了正常尺寸窗口下的批量模式、两行选择、分类菜单、名称输入、隐藏状态及应用结果的同进程交互/截图核对。窄窗口退化和超长名称下的列表行为仍未覆盖；原生 Windows 程序安装/卸载行为不属于本轮批量流程。
