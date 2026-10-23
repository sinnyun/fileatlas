# 全局 UI 字体层级审查报告（2026-10-23）

## 1. 检查基线与环境

- 检查日期：2026-10-23。
- 代码状态：HEAD `2f499a2`（fix(project-tree): isolate an unreadable root…），工作树有 301 个已修改文件、15 个未跟踪文件（未提交的批量样式收敛改动仍在工作树中）。字体相关共享组件与调用点以**当前工作树**为准核对。
- 审查规则基线：`docs/reference-ui/concepts/typography-three-tiers-2026-10-23.md`（标题 15px 半粗 / 正文 14px 常规 / 辅助说明 12px 常规）；`docs/ui-theme.md`；交接文档 `docs/ui-consistency-review-handoff-2026-10-23.md`。
- 审查色仍保留（标题红 `0xE60000`、正文绿 `0x00A040`、辅助蓝 `0x0066CC`，见 `crates/fileatlas-ui-bridge/src/theme/roles.rs:51-62`）。本报告不涉及恢复正式颜色。
- 共享文字组件定义：`crates/fileatlas-ui-bridge/src/components.rs:19-65`（`workbench_title/body/assistant`，token 见 `crates/fileatlas-ui-bridge/src/theme.rs:126-131`）；字体族由 Shell 根节点统一 `font_family(theme::UI_FONT_FAMILY)`（`crates/fileatlas-shell/src/shell.rs:546`、`crates/fileatlas-shell/src/shell/native_panels.rs:173`），当前值为 `Segoe UI`（`theme.rs:142`）。

## 2. 检查过的数据与覆盖范围

### 2.1 结构化几何数据（全量）

- 数据源：`.local/validation/profiles/ui-spacing-density-light-20261023/screenshots/` 下 55 份 `*.geometry.json`，共 13,069 条记录。
- 聚合分析脚本与输出（本地，不入库）：`.local/validation/typography-audit-20261023/analyze-typography.cjs`、`typography-report.txt`。
- 记录字段语义（`crates/fileatlas-ui-bridge/src/geometry_audit.rs:63-107`）：字体字段取自包裹 div 处 `window.text_style()` 的实际解析值；x/y/width/height 是包裹 div 的布局盒（含审查描边回补），**不是**文字行盒，因此同层记录高度有 18/20/22 等差异属正常现象，不作为行高不一致证据。

**数值结论（决定性，覆盖全部 55 个快照）：**

| 层级 | 记录数 | 实测分布 | 与规范偏差 |
|---|---:|---|---:|
| title | 496 | 全部 15px / 20 / FontWeight(600) / Segoe UI | 0 |
| body | 2,884 | 全部 14px / 18 / FontWeight(400) / Segoe UI | 0 |
| assistant | 3,278 | 全部 12px / 16 / FontWeight(400) / Segoe UI | 0 |
| button | 4,800 | 高度全部 32px（共享 token） | 0 |
| 其他（region 178、tree-row 737、card 610、navigation 46、section 40） | 1,611 | 仅框架，无字体字段 | — |

- 交接文档此前记录的两处待核实项在本轮数据中**未复现**：无 14/16 的 assistant 记录（旧 `tag_render.rs:27` 观察点），无 34.67px 超高按钮（`.max_h(32)` 修复后本轮 4,800 条按钮全部 32px）。

### 2.2 源码检索（全量）

- `crates/**` 中 `text_size / line_height / font_weight / font_family` 调用：仅存在于 `fileatlas-ui-bridge`（共享组件与 tag_capsule）和 Shell 根节点（字体族），**插件零命中**。
- `text_xs/sm/base/lg/xl`、`font_semibold/medium/bold` 等工具类：插件零命中。
- 直接 `Button::new(`：仅 UI Bridge 公共构造器（`components.rs:316`），插件零命中。
- GPUI Kit（`gpui-component-0.7.2`）内部自带文字的控件核查：弹出菜单项 `text_sm`=14px（popup_menu.rs:1237）、Checkbox/Radio 默认 Medium→14px（sizing.rs:245-252）、tooltip 主文本 `text_sm`=14px、次级文本 `text_xs`=12px（tooltip.rs:125,137）。应用未改 rem（默认 16px）。
- 主题守卫复核：`node scripts/check-ui-theme.cjs` → passed。

