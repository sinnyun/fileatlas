# 工作台统一主题与后续开发规范

2026-10-19，依据用户明确要求，GPUI Kit 原生主题成为唯一颜色系统，Material 插件生成主题 JSON。
此前剪贴板 v2 的手写深蓝配色已退役。
本规范覆盖历史 v5 的颜色、通用控件样式；每页的功能结构、控件顺序、滚动边界仍按
[活动参考](reference-ui/README.md) 和用户最新决定。概念与编码前清单见
[统一主题](reference-ui/concepts/unified-workbench-theme-2026-10-18.md)。

## 唯一颜色来源：GPUI Kit 原生主题（2026-10-19 更新）

用户明确要求所有插件、自绘控件与框架控件使用同一主题系统。GPUI Kit 的
`ThemeRegistry` / `ThemeConfig` / `Theme` 是唯一配色解析与分发者，取代以前的
RGB 常量表、`mode_color` 和 `theme/components.rs` 逐项覆盖方式。

- `fileatlas-ui-bridge::theme::runtime` 只负责请求、注册原生 ThemeSet JSON、调用原生
  `Theme::update/apply_config` 和订阅框架主题变化。所有主题修改必须从此公共入口经过。
- 新自绘视图优先通过 `ActiveTheme` / `cx.theme()` 读取框架角色；按钮、输入、菜单
  优先使用 Kit 控件及其原生状态。不得按当前模式写两套颜色，也不得在插件写主题字段。
  Bridge 的基础组件若对应 GPUI Kit 已有控件，必须以 Kit 控件作为交互/绘制实现，Bridge
  只负责 FileAtlas 的语义尺寸、主题角色和审查记录；不要再实现一套等价的基础控件。
  仅在 Kit 没有匹配语义或产品行为时，才保留产品专用绘制，并记录其理由。
- 现有无 App 上下文的辅助构造器以及 MCP 工作线程，通过 `theme::roles` 读取框架
  ThemeColor 的只读投影。投影只由框架 Theme 更新通知生成，不含独立 palette、颜色
  配置或模式求色逻辑；不能把它扩展成另一个主题管理器。
- `theme.rs` 保留产品几何、字体尺寸和角色别名；`components` 管通用形态、`panels`
  管浮窗外壳。几何不随配色迁移重排，Settings/Session 路径保留。
- 插件只组合公共组件并提供业务内容、数据、状态和行为。按钮、卡片、分割线、面板、
  弹窗外框与遮罩必须使用 `fileatlas-ui-bridge::components` / `panels`；插件不得自行
  拼遮罩、画矩形边框或设置这些基础组件的圆角、背景、边框、阴影、hover 与字体样式。
  需要新语义时先扩展一个可复用的 Bridge 基础组件，再由插件传入内容/状态，不在插件
  重复组合底层 GPUI 样式方法。插件不得直接调用 `.border_*()`、`.bg()`、`.rounded*()`、
  `.text_color()`、`.shadow*()`、`.opacity()` 或 `.group_hover()`；表面、圆角、前景和
  交互状态通过 `SurfaceRole`、`CornerRole`、`ForegroundRole` 及对应共享组件表达。
  源代码治理由 `check-ui-theme.cjs` 和 `check-ui-geometry-coverage.cjs` 持续执行。
- 普通颜色角色直接来自当前 Theme：背景/foreground、popover/foreground、primary/
  primary_foreground、accent/accent_foreground、状态色及其 foreground。
  自绘实心控件必须消费实际背景对应的 foreground，不能用另一个按钮的前景色。
- 图片覆盖信息使用当前主题 popover 的不透明底与配对前景。用户标签色、照片像素和
  OS 图标是内容数据；标签使用主题表面/文字，用户色图标仅在实际表面上调整可读性。
  推荐标签色也来自当前主题，不在视图写固定色表。

## Material 只生成原生主题配置

