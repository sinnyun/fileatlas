# 工作台统一主题与后续开发规范

2026-10-18，依据用户明确要求，将剪贴板 v2 的深蓝样式提升为全局主题。
本规范覆盖历史 v5 的颜色、通用控件样式；每页的功能结构、控件顺序、滚动边界仍按
[活动参考](reference-ui/README.md) 和用户最新决定。概念与编码前清单见
[统一主题](reference-ui/concepts/unified-workbench-theme-2026-10-18.md)。

## 唯一所有者

`photag-ui-bridge::theme` 是颜色、字体、尺寸、状态及 GPUI Kit Theme 的唯一来源。
`theme/components.rs` 将 token 同步到 Kit 的按钮、输入、菜单、列表、开关、进度、
滑块、焦点环和弹层；插件不得自行调用 `Theme::change/update` 或创建一套私有 palette。
`components` 负责通用容器和控件形态，`panels` 负责浮窗外壳、标题栏、拖动和调整大小。
它们都复用既有桥接设施，不新增插件管理器；业务动作、状态和 Cordis 清理由插件所有者负责。

| 语义 | 令牌/默认值 | 用途 |
|---|---|---|
| 根背景 | `SURFACE_BASE` / #0e1823 | 主窗、浮窗底 |
| 面板 | `SURFACE_PANEL` / #152230 | 分区、设置、菜单 |
| 卡片 | `SURFACE_CARD` / #1b2a3b | 内容块、次按钮 |
| 悬停 | `SURFACE_ELEVATED` / #24374b | 非选中卡、按钮 hover |
| 输入凹槽 | `SURFACE_FAINT` / #101b27 | 搜索、正文输入、占位 |
| 分隔与边框 | `SEPARATOR` (RGB)、`SEPARATOR_LIGHT` (RGBA) / #304357 | 禁止混用 RGB/RGBA |
| 正文 / 次要 / 弱化 | `TEXT_PRIMARY` / `TEXT_SECONDARY` / `TEXT_MUTED` | 白蓝正文与灰蓝辅助信息 |
| 强调 / 选中 | `accent()` / `selection_color()` | 默认 #2787ff / #163b63，随设置同步 |
| 状态 | `DANGER` / `WARNING` / `SUCCESS` / `STAR` | 错误、警告、成功、收藏/评分 |
| 语义装饰 | `BRANCH_ALTERNATE` / `CATEGORY` / `DATA` | 分支、标签类别、数据信息 |

新安装/缺省设置使用“工作台蓝”。已有 JSON 中显式保存的 System 和旧预设仍兼容，
外观页可以选择工作台蓝或其他强调色；所有插件必须随同变化。不得为了统一样式重写
用户配置文件。用户标签颜色、照片像素、操作系统程序图标是业务内容，保留原色。
新增语义颜色必须先放入 theme 并写明用途，不能为单页随意添加近似颜色。

## 通用组件的使用顺序

1. 常规按钮、菜单、Checkbox 使用已经应用主题的 GPUI Kit 控件，
   使用 `.primary()`、`.outline()`、`.danger()` 和 `.disabled()` 表达语义；不要在插件
   用 `.bg()` 固定按钮色，这会绕过 hover、pressed、disabled 和强调色切换。
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
use photag_ui_bridge::{components, components::WorkbenchStyle as _, theme};

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
图库文件名、元数据、高密度树允许辅助字号；主窗树行维持 30，避免破坏已有密度。
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
网格/列表/子文件夹和缩放统一放在 Gallery 筛选栏右端，左侧筛选区允许横向滚动；
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

运行验收始终使用同进程 PhoTag MCP，隔离 dev-test profile。至少查看主窗、目标插件、
菜单/下拉和与其共享组件的另一个插件：常态、hover、焦点、选中、禁用、空态/错误、
长内容、小窗。强调色切换必须检查至少两个插件和菜单，并恢复测试设置。
检查 quiet-state 和真实变化，禁止引入额外 IO/轮询通知。截图片必须打开对照，
将命令、环境、功能结果、视觉差异及未验状态记入 `docs/redesign-validation.md`。
编译成功不代表视觉或全部 Windows 行为通过。