### 2.3 截图目检（部分）

亮色 atlas 的 55 张截图已全部逐张目检；暗色 atlas 检查了主窗口、设置、启动器、上下文菜单等 5 张重点截图。另做了 4 处局部裁剪放大：

- 已目检：`001-boot-window-3200x1800`、`100-main-single-folders`、`101-sidebar-tags`、`121-statistics`、`131-popover-tags`、`160-columns-dual`、`600-settings-appearance`、`630-navigation-create-dialog`、`700-panel-clipboard-workbench`、`702-panel-program-launcher`（及 000 未单独目检）。
- 裁剪放大证据（`.local/validation/typography-audit-20261023/`）：`crop-addressbar.png`（地址栏占位符）、`crop-settings-footer.png`（设置页底部状态）、`crop-density-desc.png`（密度说明行）、`crop-launcher-categories.png`（启动台分类列表）。
- 后续批次已目检 `110–114`、`120/122`、`130/132–135`、`140/141`、`150–152`、`601–607`、`610–612`、`620–622`、`631–633`、`640`、`701/703–705`、`800`、`900`、`910`；结合此前批次与本轮最后复核的 `000`，亮色 atlas 55 张截图已全部目检。`000-boot-window.png` 是启动中的瞬时状态，出现“正在加载”占位符合预期；静态页面的交互态仍需补查。

### 2.4 未覆盖事项与环境阻碍

- 当前亮色和暗色 atlas 均在上述按钮改动之后采集；亮色另完成全量 55 张重采。
- 深色模式已采集全量 atlas；应用支持的最低窗口为 1920×1080，低于该尺寸的截图不作为验收依据。hover/focus/disabled 状态和专门的超长文案状态仍未完整覆盖。
- WorkCrate 创建项目场景（缺 `vectorcraft-cli.exe`）与未知 i18n 设置页仍是环境阻碍，未作为证据。
- 输入框/多行文本框内容文字与占位符没有独立几何记录；占位符经裁剪目检为辅助层蓝色（合规），输入中的文字无法结构化核对。

## 3. 确定问题（按修复优先级）

### P1-1 左侧竖排导航项层级不一致

- 位置：启动台 rail 项 `crates/fileatlas-plugin-launcher/src/view.rs:76` 用 `workbench_body()`；其余同槽位 rail 项均用 `workbench_assistant()`：剪贴板 `crates/fileatlas-plugin-clipboard/src/view.rs:67`、截图 `crates/fileatlas-plugin-screenshot/src/view.rs:44`、暂存篮 `crates/fileatlas-plugin-file-staging/src/view.rs:40`、WorkCrate `crates/fileatlas-plugin-workcrate-ui/src/lib.rs:235`。
- 现象：同为 42px 宽竖排图标+文字导航项，“程序”为 14px 正文层（绿），其余为 12px 辅助层（蓝）。截图 `100-main-single-folders.png` 左栏可见差异。
- 应为：同一槽位、同类文字统一一层（建议统一辅助层，与多数一致；或全部正文层，由实现者定一层后批量统一）。
- 确定性：**确定**（源码 + 几何数据：`launcher/src/view.rs:76` 在每张快照产生 body 记录，其余 rail 项产生 assistant 记录）。

### P1-2 统计面板“占用空间”数值使用标题层

- 位置：`crates/fileatlas-plugin-file-manager/src/view/statistics.rs:84-86`，`format_storage(stats.total_size_bytes)` 包在 `workbench_title()` 中。
- 现象：`121-statistics.png` 中“196 KB”为红色标题层；同面板其他数据值（评分柱、标签计数、月份轴）均为辅助层，卡片标签为正文层。
- 应为：数据值属“主要数据”→ 正文层（`workbench_body`）。标题层应只用于分区标题（`stat_section` 已用）。
- 确定性：**确定**。

### P1-3 设置页底部“设置更改会自动保存。”使用正文层

- 位置：`crates/fileatlas-plugin-settings/src/view/layout.rs:256-260`，状态行包在 `workbench_body()` 中。
- 现象：`600-settings-appearance.png` 底部该行为绿色正文层（裁剪证据 `crop-settings-footer.png`）。
- 应为：低优先级状态/提示 → 辅助说明层（`workbench_assistant`）。同页说明文字（如“统一调整全局三档间距…”）已是辅助层，可对照。
- 确定性：**确定**。