`fileatlas-plugin-material-theme` 通过 Cordis 注册独立设置页与 MCP 工具；固定
`material-colors = 0.4.2`。输入种子色、风格、对比度，输出同时含亮/暗模式的原生
GPUI ThemeSet JSON。用户通过“生成并应用”或“应用已生成主题”激活。
生成器不得更新 GPUI Theme，不得在视图生成 palette，也不得把算法复制到各插件。

种子颜色使用 GPUI Kit 原生 `ColorSelect`，点击色块从其开源多色阶色板/滑杆选择，
不要求手输色值，不在插件维护固定色表。选色仅更新草稿，“生成并应用”才持久化。
色板本身的选中/焦点/弹层均交给 Kit；所选色是输入数据，不是控件私有主题。

默认使用 GPUI 内嵌 Default Light/Dark；Material 默认参数是低色度 Neutral。
生成文件正常路径为 `%LOCALAPPDATA%/FileAtlas/themes/material.json`，参数为同目录
版本化 `material-settings.json`；隔离 profile 使用其自身 themes 目录。
Settings 新增可选 `theme_file` 保存所选 JSON 路径，已有 `color_mode` 保留。
旧 `accent_name` 作为兼容元数据读取保留，不再驱动另一套颜色表。损坏/缺失主题回退
框架默认并报告日志，原文件保留；再次生成前保存 `.bak`。详见插件 PORTS。

## 防止回到手写配色

`check-ui-theme.cjs` 同时检查插件、Shell、自绘辅助模块和 UI Bridge theme 目录，
不再豁免整个 theme 目录。禁止十六进制 UI 色表、数值 rgb/hsl/hsla 绘制、旧大写
palette 常量、`mode_color`，并禁止桥接 runtime 以外修改 Theme。
零透明绘制、用户输入色及源媒体像素不是独立 palette。
所有新增主题都应提供 ThemeConfig/ThemeSet JSON 或由 Material 生成；角色扩展优先
复用框架 token。颜色生成算法、亮暗取色和控件状态计算不得重写在产品插件中。

## 通用组件的使用顺序

1. 常规按钮、菜单、Checkbox 使用已经应用主题的 GPUI Kit 控件，
   使用 `.primary()`、`.outline()`、`.danger()` 和 `.disabled()` 表达语义；不要在插件
   用 `.bg()` 固定按钮色，这会绕过 hover、pressed、disabled 和强调色切换。
   Button 的选中/悬停由原生 primary/selected/variant 管理，不要额外注册 `.hover()`；
   这个版本的 Kit 会自行注册 hover，重复设置会触发 GPUI 断言。自绘实心控件若确实需要不同底色，文字/图标前景必须与底色配对，优先使用对应原生 foreground；用户自定义内容色才使用 `contrasting_text(background)`（只从原生前景角色选择）；不能默认使用全局正文色。
   插件不得直接写 `.hover()` 状态；卡片、可选行、辅助入口、评分、分组标题、分隔拖拽条和标签胶囊分别使用 UI Bridge `WorkbenchStyle` 的共享交互方法。需要新交互态时先扩展 UI Bridge，再由插件选择对应语义。
2. 单行编辑用 `components::text_input(&state)`，多行用 `components::textarea(&state)`；
   搜索用 `components::search_input(&state)`，提供统一的搜索图标、凹槽、原生焦点环、
   清除操作和字号。这些构造器返回原生 Kit 控件，继续支持原有 builder 与 Entity。
   Kit 的 `Theme.colors.input` 表示边框，本项目将它设为 `SEPARATOR`；背景由公共
   构造器设为 `SURFACE_FAINT`。正文编辑保留 Kit Textarea 的原生外观/焦点，不用外层私画边框屏蔽。
   公共大操作用 `components::action(id, label, primary)`，图标操作用
   `components::icon_button(id, IconName)`，分区标题用 `section_heading`。
