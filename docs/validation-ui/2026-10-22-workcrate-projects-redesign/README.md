# 2026-10-22 WorkCrate 项目管理面板重设计：统一图标按钮与组件

按活动参考 `docs/reference-ui/concepts/workcrate-projects-user-2026-10-21.png`
重做 Tier 1 项目管理界面，并抽取三层入口带为共享组件；实现清单见
`docs/reference-ui/concepts/workcrate-three-level-2026-10-20.md` 的
"2026-10-22 projects screen implementation checklist"。全部配色/控件来自
GPUI Kit 原生主题与 `fileatlas-ui-bridge::components`（`workbench_card`、
`icon_button`、`search_input`、`action`），无本地调色板。

## 实现要点

- 三层入口带（`render.rs tier_band`）：居中三胶囊——项目管理 (Tier 1, Briefcase)、
  页面管理 (Tier 2, LayoutGrid，meta 为项目名)、装配工作台 (Tier 3, PenTool，
  meta 为页码)。活动项为卡面＋边框＋半粗；可达项悬停可点击
  （Tier 1 始终可返回首页；Tier 2 在进入项目后可达）；不可达项灰显。
  保留控制 id `workcrate-projects-home` / `workcrate-pages-home`（旧场景兼容）。
- 页面/编辑器屏的旧面包屑替换为同一入口带＋窄操作条（图标＋文字按钮：
  关联工程 Link2、新增 A3 页面 Plus、同步工程与预览 RefreshCw）。
- 项目卡（`projects.rs project_card`）：标题（截断、18 半粗）＋图标行
  重命名 Pencil / 复制 Copy / 删除 Trash2（原生确认对话框）/ 更多 Ellipsis
  （菜单保留更换主图、归档·恢复），全部 `components::icon_button`＋中文
  tooltip；meta 行 “N 个画布 · 相对时间”（≥30 天显示 YYYY-MM-DD，纯函数
  `relative_time`/`civil_from_days` 含单测）；页内信息条（selection 底色，
  “01 首页标题”＋物料/尺寸/数量，取自 boards-cache 首页，`Snapshot.summaries`
  后台读取，不开官方进程）；预览区＋“项目主图 · 模版画布预览”说明；页脚
  hairline 上方 FileBox＋“.vc 关联工程/未关联工程”，右侧 ghost “进入页面矩阵
  →”。点击封面或页脚进入 Tier 2。
- 头部块：eyebrow、hero 标题“项目工程大盘”、说明文字，右侧统一搜索框
  （220，`search_input`）＋排序/归档图标开关（`icon_button`＋`.selected`）＋
  primary “新建工程”（Plus）。底部居中 480×72 “创建新物料工程
  （自动加载 VectorCraft 模版）”磁贴（Plus 圆标）。
- 状态面新增 `sort_by_name`/`show_archived` 供 MCP 断言。

## 验证（同进程 MCP，profile wc-projects，暗色主题）

命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe
--profile .local/validation/profiles/wc-projects --access-level full
--scenario .local/validation/validate-projects-redesign.cjs`
（场景已收录 `scripts/validate-workcrate-projects-redesign.cjs`）。

| 检查项 | 参考要求 | 实际表现 | 证据/剩余差异 |
|---|---|---|---|
| 三层入口带 | 居中三胶囊，活动卡面、可达可点 | 通过；Tier 2 显示项目名，页码 meta 在编辑器出现 | `projects-redesign.png`、`pages-tier-band.png` |
| 头部块 | eyebrow＋hero＋说明＋右侧搜索/排序/归档/新建 | 通过 | 截图＋状态断言 |
| 卡片解剖 | 标题＋图标行＋meta＋信息条＋预览＋页脚 | 通过（0 页项目显示“未关联工程”、无信息条） | 截图 |
| 图标开关行为 | 排序/归档切换可逆 | `sort_by_name`/`show_archived` 状态往返断言通过 | report.json |
| 卡片菜单 | 更多菜单含更换主图/归档 | 打开/关闭断言通过 | report.json |
| 层级导航 | 编辑器→页面列表→项目管理 | 真实窗口点击三个胶囊均达目标屏 | report.json |
| 空态/归档空列表 | 显示中文提示 | 归档视图切回断言通过；空文案实现 | 截图未单独保存 |
| 亮色主题/小窗口 | 布局退化 | 未验（本轮仅暗色默认 1560 宽） | 待补 |

修正记录：初版 Tier 1 胶囊在项目内不可点击（导航目标条件写反），场景
“返回项目管理”超时暴露后修正；菜单项漏包 `tracking::control` 导致控制框
缺失，已补。构建警告仅为既有 LNK4098 链接器提示。

静态检查：fmt、workcrate-ui 12 项单测、check-ui-theme（含 self-test）、
check-ui-refresh、check-architecture、check-feature-ledger 全部通过；
`cargo check -p fileatlas-plugin-workcrate-ui`、locked shell build 通过。

本机产物：`.local/validation/profiles/wc-projects/`（report.json、
screenshots/projects-redesign.png、pages-tier-band.png、
projects-redesign-result.json）。验证结束后仅余用户自有的
`vectorcraft.exe --control 17979` 进程，未受影响。