### P1-4 检查器“使用提示”卡片正文使用正文层

- 位置：`crates/fileatlas-plugin-file-manager/src/view/inspector/program.rs:139-141`，提示文字包在 `workbench_body()` 中。
- 现象：`001-boot-window-3200x1800.png` 右栏“使用提示”内容为绿色正文层。
- 应为：帮助文案 → 辅助说明层。同文件 `storage_row` 的标题与路径均已用辅助层。
- 确定性：**确定**。

### P1-5 数据库路径行以链式顺序绕过主题守卫，且同层前景色不一致

- 位置：`crates/fileatlas-plugin-file-manager/src/view/inspector/program.rs:167-170`（`storage_row` 路径行）：`.text_color(rgb(theme::text_primary())).workbench_assistant()`。
- 现象：路径文字为 12px 辅助层尺寸但着正文层绿色（`001` 截图可见）；守卫脚本 `scripts/check-ui-theme.cjs:13` 只检查 `workbench_*(…).text_color(…)` 顺序，`.text_color()` 在前则漏检。
- 应为：去掉局部前景覆盖，让辅助层继承统一 `text_secondary`；如确需区分前景，应在守卫中同时匹配两种链序。
- 确定性：**确定**（源码 + 截图）。守卫漏检属附带发现。

### P2-1 启动台分类名逐字换行并溢出行框

- 位置：`crates/fileatlas-plugin-launcher/src/view/sidebar.rs:24-33`（`category_row` 名称列为 `flex_1().min_w_0().overflow_hidden()` + `workbench_body()`，无 `text_ellipsis`/禁止换行）；行容器为 `workbench_nav`（固定 `nav_row_height`）。
- 现象：`702-panel-program-launcher.png` 与裁剪证据 `crop-launcher-categories.png`：“文件与同步”“设计创作”等名称被压缩成 2–3 行逐字换行，且第三行（如“步”）垂直溢出绿色行框。
- 应为：名称列加 `text_ellipsis()`（与 `components.rs:536` 侧栏头做法一致），或加宽/重排侧栏列；不得缩小字号。
- 确定性：**确定**（截图 + 源码）。属“文字被挤压/裁切”类。

### P2-2 驱动器行盘名压缩不可见、用量计数溢出到“…”按钮（用户报告，2026-10-23 复核确认）

- 位置：`crates/fileatlas-plugin-navigation/src/view/sections/render.rs:421-436`（行构造：名称列 `flex_1().min_w_0().text_ellipsis()`，用量文本无收缩约束）。
- 现象：裁剪证据 `crop-drives.png`（源图 `100-main-single-folders.png` 左栏）：默认侧栏宽度下盘名被压成只剩“..”几乎不可见；“388 / 464”式用量计数向右溢出，与盘名省略号及右侧“…”菜单按钮的蓝色边框重叠。
- 应为：缩短用量文本（本次已按用户指示实施，见 §8 注）；并考虑给用量文本容器补 `min_w_0` 收敛约束或在窄宽度下重排。
- 确定性：**确定**（截图 + 源码）。

### P3-1 侧栏导航行名称层级跨插件不一致（已修复）

- 位置：Navigation 侧栏行名称为辅助层——`crates/fileatlas-plugin-navigation/src/view/sections/render.rs:183-188`（`path_row`，快速访问/收藏夹/树行共用）与 `render.rs:424-429`（驱动器行盘名）；启动台侧栏行名称为正文层——`crates/fileatlas-plugin-launcher/src/view/sidebar.rs:24-33`（分类行）与 `sidebar.rs:201`（隐藏工具行）。
- 现象：同类文字（侧栏导航行名称）一处 12px 辅助蓝、一处 14px 正文绿（`100-main-single-folders.png` 收藏行 vs `702-panel-program-launcher.png` 分类行）。
- 修复：两侧现都使用 `workbench_assistant()`；并在 2026-10-23 后续亮色/暗色截图中复核。
- 确定性：**确定**（源码 + 几何数据 + 后续截图）；已关闭。
- 附注：收藏区行布局（图标+名称+菜单）本身对齐正常，未发现错位；用户报告的错位仅出现在驱动器行（见 P2-2）。

## 4. 标记与结构化覆盖缺口（非字号错误，但影响可核对性）

