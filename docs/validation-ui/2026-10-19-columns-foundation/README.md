# 主工作区分栏与程序工作区（2026-10-19）

## 实现

- 新增 `photag-plugin-columns` Cordis 功能插件：顶部提供单栏、双栏、四栏布局切换及当前栏选择。单栏保持现有主视图；双栏左右排列，四栏为 2×2。
- 每栏是完整的独立文件浏览器实例，拥有自己的地址栏、后退/前进/上级/刷新导航、目录内容、筛选与搜索状态、排序/分组/网格列表设置、选择和滚动位置。顶部通用视图操作作用于当前激活栏；地址行仅提供地址导航。
- 搜索与功能按钮仍与筛选/排序/分组栏分离；原有界面组件样式沿用。每栏使用完整 Gallery 区域，左侧全局导航和右侧检查器仍为主窗共享区域。
- 新增 `photag-plugin-workspaces` Cordis 功能插件：程序工作区可创建、打开、重命名、设为默认、保存当前状态和删除非默认工作区。启动时恢复默认布局。
- 工作区以版本化 JSON 原子持久化，不存媒体记录。正式数据文件位于 `%LOCALAPPDATA%\\PhoTag\\workspaces\\workspaces.json`；dev-test 使用其独立 profile 的 `workspaces.json`。
- Everything 的即时搜索范围仍属于当前栏的搜索行为；Everything 程序设置不放进工作区，也不在分栏控件内重复呈现。

## 验证

- Toolchain：`1.97.1-x86_64-pc-windows-msvc`。
- `cargo +1.97.1 fmt --all -- --check` 通过；`cargo +1.97.1 check --locked --offline -j 2 -p photag-shell -p photag-plugin-gallery -p photag-plugin-columns -p photag-plugin-workspaces -p photag-plugin-mcp` 通过；`cargo +1.97.1 build --locked --offline -p photag-shell -j 2` 通过。链接器仍打印既有 LNK4098 LIBCMT 默认库冲突警告。
- 治理检查通过：`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`、`node scripts/check-layering.cjs`、`git diff --check`。架构守卫仍列出两个历史无 Cargo.toml 的孤立目录警告。
- 使用项目同进程 PhoTag MCP，Windows 150% DPI，隔离 profile `target/mcp-columns-final`，dev-test 插件启用。场景脚本：`target/multicolumn-workspace-scenario.cjs`、`target/multicolumn-restore-scenario.cjs`、`target/capture-final-columns.cjs`。从空 profile 运行双栏，切换激活栏，分别导航至两个不同路径并验证后退历史；打开工作区管理器保存当前双栏。关闭再启动后，MCP `get_ui_state.program_workspace` 显示双栏与两栏路径/历史恢复；随后切到四栏，并重新打开默认工作区验证布局恢复。MCP stderr 无错误。
- 2026-10-19 单栏与点击切换修复：Shell 单栏布局现在挂载 `gallery.column.0.content`（保留旧 `gallery.content` 兼容回退），避免 Gallery 因内容注册在栏目专属 slot 而空白。栏激活改为在鼠标事件捕获阶段执行，防止 Gallery 子控件停止冒泡后活动栏仍停留在旧值。隔离 MCP 验证点击双栏右侧内容后，`program_workspace.active_column` 从 `0` 变为 `1`，revision 从 `1` 变为 `2`，活动边框随之移到右栏；单栏截图显示 Gallery 空文件夹状态而非空白内容。验证 profile 均使用独立 dev-test 数据库。
- 2026-10-19 地址行修复：栏目地址区恢复可点击的路径面包屑，当前目录段固定可见，上级路径在剩余宽度内横向滚动；编辑按钮切回路径输入。地址导航与打开文件夹/刷新操作组合为单行，操作按钮不收缩。使用 `target/column-breadcrumb-layout.cjs` 在同进程 MCP 导航到真实 fixture 目录后截取双栏与四栏；使用 `target/column-breadcrumb-click.cjs` 点击左栏 `03-nested` 面包屑，MCP `gallery.path_prefix` 从 `...\03-nested\level-one\level-two` 变为 `...\03-nested`，确认跳转发生在对应栏。窗口为 Windows 150% DPI，profile `target/mcp-column-breadcrumb-click-final`；坐标依 `main_window_status.geometry.screenshot_to_pointer` 换算。

## 截图与边界

- [双栏：独立目录、共享顶部筛选操作](dual.png)
- [四栏：2×2 布局与每栏地址区](quad.png)
- [程序工作区管理器（中文）](workspace-manager.png)
- [重启恢复的双栏工作区](restored-startup.png)
- [点击右栏后的活动栏状态](active-column-after-click.png)
- [单栏 Gallery 空文件夹状态](single-gallery-render.png)
- [双栏可点击地址面包屑](address-breadcrumbs.png)
- [四栏地址行按钮保持单行](quad-address-row.png)
- [面包屑跳转到父目录后的结果](breadcrumb-navigation-result.png)

已在可用主窗口尺寸下通过同进程截图检查双栏与四栏结构、顶部栏分组、工作区管理器和恢复结果。尚未在本轮实际操作窗口缩放/极窄宽度下验证横向滚动；选择项和滚动偏移已纳入持久化模型，但本轮未用有大量结果的文件夹验证这两种长列表恢复。逐项 CRUD 运行点击、主题强调色切换和高密度操作状态尚未做完整视觉遍历。右侧检查器容器和左侧导航是全局共享；检查器贡献与侧栏导航选择目标跟随当前激活栏，各栏内部地址导航独立。`workspace.current_snapshot` 由宿主维护并供管理器保存。