3. 按钮统一从 `components::button/action/icon_button` 创建；插件只选 `.primary()`、`.danger()`、`.outline()`、`.ghost()`、`.selected()` 等语义，不得直接创建 Kit Button 或设置按钮的背景/前景、阴影、字重、字号、宽高、内边距及圆角。共享按钮标准高 32px，最小宽 32px，宽度随图标/文案及共享内边距自然增长；设置中的全局布局密度统一缩放按钮、输入控件和导航行高度。全项目文本只允许 `WorkbenchTypography` 的 `workbench_title/body/assistant` 三种角色：标题 15px 半粗，正文 14px 常规，辅助说明 12px 常规。段落分别使用三档间距令牌；插件不得直接设置字号、字重或私有字体。内容放不下时扩大面板、调整布局或缩短文案，不缩小文字。该三层规范覆盖所有顶栏文字、导航、状态、小标签、徽标、表格与插件窗口；不得将顶栏或低对比度文字留在体系之外。
4. 容器用 `components::section()`；已有带 ID 的 Div 引入 `WorkbenchStyle` 后用
   `.workbench_section()`、`.workbench_card(selected)`、`.workbench_nav(selected)`。
   这些方法同时适用于普通 Div 与 Stateful Div，保留调用方 ID、事件和生命周期。
   selected 的 hover 必须保留选中色，不能在调用后用独立 hover 再覆盖它。
5. 悬浮面板使用既有 `PanelCatalog` 注册、`panels::mount` 承载；不要自行画另一套
   标题栏/关闭/拖动框架。新通用需求先扩展 UI Bridge，再由多个插件消费。

```rust
use fileatlas_ui_bridge::{components, components::WorkbenchStyle as _, theme};

let card = div().id("entry").workbench_card(selected)
    .p(px(theme::space_expanded())).child(content);
let save = components::action("save", "保存", true).on_click(save_handler);
```

组件只负责视觉和公共控件状态，不读数据库、不启动后台任务、不注册私有命令；
业务回调仍进行新鲜的目标验证。禁止把同样的控件复制到每个插件再分别修改。

## 字体、间距与布局设置

全局 Segoe UI。所有页面、面板、弹窗与插件统一使用三层字体规范：[统一文字三层概念板](reference-ui/concepts/typography-three-tiers-2026-10-23.png)，实现清单见[尺寸与间距清单](reference-ui/concepts/typography-three-tiers-2026-10-23.md)。唯一字号为标题 15、正文 14、辅助说明 12；注释、标签、计数、状态、按钮、表格和导航文字都归入这三层，不再保留更小字号或独立加粗层。标题半粗，正文和辅助说明常规。全局间距只使用紧凑、标准、宽松三档，基础值分别为 4、8、16px，由 `theme::space_compact/standard/expanded()` 提供；设置中的“全局布局密度”统一缩放间距、共享按钮、输入控件和导航行高度。颜色继续通过 GPUI Kit 原生主题和 Material 生成主题配置。插件不得内置间距/高度/调色值。面板放不下内容时调整区域尺寸、排版或文案，不缩字号。

### 临时文字层级审查色的恢复步骤

文字审查色是用户要求的临时标记，必须保留到用户明确说“可以结束颜色审查”为止。收到该指示后，先在“设置 → 外观 → 字体层级审查”关闭“显示字体层级审查色”；Settings 会持久化 `show_typography_audit_colors=false`，UI Bridge 会重新应用当前 GPUI Kit 原生主题，标题/正文/辅助说明及按钮、链接、菜单前景恢复主题配色。这个操作不关闭组件审查框，也不改字体层级、字号或间距。

代码清理只在用户明确要求移除临时审查能力时进行：再删除 `theme::roles` 的临时角色映射、`theme::runtime::apply_typography_audit_theme()`、对应设置字段与 UI，并恢复普通 `Theme::apply_config()` 路径。随后检查 `WorkbenchTypography` 和全项目 `.text_color(...)` 调用，保留语义图标、状态色及数据颜色；运行主题守卫并通过同进程 MCP 检查亮/暗色、普通/选中/悬停/焦点/禁用态。不能只因为界面颜色已恢复就删除审查开关或几何审查能力。

