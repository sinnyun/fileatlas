# PhoTag 界面 v5 改造实施文档

> **自动续接：** 用户只说“继续”时，根据交接文档台账推进下一未完成阶段：先完成 C0–C4 编码，再执行 V1–V4 验证。
> 编码阶段不运行格式/编译/测试/应用/MCP；验证阶段按清单执行，运行时调试和截图必须使用 PhoTag 插件 MCP。
> 显式指定任务范围时遵照用户指定。详见完整开发交接。
>
> **2026-10-14 交接更新：** 当前状态与小步执行顺序以
> [完整开发交接](development-handoff-2026-10-14.md) 为入口。本文件的设计规格仍有效，
> 但早期工作树/11错/“未运行界面”记录均为历史快照。现在已有插件 MCP 实机截图，
> 最新源码由于04/06批量/标签面板中间态有19个Gallery编译错误，最后成功EXE落后于源码。

2026-10-13 编写。本文档保存界面重设阶段的设计和实施细节。默认根据台账自动推进下一
未完成阶段；需要了解相应设计与阶段背景时查阅本文件相关章节，并同步更新
第 10 节的进度台账。

设计输入是 `docs/reference-ui/final-v5/` 的 9 张设计稿和
`docs/reference-ui/concepts/photag-v5-companion-interfaces-2026-10-13.md`
（逐稿说明）+ `docs/reference-ui/concepts/photag-reference-layout-v5-2026-10-13.md`
（区域映射）。设计稿是视觉方向，不是像素规范；不要用生成图里的文字瑕疵、
示例版本号、示例数据当实现目标。

---

## 1. 背景与目标

- 项目处于界面重新设计阶段：界面以实现 v5 设计稿为目标，不再对照原版 egui 界面。
- 后端行为、数据语义仍以 `vendor/photag-source/` 为参考；界面代码只改外观、布局、
  文案与控件形态，不改数据库语义与持久化文件格式。
