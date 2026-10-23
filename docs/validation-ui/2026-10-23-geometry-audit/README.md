# 2026-10-23 几何与交互组件覆盖审查

## 当前状态

审查进行中，审查色保持开启。此轮将“文本交互框架”纳入范围：单行文本、搜索、地址栏、多行编辑、设置页路径/选择，以及它们所在的表单行和面板。任何共享组件外观标记缺失都按基础组件覆盖缺口处理，迁移到 UI Bridge 后再复查；不在插件内临时补色。

2026-10-23 字体层级专项审查结果见 [全局 UI 字体层级审查报告](../../ui-consistency-review-findings-2026-10-23.md)：13,069 条几何记录三层零数值偏差；6 项确定问题（导航项层级不一致、统计值/设置状态/使用提示层级误分类、一处守卫规避、启动台分类名换行溢出）与 4 项标记覆盖缺口待批量修复。

## 共享标记

- 浮动面板外框红；通用区域橙；卡片黄；导航行绿；共享按钮蓝；共享文本输入紫。
- 三层共享文字包装器（标题/正文/辅助说明）的实际占用盒另用青色框出；此轮补充回应了侧栏状态、说明行、记录计数/保存状态等仍无法逐项核对的问题。
- 按钮高度 32，最小宽度 32，文字决定自然宽度；插件按钮调用处已清理固定宽/高、压缩变体及重复 padding 覆盖。
- `components::text_input` / `search_input` 标记单行编辑与搜索框；`components::textarea` 标记多行编辑；设置页 Select/Path 触发器使用共享按钮。
- 所有文本框高度从共享构造器/主题取值（单行 32，多行可随内容布局；WorkCrate 画布文本框 120 高度也取主题常量）。文本框周围的字段/操作间距统一为 `FORM_CONTROL_GAP=8`，以便对照同一尺度。
- 文字审查色红/绿/蓝与上述几何色是两套独立审查。未获用户结束指示前不能恢复文字颜色。

## 源码覆盖核对

代码图谱命中 UI Bridge 的 `text_input`、`search_input`、`textarea` 共享入口。对 `crates/**/*.rs` 检索 GPUI Kit `Input::new` / `Textarea::new` 以及所有共享输入调用后，没有发现插件绕开公共输入构造器的直接创建。搜索/表单文本输入的调用点全部使用共享入口。此次发现并修复多行 `textarea` 尚无紫色边框的漏标。

逐屏间距比较还发现 Clipboard 新建分组输入曾覆盖共享控件高度并关闭原生外观/边框，Clipboard 正文编辑器也覆盖行高和文字色；这些覆盖已移除，确保字段始终有审查框并继承统一文字层级。Navigation 标签对话框、收藏夹添加、FileManager 地址/搜索/分支操作、Launcher 名称编辑、WorkCrate 文本编辑等字段周围原有 3/4/6/9/12 px 混用，已归并到主题 `FORM_CONTROL_GAP=8`。表单行与相邻操作仍采用既有业务顺序。

固定宽度文本字段已改用主题宽度档：常规字段 200、宽字段 240、特宽字段 320；之前 180、220、240、320 的插件内散值已统一到这些档位。仍有字段通过父布局采用 `w_full` / `flex_1`，需截图确认父框架在窄窗口和长值下是否适配良好，而不在组件上增加局部主题。

本轮检查字体规范时发现设置中心“程序快捷键”页在正文组件后覆写 `line_height=20`（正文 token 为 18），软件检查页容器又覆写相对行高，均可能让继承的同层文本脱离三层规范。两处覆盖已删除；主题守卫现同时禁止插件自行指定字号和行高，并增加了行高反例探针。源码中剩余 `line_height` 仅在 UI Bridge 共享三层文字/输入和标签胶囊组件中使用主题 token。

## 本轮验证