字体审查色由独立设置控制，标题红、正文绿、辅助说明蓝，仅用于辨认层级，不代表正式产品色或推荐对比度。组件审查框由另一个独立设置控制。

### 几何与组件覆盖审查色

几何审查只在 UI Bridge 基础组件层标记：浮动面板外框红、通用 section 橙、card 黄、可交互行绿、共享按钮蓝、共享单行/多行输入框紫、标题/正文/辅助说明占用框青。颜色定义集中在 `theme.rs` 的 `AUDIT_FRAME_*`；由 `components.rs` 的共享构造器和 `panels.rs` 的浮窗框架消费。`review_section/review_row/review_card` 只负责审查边界和几何记录，不替代产品样式；section 标题、侧栏分组、设置行、驱动器/快速访问/收藏夹、文件分组、文件/目录卡片及侧栏底部控制组应使用对应共享构造器或审查包装器。插件不应自行加审查色或私有公共组件样式。设置页的文本、路径和选择控件由 UI Bridge 声明式设置页统一绘制；选择器触发器复用共享按钮，所有插件的编辑控件必须从 `components::text_input/search_input/textarea` 进入，禁止直接创建 GPUI Kit `Input` / `Textarea`。

文本交互控件复核：对全仓 Rust 调用点核对后，单行输入、搜索、地址栏、表单文本均通过共享 `text_input/search_input`；多行编辑通过 `textarea`。复核发现 `textarea` 原先没有几何审查框，已补上紫色基础组件标记。字段固定宽度改用主题 `CONTROL_WIDTH_*` 档位；其余由父级 `w_full/flex_1` 排版。设置页下拉选择用共享按钮构造器。文本字段/下拉/路径/复选切换均应覆盖正常、焦点、禁用与长值状态；此轮仍在逐屏检查，不能据此宣告输入交互审查完成。

审查截图中若有界面框架、卡片、行或按钮没有显示对应颜色，记录其界面/区域/组件类型为“基础组件覆盖缺口”，并迁移为 UI Bridge 已有或新建的共享基础组件，然后重新截图。不得先给该插件框架单独加边框、间距或尺寸来让截图看起来统一；那会绕过覆盖检查。遇到布局空间不足，仍按现有文字规范调整布局/文案。

结束几何审查后，将 `theme::GEOMETRY_AUDIT` 置为 `false` 或删除对应审查分支及 `AUDIT_FRAME_*` 常量；恢复按钮、输入框和容器原生主题边框。颜色审查与几何审查分别验收，文字临时红绿蓝色仍须等用户明确批准后才按上文恢复。
使用 `FONT_SIZE_*`、`ROUND_*` 与 `space_compact/standard/expanded()` 三档间距（基础值 4/8/16，由全局布局密度统一缩放）。禁止恢复 `SPACE_1..4` 或按区域另定义 gap 令牌。区域卡圆角 8、控件 6；卡片/浮层阴影走共享
`theme::card_shadow/control_shadow/region_shadow/floating_shadow` token
（颜色解析原生 overlay 角色），窗口画布用 `theme::surface_canvas`（原生
muted 角色）。普通输入 32、大操作 38、图标按钮 30、导航行 40；其他常用控件高度/宽度使用 `CONTROL_HEIGHT_*` / `CONTROL_WIDTH_*`，避免插件重复定义相同职责的尺寸。
所有可见文字统一使用上述三种字号；主窗树行维持既有行高 30，避免破坏信息密度。文字不随空间缩小，必要时省略文案或滚动。
图标使用主题的 Lucide helper（12/16/20），禁止用空格对齐计数或用字形冒充通用图标。

相同布局角色用相同规则，业务布局按用途保留：