1. **标签胶囊文字无审查标记与几何记录**：`crates/fileatlas-ui-bridge/src/tag_capsule.rs:66-90` 直接用 `theme::FONT_SIZE_ASSISTANT/LINE_HEIGHT_ASSISTANT`（数值合规）但未接入 `observe_typography`，截图中胶囊文字（如 `131-popover-tags.png`、`160-columns-dual.png` 的标签胶囊）无青色占用盒，无法逐项核对。建议接入共享观察器或复用 `workbench_assistant`。
2. **GPUI Kit 原生组件自带文字不在几何采集范围**：弹出菜单项、Checkbox/Radio 标签没有独立几何记录；字体数值已核实为正文层 14px，截图中的菜单项也呈现正文审查色。tooltip 主文本 14px 按正文层处理，次级快捷键提示 12px 按辅助说明层处理；UI Bridge 主题审查映射已覆盖这两个角色，`validate-typography-tooltip.cjs` 的同进程截图验证主文案为正文色。剩余缺口是原生 checkbox/radio 无独立文字几何记录。
3. **Input/Textarea 内容文字无独立几何记录**（交接文档已知盲区）；占位符经目检为辅助层蓝色，合规。其余输入态文字无法结构化核对。
4. **模态弹窗无浮动面板红色外框标记**：`630-navigation-create-dialog.png` 等弹窗表面没有外框审查标记，弹窗边界无法结构化查询（不影响字体，但几何审查覆盖不到弹窗表面）。待核实是否应纳入采集。

## 5. 已确认符合规范的类别及证据

- 三层字号/字重/行高/字体族：13,069 条记录零偏差，全部 Segoe UI，全量覆盖 55 个快照（含主窗、多栏、检查器、全部设置页、全部浮动面板、弹窗、弹层）。
- 共享按钮：4,800 条全部高 32px、文字正文层（`components.rs:314-361`），无插件固定尺寸残留（本轮数据）。
- 插件源码无任何本地字号/行高/字重/字体族/文字工具类调用；`node scripts/check-ui-theme.cjs` 通过。
- 输入框占位符：辅助层蓝色（裁剪目检 `crop-addressbar.png`）。
- 已目检页面中：顶部筛选栏、分组标题/计数、侧栏树行（驱动器计数、文件夹行）、文件名/尺寸、检查器各分区、设置页表单标签与说明、剪贴板面板历史条目（内容正文、`14:31 · N 字` 等元数据辅助层）、启动台底部状态行（辅助层，源码 `launcher/src/view/layout.rs:267-276`）、多栏双列布局均一致符合三层。
- 交接文档此前两项待核实（`tag_render.rs:27` 的 14/16 观察、34.67px 标签按钮）本轮未复现，判为已消除。

## 6. 结构化检查器能力边界（本轮再次确认）

- 无父子关系、显式 gap/padding token（仅部分组件带 token 字段）、交互状态（hover/focus/disabled/selected）、裁切/遮挡信息；文字占用盒为包裹 div 布局盒而非行盒。
- 因此：同层文字的**数值一致性**可全量判定；**语义层级归类是否正确**（如 P1-2/P1-3/P1-4）与**漏标**只能靠源码 + 截图；挤压/溢出只能靠截图。
- 坐标近邻推断不作为父子/间距结论（汇总器已删除该类推断）。

## 7. 实际执行的命令与结果

- `git log --oneline -5`、`git status --porcelain`（301 M / 15 ??）。
- `node .local/validation/typography-audit-20261023/analyze-typography.cjs`（自建聚合脚本，输出 `typography-report.txt`；exit 0）。
- `node scripts/check-ui-theme.cjs` → “UI theme ownership: passed”。
- `grep -rn` 系列检索（text_size/line_height/font_weight/font_family、text_* 工具类、Button::new、指定字符串定位）：除 UI Bridge 与 Shell 根节点外零命中。
- gpui-component 0.7.2 源码核查（cargo registry 路径）：popup_menu/checkbox/radio/tooltip/empty 文字尺寸。
- PowerShell System.Drawing 裁剪放大 4 处截图区域（输出至 `.local/validation/typography-audit-20261023/`）。
- 未重新构建、未重新采集截图（沿用现有 atlas；未修改任何应用代码、样式或审查色）。

## 8. 修复批次建议（去重后清单）

