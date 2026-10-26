# 画布底部工具组弹出位置 — 2026-10-27

## 问题与修正

FileAtlas 将画布引擎内置工具栏隐藏后，在底部组合自己的工具栏；工具组菜单却仍用引擎内置工具栏的默认顶部坐标。于是“形状”“连接线”“资源/图像”组打开的子工具会与实际点击按钮分离。

嵌入画布的形状、连接线、资源/图像组扩展选项统一显示在底部工具栏上方的固定独立面板。所有组共用该锚点，不按各自按钮计算位置，避免弹层压住工具栏。撤销/重做与功能工具仍在同一行，但现在各自使用独立背景胶囊，中间留有间距。独立运行引擎的原有顶部工具栏定位不变。

## 同进程验证

- 同进程隔离 MCP 场景：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --plugins whiteboard --profile .local/validation/profiles/canvas-toolbar-popovers-20261027 --access-level standard --scenario scripts/validate-canvas-toolbar-popovers.cjs`。
- 默认画布截图及形状组、资源/图像组分别打开截图均已检查。验证动作使用 `whiteboard.tool_group {group}`，经 WhiteboardService 队列进入同一 WhiteboardView/Canvas Engine 固定面板布局；截图确认两组扩展工具都显示在底部工具栏上方且不与工具栏重叠。撤销/重做和功能工具栏的背景已分开。
- `cargo +1.97.1 fmt --all -- --check`、锁定 Whiteboard 与 Canvas Engine 检查、Shell 构建以及 UI theme、UI refresh、architecture、feature-ledger 守卫通过。构建有既有 Windows `LNK4098` 默认库冲突警告，构建成功。

截图与报告位于 `.local/validation/profiles/canvas-toolbar-popovers-20261027/`。此次通过同进程 MCP 打开工具组选项面板；实际按钮回调共用相同的无坐标切换方法。小窗口、悬停工具提示及各组内选择后画布工具行为未作为本次布局修正的一部分重测。