- 主窗：横向工具/筛选带，导航树、媒体区、检查器和固定状态带；两侧宽度保留既有
  Session 恢复路径，不添加第二套宽度配置。
  驱动器、快速访问和每个收藏分组必须使用 `components::sidebar_section` /
  `sidebar_header` 的相同边框、字号、内边距和按钮位置；收藏分组是平级区域，
  不再包在“文件夹收藏/库文件夹/收藏集合/收藏分组”复合框内。整列正文在小窗滚动。
  每组＋只收藏已导航的当前目录，不能改回文件选择器；没有目录上下文时禁用。
- 工具插件：默认在通用浮窗中；分类栏固定、主区 `flex_1().min_w_0().min_h_0()`，
  需要编辑详情时才加右栏。剪贴板保留三栏，启动台保留大图标网格。
- 设置：导航在左；命令/范围/快捷键/操作在同一行；正文可滚动，页脚固定。
- 卡片内容内边距 12–16，分区间距 10；横向标签和对应操作必须保持行内关系。
  不要用全局主题改动擅自重排业务界面，也不要强制全部功能采用同一种栏数。
- 设置持久化继续走 SettingsService，几何恢复继续走既有 Session/面板设施；新增
  布局偏好先检查公共模型，不能另建私有全局设置系统。主题切换后刷新所有消费视图。

小窗先允许正文滚动，给关键操作保留空间；长名称截断并提供原有完整信息入口，
编辑正文允许换行。布局最小尺寸、默认尺寸和固定/滚动区域需在页面清单写明。
主题规范不等于固定像素截图，每次调整都要检查真实 DPI、字体与可用窗口。

## 开发与审查必须执行

主窗的导航/搜索操作图标化，图标按钮需保留本地化 tooltip 与 accessibility label。
网格/列表/子文件夹和缩放统一放在 FileManager 筛选栏右端，左侧筛选区允许横向滚动；
不能重新把视图控件塞回地址栏旁。状态栏仅显示状态反馈，不再放标签/更多入口。

改界面前读本规范、活动参考、对应图片和配套清单。新 UI 先选择可复用公共组件；
遇到公共缺口先补共享设施。大范围视觉调整仍按 AGENTS.md 先生成、检查概念；
局部修正不重新生成一个接近现状的图来降低验收标准。

提交前运行：`node scripts/check-ui-theme.cjs --self-test`、
`node scripts/check-ui-theme.cjs`、refresh/architecture/layering/feature-ledger 守卫，
以及显式 Rust 1.97.1 的 fmt、locked Shell check/build 和受影响测试。
CI 已接入主题守卫。它扫描插件、Shell 和公共 UI 源码中的局部颜色/Theme 改写、直接
GPUI Kit 控件构造、插件 `.style(...)`/Style 对象、常见背景/前景/圆角/边框/阴影/悬停
样式，以及插件直接 Canvas/paint API；已知标准控件重命名后的常见 `use ... as` 和
`type Alias = Button` 形式也有负向探针。软件检查页的五个 Windows 注册表声明是精确例外。
直接 GPUI Canvas 在插件中只允许现存七个逐文件计数冻结的历史入口，增加调用会失败；
新绘制能力应先进入 UI Bridge。`check-ui-geometry-coverage.cjs` 另核对共享审查组件和
关键界面入口，并禁止插件直接使用 modal scrim helper。

这些守卫是词法检查，不是 Rust AST：宏重写、非标准 re-export/类型间接层、手写布局型
自定义组件，以及任意计算颜色仍不能仅靠正则完全识别。常见直接违规有 self-test 探针；
守卫通过不等于所有 UI 构造均已证明为共享组件，也不能证明运行时状态、间距或截图正确，
仍需按活动设计和同进程 MCP 实测。不能通过改写颜色表达式绕过规范。

