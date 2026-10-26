# 当前界面参考入口

> 2026-10-22 WorkCrate：项目管理、页面矩阵与装配工作台三层以本表登记的 2026-10-21 用户稿为活动参考。装配工作台中间区域分预览与编辑两态：预览态显示官方画板位图；编辑态把选择、拖拽、缩放、图片与文本改动一律转发给官方 VectorCraft 执行，FileAtlas 只采集输入、映射坐标并显示官方界框，不自建绘制、裁切或正文排版。行为以 [当前重构要求](../workcrate-vectorcraft-refactor.md) 与 [画布编辑交接文档](../superpowers/plans/2026-10-21-workcrate-canvas-editing.md) 为准。

> 2026-10-23 WorkCrate 局部扩展：插件设置集中管理完整程序配置和默认信息，增加项目范围及按列提升；页面内容首行改为独立物料类别并取消标题继承。沿用设置母版和检查器结构，本轮清单见 [默认信息分层](concepts/workcrate-editing-tools-2026-10-20.md)。

> 2026-10-23 物料栏局部更新：按用户最新明确方案，纸内横条左侧编号/类别独立，右侧所有基本及附加信息聚合为一个右对齐单行文本并以 | 分隔，覆盖旧多行与逐字段排版。工作台及检查器其余布局保持；清单仍见上述 WorkCrate 编辑扩展说明。

界面实现先读 [按图实现与验收流程](../ui-reference-workflow.md)，再打开本表指向的图片及说明。
此表记录当前选用稿；`concepts/` 中未被选用的图片不因日期较新就自动成为规范。
用户本次明确选择的设计优先于本表，执行者须在同一改动中更新对应条目。
所有界面共用[标题、正文、辅助说明三层字体规范](concepts/typography-three-tiers-2026-10-23.md)与[概念板](concepts/typography-three-tiers-2026-10-23.png)；组件样式不因界面或插件而异。