- 活动界面参考：`docs/reference-ui/README.md` 指向的 2026-10-19 多栏工作区概念，图片已打开；本次为组件边界审查，不改变业务布局。
- 构建：`cargo +1.97.1 check -p fileatlas-ui-bridge --locked -j2` 通过；`cargo +1.97.1 build -p fileatlas-shell --locked -j2` 通过。链接器仅有既存 `LNK4098` 警告。
- 同进程 MCP：Rust 1.97.1 MSVC，profile `.local/validation/profiles/ui-geometry-textfields-20261023/`，`settings.json` 设置为 `color_mode=light`，命令 `$env:PANEL_ATLAS_PHASE='panels'; node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/ui-geometry-textfields-20261023 --plugins dev-test --access-level standard --scenario scripts/capture-panel-atlas.cjs`。剪贴板面板截图已打开检查：搜索框、地址输入和多行文本编辑框紫框清晰，分类下拉由蓝框共享按钮承载；外围面板/分区/卡片/导航也分别显示对应标记。图片在 `.local/validation/profiles/ui-geometry-textfields-20261023/screenshots/700-panel-clipboard-workbench.png`。
- 之前全量组件边框截图位于 `.local/validation/profiles/ui-geometry-atlas-20261023/screenshots/`；包括 100–162 主窗/筛选/检视器/多栏、600–607 设置页、700+ 浮窗。该批确认主框架和按钮、单行输入审查色可见。截图尺寸 3200×1800。
- 构建：`cargo +1.97.1 build -p fileatlas-shell --locked -j2`、`cargo +1.97.1 fmt --all -- --check` 通过；theme self-test、refresh、architecture、layering、feature-ledger 均通过。完整主题守卫报告 12 个临时审查色色值（四层文字、一个运行时文字覆盖、七个几何边框色，其中新增一色用于文字占用框），这是当前保留审查色的有意结果；其自测通过。
- 面板 atlas 未终止进程；唯一功能场景错误为 WorkCrate 环境缺少 `vectorcraft-cli.exe`，不影响剪贴板字段截图。启动早期截图请求曾返回“窗口尚未就绪”，重试与 3200×1800 截图成功。
- 本节只记录已实际复核的源码/截图；焦点、禁用、长值、小窗及所有插件内部表单仍需逐状态逐屏检查，不能视作完整验收。

## 结构化几何审查器（第一阶段已接入）

仅靠彩色截图仍需要人工找元素。GPUI 当前可访问 `LayoutId` 与 `Window::layout_bounds()`，但没有供运行时 MCP 查询的完整布局树；`Div::debug_selector` 只在 `test-support` 下写入测试上下文，常规构建是 no-op。因此不能把现有 `get_ui_state` 当作控件几何报告。

第一阶段已经由 UI Bridge 在 GPUI prepaint 阶段采集共享标题/正文/辅助说明、通用按钮、region/section/tree-row 的实际边界，并附带采样源文件和行号；文字记录含当前解析到的字号、行高、字重和字体族。只读 MCP 工具 `fileatlas.debug / debug.ui_geometry` 可按窗口与类型查询。既有 `scripts/capture-panel-atlas.cjs` 在每张面板截图后自动落盘同一时刻的 JSON 快照，避免另外配置逐屏截图。该方案复用既有 Debug 插件，不另建平行插件/管理器。

目前每条记录含 `{window, tier, component_id, source_file, source_line, x, y, width, height, font_size, line_height, font_weight, font_family}`。在 profile `.local/validation/profiles/ui-geometry-textfields-20261023/` 的亮色主窗和面板快照中，最新四阶段重采共 55 个几何快照、13,724 条记录：title 748、body 2,879、assistant 3,256、button 5,085、region 206、section 84、card 610、navigation 46、tree-row 810。title 全部为 15/20px、600；body 全部为 14/18px、400；assistant 有 3,254 条为 12/16px、400，另有 2 条为 14/16px，均来自 Navigation 标签名按钮（`tag_render.rs:27`）。这两条仍待核实；Canvas 观察点可能读取 Button 子树中的窗口文字样式，不能仅据此判定用户可见字号错误。按钮边界检查另发现两个标签名按钮为 34.67px，而共享高度 token 为 32px；需追查它们所在的可拉伸标签行，不能将差异忽略。

覆盖缺口：目前 GPUI Kit `Input`/`Textarea` 仍有紫色审查框和共享字号/行高/高度 token，但未作为独立 geometry 记录返回；快照尚无 parent_id、显式 gap/padding token、控件交互状态或完整布局树，所以现阶段可比较组件盒和字体数值，不能自动推导所有同级间距/父内边距、裁切/遮挡，也不能声称全量控件已经覆盖。坐标近邻推算会把不同列或嵌套框架中的按钮误配，汇总器已删除这类间距/父内边距推断，只保留真实观测边界和 token 偏差。Canvas 观察钩子不参与 flex/grid 排版，运行截图仍须确认边框无布局偏移。

全量源码定位现由 `scripts/audit-ui-geometry-source.cjs` 执行，当前扫描 331 个 Rust 文件，记录 176 个插件直接数值 gap（15 种值：1、2、3、4、5、6、7、8、9、10、12、14、16、18、24），以及其他 210 个 padding/margin 数值位置；另有 211 个直接数值宽高/size 调用。唯一直接 `Button::new` 位于 UI Bridge 公共构造器，插件入口已统一。主题守卫未发现插件字号、行高或字重覆盖（完整守卫输出仅有故意保留的审查色）。详细逐行清单落于隔离 profile 的 `source-geometry-audit.txt` 与 `direct-gap-sites.txt`，不纳入版本库。

