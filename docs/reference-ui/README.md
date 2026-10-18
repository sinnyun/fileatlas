# 当前界面参考入口

界面实现先读 [按图实现与验收流程](../ui-reference-workflow.md)，再打开本表指向的图片及说明。
此表记录当前选用稿；`concepts/` 中未被选用的图片不因日期较新就自动成为规范。
用户本次明确选择的设计优先于本表，执行者须在同一改动中更新对应条目。

| 界面 | 当前参考图 | 行为与布局说明 |
|---|---|---|
| 主窗顶部、筛选栏、底栏 | [用户本轮截图](concepts/main-toolbar-icons-user-2026-10-18.png) | [定向调整清单](concepts/main-toolbar-icons-2026-10-18.md)：顶部操作图标化，网格/列表/子文件夹/缩放移到筛选栏右端，移除底部标签/更多 |
| 左侧收藏分组（覆盖旧复合收藏区） | [用户分组参考](concepts/sidebar-favorite-groups-user-reference-2026-10-18.png)、[深蓝整合概念](concepts/sidebar-favorite-groups-2026-10-18.png) | [分组栏清单](concepts/sidebar-favorite-groups-2026-10-18.md)：每组独立框，框/标题与驱动器和快速访问共用组件；组内＋直接收藏主窗当前目录 |
| 全局主题、公共组件（覆盖旧稿颜色和控件风格） | [工作台统一主题](concepts/unified-workbench-theme-2026-10-18.png) | [统一主题清单](concepts/unified-workbench-theme-2026-10-18.md)、[主题开发规范](../ui-theme.md)。按用户要求沿用剪贴板 v2 深蓝；下列各页的功能布局和控件关系继续有效 |
| 设置 → 快捷键 | [2026-10-18 快捷键页](concepts/shortcuts-settings-2026-10-18.png) | [快捷键页实现清单](concepts/shortcuts-settings-2026-10-18.md)。覆盖 v5 sheet 02 的快捷键部分；新增插件注册能力来自用户明确要求 |
| 设置其他页、菜单、对话框 | [final-v5 索引](final-v5/README.md) 对应画面 | [v5 companion notes](concepts/photag-v5-companion-interfaces-2026-10-13.md)，并应用后续用户功能决定（例如已删除的内置预览工具栏不恢复） |
| 主窗及文件预览侧栏 | [sidebar-pages-preview v2](concepts/sidebar-pages-preview-2026-10-14-v2.png) | [开发基线](../development-baseline-2026-10-14.md) 及相邻概念说明；其他已明确选用的局部稿继续覆盖对应局部 |
| 剪贴板工作台 | [clipboard-workbench v2](concepts/clipboard-workbench-layout-2026-10-18-v2.png) | [v2 说明](concepts/clipboard-workbench-layout-2026-10-18-v2.md) |

这是活动参考导航，不是功能完成清单。实际实现、截图和验证边界记录于
[redesign-validation.md](../redesign-validation.md) 与其中链接的 `validation-ui/` 证据。
旧验证记录如与新截图矛盾，应在原记录标记更正并链接新证据，不能沿用旧的“已完成”。
