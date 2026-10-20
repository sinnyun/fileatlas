# 当前界面参考入口

界面实现先读 [按图实现与验收流程](../ui-reference-workflow.md)，再打开本表指向的图片及说明。
此表记录当前选用稿；`concepts/` 中未被选用的图片不因日期较新就自动成为规范。
用户本次明确选择的设计优先于本表，执行者须在同一改动中更新对应条目。

| 界面 | 当前参考图 | 行为与布局说明 |
|---|---|---|
| 主窗公共操作带与栏目地址导航 | [FileAtlas 0.2 多栏概念图](concepts/multi-column-workspaces-2026-10-19.png) | [多栏与工作区清单](concepts/multi-column-workspaces-2026-10-19.md)：筛选/视图操作和搜索/功能分组位于各栏地址行上方；各栏保留独立地址和完整文件视图。沿用现有控件样式；2026-10-18 工具栏图仅作早期控件调整历史 |
| 左侧收藏分组（覆盖旧复合收藏区） | [用户分组参考](concepts/sidebar-favorite-groups-user-reference-2026-10-18.png)、[深蓝整合概念](concepts/sidebar-favorite-groups-2026-10-18.png) | [分组栏清单](concepts/sidebar-favorite-groups-2026-10-18.md)：每组独立框，框/标题与驱动器和快速访问共用组件；组内＋直接收藏主窗当前目录 |
| 全局主题、公共组件（覆盖旧稿颜色和控件风格） | [低饱和度浅/暗主题](concepts/semantic-muted-theme-2026-10-19.png) | [低饱和度语义清单](concepts/semantic-muted-theme-2026-10-19.md)、[主题开发规范](../ui-theme.md)。两种模式保留相同功能布局和控件关系；2026-10-19 起以 GPUI Kit 原生 Theme 为唯一颜色来源，Material 生成 JSON；概念图不再指定旧手写 RGB |
| 设置中心全部页面 | [简洁外观](concepts/settings-minimal-2026-10-18.png)及[十页完整概念索引](concepts/settings-complete-2026-10-18.md) | 用户要求先完整概念再逐页还原；统一使用[声明式设置页](../settings-page-contract.md)，覆盖旧设置布局 |
| 菜单、对话框 | [final-v5 索引](final-v5/README.md) 对应画面 | [v5 companion notes](concepts/fileatlas-v5-companion-interfaces-2026-10-13.md)，应用后续用户决定 |
| 主窗及文件预览侧栏 | [sidebar-pages-preview v2](concepts/sidebar-pages-preview-2026-10-14-v2.png) | 侧栏/检查器结构沿用[历史界面基线](../development-baseline-2026-10-14.md)；主窗整体定义见[FileAtlas 0.2 基线](../development-baseline-0.2-2026-10-19.md)，中间文件区支持单/双/四栏 |
| 项目文件夹工作台 | 左侧导航 rail 沿用当前 FileAtlas 0.2 结构；项目树复用[收藏分组参考](concepts/sidebar-favorite-groups-2026-10-18.png)的紧凑文件夹层级与选中行样式，并占用与文件夹/标签相同的侧栏内容列 | [项目树清单](concepts/sidebar-favorite-groups-2026-10-18.md)：项目根按列表显示，当前项目树内联展开，点状连接线标明父子层级；点击另一根切换树，新增根自动选中展开。点击子目录导航主窗当前栏并保持项目页签打开，不创建浮动面板 |
| 剪贴板工作台 | [clipboard-workbench v2](concepts/clipboard-workbench-layout-2026-10-18-v2.png) | [v2 布局](concepts/clipboard-workbench-layout-2026-10-18-v2.md)；图片历史显示补充[图片预览清单](concepts/clipboard-workbench-images-2026-10-18.md) |
| 程序启动台 | [桌面图标布局 v3](concepts/program-launcher-2026-10-17-v3.png) | [v3 清单](concepts/program-launcher-2026-10-17-v3.md)：本次保留原布局，在程序信息区的名称后增加一行只读单句说明；右键卸载仅针对系统提供有效卸载命令的已安装程序 |
| 截图快捷启动管理 | 轻量管理面板；设置 → 软件显示安装状态 | [Shotoshot 集成清单](concepts/shotoshot-launcher-2026-10-18.md)：FileAtlas 启动时只检测一次；缺失时用户自行安装。截图选区、编辑、标注均由上游提供，最终图像保存到 FileAtlas 专用目录 |
| 主工作区多栏与程序工作区 | [多栏概念图](concepts/multi-column-workspaces-2026-10-19.png)及[最新用户标注与清单](concepts/multi-column-workspaces-2026-10-19.md) | 沿用现有控件样式；网格/列表旁增加分栏按钮，选择单栏/双栏/四栏；操作带分为筛选/视图区与搜索/功能区；Everything 配置迁入设置 |
| 文件暂存篮 | [网格卡片与拖放概念](concepts/file-staging-grid-drag-2026-10-20.png) | [面板清单](concepts/file-staging-basket-2026-10-20.md)、[插件端口与限制](../../crates/fileatlas-plugin-file-staging/PORTS.md)：图片缩略图与标题为主；拖到文件夹按默认行为移动，复制通过显式操作 |

这是活动参考导航，不是功能完成清单。实际实现、截图和验证边界记录于
[redesign-validation.md](../redesign-validation.md) 与其中链接的 `validation-ui/` 证据。
旧验证记录如与新截图矛盾，应在原记录标记更正并链接新证据，不能沿用旧的“已完成”。