- 组件库是 [gpui-kit](https://github.com/longbridge/gpui-kit)（0.7.1 facade，含
  gpui-component 0.7.2 + gpui-base 0.7.2）。新界面优先使用其原生组件
  （Button/Input/Checkbox/Radio/Popover/DropdownMenu/Icon/Progress/Slider 等）。
- 现状：应用是单窗口 + 槽位注册的覆盖层体系（见第 5 节）。预览、设置、播放、
  标签弹窗都不是独立窗口，而是 `workspace.modal` 等槽位里的覆盖层。

## 2. 硬性约束（违反即失败）

1. **界面中文**。所有面向用户的文案必须经 `photag_ui_bridge::i18n::tr("English")`
   输出，并在 `keys.json` + `en.ftl` + `zh-CN.ftl` 中登记；中文界面来自 zh-CN 词条。
   不得把槽位 id、存储 key、调试信息渲染进界面（可以在 tooltip/悬停提示）。
2. **不造假**。没有数据的控件不做假数据：没有真实波形就不画波形（用中性占位）；
   没有真实版本号就不抄设计稿的“v1.3.0（示例版本）”；没有的重试 API 就先补后端
   方法再画按钮。禁止 TODO、空 handler、占位行为冒充满实现。
3. **不动数据层**。不改 SQLite schema、不改 `%LOCALAPPDATA%\PhoTag` 下文件格式。
4. **文件长度**：Rust 文件尽量 <500 行，最多 800 行。大视图拆到 `src/view/` 子模块。
5. **Cordis 所有权**：视图贡献随插件启动注册、随插件停止移除（用
   `register_views_for_plugin`）；不要另建并行注册机制。
6. **构建工具链必须显式选择**（见第 3 节）：默认 stable 1.94.1 低于 MSRV 1.95，
   不加 `+1.97.1` 的 cargo 命令会直接失败，且这不是“项目坏了”。
7. 每次界面改动的验证记录追加到 `docs/redesign-validation.md`（格式见该文件头部）。
   编译通过/无头测试 ≠ 界面完成。
8. 每个 crate 改动公共签名时同步更新该 crate 的 `PORTS.md`。

## 3. 环境、命令与启动方式

本机已装 1.96.0 与 1.97.1。**所有 cargo 命令走 `+1.97.1`**：

```bash
# 格式检查（必须通过）
cargo +1.97.1 fmt --all -- --check
# 语法/类型检查（界面改动的底线验证）
cargo +1.97.1 check --locked -p photag-shell -j 2
# 相关 crate 测试
cargo +1.97.1 test --locked -p photag-ui-bridge --lib -j 2
cargo +1.97.1 test --locked -p photag-plugin-i18n -j 2
cargo +1.97.1 test --locked -p photag-plugin-gallery -j 2      # 按改动范围替换 crate 名
# 启动窗口（手动操作验证）
cargo +1.97.1 run --locked -p photag-shell -j 2
```

启动配置（环境变量）：

- `PHOTAG_DEV_PLUGINS=core`（默认）或 `all` 或逗号列表：`gallery,navigation,settings,...`
- `PHOTAG_DEV_TEST_DIR=<dir>`：隔离配置目录（dev-test 时用，别动真实 `%LOCALAPPDATA%\PhoTag`）
- `--mcp`：启动 MCP 服务器（与窗口同进程，配合 MCP 客户端读窗口状态）
- 真实数据目录：`%LOCALAPPDATA%\PhoTag`（settings.json/session.json/language.json/photag.db）

## 4. 设计令牌（已落地在 `crates/photag-ui-bridge/src/theme.rs`）

以下值已写入代码，直接使用，不要另造常量、不要在视图里写死颜色。

### 4.1 表面与文字色（`rgb(...)` 用）

| 令牌 | 值 | 用途 |
|---|---|---|
| `SURFACE_BASE` | `0x22252a` | 窗口根：标题带、侧栏底、状态栏 |
| `SURFACE_PANEL` | `0x272a2f` | 内容区（画廊主体）|
| `SURFACE_CARD` | `0x2b2f34` | 卡片、检查器面板、抬升区块 |
| `SURFACE_ELEVATED` | `0x32363c` | 悬停底、抬升控件 |
| `SURFACE_MODAL` | `0x2d3238` | 菜单、弹出层、对话框 |
| `SURFACE_FAINT` | `0x1e2126` | 内凹槽：搜索框、日历、缩略图占位 |
| `SURFACE_AUDIO` | `0x1a1d21` | 音视频舞台底 |
| `SURFACE_EXTREME` | `0x151719` | 预览舞台、胶片条底 |
| `TEXT_PRIMARY` | `0xe9ebee` | 主文字 |
| `TEXT_SECONDARY` | `0xa9aeb5` | 次要文字 |
| `TEXT_MUTED` | `0x7b8189` | 弱化文字/计数 |
| `DANGER` / `DANGER_HOVER` | `0xe5484d` / `0xf06065` | 删除、错误 |
| `WARNING` | `0xd8a250` | 警告 |
| `SUCCESS` | `0x3fb56a` | 成功 |
| `STAR` | `0xf2c14e` | 星级 |

叠加填充（`rgba(...)` 用，值即 `0xRRGGBBAA`）：

| 令牌 | 值 | 用途 |
|---|---|---|
| `FILL_SUBTLE` | `0xffffff0a` | 控件静息底（chips、ghost 按钮）|
| `FILL_HOVER` | `0xffffff12` | 悬停底 |
| `FILL_ACTIVE` | `0xffffff1c` | 按下/激活底 |
| `SELECTION_FILL_ALPHA` | `0x3d` | 选中填充透明度（配合 `accent()`）|
| `SEPARATOR` | `0x1d2025` | 石墨面之间的深色细分隔线（solid）|
| `SEPARATOR_LIGHT` | `0xffffff12` | 深色槽/菜单内的浅分隔线 |
| `SCRIM` | `0x0d0f12b8` | 模态遮罩 |

### 4.2 几何与排版

| 令牌 | 值 | 说明 |
|---|---|---|
| `WINDOW_WIDTH/HEIGHT` | 1280 / 800 | `theme.rs` 中 `WINDOW_*` 同名常量 |
| `WINDOW_MIN_WIDTH/HEIGHT` | 960 / 600 | 最小窗口 |
| `TITLE_BAND_HEIGHT` | 56 | 顶栏（应用内，不含系统标题栏）|
| `STATUS_BAR_HEIGHT` | 40 | 底栏 |
| `SIDEBAR_WIDTH` / MIN / MAX | 264 / 200 / 400 | 侧栏（可拖拽）|
| `INSPECTOR_WIDTH` | 300 | 右侧检查器 |
| `SEARCH_FIELD_HEIGHT` | 32 | 搜索框 |
| `FILTER_CHIP_HEIGHT` | 30 | 筛选 chip |
| `SIDEBAR_ROW_HEIGHT` | 30 | 侧栏行高 |
| `THUMB_SIZE_MIN/MAX/DEFAULT` | 80 / 300 / 150 | 缩略图尺寸 |
| `ROUND_SM/MD/LG/XL/PILL` | 4 / 6 / 8 / 10 / 999 | 圆角 |
| `THUMB_ROUND` | 6 | 缩略图圆角 |
| `FONT_SIZE_TINY/SMALL/BODY/BUTTON/HEADING/TITLE` | 11 / 12 / 13 / 13 / 15 / 18 | 字号 |
| 字体 | `UI_FONT_FAMILY`="Segoe UI"、`SEMIBOLD_FONT_FAMILY`="Segoe UI Semibold" | 不含中文回退时由系统处理 |

### 4.3 用色规则（防呆）

- 选中态统一 = `accent()` 3px 左条 + `rgba((accent()<<8)|0x59)` 填充（侧栏行）；
  或 = 2px `accent()` 描边（卡片/缩略图）。
- 焦点态 = 选中描边 + `rgba((accent()<<8)|0x80)` 外圈。
- 主要按钮 = `Button::new(..).primary()`（主题里 primary=accent）；危险 =
  `.danger()`；中性操作 = `.ghost()`；次要描边 = `.outline()`。
- 分隔线：区域之间用 `SEPARATOR` 实色 1px；深色槽内用 `rgba(SEPARATOR_LIGHT)`。
- 星级：填充 `STAR`，空星 `TEXT_MUTED`。
- 强调色运行时来自 `theme::accent()`；切换强调色后必须调用
  `theme::apply_accent_components(cx)` 同步 gpui-kit 原生控件（已有此函数）。

## 5. 架构事实（写代码前必读）

### 5.1 单窗口 + 槽位注册

应用只有一个窗口（`crates/photag-shell/src/main.rs` 的 `open_window`）。功能界面
以“视图贡献”注册到命名槽位，`crates/photag-shell/src/shell.rs` 的 `render()` 按槽位
摆放。槽位分发代码在 shell.rs 的 `match slot.as_str()`（搜 `"command.search"`）。

现有槽位与归属：

| 槽位 | 现状所有者 | 摆放位置（shell） |
|---|---|---|
| `command.search` | gallery | 顶栏中部 |
| `command.size` | gallery | 顶栏中部（缩略图尺寸）|
| `command.filters` | gallery | 顶栏下方筛选行 |
| `workspace.status` | gallery | 底栏 |
| `command.tools` | gallery | 工具弹层 |
| `workspace.sidebar` | navigation | 左侧栏 |
| `gallery.content` | gallery | 内容主区 |
| `workspace.main` | preview(AI)/media-playback | 主区底部覆盖 |
| `workspace.modal` | gallery(0)/preview(0)/settings(0) | 全窗口覆盖层 |
| `settings.language` | i18n | 特例：被过滤，在设置内渲染 |

**本期新增两个槽位**：

| 新槽位 | 所有者 | 用途 |
|---|---|---|
| `command.breadcrumb` | navigation | 顶栏面包屑（全部文件 / 文件夹名）|
| `gallery.inspector` | gallery | 右侧“文件信息”检查器列 |

新增后必须在 shell.rs 的槽位分发中加分支，否则视图不会出现。

### 5.2 视图注册模板

注册入口（每个插件 lib.rs 里搜索 `register_views_for_plugin`）：

```rust
use photag_ui_bridge::{UiViewContribution, register_views_for_plugin};

let view = UiViewContribution::new(
    "navigation/breadcrumb",   // 全局唯一 id，格式 "crate/名称"
    "command.breadcrumb",      // 槽位
    0,                         // order：同槽位内排序，小的在前
    move |window, cx| { /* 返回 AnyView */ }.into(),  // 工厂，GPUI 线程上调用
);
register_views_for_plugin(&cordis, "navigation", vec![view])?;
```

多区域共享一个 Entity 的模式参照
`crates/photag-plugin-gallery/src/view/regions.rs`（GalleryRegion 枚举 + observe 转发）：
gallery 把同一个 `GalleryView` 实体投影到 6 个不同槽位。navigation 的新面包屑和
gallery 的新检查器都应复用各自已有实体，不要再造第二份状态。

### 5.3 浮层开关：`UiSurfaceState`

`photag_ui_bridge::UiBridgeService.surfaces`（`crates/photag-ui-bridge/src/lib.rs`）：

```rust
surfaces.set_open("settings", bool);   // 打开/关闭
surfaces.toggle("settings");           // 翻转
surfaces.is_open("settings");          // 查询
surfaces.set_capturing_key("settings", bool); // 输入框捕获键盘时置位
surfaces.suppress_next_escape("settings");    // 提交型按键吞掉下一次 Esc
```

已用 surface id：`settings`。设置打开入口是命令 `settings/toggle`
（settings 插件注册，shell 渲染成按钮；见 shell.rs 搜 `settings/toggle`）。
本期约定：新增浮层如需全局开关，沿用此机制，id 用语义名（如 `bulk-picker`）。
预览/播放不开新 surface：它们的视图自己根据服务状态决定渲染内容。

### 5.4 覆盖层特性（重要）

- **设置** = `workspace.modal` 里的可拖动卡片；`surfaces.is_open("settings")` 控制；
  Esc 关闭逻辑在 shell.rs 的 `on_key_down`（搜 `take_suppressed_escape`）。
- **预览** = `workspace.modal` 全窗口覆盖（`preview/image-view`）；渲染内容由
  `PreviewService` 的当前项决定；关闭调 `service.close()`。
- **播放** = media-playback 注册在 `workspace.main`（底部条）。v5 要求改成
  独立播放器覆盖层（`workspace.modal`），见 7.S6。
- 覆盖层内部不要渲染系统级窗口按钮；覆盖层的 ✕ 调对应服务的关闭方法。

### 5.5 i18n 流程（新增文案的唯一正确做法）

每一条新文案三步：

1. 代码里写 `photag_ui_bridge::i18n::tr("English source")`（英文原文是键）。
2. 在 `crates/photag-plugin-i18n/locales/keys.json` 加映射，msg-id 用稳定哈希：

   ```bash
   echo -n "Add folder" | sha1sum | cut -c1-12   # 生成 12 位后缀
   # => "Add folder": "msg-3a72e4f0b835"
   ```

3. 在 `en.ftl` 和 `zh-CN.ftl` 各加一行（id 必须一致）：

   ```
   msg-3a72e4f0b835 = Add folder
   msg-3a72e4f0b835 = 添加文件夹
   ```

4. 跑 `cargo +1.97.1 test --locked -p photag-plugin-i18n -j 2` 验证目录一致性。
5. **先搜 `keys.json` 里是否已有该英文**（现有 388 条），有就复用，禁止重复登记。

带前缀的标签（`"• 名称"`、`"✓ 名称"` 等）`tr()` 会自动剥前缀查表，直接用整串即可。
视图语言切换刷新：新增 Entity 必须带 `locale_revision` 字段并在 render 里对比
`photag_ui_bridge::i18n::revision()`（参照 gallery regions.rs 或 navigation
render.rs 的写法）。

### 5.6 gpui-kit 组件速查（API 已核实）

```rust
use gpui_kit::component::button::{Button, ButtonVariants};

Button::new("id").label("文本").primary()          // 也支持 .ghost()/.danger()/.outline()/.warning()
    .icon(gpui_kit::component::IconName::Plus)     // lucide 图标
    .compact()                                     // 紧凑尺寸；.toggled(bool) 切换态
    .on_click(cx.listener(...));

// 勾选与单选（原生交互 + 主题着色）
Checkbox::new("id").checked(bool).label("文本").on_click(|&checked, _, _| {});
Radio::<usize>::new("id").checked(bool).label("文本").on_click(...);  // 整组用 RadioGroup

// 图标：IconName 是 lucide 枚举，视图里用 .child(Icon::new(IconName::Folder))
// 弹层：Popover::new("id").trigger(el).content(|_, _, cx| ...)（菜单用 DropdownMenu）

// 输入：Input / InputState（已有大量用例，搜 InputState::new）
// 进度：ProgressBar；滑杆：Slider/SliderState；复选框组：Checkbox 直接铺
```

已确认可用的 IconName（lucide）：Aperture, Monitor, Keyboard, Toolbox, Tag, Tags,
Sparkles, RefreshCw, FolderPlus, Folder, FolderOpen, FolderClosed, Image, Film,
Music, Play, Pause, RotateCw, FlipHorizontal2, ZoomIn, ZoomOut, Wrench, Palette,
SlidersHorizontal, GalleryHorizontal, Calendar, Star, CircleCheck, TriangleAlert,
ExternalLink, ChartColumn, HardDrive, Clock, Info, CircleAlert, X, Check, Plus,
Minus, ChevronDown, ChevronLeft, ChevronRight, Search, Settings, Menu, Ellipsis,
LayoutGrid, ArrowUpDown, ListChecks, MapPin。

窗口/对话框：原生文件对话框保持系统原生（现有 file-operations/metadata-export
已在用）；不要用 gpui-kit 的 alert dialog 做 v5 设计里的卡片模态，卡片模态按
v5 自绘（SURFACE_MODAL + 圆角 10 + 边框 SEPARATOR_LIGHT）。

### 5.7 定位代码锚点（行号会漂移，优先用引号里的字符串搜索）

| 位置 | 搜索标记 |
|---|---|
| 槽位分发 | shell.rs `match slot.as_str()` / `"gallery.content"` |
| 顶栏骨架 | shell.rs `// Source main.rs:1007-1097: full-width command band.` |
| 底栏骨架 | shell.rs `// Source main.rs:1099-1277: full-width status band.` |
| 侧栏拖拽 | shell.rs `"sidebar-resizer"` |
| 工具弹层 | shell.rs `"shell-tools"` |
| 窗口选项 | main.rs `open_window` / `WINDOW_MIN_WIDTH` |
| 视图注册（gallery）| gallery `view/regions.rs` `pub(super) fn contributions` |
| 画廊卡片 | gallery `view/card.rs`、`view/card_motion.rs` |
| 画廊筛选行 | gallery `view/regions.rs` `render_filters` |
| 画廊状态栏 | gallery `view/regions.rs` `render_status` |
| 画廊弹层 | gallery `view/regions.rs` `render_modal` |
| 画廊统计 | gallery `view/statistics.rs` |
| 标签选择器 | gallery `view/tagging_picker.rs` |
| 批量操作 | gallery `view/bulk_actions.rs`、`view/file_actions.rs` |
| 侧栏渲染 | navigation `view/render.rs`、`view/folder_row.rs`、`view/tag_render.rs` |
| 标签动作 | navigation `view/tag_actions.rs` |
| 设置页 | settings `view/tabs.rs`、`view.rs`、`updates.rs`、`ai_groups.rs` |
| 预览渲染 | preview `view/render.rs`、`view/toolbar.rs`、`view/overlays.rs`、`view/info.rs`、`view/filmstrip.rs` |
| 播放 | media-playback `src/view.rs` |
| 导出 | metadata-export `src/view.rs` |

---

## 6. 九张设计稿逐稿实现规范

### 6.0 全局共同模式

- **按钮**：主要动作 accent 实心白字（如 添加文件夹/继续/保存/创建/立即检查/下载）；
  取消/关闭/次要 = ghost；危险删除 = DANGER 实心白字。表单行内按钮高 30–32。
- **卡片模态**：背景 `SURFACE_MODAL`，圆角 10，1px 边框 `rgba(SEPARATOR_LIGHT)`，
  遮罩 `SCRIM`；标题行：图标+标题 15 semibold + 右侧 ✕；底部按钮行右对齐；
  面板尺寸固定、内部滚动（内存要求：切页/换内容时面板不挪位不变形）。
- **Toast/通知卡**：右下角堆叠（`right:16, bottom:STATUS_BAR_HEIGHT+16`），宽 380，
  圆角 8，SURFACE_MODAL + 边框；结构 = 状态图标 + 标题 13 semibold + 描述 12 muted
  +（可选）动作按钮 + ✕。成功可 4 秒自动消失；错误/警告常驻直到手动关闭。
- **分段/单选/复选高亮**：选中用 accent 相关（单选圆点 accent，复选方块 accent，
  分段选中项 accent 实心白字）。
- **悬停**：行悬停 `rgba(FILL_HOVER)`，按下 `rgba(FILL_ACTIVE)`。
- **列表行高**：30（紧凑列表）/ 36（表单行）；组头 28 semibold TEXT_SECONDARY。
- **禁用态**：文字 TEXT_MUTED + 填充透明度减半，不响应点击。

### 6.1 稿 00 主界面（`final-v5/00-main-library.png`）

目标结构（自上而下、自左而右）：

```
┌────────────────────────────────────────────────────────────────────┐
│ 顶栏 56：[☰][光圈logo][PhoTag] │ [⌂ 全部文件 / 文件夹]   [尺寸滑杆][搜索][工具▾][检查器开关] │
├──────────┬─────────────────────────────────────────────┬───────────┤
│ 侧栏 264  │ 筛选行 30：chip×6 + 按月分组开关               │ 检查器 300 │
│ 全部文件   │ ┌─ 月组头 2026年10月 · 9 项 ─┐                 │ 文件信息   │
│ 文件夹 +   │ │  □图  □图  □图  □图        │                 │ 文件名     │
│ 标签  🔍 + │ │  文件名 / 尺寸  (标题在图下) │                 │ ★★★★☆    │
│ ...       │ │  ...                       │                 │ 基本信息   │
│ 设置(底部) │ └────────────────────────────┘                 │ 标签/EXIF  │
│           │                                                │ [显示][⋯]  │
├───────────┴───────────────────────────────────────────────┴───────────┤
│ 状态栏 40：9 项 · 已选择 1 项                     [🏷 标签▾] [⋯ 更多▾]   │
└───────────────────────────────────────────────────────────────────────┘
```

要点：

1. 顶栏内不再写 "PhoTag" 纯文本，改为 [侧栏开关 ☰] + 光圈图标 + PhoTag（semibold 15）。
2. 面包屑为新槽位 `command.breadcrumb`（navigation 提供）：家图标 + “全部文件” +
   “/” + 当前文件夹名；点击“全部文件”清空文件夹选择。
3. 缩略图尺寸控件移动顶栏中部偏右：小图图标 + 滑杆（约 110 宽）+ 大图图标。
4. 搜索框 32 高、圆角 6、`SURFACE_FAINT` 底、放大镜图标 + “搜索媒体…”占位 +
   有内容时右侧 ✕；宽约 240。
5. 工具按钮 “工具 ▾” ghost；最右是检查器开关按钮（PanelRight 图标，可开合检查器）。
6. **内容区不再包一层抬升圆角卡片**（现行 shell 的 rounded(10)+面板底要删掉），
   网格直接铺在 `SURFACE_PANEL` 上；内容列与侧栏、检查器之间用 1px `SEPARATOR`。
7. 卡片：图像圆角 `THUMB_ROUND`；**标题移到图下方居中**：文件名 12 `TEXT_PRIMARY`
   （省略号），第二行 11 `TEXT_MUTED`：图片=“6000 × 4000”，视频/音频=时长“02:14”。
   视频图内左下保留 ▶+时长小胶囊；音频卡用 `SURFACE_FAINT` 底 + 音符图标居中
   （禁止伪造波形，见第 9 节）。
8. 选中：图外框 2px `accent()` + 右上角 18px 圆角 4 的 accent 勾选徽章；键盘焦点
   在此基础上加 1px `rgba((accent()<<8)|0x80)` 外圈。
9. 检查器（新槽位 `gallery.inspector`）：未选择时居中“未选择文件” muted；单选时
   文件名 15 semibold + 可点星级 + 分区（基本信息/标签/EXIF/文件路径，组头带
   折叠 chevron；基本信息展开，其余折叠）；标签胶囊 = 色点+名+✕ 和 [+ 添加]；
   底部固定 [在资源管理器中显示] outline + [⋯] 方形按钮（放现有文件操作的
   常用入口）。多选时显示“已选择 N 项”+ [为选中媒体分配标签] [批量操作]。
10. 状态栏：左“N 项”或“N 项 · 已选择 M 项”（12 secondary）；右 [🏷 标签▾]
    [⋯ 更多▾] ghost 按钮（标签=打开批量标签选择器；更多=批量操作/统计信息/
    导出元数据入口，允许复用现有工具弹层内容）。
11. 扫描进度不再放状态栏，改右下 Toast 卡（见 6.8 与 7.S9）。

### 6.2 稿 01 预览与播放（`final-v5/01-preview-playback.png`）

**图片预览**（全窗口覆盖层，`preview/image-view`）：

```
[光圈logo][PhoTag] │ 文件名     …spacer…   [ⓘ 信息] [✦ AI] [〉〉 EXIF] [✕]
┌──────────────────────────────────────────────┬──────────────┐
│                图片舞台 SURFACE_EXTREME        │ 文件信息 浮层  │
│                                              │ 文件名/类型/尺寸│
│                                              │ 标签/AI/EXIF  │
├──────────────────────────────────────────────┴──────────────┤
│ [适应窗口▾] │ 100% [🔍—滑杆—🔍+] 重置 │ [↻旋转][⇋镜像] │ [🏷标签][⋯更多] │
├─────────────────────────────────────────────────────────────┤
│ [‹] [缩略图×7 选中描边] [›]         胶片条 SURFACE_EXTREME      │
└─────────────────────────────────────────────────────────────┘
```

- 顶栏 56：logo + 应用名 + 竖分隔线 + 文件名（13 secondary，省略号）+
  右侧 [信息][AI][EXIF] ghost 按钮（信息=开合右侧信息浮层；AI=打开 AI 标签建议
  模态，见 6.4；EXIF=滚动/展开信息浮层的 EXIF 分区）+ ✕（调 `service.close()`）。
  替换现行第一行无顶栏、左上 info 圆钮、右上 ✕ 的浮动方案。
- 右侧信息浮层：右 16 / 上 76，宽 310，最大高约 62%，`SURFACE_MODAL` 圆角 10，
  内滚；行 = 标签 12 muted 左 / 值 12 primary 右（文件名/文件类型/图像尺寸/
  文件大小/拍摄日期）；分区：标签（已展开，胶囊+[+]；**只展示真实已有标签**）、
  基本信息（相机/镜头/焦距/光圈/快门/ISO/色彩空间/GPS——来自现有 EXIF 查询，
  没查到的字段整行省略）、AI、EXIF、文件路径（折叠）。设计稿里的“打开次数”
  若数据库无此字段就整行不做（见第 9 节）。
- 底部工具栏 56：整宽条（`SURFACE_BASE` + 顶部 1px SEPARATOR），左起
  [适应窗口 ▾] │ 百分比文本 │ [🔍− 滑杆120 🔍+] │ 重置 │ 分隔 │ [旋转][镜像] │
  弹性空白 │ [标签][更多]。按钮全部 30 高 ghost；“标签”沿用现有标签下拉，
  “更多”收拢现有工具栏次要项（删除/复制/移动/在文件夹中显示/用默认应用打开/
  胶片开关等，保持现有行为不变）。
- 胶片条 96 高，`SURFACE_EXTREME` 底：左 ‹ / 右 › 圆形按钮（32），缩略 64×64
  圆角 4，选中 2px accent 描边，图下文件名 11 muted 居中，图距 8。
- 舞台点击/双击/滚轮缩放行为沿用现有预览实现；chrome 2 秒自动淡出保留。

**视频/音频播放**（独立覆盖层，播放中在 `workspace.modal` 呈现）：

- 结构同预览顶栏（logo + 文件名 + ✕）。
- 视频：舞台居中视频帧；音频：`SURFACE_AUDIO` 底 + 居中音符图标 48 + 文件名
  （**不画波形**，设计稿波形属插画；如后续 ffmpeg 能出真实波形再加，见第 9 节）。
- 控制条 56 整宽，`SURFACE_BASE` + 顶部分隔线，自左：
  [⏸/▶] [00:24 / 02:14] [——进度滑杆(flex)——] [🔊 音量滑杆 80] [1× ▾]（视频还有
  [|◀][▶|] 帧步进）弹性空白 [标签] [更多]。已播放/剩余时长用现有 playback 快照。
- 关闭 ✕ 停止播放并关闭覆盖层。

### 6.3 稿 02/03 设置（`final-v5/02-...png`、`03-...png`）

```
[光圈logo][PhoTag] │ 设置                                        [✕]
┌──────────┬────────────────────────────────────────────────────┐
│ ▣ 外观    │  页面标题 15 semibold                              │
│ ☰ 工具栏  │  分区标题 13 semibold + 说明 12 muted               │
│ ⌨ 快捷键  │  表单…                                              │
│ ◇ 标签    │                                                    │
│ ✦ AI     │                 （内部滚动）                          │
│ ↻ 更新    │                                                    │
├──────────┴────────────────────────────────────────────────────┤
│ ⓘ 设置更改自动保存                              [关闭]           │
└───────────────────────────────────────────────────────────────┘
```

- 整体尺寸约 880×620（固定，内部滚动；满足“浮层面板几何稳定”要求）。
- 左导航 180：条目 36 高，图标（Monitor/SlidersHorizontal/Keyboard/Tag/Sparkles/
  RefreshCw）+ 文本 13；选中 = accent 填充 0x59 + 左 3px accent 条；点击切页。
- **横向 6 tab 改为纵向 6 项**（替换 settings/view.rs 现行的 `tabs()`）。
- 页脚：左 [ⓘ] 设置更改自动保存（12 muted）；右 [关闭] ghost。
- 页面内容（控件都存在，只需按 v5 重排样式）：
  - 外观：界面外观（使用系统强调色 checkbox+说明）；强调色来源（8 个预设：
    7 色板 + 系统强调色，radio + 色点圆 + 名称，中文名按 keys.json 现有翻译，
    没有的补：石板蓝/灰青绿/柔和铜/灰玫瑰/鼠尾草绿/灰紫/暖灰褐）；界面示例
    （一条小预览条：logo + 3 个图标 + 星级 + 图像图标 + accent 下划线）。
  - 工具栏：按 缩放/图片处理/标签/文件/查看 分组的显示开关（checkbox），
    底部 锁定工具栏位置 / 重置工具栏位置 两行。开关绑定现有设置字段。
  - 快捷键：排除修饰键（下拉 Ctrl 等）、打开媒体文件（键位钮 + 更改）、
    评分快捷键说明行、标签目标快捷键 F1–F9（每行 = 键帽 + 标签下拉，含“无”）。
  - 标签：搜索匹配下拉（包含/开头/精确等，按现有设置项）、保留搜索词、
    合并标签子级（radio：重新归类（Reparent）/删除子级（Delete）+ 各自说明）。
  - AI：预览时自动分析 checkbox；必选标签组卡列表（组名 + 标签胶囊 + 添加标签
    下拉 + 删除组；底部虚线 [+ 添加标签组]）——适配现有 ai_groups.rs 数据。
  - 更新：当前版本行；启动时检查更新 checkbox；[立即检查 primary][打开发布页
    ghost]；更新状态卡（发现更新 + [下载]；发现→下载中显示进度与速度；
    已下载 + [重启并安装 primary][稍后再说 ghost]）。复用现有 updater 服务，
    没有的状态不显示。
- 01/02/03 稿的窗口控制钮（— □ ✕）不实现（系统标题栏提供）；标题行只保留 ✕。

### 6.4 稿 04 标签管理（`final-v5/04-tag-management.png`）

全部做成 `workspace.modal` 覆盖层（navigation 注册新模态视图，共享 SidebarView
实体；打开状态可用 `surfaces` 或视图内字段）。现行“内联表单/菜单”实现替换：

1. **编辑标签模态**（约 640×480）：标题“编辑标签”+✕；左：标签名称 Input；
   标签颜色 = 圆形色点网格（两行，选中带 accent 外圈；沿用现有颜色数据，
   如现有字段是十六进制文本则色点由 `theme::parse_hex_color` 渲染）；
   标签图标选择 = 当前图标展示框 + 图标网格（沿用现有图标枚举，用 lucide 展示）；
   checkbox“在缩略图上显示标签标记”；右：关联标签复选树（含展开箭头）+ 顶部
   checkbox“同时应用到所有子标签”；页脚 [保存 primary][取消 ghost]。
2. **创建标签小卡**（约 420 宽）：名称 Input + 色点行 + [创建 primary][取消 ghost]。
3. **删除标签模态**：被删标签胶囊；问题“是否同时删除其下的所有子标签？”；
   radio“同时删除所有子标签（包括子标签下的内容）”/“仅删除此标签，保留子标签
   （子标签将变为顶级标签）”；[删除 danger][取消 ghost]。行为保持现有删除逻辑。
4. **合并标签模态**（约 640×420）：说明“选择目标标签后将立即执行合并，源标签
   将被删除。”；源标签胶囊；选择目标标签 = 搜索 Input + 复选树；右侧合并选项
   radio（重新归属到目标标签 / 删除子级）+ 说明；[取消 ghost]。选中目标后
   立即执行（现行行为），执行完关闭。
5. **分配标签（批量）面板**（gallery 的 tagging_picker 改造）：左面板 430 +
   右侧媒体预览卡（缩略 + 文件名 + 尺寸）；标题“为选中媒体分配标签 · N 项”+✕；
   搜索框；复选树；下方“最近使用的标签”彩色胶囊行（现有 recent 数据）；
   按钮 [分配选中标签 (N) primary][移除选中标签 (N) ghost][清空所有标签 ghost]。
6. **AI 标签建议模态**（preview 的 ai_suggestions 改造）：左媒体预览卡；
   右：模型下拉（真实模型名）、可折叠“模型加载状态”（已加载绿点/加载中/不可用）、
   “标签建议 (N)”行 = checkbox + 色点 + 名称 + 百分比 + [应用 primary 小按钮]
   [创建标签 ghost 小按钮]。保留现有 AI 服务行为与降级提示。

以上模态与现行为一一对应，只改形态与布局；打开入口：侧栏标签 ⋯ 菜单 / 右键
菜单 / 预览顶栏 [AI] / 状态栏 [🏷 标签]。

### 6.5 稿 05 筛选与菜单（`final-v5/05-filters-menus.png`）

1. **筛选 chip 行**（gallery render_filters 重构）：chip = 圆角 6、30 高、
   `FILL_SUBTLE` 底、1px `rgba(SEPARATOR_LIGHT)` 边框、内 10px、图标 14 muted +
   文本 13 + ChevronDown 12；定义：全部类型(图集图标)/标签/扩展名/日期/评分/
   最新优先(ArrowUpDown)/按月分组(Calendar，切换开关)。**筛选生效后 chip 变为
   激活态：accent 文本 + 尾部换成 ✕**（点击清该筛选）。弹出层用 Popover 承载，
   内含单选/复选列表（现 DropdownMenu 的纯文字项要升级成下面内容）。
2. **搜索范围**：点搜索框左侧或右上按钮弹出：标题“搜索范围”+ 说明；checkbox
   文件名（匹配文件名）/文件路径（匹配文件所在路径）/标签（匹配文件标签）。
3. **全部类型弹层**：radio 列表：全部文件/照片/视频/音频（带图标）。
   **排序弹层**：radio：最新优先/最早优先/文件大小（从大到小）/（从小到大）/
   名称（A–Z）/（Z–A）/评分（从高到低）/（从低到高）。绑定现有排序枚举。
4. **扩展名弹层**：分段控件 [包含所选 | 排除所选]；checkbox 清单（JPEG (.jpg,
   .jpeg)、PNG、WebP、TIFF、MP4、WAV——以现有扩展名集合为准，动态列出）；
   底部 [清除所有] ghost。
5. **日期弹层**：标题“日期范围”；从/到 两个日期框（DatePicker）+ 月历
   （gpui-kit DatePicker 组件，主题已着色）；[清除日期]。**评分弹层**：
   五分星行 + 范围滑杆 0–5（两端标注 0 与 5），复用现有评分筛选字段。
6. **工具菜单**（“工具 ▾”）：批量操作（→ 6.6 大对话框）/ 统计信息 / 导出元数据；
   分组标签“查看”；按月分组。现有弹层内容改 v5 行样式（30 高、图标+文字）。
7. **媒体右键菜单**：菜单顶部信息卡（缩略 + 文件名 + 尺寸 + 大小）；项：预览 /
   打开 / 在文件夹中显示 / 分隔 / 标签 ▸ / 评分 ▸ / AI 建议 / 分隔 / 重命名 F2 /
   复制到 ▸ / 移动到 ▸ / 分隔 / 删除（红，右侧快捷键 Delete）。数据全部来自现有
   动作；子菜单保持现有 PopupMenuItem 能力。
8. 稿 05-7 的“当前筛选条”（一行可移除胶囊）**不单独实现**：激活 chip 自带 ✕ 已
   覆盖该用途（第 9 节记录）。

### 6.6 稿 06 文件操作弹窗（`final-v5/06-file-operations-dialogs.png`）

1. **批量操作大对话框**（约 860×560，替换现有 bulk_actions 小面板）：
   左区三块：1 选择操作（radio：删除（移至回收站）/复制到…/移动到…）；
   2 选择来源（radio：当前选中的项目（N 项）/按筛选条件选择（从整个库中））；
   3 筛选条件（媒体类型下拉 / 文件名 Input / 文件扩展名 = 下拉[是/否]+值 Input /
   拍摄日期 = 下拉[在范围内]+起止 DatePicker / 评分 = 下拉[大于等于]+星级 /
   标签 = 下拉[包含任一]+标签胶囊+[+]）；右区“匹配结果 (N 项)”列表（缩略 96×64 +
   文件名 + 尺寸或时长）；页脚 [取消 ghost][继续 primary]。继续进入现有确认/执行流。
2. **批量删除确认**：警告三角 32 WARNING；标题“确认删除选中的 N 个项目?”15（稿为
   17，取 15 与模态标题一致即可）；说明“这些项目将被移至系统回收站。”12 muted；
   缩略图行（视频带 ▶ 时长胶囊）；[取消 ghost][删除 N 个项目 danger]。
3. **单个删除确认**：同款；说明“文件将被移至系统回收站。”；[删除 danger]。
   gallery 里现在用 `window.open_alert_dialog` 的删除路径要改成此自绘模态。
4. **新建文件夹模态**：位置行（只读 Input 显示父路径）；文件夹名称 Input（聚焦）；
   [取消 ghost][创建 primary]。
5. **重命名文件模态**：左预览卡（缩略 + 尺寸）；右 原文件名（文本行）+ 新文件名
   Input（预填原名全选）；[取消 ghost][保存 primary]。
6. **文件夹右键菜单**：新建子文件夹 / 重新扫描 / 在资源管理器中打开 / 从库中移除
   （现有菜单项，改成 v5 行样式）。
7. **标签拖放位置菜单**：删除常显英文拖放条（“Drop above to place before”等），
   改为拖拽经过标签行时在该行旁浮出三选项菜单：[⬆ 放在前面][↕ 放入此标签]
   [⬇ 放在后面]（对应 TagDropPos::Before/Into/After，居中提示“标签在标签树中的
   放置位置”可选）。若无能力做浮出定位，退化为该行下方嵌入的紧凑三按钮行，
   但仍不得显示英文。保留现有放置行为。

### 6.7 稿 07 统计/导出/更新（`final-v5/07-statistics-export-update.png`）

1. **统计信息模态**（约 720 宽）：标题“统计信息”+✕；顶部 4 张统计卡（图标+
   标签+大数字+“项”）：全部文件/图片/图频/音频；左列 存储空间卡（总大小大字 +
   分项行 图片/视频/音频）。右列 评分分布卡（0–5 星柱状，用 div 高度手绘即可）；
   左下 最常用的标签（Top 5）行 = 色点+名+横向进度条+数量；右下 按月份统计
   （拍摄日期）柱状；页脚 [关闭 ghost]。数据用现有 statistics 查询。
2. **导出元数据模态**：入口在工具菜单 →（先弹系统原生保存对话框选路径，保留
   现有行为）→ 模态显示状态机：
   - 正在导出：步骤徽标 1 + 标题；说明“将 N 条记录导出为 JSON 文件…”；
     目标文件：<文件名>；进度条 + 百分比；“正在写入文件… i / N”；
     信息行“文件保存位置通过系统文件保存对话框选择（使用操作系统原生对话框）。”
   - 导出完成：绿勾 + “成功导出 N 条记录” + 文件：<名> + 大小：<大小> + [关闭]。
   - 导出失败：红叉 + “导出失败” + 错误：<现有错误文案> + [关闭]。
   替换现行 Tools 弹层内的小面板。
3. **更新模态**（替换 440 宽小卡）：左列 当前版本 / 最新版本（+“新版本可用”
   绿色小胶囊）/ 检查状态行（✓ 正在检查更新…、✓ 已获取最新版本信息）/ 更新说明
   （真实 release notes；现有 updater 若拿不到正文就不显示该块）；右列步骤 2
   正在下载（进度条 + 百分比 + 已下载/总量 + 速度 + “下载完成后将提示安装…”）；
   步骤 3 准备安装（说明 + [重启并安装 primary][稍后再说 ghost]）。保留现有
   updater 服务与状态。

### 6.8 稿 08 状态与反馈（`final-v5/08-states-feedback.png`）

1. **空库态**：内容区居中：72 圆角方槽（`SURFACE_FAINT`，内图像图标 40 muted）
   + “未添加媒体”15 + “添加包含照片和视频的文件夹以开始浏览”12 muted +
   [＋ 添加文件夹 primary]。按钮调 navigation 的挑文件夹流程（现有 pick_folder
   命令/服务；navigation 需暴露命令 id 供 gallery 空态触发，搜现有
   `pick_folder`）。
2. **无结果态**：居中放大镜图标 + “没有匹配项”15 + “尝试调整或清除筛选条件以
   查看媒体”12 + [清除筛选 ghost]（清全部筛选）。整词条与筛选行为联动。
3. **扫描进度卡**：右下 Toast 堆叠；卡 = 文件夹图标 + “正在扫描：<文件夹名>” +
   “正在扫描文件… (120 / 200)” + 进度条 60% + [取消 ghost] + ✕；取消中状态
   显示“正在取消… 请稍候”且按钮禁用。数据来自 indexing 服务现有进度流；
   查找现有进度 UI 的所有者（搜 `scan`/`取消`）把 UI 搬到此处。
4. **扫描失败 Toast**：警告图标 + “文件夹扫描失败” + 原因 + [重新扫描 primary
   小按钮] + ✕；重扫调现有重扫流程。
5. **写回失败**：检查器底部 DANGER 淡底卡：红叉 + “写回元数据失败” + 原因
   （现有 failure 文案）+ [重试 primary 小][忽略 ghost 小]；下方提示
   “已加入重试队列（第 N 次）”在重试后出现。**前提**：tagging 服务需有公开的
   重试方法（把失败项重新排入写回队列）；没有就先在 tagging crate 加该方法
   （复用现有队列/失败表，不新建机制），再加按钮。忽略 = 现有 clear_failure。
6. **缩略图占位**：加载中 = `SURFACE_FAINT` 槽 + 居中 Spinner；失败 = 槽 + 居中
   警告三角（WARNING）。替换现有灰块占位。
7. **Toast 风格**与 6.0 共同模式一致；现有 gallery “操作 toast”按此重绘。

---

## 7. 分阶段执行计划（按顺序做，勿跳段）

> 每个阶段收尾时必须：fmt + check 通过；把“验证记录”追加到
> `docs/redesign-validation.md`；更新本文件第 10 节台账。

### S0 主题令牌（已完成 2026-10-13）

- [x] `photag-ui-bridge/src/theme.rs` 全部令牌改为 v5 值，重写
  `apply_components` / `apply_accent_components`（已落盘）。
- [ ] 编译核对（S1 一并做）：`cargo +1.97.1 check --locked -p photag-shell -j 2`。

### S1 Shell 布局骨架

目标：顶栏 56 / 主行（侧栏 264 + 内容 + 检查器 300）/ 状态栏 40；新增两个槽位。

- [x] shell.rs：顶栏重排（☰、logo、PhoTag、`command.breadcrumb` 占位、尺寸槽、
      搜索槽、工具按钮、检查器开关）；“PhoTag”用光圈图标 + 文本；
- [x] shell.rs：内容列去掉圆角抬升卡壳（rounded(10)+bg+内边距），改直铺；
      区域间加 1px `SEPARATOR`（顶栏下、侧栏右、检查器左、状态栏上）；
- [x] shell.rs：槽位分发加 `command.breadcrumb` 与 `gallery.inspector` 分支；
      新增 `inspector_open: bool` 状态（检查器开关按钮控制，默认开）；
- [x] shell.rs：状态栏高度 40、文字 12；工具弹层定位到顶栏“工具 ▾”下方
      （absolute，right 对齐按钮），内容不变；
- [x] main.rs：窗口最小尺寸已用令牌（960×600）核对即可；
- [ ] 验收：启动窗口，见三段式布局；侧栏拖拽仍工作；已有功能不丢。
      未做：S1 至今只有代码落盘，`cargo check -p photag-shell` 因 S2 未完
      （gallery 编译失败）尚未跑通，界面从未实机运行。

涉及新词条（如果 keys.json 没有）：`All files`→全部文件、`Folders`→文件夹、
`Add`→添加/新增。

### S2 Gallery 主界面

- [x] 卡片：标题移到图下（文件名 12 + meta 11 居中，图 6 圆角）；音频/视频卡
      图标占位；卡间距 16、边距 16（源数据无 duration，不伪造时长胶囊）；
- [x] 选中/焦点样式（2px accent 描边 + 勾选徽章 + 焦点外圈）；悬停 `FILL_HOVER`；
- [x] 缩略图占位（加载中 Spinner / 失败警告图标）；
- [x] 月组头重排（chevron + 15 semibold + 12 muted 计数）；
- [x] 空库态 / 无结果态（6.8-1/2）已实现（`view/empty_state.rs`，由
      `render.rs:103` 分派）；“Add folder” dispatch由 Navigation 的
      Cordis-owned listener 消费并调用真实 folder picker / indexing流程；代码已写未验证。
- [x] 筛选 chip 行重构与六个筛选弹层（搜索范围/类型/排序/扩展名/日期/评分）已接入，
      同步包含/排除、选择后关闭规则和日期 Calendar/session；代码已写未验证。
- [x] 状态栏内容、标签/更多操作及工具菜单已有真实控件和功能处理器；本轮未运行验证。
- [x] 检查器已注册到 `gallery.inspector` 并显示真实字段；布局未做新版本MCP验收。
- [ ] 验收：主界面与 00 稿对照（布局/卡片/筛选/状态栏/检查器），交互可用。

#### S2 剩余接线清单（2026-10-13 交接，按序执行）

当前工作树状态：`cargo +1.97.1 check --locked -p photag-plugin-gallery -j 2`
失败，11 个错误全部来自已写入但尚未接线的 `view/inspector.rs`。除第 7.1 节
（编译核对）外，本阶段其余功能代码已落盘，缺的是下面的接线与两处内容重写。
未做任何界面实机验证。

7.1 编译核对（S0/S1 欠账，先补）

- [x] 完成 7.2 后运行 `cargo +1.97.1 fmt --all` +
      `cargo +1.97.1 check --locked -p photag-shell -j 2`，把结果写进
      `docs/redesign-validation.md`（只写“编译核对”，不得写成界面已验证）。

7.2 `filter_row.rs` 自身缺陷（先修，再接线）

- [x] 删除文件末尾的 `fn window_ref() -> &'static mut Window { unreachable!(...) }`
      ——这是禁用的占位死代码；扩展名弹层的 “Clear all” 改用 7.2a 的 dismiss 写法。
- [x] 7.2a Popover 内关闭弹层的唯一合法写法：content 闭包签名是
      `Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E`，
      `state.dismiss(&mut Window, &mut Context<Self>)` 无法从行内 `on_click`
      （那里只有 `&mut App`）调用，也不能把 `&mut PopoverState` 存进 `'static` 闭包。
      在 content 闭包开头取 `let popover = cx.entity();`，需要关闭时用
      `popover.update(cx, |state, cx| state.dismiss(window, cx));`（`cx` 为
      `&mut App`、`window` 为 `on_click` 自己的 `&mut Window`）。
- [x] 类型弹层（`type_chip`）、排序弹层（`sort_chip`）、日期弹层的 “Clear dates”
      现都写成了 `state.dismiss(window, cx)` —— 全部按 7.2a 改写；单选弹层选完即关，
      多选弹层（标签/扩展名/评分）保持打开，不关。
- [x] `segment_pair` 的两个胶囊按钮当前没有 `on_click`（包含/排除切换是死的）：
      加参数 `weak: WeakEntity<GalleryView>`，包含/排除分别写
      `this.tag_filter_exclude_mode = exclude; cx.notify()`（标签弹层）或
      `this.set_extension_click_mode(exclude, cx)`（扩展名弹层）。
- [x] `sort_chip` 里的 `let _ = cx;` 去掉（参数改 `_: &mut Context<GalleryView>`）。
- [x] 已核实：`Checkbox`/`Radio` 不绑 `on_change` 时点击会冒泡到行级 `on_click`，
      所以列表行只用行 `on_click` 即可；`PopoverState` 从
      `gpui_kit::component::popover` 再导出可用。

7.3 gallery crate 接线

- [x] `view.rs`：加 `mod filter_row;`；删 `mod controls;`（第 7 行）。
- [x] `view.rs`：`GalleryView` 加字段
      `surfaces: Arc<photag_ui_bridge::UiSurfaceState>` 与
      `tag_filter_exclude_mode: bool`（初值 `false`，不入 session），
      同步 `new()` 参数与 `gallery()` 工厂传参。
- [x] `lib.rs`：`apply()` 里
      `context.try_service::<photag_ui_bridge::UiBridgeService>()` 取
      `Arc<clone(&bridge.surfaces)>`，传入 `view::gallery(...)`（约 553 行调用点）。
- [x] `sliders.rs`：`SliderDrag` 已是 `pub(super)`，把 `change_slider` 改 `pub(super)`。
- [x] `card.rs`：`format_bytes`、`format_timestamp` 改 `pub(super)`。
- [x] `tagging_picker.rs`：`restore_selection` 改 `pub(super)`；仿 `assign_context_tag`
      新增 `pub(super) fn remove_context_tag(&mut self, media_ids: &[i64], tag_id: i64, cx)`：
      tagging 服务 remove → 历史记 `UndoEntry::RemovedTags` → `operation_status`
      文案 → `reload()` → `restore_selection(media_ids)`。
- [x] `behavior.rs`：从 `toggle_exif` 抽出
      `pub(super) fn ensure_exif_loaded(&mut self, cx)`（按当前选中项路径读 EXIF
      填 `exif_cache`，同 id 已缓存则跳过），`toggle_exif` 调它。
- [x] `inspector.rs`：`ButtonVariants`（`.ghost()`/`.primary()`）当前不在作用域，
      补 `use gpui_kit::component::button::ButtonVariants;`。
- [x] `regions.rs`：`contributions()` 注册检查器视图到槽 `gallery.inspector`
      （内容委托 `inspector::render_inspector`）；`render_filters` 整段改委托
      `filter_row::render`；`render_search` 的绝对定位面板改
      `filter_row::search_scope_panel`（仍只在搜索框聚焦且非空时显示）；
      `render_status`、`render_tools` 按 6.5-6 重排；旧 `chip()` 仍由现有模态关闭/写回失败按钮使用，保留至这些面板重构。
- [x] 删除 `view/controls.rs`（“Clear filters” 逻辑并入 filter_row 或 regions）。

7.4 shell 与 navigation

- [x] `photag-shell/src/shell.rs`：工具弹层自动生成的 Command 按钮循环里过滤
      `"metadata-export/export"`（改由 render_tools 的“导出元数据”行承担）；
      检查器开关按钮改写 `surfaces.set_open("inspector", next)`，渲染读 `surfaces`。
- [x] `photag-shell/src/main.rs`：构造前 `bridge.surfaces.set_open("inspector", true)`；
      既有 100ms 循环（约 514-533 行）里把 `surfaces.is_open("inspector")` 同步进
      `shell.inspector_open` 并在变化时 `notify`，这样检查器内的 ✕ 能真关掉右栏。
- [x] `photag-plugin-navigation`：监听 `UiCommandInvoked` 的
      `"navigation/add-folder"`，走现有 pick_folder 流程（空库态按钮已在 dispatch）；
      代码已写，未验证 native picker/Windows行径。

7.5 文案

- [x] i18n 目前只登记了 4 个新键（`Toggle sidebar`/`Toggle inspector`/
      `Cancel (Esc)`/`File information`）。`filter_row.rs` 与 `inspector.rs` 里的
      英文原文全部未登记，须按“msg- + sha1(英文原文) 前 12 位 hex”补
      `keys.json`、`en.ftl`、`zh-CN.ftl` 三处，先查 keys.json 防重复；中文按稿 00/05
      用词（文件信息/关闭检查器/未选择文件/为选中媒体分配标签/批量操作…/更多/
      全部类型/媒体类型/排序方式/日期范围/清除日期/评分范围/搜索范围/匹配文件名/
      匹配文件所在路径/匹配文件标签/包含所选/排除所选/按月分组/基本信息/文件类型/
      文件大小/图像尺寸/拍摄日期/文件路径 等）。
- [x] 测试源码样本已补 `{scanned}` 参数；验证任务运行
      `cargo +1.97.1 test --locked -p photag-plugin-i18n -j 2` 校验键一致（本轮未运行）。

### S3 Navigation 侧栏

- [x] 侧栏分区：全部文件（LayoutGrid 图标+计数）、文件夹（组头 + ＋）、标签
      （组头 + 🔍 + ＋）、底部固定“设置”入口（调 settings/toggle）；
- [x] 行样式：选中 accent 3px 条 + 0x59 填充；计数右对齐 muted；
- [x] 标签行简化：删除常显 8 按钮行，保留 ⋯ 菜单（9 项保留）；
- [x] 面包屑视图（新）注册 `command.breadcrumb`；
- [ ] 文件夹右键菜单 + 拖放位置菜单（6.6-6/7）；
- [ ] 验收：侧栏与 00 稿对照；文件夹/标签交互不回归。

### S4 标签弹窗体系

- [x] navigation 增加模态视图（编辑/创建/删除/合并/move）到 `workspace.modal`；
      替换现行内联表单、合并面板、删除确认（6.4-1..4）；
- [x] gallery 分配标签面板重排（6.4-5），含树、recent和真实媒体预览；preview AI 建议模态（6.4-6）既有实现保留；
- [ ] 验收：各弹窗按 04 稿呈现；操作行为与现状一致（改标签/合并/删除真实生效）。

### S5 预览重排

- [x] 顶栏（logo/文件名/信息/AI/EXIF/✕）；信息浮层 w310（真实 EXIF 行）；
      底部工具栏（适应窗口/百分比/缩放滑杆/重置/旋转/镜像/标签/更多）；
      胶片条 + ‹› 按钮；移除旧左上 ⓘ 与右上 ✕ 方案；
- [ ] 验收：与 01 稿图片预览对照。

### S6 播放重排

- [x] media-playback 从 `workspace.main` 底部条改为 `workspace.modal` 播放器
      覆盖层（顶栏 + 舞台 + 控制条，6.2 下半）；音频走中性占位；
- [ ] 验收：与 01 稿视频/音频对照；播放/暂停/进度/音量/速度真实可用。

### S7 设置重排

- [x] 纵向导航六项 + 页脚（6.3）；六页控件按 02/03 稿重排；
- [ ] 打开/关闭、Esc、拖动、强调色即时生效不回归；
- [ ] 验收：与 02/03 稿对照。

### S8 统计/导出/更新

- [x] 统计模态重排（6.7-1）；导出模态状态机（6.7-2，原生保存对话框在前）；
- [x] 更新模态重排（6.7-3）；
- [ ] 验收：与 07 稿对照；导出跑一次真实流程。

### S9 状态与反馈

- [x] 扫描进度卡（Navigation侧栏，取消可用）；即时add/rescan错误可Retry；异步scan worker错误无返回通道，失败Toast/重扫未实现。
- [x] 写回失败卡 + [重试][忽略]（tagging公开精确payload重试方法）；
- [ ] gallery 操作 toast 按 6.0 重绘；
- [ ] 验收：与 08 稿对照；扫描与写回失败路径实际触发验证。

### S10 总验证与记录

- [ ] `cargo +1.97.1 fmt --all -- --check`；`check --locked -p photag-shell -j 2`；
      全量受影响 crate 测试；
- [ ] 启动 `PHOTAG_DEV_PLUGINS=all`（或核心子集）实机操作每稿主路径；
- [ ] 更新 `docs/redesign-validation.md`：逐稿记录命令、环境、观察结果、缺失项；
- [ ] 按第 2 节复核：无新 TODO/假数据；PORTS.md 同步。

---

## 8. 验证要求

1. 每个阶段至少：`fmt --all -- --check` + `check --locked -p photag-shell -j 2` 通过。
2. 涉及 crate 的现有测试必须继续通过（gallery/navigation/settings/preview/
   media-playback/i18n/ui-bridge）。
3. 界面验证 = 启动窗口实际看与操作 +（可选）截图对比设计稿；在
   `docs/redesign-validation.md` 记录：对应稿文件名、命令、环境、观察结果、
   缺失控件/状态、未验证项（Windows 原生对话框、拖放、编解码、Shell 集成等
   在实测前一律标“未验证”）。
4. 不许把“编译通过”或“无头测试通过”写成“界面完成”。

## 9. 已知偏差与本期不做（诚实清单）

- 不自绘窗口控制按钮（— □ ✕）：保留系统标题栏；设计稿顶栏从内容区顶部开始。
- 音频不伪造波形：用中性占位；设计稿波形是插画。
- 稿 05-7“当前筛选条”独立胶囊行不做：激活 chip 自带 ✕。
- 稿 07-4 的“打开次数”等设计稿字段若数据库/EXIF 查询没有来源就整行省略。
- 更新说明正文、速度等以现有 updater 服务实际能提供的为准，拿不到就隐藏该块。
- 设计稿中的示例数据（v1.3.0 示例版本、示例扫描名称等）不照抄，用真实数据。
- 工具弹层保留现状（顶栏下方），未做设计稿以外的重构。
- MCP 的 UI 工具与新布局同步的工作不在本期（除非验证需要）。

## 10. 进度台账

**2026-10-14 编码状态：代码已编写，未验证。** S2–S9的当前实现见本表备注及
`docs/development-handoff-2026-10-14.md` 的C0–C4记录；Gallery公开媒体操作面板和
Indexing终止失败通道已写，静态复核补入了缩略图批量数据库错误和回填worker panic反馈；V1–V4没有在本轮执行。

| 阶段 | 状态 | 更新日期 | 备注 |
|---|---|---|---|
| S0 主题令牌 | 完成（编译核对通过，视觉仍待逐项验收）| 2026-10-13 | theme.rs 已改为 v5 值；fmt/check 通过 |
| S1 Shell 骨架 | 进行中（编译通过，MCP 局部实机验证）| 2026-10-13 | 三段式与 inspector 槽已编译；MCP 验证检查器开关；breadcrumb 归 S3 |
| S2 Gallery | 进行中 | 2026-10-14 | 编码续接：已修复日志中类型/可见性/trait缺口；批量CurrentSelection全量解析所选ID；session日期同步Calendar；bulk action/source typed MCP与live状态、`open_media_menu`/close及媒体ID状态已写。操作面板复用Gallery真实动作；原生右键菜单保留指针触发。代码已写，未验证 |
| S3 Navigation | 进行中 | 2026-10-14 | 同屏分区/breadcrumb/紧凑行/默认折叠及真实添加/刷新处理已写；Navigation标签模态公开请求覆盖create/edit/delete/merge/move/close；代码已写，未验证，第二轮视觉待MCP重拍 |
| S4 标签弹窗 | 进行中 | 2026-10-14 | Navigation模态/parent picker/Esc与Gallery层级/近期标签/跨页真实媒体预览已写；公开 Navigation 请求及MCP可打开create/edit/delete/merge/move；Playback新增真实标签树赋值/移除；代码已写，未验证，尚欠完整交互验收 |
| S5 预览 | 进行中 | 2026-10-14 | 新顶栏/More/slider/filmstrip既有实现保留；本轮舞台改实色SURFACE_EXTREME，信息行改紧凑两列并增加File/Library/Image/Video/EXIF折叠组。代码已写，未验证；先前截图仍需新版本MCP复核布局 |
| S6 播放 | 进行中 | 2026-10-14 | 新增PlaybackSnapshot.media_id/open_media兼容入口，Gallery/Preview/MCP音视频传ID；Playback真实标签赋值和FileOperations reveal入口已写，Shell停机顺序及PORTS已更新。未验证；播放器布局仍待设计稿对照 |
| S7 设置 | 进行中 | 2026-10-14 | 纵向六项、确定尺寸、实色背景、独立滚动正文、即时保存页脚已有代码；正文空白修复未重拍，代码已写未验证 |
| S8 统计/导出/更新 | 进行中 | 2026-10-14 | 分区统计与导出/更新状态卡已有代码；MCP导出改走MetadataExportService共享状态机，get_ui_state可读状态；启动更新静默已验，其余新稿面未完整验收 |
| S9 状态与反馈 | 进行中 | 2026-10-14 | Gallery writeback失败卡支持基于失败payload的真实Retry/忽略；Navigation扫描显示进度/取消，立即add/rescan和Indexing终止错误均有Retry scan卡；图片/视频缩略图批量写入错误及回填worker panic也进入终止失败流。代码已写未验证；异步重试次数未显示，toast和整稿验收仍待后续验证 |
| S10 总验证 | 未开始 | | |

历史交接工作树快照（2026-10-13，未提交）：改动 13 个已跟踪文件
（theme.rs / shell.rs / main.rs / gallery 的 view.rs、lib.rs、card.rs、
card_motion.rs、month_header.rs、render.rs / preview 的 tag_picker.rs /
i18n 三个 locale 文件），新增未跟踪
`view/empty_state.rs`、`view/filter_row.rs`、`view/inspector.rs`、
本文件。i18n 仅登记 `Toggle sidebar`、`Toggle inspector`、`Cancel (Esc)`、
`File information` 四条；`filter_row.rs`/`inspector.rs` 的英文原文尚未登记
（见 7.5）。此时 `cargo +1.97.1 check --locked -p photag-plugin-gallery -j 2`
失败，属预期中间态，不是回归。

更新规则：状态只写 未开始 / 进行中 / 完成；每次推进同步“备注”一列
（写完成项数量或阻塞原因），并在 `docs/redesign-validation.md` 留验证记录。

### 本次接续更新

7.2 → 7.5 完成，filter_row 拆为主模块/多选弹层/公共控件，inspector
拆为主模块/格式与区域控件，Gallery scope actions 单独成文件，改动文件均
低于 800 行。旧 controls.rs 已移除；旧 regions::chip 仍被模态使用。

用户要求所有运行时调试和交互测试通过 PhoTag MCP，已强化 AGENTS.md。
启动自动更新弹窗按用户方向屏蔽，后台检查与手动更新入口保留。MCP 新增
检查器开关、手动更新检查和状态字段。实机证据与剩余项见验证记录；S2 不
标记为完整验收，00/05 所有弹层、状态与原生操作尚未全部验证。

### 2026-10-14 用户暂停开发并整理交接

用户因额度不足要求整理完整交接，所有子智能体已停止，MCP验证子进程已结束。
最新代码的fmt检查通过；Shell check失败19错，集中于Gallery新04/06模块。
S2/S3/S4/S5/S6/S7/S8均保留为进行中，不标记完成；第二轮侧栏、设置正文尺寸、
日期Calendar和批量/标签面板尚待新EXE+MCP截图。最后可运行的
`photag-shell-v5-iteration.exe`不包含这些最后改动。

详细修复锚点、公开接口、未登记文案、真实截图、验证命令和P0-P4顺序见
[完整开发交接](development-handoff-2026-10-14.md)。使用
`scripts/photag-mcp-debug.cjs`继续同进程插件MCP调试；保留启动更新静默。