上述局部 spacing 和尺寸值尚未全部按组件语义归并到主题角色，不能把不同布局所需数值盲目替换成同一个值。共享按钮 token 为高度 32px、最小宽度 32px；当前截图中两个导航标签名按钮超高，已尝试紧凑 padding，并在 UI Bridge 统一按钮几何上加入最大高度约束。该调整完成了构建，但此最新改动尚未经过 MCP 重采；按用户最新要求，先完成全量定位与批量修复后再集中运行一次最终验证。Navigation 树行有独立 40px 行高 token，属于导航行而非按钮尺寸。

本轮复用 `scripts/capture-panel-atlas.cjs` 重采 browser/settings/modals/panels 四阶段，profile 为 `.local/validation/profiles/ui-geometry-textfields-20261023/`，亮色模式，尺寸 3200×1800；后又重采 browser phase，当前共 55 个截图与 JSON 快照、13,722 条记录。最新 browser 截图来自仅加 `.compact()` 后的程序；共享 `.max_h(32)` 代码随后已构建但未截图验证。WorkCrate 工具缺失、旧设置键和测试数据问题未作为视觉通过证据。按钮高度偏差和字体观察点偏差待最终批次复核。所有审查色继续保留，等待用户明确说可以结束后再恢复设计色彩。

## 后续审查队列

先检查本次面板阶段的截图和 textarea 紫框；随后按截图索引逐个检查插件面板、设置页、弹窗和多栏布局中的输入、选择、复选/单选框架及其周围行/分区。重点记录缺少基础组件边框、字段宽度分歧、局部 padding/尺寸、边框嵌套和拥挤/裁切。每轮修正后通过同进程 MCP 重截并更新此记录。结束几何颜色审查须待全量框架覆盖复核完毕；文字审查色则还要等用户明确批准后单独恢复，步骤见 `docs/ui-theme.md`。


## 2026-10-23 后续批次：三档间距与全局密度设置

按用户指定将此前发现的所有插件数值 gap、padding、margin 批量归并为紧凑/标准/宽松三档，基础值分别为 4/8/16px。386 处显式布局间距现通过 UI Bridge 的 `space_compact()`、`space_standard()`、`space_expanded()` 获取，并由全局密度统一缩放；另将树节点缩进的基础项和层级步进改为共享紧凑/宽松令牌。主题守卫现禁止插件和 Shell 继续写数值 gap/padding/margin 或使用退役的语义间距别名。`scripts/normalize-ui-gap-tokens.cjs` 是可重复运行的三档迁移器。

Settings 外观新增“全局布局密度”（紧凑/标准/宽松），默认标准并向后兼容旧 JSON；它缩放全局三档间距、共享按钮、输入控件与导航行高度。颜色手动调整继续使用已有 GPUI Kit 主题来源和 Material 主题设置页。字体层级和审查颜色没有清除。

本批集中验证：

- `cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check -p fileatlas-ui-bridge -p fileatlas-plugin-settings -p fileatlas-plugin-navigation -p fileatlas-plugin-file-manager -p fileatlas-plugin-project-tree -p fileatlas-shell --locked -j2` 与 `cargo +1.97.1 build -p fileatlas-shell --locked -j2` 通过。链接时只有既存 `LNK4098` 警告。
- `node scripts/check-ui-theme.cjs --self-test`、`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs` 均通过。
- 使用 `scripts/validate-ui-density.cjs` 经同进程 MCP 操作密度段控，观察 74 个共享按钮中的首个按钮高度：标准 32px、紧凑 28.67px、宽松 35.33px；隔离 profile 的 `settings.json` 保存 `ui_density=comfortable`。快照在 `.local/validation/profiles/ui-density-control-final-pass-20261023/`。
- 全量 atlas 命令为 `$env:PANEL_ATLAS_PHASE='all'; node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/ui-spacing-density-light-20261023 --plugins dev-test --access-level full --scenario scripts/capture-panel-atlas.cjs`。同进程存活，55 张截图和 55 份几何 JSON 共 13,069 条元素记录；亮色设置页截图已实际打开检查，三档控件与文字/框架审查色可见。隔离 profile 在 `.local/validation/profiles/ui-spacing-density-light-20261023/`。
- atlas 中 4 项问题仍是环境/旧插件页问题：未知的 `fileatlas.i18n` / `i18n` 设置页，以及缺少 `vectorcraft-cli.exe` 时的 WorkCrate 创建场景；不能据此声称这些流程通过。

结构化审查源码清单已更新：331 个 Rust 文件中显式数值 gap/padding/margin 已归零；剩余的是 211 处直接 width/height/size 几何值，仍需区分基础组件尺寸、内容/画布尺寸和窗口约束，后续批次继续统一。当前布局间距层级和密度功能已落地，但所有框架尺寸/组件覆盖及窄窗口状态仍未完成；审查色必须保留到用户明确验收。
