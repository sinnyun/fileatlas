# 主工具栏布局下拉与子文件夹图标（2026-10-19）

## 子文件夹开启态强调色底框（最新用户反馈）

- 用户要求开启状态更明显：现在开启后使用共享主题的 primary 变体，显示实色强调色圆角底框及高对比图标，关闭后恢复 ghost。此前 `.toggled(...)` 仅提供辅助功能开关语义，不能代替持久的开启状态视觉；本次继续保留该语义，增加 primary 呈现。
- 已在同进程 FileAtlas MCP 实际点击关闭→开启→关闭，移开鼠标检查底框持续显示，悬停后仍保持强调色。默认工作台蓝与 Sage Green 两个隔离 profile 均通过，开启态静止 1500 ms 的 FileManager snapshot revision 不变；状态见 [filled-state-report.json](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/filled-state-report.json)。
- Windows 150% DPI、3286×1831。命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-subfolder-fill-blue --access-level standard --scenario docs/validation-ui/2026-10-19-toolbar-dropdown/validate-subfolder-fill.cjs`；第二次使用 `target/mcp-subfolder-fill-green`，其隔离 `settings.json` 预设 `accent_name: Sage Green`。均启用 dev-test；不更改普通用户设置。
- `cargo +1.97.1 fmt --all -- --check`、locked offline Shell check/build（`-j 2`）及 theme/refresh/architecture/feature-ledger/layering 守卫通过；仍有既有链接器 LNK4098 和孤立目录提示。
- 已打开逐项对照原尺寸截图：![filled-blue-off.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/filled-blue-off.png)、![filled-blue-on.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/filled-blue-on.png)、![filled-blue-on-hover.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/filled-blue-on-hover.png)、![filled-green-on.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/filled-green-on.png)。这些文件覆盖旧 `corrected-toolbar-on.png` 的开启态视觉；控件位置和尺寸相同。
- 本次未另行覆盖极窄窗口和键盘焦点；控件没有新增禁用条件。

## 图标样式与位置修正（后续用户反馈）

首次调整将布局入口留在缩放条之后、使用默认按钮样式，四栏采用了四列图标，子文件夹开关也始终朝右。这些截图不作为上述细节的验收通过证据。本节及 `corrected-*` 截图为当前结果。

- 对照当前概念和本轮用户反馈：顺序改为网格、列表、布局下拉、子文件夹开关、缩放。布局入口复用共享 ghost 图标按钮，高度 30，宽度 46 为箭头预留空间；单栏 Square、双栏 Columns2、四栏 Grid2x2；子文件夹开关分别使用 ToggleLeft/ToggleRight。
- FileManager 将原视图控件投影为生命周期所属的 `.view-modes` 和 `.view-options`，Shell 在两者之间组合 Columns 的 `command.columns`。各投影仍观察同一个 FileManager 实体。MCP 增加只读 `program_workspace.current_snapshot`，可核对当前各栏配置；旧的顶层 `file_manager`/`browser` 诊断仍代表第一栏。
- Windows 150% DPI，3286×1831 截图；隔离 `target/mcp-toolbar-position-final`，dev-test，Standard。执行 `cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check -p fileatlas-shell --locked --offline -j 2`、`cargo +1.97.1 build -p fileatlas-shell --locked --offline -j 2`，以及 theme/refresh/architecture/feature-ledger/layering 守卫、`git diff --check`，均通过。保留既有 LNK4098 和孤立目录提示。
- 复现：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-toolbar-position-final --access-level standard --scenario docs/validation-ui/2026-10-19-toolbar-dropdown/validate-position-fix.cjs`，首次运行使用新隔离 profile。输入全部由同进程 FileAtlas MCP 派发。
- 实测通过：点击菜单选择单/双/四栏；菜单选择活动栏 2/4；第一栏设为列表与子文件夹开启，切换到第二栏后保持独立默认设置；第二栏开关往返；返回第一栏恢复原设置；回到网格与关闭状态；静止 1500 ms 的布局 revision、FileManager snapshot revision 不变。完整状态见 [position-fix-report.json](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/position-fix-report.json)。
- 已打开检查 ![corrected-toolbar-off.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/corrected-toolbar-off.png)、![corrected-toolbar-on.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/corrected-toolbar-on.png)、![corrected-quad-menu.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/corrected-quad-menu.png)、![corrected-final.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/corrected-final.png)。图标水平对齐、箭头完整、控件顺序正确，四栏显示 2×2 图标，菜单含所有布局与活动栏选项。
- 本轮未覆盖强调色切换、极窄窗口及键盘焦点的完整遍历；这些不记为通过。控件无新增禁用条件。

## 首次调整记录（图标与位置结论以上节为准）

- 对照：用户本轮标注截图；主工作区工具栏仍遵循 `reference-ui/concepts/multi-column-workspaces-2026-10-19.md`。本次局部修改为布局图标打开下拉面板；菜单保留单/双/四栏及多栏活动栏选择。独立数字焦点按钮已移除，活动栏选择保留在菜单。子文件夹按钮使用 ToggleRight 图标，开关逻辑不变。
- 环境：Windows，150% DPI；Rust `1.97.1-x86_64-pc-windows-msvc`；同进程 FileAtlas MCP `fileatlas-shell --mcp`，隔离 profile `target/mcp-toolbar-dropdown-final`，`dev-test` 开启，AI control 为 Standard。
- 命令：`cargo +1.97.1 fmt --all -- --check`；`cargo +1.97.1 check -p fileatlas-plugin-columns -p fileatlas-plugin-file-manager -p fileatlas-plugin-i18n --locked -j 2`；`cargo +1.97.1 build -p fileatlas-shell --locked -j 2`；`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`；MCP 由 `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-toolbar-dropdown-final --access-level standard --scenario target/toolbar-dropdown-scenario.cjs` 启动。
- MCP 实测：下拉显示三种布局并勾选当前项；双栏状态中显示并勾选活动栏 1/2；点击活动栏 2 后 `program_workspace.active_column` 变为 `1`。工具提示和菜单项在中文界面显示为中文。子文件夹按钮的 ToggleRight 点击后 `browser.show_descendants` 从 `false` 变为 `true`。截图已实际打开检查，菜单无 tooltip 遮挡。
- 结果：fmt、受影响 crate 编译、Shell 构建和主题/刷新/架构/feature-ledger 守卫通过。架构守卫仍报告两个历史孤立目录；Shell 链接器报告既有 LNK4098。
- 截图：![layout-menu-dual.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/layout-menu-dual.png)、![descendants-toggle-on.png](../../../.local/validation/artifacts/2026-10-19-toolbar-dropdown/descendants-toggle-on.png)。
- 范围：运行时检查覆盖单/双栏菜单和活动栏路由、子文件夹图标点击；四栏选项在同一菜单可见，但本次未另做四栏点击验证。未检查小窗口菜单翻转/边界及全部 hover/focus/disabled 状态。