字体与间距另由 `scripts/check-ui-design-tokens.cjs` 守护，并在 CI 与本地主题检查一同运行。
守卫锁定标题/正文/辅助说明字号与行高、紧凑/标准/宽松间距的 4/8/16px 基值及密度缩放，
禁止插件、Shell 和非共享组件自行覆写字体，禁止继续使用 GPUI Kit 的 `.gap_2()`、`.p_3()`
等独立数值快捷间距。布局间距必须走 `theme::space_compact/standard/expanded()`；
无间距的 `.gap_0()` 可保留。守卫也会解析 `.gap(px(...))` 等常见间距调用，拦截变量、算术和条件表达式，
防止用动态表达式绕开三档令牌。当前被逐项冻结的几何遗留例外位于 FileManager 的月份行/树缩进/筛选缩进、
Navigation 的标签树缩进、Settings 内容区内边距及 UI Bridge 标签胶囊内部参数；其路径、表达式和数量均固定，
新增或改写都会失败，不能作为新插件范例。该守卫不限制业务画布、图像、窗口等语义尺寸，也不能替代实画布局验收。

共享基础组件的审查覆盖由 `scripts/check-ui-geometry-coverage.cjs` 守护，并在 CI 检查。它确认已知的 section/row/card 构造器和侧栏分组、设置行、文件分组及文件/目录卡片边界持续接入审查框；新增界面仍须在 3000×2000 全面板 atlas 中检查未标记区域。

组件边界审查框由“设置 → 外观 → 界面审查 → 显示组件审查框”控制，字体层级颜色由“设置 → 外观 → 字体层级审查 → 显示字体层级审查色”独立控制；两项设置持久化在现有 Settings 文件中，当前审查阶段默认开启，关闭各自不会影响另一项。共享基础组件、浮动面板和输入控件的审查描边由 UI Bridge 在 GPUI Canvas paint 阶段通过 `Window::paint_quad` 绘制，不参与布局测量；几何快照直接记录组件边界，不再加上边框补偿。文本输入和多行输入由 UI Bridge 包在外层基础框架中，原生控件继续处理输入与焦点，审查框独立覆盖绘制。GPUI Kit 的 [Inspector feature](https://github.com/longbridge/gpui-kit/blob/main/README.md)（debug/inspector feature，Windows 快捷键 Ctrl+Shift+I）用于逐个点选和检查元素，不是全组件边界覆盖层，因此不能替代本审查模式。GPUI 的 [Canvas paint callback](https://docs.rs/gpui/0.2.2/gpui/fn.canvas.html) 和 [`Window::paint_quad`](https://docs.rs/gpui/0.2.2/gpui/struct.Window.html#method.paint_quad) 是当前采用的原生绘制接口，不需要额外引入审查库。Android 系统的 [Show Layout Bounds](https://developer.android.com/studio/debug/dev-options#drawing) 是相近的开发者诊断模式，但属于 Android 平台，不是 Windows/GPUI 可直接复用的库。

运行验收始终使用同进程 FileAtlas MCP，隔离 dev-test profile。至少查看主窗、目标插件、
菜单/下拉和与其共享组件的另一个插件：常态、hover、焦点、选中、禁用、空态/错误、
长内容、小窗。强调色切换必须检查至少两个插件和菜单，并恢复测试设置；颜色模式切换需检查
亮/暗两态、至少两个插件与设置托槽，并恢复隔离测试 profile 的设置。
检查 quiet-state 和真实变化，禁止引入额外 IO/轮询通知。截图片必须打开对照，
将命令、环境、功能结果、视觉差异及未验状态记入 `docs/redesign-validation.md`。
编译成功不代表视觉或全部 Windows 行为通过。

## 声明式设置页（2026-10-18）

用户要求后续插件使用通用配置自动生成设置页。常规设置必须使用 UI Bridge
`settings_page` 的 Page/Section/Row/Control 声明及现有 view catalog 注册，不再各自
设计设置外框、分区、字号和布局。复杂业务交互可保留局部 Entity/Custom，仍复用公共
标题、分区、行和原生控件。详见[公共契约](settings-page-contract.md)及
[十页概念](reference-ui/concepts/settings-complete-2026-10-18.md)。

## 历史配色记录

旧手写语义色与缩略图对比修正记录仍保留作为历史证据。其固定 RGB、手工混色和
组件覆盖实现已由本页原生主题契约取代；布局与可读性要求继续有效。
