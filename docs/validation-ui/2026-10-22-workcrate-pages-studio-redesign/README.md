# 2026-10-22 WorkCrate 页面管理与装配工作台重设计：统一图标按钮与组件

按活动参考 `docs/reference-ui/concepts/workcrate-pages-user-2026-10-21.png`
（Tier 2）与 `workcrate-studio-user-2026-10-21.png` + `workcrate-inspector-2026-10-21.png`
（Tier 3）重做两屏；实现清单见
`docs/reference-ui/concepts/workcrate-three-level-2026-10-20.md` 的
"2026-10-22 pages and studio screen implementation checklist"。全部配色/控件
来自 GPUI Kit 原生主题与 `fileatlas-ui-bridge::components`，无本地调色板；
画布编辑语义（手势、映射、拖放、官方会话）零改动。

## 实现要点

- 页面管理（`pages.rs`）：头部块替换通用窄条——eyebrow "TIER 2 · PAGE MATRIX ·
  {项目名}"、hero 标题（项目名，24 半粗）、统计 “VectorCraft 内部多画布 · 共 N
  个独立物料 · 已选中 M 个”；右侧动作簇：多选图标开关（ListChecks，
  `.selected`，退出多选清空选择）、全选/取消全选 ghost、关联工程 Link2、
  同步工程与预览 RefreshCw、primary 新增画布 Plus。screen_bar 不再出现在
  页面屏（`render.rs` 条件改为仅编辑器显示）。
- 页面卡：复选框图标钮（Square/SquareCheck，常显；勾选即开启多选）＋截断
  标题（18 半粗）＋右上 “物理 · 像素” 规格；物料信息条（selection 底色，
  “01 {物料 尺寸 数量}”，无字段时不显示）；预览区保留原行为（多选时点击
  切换选择，否则打开页面，选中时 accent 边框）；页脚 hairline 上方 “第 N 页”
  ＋ ghost 编辑（Pencil）。
- 导出页脚：左侧 “本地预览 · 外部编辑保存后请同步”＋“已选 M 页”；右侧
  取消多选、PNG/JPEG ghost 切换、outline 导出选中页为图片（FileImage）、
  primary 导出全部为 PDF。
- 装配工作台（`render.rs render_editor`、`canvas_surface.rs canvas_toolbar`、
  `object.rs`、`typography.rs`、`inspector.rs`）：
  - 左栏迷你页卡：accent 编号 “01”＋截断标题＋物理尺寸行＋缩略图，活动卡
    accent 边框；`rail-{id}` 与点击切页保留。
  - 画布工具条统一：添加图片（ImagePlus）、新建文字（Type）图标＋文字
    outline；ZoomOut/百分比/ZoomIn、Undo2/Redo2 图标钮（`icon_action`，与
    `screen_action` 同样的 field_saves/canvas_busy 守卫）；放置设计文件
    （FilePlus）、导出本页图片（FileImage）outline；状态文本；primary
    在 VectorCraft 中编辑（ExternalLink）。旧 `action()` 助手随之删除。
  - 中央 meta 行：标题 · 物理尺寸 · 画板 pt 尺寸，右侧 Frame 图标＋像素规格。
  - 对象面板与页面检查器统一为 `section_card`（accent 圆点＋半粗标题＋
    灰 caption＋圆角描边卡）：画布对象·官方选区、位置与尺寸·官方 bounds、
    文字/文字内容·官方文本、链接图片·外部文件、文字格式·字体与字号
    （原 `section()` hairline 标题删除）、画板·添加图片、页面信息·画板规格、
    页面内容·同步物料栏。字段行、预设下拉、对象字段 id 与行为不变。
- 保留全部旧控制 id（`pages-multi-select`、`pages-select-all`、
  `pages-clear-selection`、`pages-exit-multi-select`、`select-page-{id}`、
  `open-page-{id}`、`page-preview-{id}`、`export-*`、`workcrate-new-page`、
  `attach-document`、`refresh-native`、`rail-{id}`、`field-*`、`preset-*`、
  `page-add-field`、`workcrate-canvas-*`、`place-design-file`、
  `export-current-image`、`workcrate-canvas-handoff`），既有场景脚本无需修改。

## 验证（同进程 MCP，profile wc-pages-redesign-20261022，暗色主题）

命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe
--profile .local/validation/profiles/wc-pages-redesign-20261022 --access-level full
--scenario scripts/validate-workcrate-pages-redesign.cjs`

| 检查项 | 参考要求 | 实际表现 | 证据/剩余差异 |
|---|---|---|---|
| Tier 2 头部 | eyebrow＋hero＋统计＋右侧动作簇 | 通过 | `pages-redesign.png` |
| 页面卡解剖 | 复选框＋标题＋规格＋信息条＋预览＋页脚 | 通过（新项目无物料字段时信息条不显示） | 截图 |
| 多选开关 | 图标开关可逆，退出清空选择 | `multi_select`/`selected_pages` 状态往返断言通过 | report.json |
| 复选框选择 | 勾选开启多选并选中该页 | 状态断言通过；取消全选清空 | report.json |
| 导出页脚 | 统一图标按钮＋提示＋计数 | 通过 | 截图 |
| Tier 3 工具条 | 添加图片/新建文字/缩放/撤销重做/放置/导出/移交 | 10 个控制框全部存在 | report.json |
| 缩放图标 | 点击改变显示百分比 | 100%→80%→100% 状态断言通过 | report.json |
| 左栏迷你卡 | 编号＋缩略图＋尺寸，活动 accent 边框 | 通过 | `studio-redesign.png` |
| 检查器卡片 | 统一 section_card 样式 | 通过（画布对象/文字/画板/页面信息/页面内容） | 截图 |
| 层级导航 | 编辑器→页面→项目 | 真实窗口点击均达目标屏 | report.json |
| 亮色主题/小窗口 | 布局退化 | 未验（本轮仅暗色默认窗口宽） | 待补 |
| 多页左栏切换 | rail 点击换页 | 未在本场景覆盖（`rail-{id}` 保留，既有
`validate-workcrate-page-switch.cjs` 场景仍指向同一 id） | 待补跑 |

静态检查：fmt、workcrate-ui 12 项单测、check-ui-theme、check-ui-refresh、
check-architecture、check-feature-ledger 全部通过；locked shell build 通过
（警告仅为既有 LNK4098 链接器提示）。

本机产物：`.local/validation/profiles/wc-pages-redesign-20261022/`
（report.json、screenshots/pages-redesign.png、studio-redesign.png、
pages-redesign-result.json）。验证结束后仅余用户自有的
`vectorcraft.exe --control 17979` 进程，未受影响。

## 复核补齐（对照用户确认的三张参考图）

- 页面卡信息条改为常显："01 {页面标题}" 必有，物料/尺寸/数量字段存在时追加
  在右侧（与参考稿 "01 1024 工程师文化节 …" 一致）。
- 导出按钮带数量：导出选中页为图片 ({M})、导出全部为 PDF ({N} 页)。
- hero 标题旁补 "页面管理 (Tier 2)" 小徽标（selection 底色 accent 字）。
- 复测 profile `wc-pages-redesign-20261022b`（徽标+数量）与
  `wc-pages-redesign-20261022c`（信息条常显）全部断言再次通过，截图已更新。
- 与参考稿的其余差异均为无后端能力支撑的元素，未虚构：卡片页脚的
  "N 个物料图层" 计数与复制/删除页图标（无本地图层数据与页级复制/删除
  命令）、"导出 .vectorcraft 工程" 按钮（导出 crate 未提供该输出）、
  头部返回箭头（三层入口带已承担导航）。