| # | 修复 | 位置 | 目标层 |
|---|---|---|---|
| 1 | 启动台 rail 项统一层级（建议 assistant） | `fileatlas-plugin-launcher/src/view.rs:76` | 辅助 |
| 2 | 统计“占用空间”数值改正文层 | `fileatlas-plugin-file-manager/src/view/statistics.rs:84-86` | 正文 |
| 3 | 设置底部自动保存状态改辅助层 | `fileatlas-plugin-settings/src/view/layout.rs:256-260` | 辅助 |
| 4 | “使用提示”内容改辅助层 | `fileatlas-plugin-file-manager/src/view/inspector/program.rs:139-141` | 辅助 |
| 5 | 移除路径行正文前景覆盖（并补守卫双链序检测） | `fileatlas-plugin-file-manager/src/view/inspector/program.rs:167-170`；`scripts/check-ui-theme.cjs:13` | 辅助 |
| 6 | 启动台分类名加 text_ellipsis（或加宽列） | `fileatlas-plugin-launcher/src/view/sidebar.rs:24-33` | 布局 |
| 7 | 驱动器行用量文本容器补收缩约束，复核盘名可见性 | `fileatlas-plugin-navigation/src/view/sections/render.rs:421-436` | 布局 |
| 8 | 侧栏导航行名称统一层级 | `fileatlas-plugin-navigation/src/view/sections/render.rs:183-188`；`fileatlas-plugin-launcher/src/view/sidebar.rs:24-33,201` | 已统一为辅助层 |
| 9 | 标签胶囊接入审查观察器 | `fileatlas-ui-bridge/src/tag_capsule.rs` | 工具 |
| 10 | tooltip 主文案映射正文色、次级提示映射辅助色，并同进程截图验证 | `fileatlas-ui-bridge/src/theme/runtime.rs`、`scripts/validate-typography-tooltip.cjs` | 已验证 |
| 11 | 面包屑/筛选行两处按钮改动后的 atlas 重采；小窗/深色/hover 状态补采 | `scripts/capture-panel-atlas.cjs` | 验证 |

## 9. 用户指示后已实施的变更（2026-10-23，报告完成后）

- 用户确认驱动器行文字错位，并指示“不用显示完整数据，直接显示多少 TB 和 GB”。已实施：
  - `crates/fileatlas-plugin-navigation/src/view/sections/render.rs:21-34`：新增 `size_label`（≥1 TB 显示一位小数 TB，≥1 GB 显示整数 GB，≥1 MB 显示 MB，否则 KB）；驱动器行用量文本改为仅显示总容量（如 `3.6 TB`、`465 GB`），已用比例仍由行内进度条表达。
  - 移除失去唯一调用者的 `group_thousands`（`crates/fileatlas-plugin-navigation/src/view/sections.rs`）。
  - `cargo +1.97.1 check -p fileatlas-plugin-navigation --locked -j 2` 通过；`cargo +1.97.1 fmt --all -- --check` 通过。尚未重建 Shell、未重截界面；atlas 与几何 JSON 仍是改动前状态，下次集中验证时复核盘名可见性与行布局。

## 10. 字体审查问题修复进展（2026-10-23）

根据本报告第 3、4、8 节的确定项，已在工作树修复并复核：

- 启动台左侧快捷入口与分类/隐藏工具导航文字统一到辅助层；分类长名和隐藏工具名采用省略号适配固定行宽（P1-1、P2-1、P3-1）。
- 统计“占用空间”数值改为正文层；设置底部自动保存状态、检查器使用提示改为辅助层（P1-2 至 P1-4）。
- 检查器数据库路径移除层级组件之前的局部文字颜色覆盖；主题守卫现在探测文字色覆盖层级组件的两种链式顺序，补上反向顺序自测。顺带移除了多个在层级组件调用前设置、随后会被主题层级覆盖的冗余文字颜色调用（P1-5）。
- 驱动器容量显示采用简短单位值，容量文字可收缩并省略，实测快照中未再压到右侧菜单按钮（P2-2）。
- 标签胶囊文字现有辅助层审查占用框，且保持其主题/用户标签前景色不被审查包装器覆盖（第 4 节第 1 项）。