| 界面 | 当前参考图 | 行为与布局说明 |
|---|---|---|
| 主窗公共操作带与栏目地址导航 | [FileAtlas 0.2 多栏概念图](concepts/multi-column-workspaces-2026-10-19.png) | [多栏与工作区清单](concepts/multi-column-workspaces-2026-10-19.md)：筛选/视图操作和搜索/功能分组位于各栏地址行上方；各栏保留独立地址和完整文件视图。沿用现有控件样式；2026-10-18 工具栏图仅作早期控件调整历史 |
| 左侧收藏分组（覆盖旧复合收藏区） | [用户分组参考](concepts/sidebar-favorite-groups-user-reference-2026-10-18.png)、[深蓝整合概念](concepts/sidebar-favorite-groups-2026-10-18.png) | [分组栏清单](concepts/sidebar-favorite-groups-2026-10-18.md)：每组独立框，框/标题与驱动器和快速访问共用组件；组内＋直接收藏主窗当前目录 |
| 全局主题、公共组件（覆盖旧稿颜色和控件风格） | [八类语义色工作台](concepts/unified-eight-family-theme-2026-10-24.png) | [八类语义色清单](concepts/unified-eight-family-theme-2026-10-24.md)、[主题开发规范](../ui-theme.md)。最多八种可配置语义色类，亮/暗和交互状态用同类色阶；GPUI Kit 是唯一主题解析器，Material 生成 JSON，UI Bridge 是自绘控件的公共主题投影层 |
| 设置中心全部页面 | [简洁外观](concepts/settings-minimal-2026-10-18.png)及[十页完整概念索引](concepts/settings-complete-2026-10-18.md) | 用户要求先完整概念再逐页还原；统一使用[声明式设置页](../settings-page-contract.md)，覆盖旧设置布局 |
| 菜单、对话框 | [final-v5 索引](final-v5/README.md) 对应画面 | [v5 companion notes](concepts/fileatlas-v5-companion-interfaces-2026-10-13.md)，应用后续用户决定 |
| 主窗及文件预览侧栏 | [sidebar-pages-preview v2](concepts/sidebar-pages-preview-2026-10-14-v2.png) | 侧栏/检查器结构沿用[历史界面基线](../development-baseline-2026-10-14.md)；主窗整体定义见[FileAtlas 0.2 基线](../development-baseline-0.2-2026-10-19.md)，中间文件区支持单/双/四栏 |
| 项目树导航 | 左侧 Navigation rail 沿用当前 FileAtlas 0.2 结构；项目树复用[收藏分组参考](concepts/sidebar-favorite-groups-2026-10-18.png)的紧凑文件夹层级与选中行样式，并占用与文件夹/标签相同的侧栏内容列 | [项目树清单](concepts/sidebar-favorite-groups-2026-10-18.md)：项目根按列表显示，当前项目树内联展开，点状连接线标明父子层级；点击另一根切换树，新增根自动选中展开。点击子目录导航主窗当前栏并保持项目树页打开，不创建浮动面板。此条目不代表项目管理面板 |
| 剪贴板工作台 | [clipboard-workbench v2](concepts/clipboard-workbench-layout-2026-10-18-v2.png) | [v2 布局](concepts/clipboard-workbench-layout-2026-10-18-v2.md)；图片历史显示补充[图片预览清单](concepts/clipboard-workbench-images-2026-10-18.md) |
| 程序启动台 | [桌面图标布局 v3](concepts/program-launcher-2026-10-17-v3.png)；批量管理采用[2026-10-26 概念](concepts/program-launcher-batch-management-2026-10-26.png) | [v3 清单及批量扩展](concepts/program-launcher-2026-10-17-v3.md)：保留原图标布局；批量管理切换为可筛选列表与批量编辑检查器，分类、名称前后缀、可见性统一应用 |
| 截图快捷启动管理 | 轻量管理面板；设置 → 软件显示安装状态 | [Shotoshot 集成清单](concepts/shotoshot-launcher-2026-10-18.md)：FileAtlas 启动时只检测一次；缺失时用户自行安装。截图选区、编辑、标注均由上游提供，最终图像保存到 FileAtlas 专用目录 |
| 主工作区多栏与程序工作区 | [多栏概念图](concepts/multi-column-workspaces-2026-10-19.png)及[最新用户标注与清单](concepts/multi-column-workspaces-2026-10-19.md) | 沿用现有控件样式；网格/列表旁增加分栏按钮，选择单栏/双栏/四栏；操作带分为筛选/视图区与搜索/功能区；Everything 配置迁入设置 |
| 文件暂存篮 | [网格卡片与拖放概念](concepts/file-staging-grid-drag-2026-10-20.png) | [面板清单](concepts/file-staging-basket-2026-10-20.md)、[插件端口与限制](../../crates/fileatlas-plugin-file-staging/PORTS.md)：图片缩略图与标题为主；拖到文件夹按默认行为移动，复制通过显式操作 |
| 插件化画布工作区 | [tldraw 布局调整稿](concepts/fileatlas-canvas-tldraw-layout-2026-10-26.png) | [观察与实现清单](concepts/fileatlas-canvas-tldraw-layout-2026-10-26.md)：画布占据主要空间；左上文档操作、右上样式、底部中间单一浮动图标工具栏、左下缩放；保留窄的资源栏与图层检查器。 |
| WorkCrate 项目管理 | [2026-10-21 用户稿](concepts/workcrate-projects-user-2026-10-21.png)；生成概念见[项目管理概念](concepts/workcrate-projects-concept-2026-10-20.png) | [三层清单](concepts/workcrate-three-level-2026-10-20.md)：顶部三层入口带；卡片为标题＋画布数/时间＋页内信息条＋主图预览区＋底部 `.vc` 关联工程与进入页面矩阵；右上新建工程，底部创建新物料工程。图内画布数量、名称与日期均为示例内容。 |
| WorkCrate 页面管理 | [2026-10-21 用户稿](concepts/workcrate-pages-user-2026-10-21.png)；生成概念见[页面管理概念](concepts/workcrate-pages-concept-2026-10-20.png) | [三层清单](concepts/workcrate-three-level-2026-10-20.md)：返回＋项目名＋页数/选中统计；三列画布卡片带勾选框、信息条、预览区、物料图层数与编辑入口；底部为导出选中页图片、导出 `.vectorcraft` 工程、导出全部 PDF。默认 A3 横向；勾选状态驱动导出范围。 |
| WorkCrate 装配工作台 | [2026-10-21 用户稿](concepts/workcrate-studio-user-2026-10-21.png)；右侧面板细化见[检查器概念](concepts/workcrate-inspector-2026-10-21.png) | [三层清单](concepts/workcrate-three-level-2026-10-20.md)：左侧页面缩略图列表＋新建页，中央大画布，右侧依次为页面信息·画布规格、页面内容·统一物料栏、页面图片·链接设计资产。中部画布的只读预览态由官方渲染位图填充；[简化排版语义](concepts/workcrate-simple-layout-2026-10-20.md)规定物料尺寸仅作标记；[文本与字段扩展](concepts/workcrate-editing-tools-2026-10-20.md)保留物料字段与预设能力。 |
| WorkCrate 画布编辑态 | [2026-10-22 概念图](concepts/workcrate-canvas-edit-concept-2026-10-22.png) | 顶部预览/编辑切换 + 缩放 + 添加图片/链接图片/更新图片/替换图片 + 撤销/重做 + 在 VectorCraft 中编辑；中央官方位图上叠加选中界框与 8 个句柄，拖拽期间显示半透明预测框与位移提示；右侧选中对象（名称/类型/X/Y/宽/高）、文本内容（编辑后应用文字）、链接图片（更新链接/替换文件）。界框坐标与所有编辑结果都来自官方返回，FileAtlas 不本地计算变换。图内页面名、尺寸与图片内容均为示例。 |

