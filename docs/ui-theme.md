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
- 现有无 App 上下文的辅助构造器以及 MCP 工作线程，通过 `theme::roles` 读取框架
  ThemeColor 的只读投影。投影只由框架 Theme 更新通知生成，不含独立 palette、颜色
  配置或模式求色逻辑；不能把它扩展成另一个主题管理器。
- `theme.rs` 保留产品几何、字体尺寸和角色别名；`components` 管通用形态、`panels`
  管浮窗外壳。几何不随配色迁移重排，Settings/Session 路径保留。
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
2. 单行编辑用 `components::text_input(&state)`，多行用 `components::textarea(&state)`；
   搜索用 `components::search_input(&state)`，提供统一的搜索图标、凹槽、原生焦点环、
   清除操作和字号。这些构造器返回原生 Kit 控件，继续支持原有 builder 与 Entity。
   Kit 的 `Theme.colors.input` 表示边框，本项目将它设为 `SEPARATOR`；背景由公共
   构造器设为 `SURFACE_FAINT`。正文编辑保留 Kit Textarea 的原生外观/焦点，不用外层私画边框屏蔽。
   公共大操作用 `components::action(id, label, primary)`，图标操作用
   `components::icon_button(id, IconName)`，分区标题用 `section_heading`。
3. 容器用 `components::section()`；已有带 ID 的 Div 引入 `WorkbenchStyle` 后用
   `.workbench_section()`、`.workbench_card(selected)`、`.workbench_nav(selected)`。
   这些方法同时适用于普通 Div 与 Stateful Div，保留调用方 ID、事件和生命周期。
   selected 的 hover 必须保留选中色，不能在调用后用独立 hover 再覆盖它。
4. 悬浮面板使用既有 `PanelCatalog` 注册、`panels::mount` 承载；不要自行画另一套
   标题栏/关闭/拖动框架。新通用需求先扩展 UI Bridge，再由多个插件消费。

```rust
use fileatlas_ui_bridge::{components, components::WorkbenchStyle as _, theme};

let card = div().id("entry").workbench_card(selected)
    .p(px(theme::SPACE_3)).child(content);
let save = components::action("save", "保存", true).on_click(save_handler);
```

组件只负责视觉和公共控件状态，不读数据库、不启动后台任务、不注册私有命令；
业务回调仍进行新鲜的目标验证。禁止把同样的控件复制到每个插件再分别修改。

## 字体、间距与布局设置

全局 Segoe UI，正文/按钮 14、辅助 12、细注 11、分区标题 15、页标题 18。
使用 `FONT_SIZE_*`、`ROUND_*`、`SPACE_1..4`（4/8/12/16）、`PANEL_GAP`（10）。
卡片/区域圆角 8，控件 6；普通输入 32、大操作 38、图标按钮 30、导航行 40。
文件管理窗口文件名、元数据、高密度树允许辅助字号；主窗树行维持 30，避免破坏已有密度。
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
CI 已接入主题守卫。守卫扫描所有插件及嵌套 view、Shell、公共组件中的六/八位
十六进制颜色和局部 Theme 改写，主题目录集中管理；软件检查页的五个 Windows 注册表
声明是精确例外；同时阻止插件绕开公共输入构造器直接创建 Input/Textarea。
它是词法检查，不识别任意十进制颜色、别名、计算颜色，也不能证明
控件间距一致或截图正确；不能通过改写颜色表达式绕过规范。

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