**本轮构建后的验证：**Windows Rust 1.97.1 MSVC；格式检查、UI theme 守卫（含自测）、UI refresh、architecture、feature ledger 均通过；`cargo +1.97.1 check -p fileatlas-ui-bridge -p fileatlas-plugin-settings -p fileatlas-plugin-navigation -p fileatlas-plugin-file-manager -p fileatlas-plugin-launcher -p fileatlas-plugin-clipboard -p fileatlas-plugin-workcrate-ui --locked -j2` 通过；`cargo +1.97.1 build -p fileatlas-shell --locked -j2` 通过（仅既有 LNK4098）。

重采 profile `.local/validation/profiles/typography-repair-20261023/`：55 张同进程 MCP 截图、55 个几何 JSON、13,813 条记录。三层全部无数值变体：标题 15/20/600 共 497 条，正文 14/18/400 共 2,703 条，辅助说明 12/16/400 共 4,190 条，字体族均为 Segoe UI；共享按钮 4,810 条全部高 32px。atlas 3 个未完成环境场景为未知 `fileatlas.i18n`/`i18n` 设置页和缺少 `vectorcraft-cli.exe` 的 WorkCrate 创建项目操作，MCP 进程正常退出。亮色目检 `100-main-single-folders.png`、`121-statistics.png`、`600-settings-appearance.png`、`702-panel-program-launcher.png`、`001-boot-window-3200x1800.png`，确认驱动器容量不再压住菜单、省略文本没有超出导航行、统计数值为正文色、提示/底部状态为辅助色。上述输出仅在 `.local/validation/`，不入库。

**仍未完成：**暗色模式 atlas 仅抽查 5 张；窗口小尺寸、全部 hover/focus/disabled 状态及超长文案仍需覆盖。GPUI Kit 原生 Checkbox/Radio 标签尚无独立审查框与结构化文字记录。审查色继续保留。

### 10.1 后续截图检查（2026-10-23）

- 复核亮色截图 `133`–`135`、`140`–`141`、`150`–`152`、`161`–`162`、`604`：日期/评分/排序弹层、搜索范围、文件上下文菜单、类型/月分组、四列布局和软件更新设置页。未发现新的确定性文字溢出；分组长标题已有省略处理，排序项与菜单行未发现换行/重叠。
- 源码复核确认 Navigation 路径树行、Launcher 分类/隐藏工具名均为 `workbench_assistant()`。旧版报告中“Navigation 辅助层 vs 启动台正文层”的 P3-1 已在此前修复批次统一，现从待办中关闭；不是新发现。
- 新采暗色全量 atlas：`.local/validation/profiles/typography-review-dark-20261023/`，55 张同进程截图、55 份几何记录、13,784 条数据；标题 15/20/600（497）、正文 14/18/400（2,703）、辅助说明 12/16/400（4,161），均为 Segoe UI；4,810 个共享按钮高 32px，无字体数值变体、无按钮高度偏差。
- 暗色重点目检 `001-boot-window-3200x1800.png`、`100-main-single-folders.png`、`600-settings-appearance.png`、`702-panel-program-launcher.png`、`150-file-context-menu.png`。审查色在暗背景上的可辨识度足以分层；该配色仅为诊断色，当前检查不评价最终主题对比度。驱动器行、检查器字段、设置页和启动器侧栏未见新增的明显截断/遮挡。
- 暗色 atlas 有 4 个场景问题，环境项与亮色基线相同：两个未知插件设置页，以及缺少 VectorCraft CLI 的 WorkCrate 新建项目场景；MCP 子进程存活完成采集。这些场景不得记为已通过。
- 曾采集的 1000×680 数据低于产品支持尺寸，仅作为非验收压力记录，不据此判断布局通过或失败。按用户确认的最低支持尺寸 1920×1080，改用 `scripts/validate-ui-minimum-window.cjs` 通过同进程 MCP 检查主窗口、剪贴板工作台、启动台与设置外观页；profile 为 `.local/validation/profiles/ui-minimum-window-audit-20261023/`。四张截图共 1,449 条几何记录：标题/正文/辅助说明均为 15/14/12px，314 个共享按钮均高 32px，23 个导航组件均高 40px，字号变体与按钮高度偏差均为 0。截图和几何 JSON 位于 profile 的 `screenshots/`。本轮只覆盖这四个窗口状态；其他面板、交互状态和超长文案仍未完整覆盖。