这是活动参考导航，不是功能完成清单。实际实现、截图和验证边界记录于
[redesign-validation.md](../redesign-validation.md) 与其中链接的 `validation-ui/` 证据。
旧验证记录如与新截图矛盾，应在原记录标记更正并链接新证据，不能沿用旧的“已完成”。


> 2026-10-23 页面顺序补充：页面矩阵和编辑侧栏增加上移/下移与拖动手柄；标题固定“页面01…”、物料编号仅“01…”，均随顺序更新。页面信息只保留只读自动标题，删除尺寸标记/画板像素及编辑入口。用户此指令覆盖旧稿对应条目；清单见 WorkCrate 编辑扩展说明。


> 2026-10-23 用户局部删除：页面编辑右侧不再显示“位置与尺寸”卡片及 X/Y/宽高/应用控件；其余编辑卡片保留，覆盖旧画布编辑概念的对应数字输入设计。


> 2026-10-23 文字编辑补充：移除右侧“文字格式”（字体/字号/对齐）卡片，新建文字默认微软雅黑；用户明确保留画布移动/缩放句柄和画布放大缩小。


> 2026-10-23 对象信息位置：画布对象的选中数量、名称、类型从右侧卡片移到编辑页底部状态栏右侧；左侧操作反馈保留，原三栏及其余编辑内容不变。


> 2026-10-23 右侧顺序补充：删除页面信息卡片，将页面内容卡片移至右侧最顶端；其后保留文字/图片编辑，页面自动标题继续在列表及编辑顶栏显示。


> 2026-10-23 WorkCrate 项目数据：新建工程自动在原生文件旁保存 `.workcrate/<项目UUID>/`；项目菜单增加“迁移项目数据”处理旧项目，项目总览在新建旁增加“打开项目文件夹”恢复移动后的项目。保持活动项目稿其余布局；图片链接继续由 VectorCraft 自身维护。清单见 WorkCrate 三层说明新增的 adjacent project data checklist。
