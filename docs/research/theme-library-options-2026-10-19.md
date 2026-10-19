> 后续实施状态：用户已选择 GPUI Kit 原生主题 + Material JSON 生成插件。本文件保留选型时的现状与方案比较；当前契约以 docs/ui-theme.md、AGENTS.md 和 photag-plugin-material-theme/PORTS.md 为准。生成器最终固定 0.4.2 以符合工作区 MSRV，未采用调研时的最新版 0.5.0。

# 主题配置开源方案调研（2026-10-19）

范围：核对现有依赖和实现，搜索能减少手写配色的开源能力；本记录不实施主题迁移。兼容性判断以本机已锁定源码为准，网上最新版只用于补充选型信息。

## 当前情况

现状是“现成 UI/主题框架 + 自定义配色层”。不是从零实现按钮和主题框架，但大部分颜色来源、亮暗解析、文字对比度和组件主题覆盖是项目手写的。

- `Cargo.toml:56` 固定 `gpui-kit = "=0.7.1"`；当前 `Cargo.lock` 实际解析到 `gpui-component 0.7.2`。Kit 的 `component` 已导出组件库主题 API。
- `crates/photag-ui-bridge/src/theme.rs` 定义大量 RGB/RGBA、模式访问器、强调色与选中底色；`theme/contrast.rs` 实现混色、对比度和内容徽标的配色。
- `theme/components.rs::apply_components` 先调用框架 `Theme::change`，再逐项覆盖按钮、输入、菜单、表格、侧栏等颜色，因此实际采用的主要是项目 palette。
- 项目代码没有使用 `ThemeRegistry`、`load_themes_from_str` 或 `Theme::apply_config` 加载现成主题。自绘视图读取项目的无 App 上下文颜色访问器；框架控件读取 `Theme`，形成需要手工保持一致的两套颜色表示。

## 候选方案

| 方案 | 实际提供什么 | 对当前项目的接入方式 | 适合程度 / 限制 | 软件许可 |
|---|---|---|---|---|
| GPUI Kit / GPUI Component ThemeRegistry | 主题 JSON、模式切换、注册表、组件状态色、语义 token；仓库提供多套主题 | 当前依赖已包含，无需另装主题管理器；选主题 JSON，使用公共 Theme 作为颜色来源 | 首选；自绘部分仍要改读语义角色；不是自动对比度生成器 | Apache-2.0；单个外来主题保留其来源和许可 |
| Material Color Utilities 的 Rust 移植 `material-colors` | 从种子色生成亮暗方案、成对的 on-color 角色、色调与对比算法 | Rust 库生成颜色，再适配到现有 GPUI Theme | 适合需要任意强调色自动生成整套主题；需额外角色映射和移植版本核对 | MIT OR Apache-2.0 |
| Radix Colors | 按用途组织的 12 阶亮/暗色阶与透明变体，自定义色阶工具 | 将上游颜色数据映射到 GPUI ThemeConfig；无需在应用运行 npm | 适合稳定的灰阶/蓝灰及状态色；官方包面向 JS/CSS，不能直接当 GPUI 主题管理器 | MIT |
| Catppuccin Rust / 现有 GPUI Catppuccin JSON | Latte、Frappé、Macchiato、Mocha 的协调预设色 | 优先直接用 GPUI 仓库已适配 JSON；另一个选择是 Rust palette crate | 适合作为柔和预设；调色板自身不覆盖全部按钮/hover/disabled 语义，也不自动生成任意强调色方案 | Rust palette MIT；JSON 按文件来源核对 |

### 1. GPUI 原生方案：当前就有，优先收敛到它

已直接核对本机 Cargo registry 下 `gpui-component-0.7.2/src/theme/`：

- `registry.rs`：`ThemeRegistry::load_themes_from_str`、`watch_dir`、`themes`、`sorted_themes`。
- `schema.rs`：`ThemeSet`、`ThemeConfig`、`ThemeConfigColors`；颜色/前景/hover/active 字段由框架解析并应用。
- `mod.rs`：`Theme::change`、`Theme::update`、`apply_config`、`semantic_tokens`、`apply_semantic_config_str`；支持把当前主题同步到 Base 层。
- `schema.rs::SemanticThemeConfig` 已有颜色、字号、间距、圆角、阴影的语义配置。
- `default-theme.json` 随 crate 内嵌 Default Light / Default Dark，注明以 shadcn neutral 配色为来源。其它主题在上游 `themes/`，需要选择并随产品打包，不代表当前 PhoTag 已内置这些主题。

官方文档提供从注册表查找并应用主题的用法；仓库已有 Catppuccin 等主题 JSON。外观可以先比较框架默认中性色和 Catppuccin Latte/Mocha，具体是否符合低饱和度要求仍需在 PhoTag 的实际图片网格/标签中确认。目录热加载是可选能力；初次迁移可优先内嵌 JSON，不必增加文件监听生命周期。

来源：[官方 Theme 文档](https://gpui-kit.com/component/theme/)、[仓库主题集](https://github.com/longbridge/gpui-kit/tree/main/themes)、[Catppuccin JSON](https://raw.githubusercontent.com/longbridge/gpui-kit/main/themes/catppuccin.json)。

### 2. Material：减少“每个强调色都手工配一遍”

Google 上游 MCU 提供动态方案、对比算法、色调 palette 和颜色混合；当前官方列出的语言未包含 Rust。`material-colors` 是社区 Rust 移植，文档提供 `ThemeBuilder` 从一个源色创建方案、JSON 序列化支持等能力。它能作为颜色生成后端，但不会直接给 GPUI 的所有控件装配主题。

若后续要求用户自由选择强调色，并同步生成背景、容器、正文与 on-color，再评估这一层更有价值。采用低色度方案可以作为设计方向，但最终低饱和程度和控件对比需在选定版本生成结果上检查。本轮未添加依赖或编译验证它，不作已兼容声明。

来源：[Google MCU](https://github.com/material-foundation/material-color-utilities)、[Rust 移植仓库及许可](https://github.com/Aiving/material-colors)、[Rust API](https://docs.rs/material-colors/latest/material_colors/)。

### 3. Radix：用途明确的现成色阶

其色阶按应用背景、组件表面、边框、实心填充、文字等用途分层。9/10 阶用于实心填充/hover，11/12 阶用于文字；某些亮黄色系实心填充需要深色前景，不能统一配白字。此规则直接对应此次图片角标的错误类型。它仍要求调用方选对角色，不能保证任意色阶、透明背景和前景随意组合都合格。

来源：[色阶用途](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)、[自定义色阶](https://www.radix-ui.com/colors/docs/overview/custom-palettes)、[源码与 MIT 许可](https://github.com/radix-ui/colors)。

### 4. Catppuccin：现成预设，优先复用已有适配

官方 Rust crate 直接提供 palette，也有 serde 等可选特性。对本项目更省事的路线是使用已有 GPUI JSON，而非新增 crate 后再手写一遍组件映射。palette 本身不是主题注册系统，也不能取代语义角色选择。

来源：[Catppuccin Rust](https://github.com/catppuccin/rust)、[GPUI 已适配配置](https://raw.githubusercontent.com/longbridge/gpui-kit/main/themes/catppuccin.json)。

## 建议的迁移方向（待实施）

1. 由现有 UI Bridge 集中加载/应用框架 ThemeConfig，Settings 插件继续持有主题选择与模式的持久化，Cordis 继续持有生命周期。
2. 删除大面积“先套框架主题、再覆盖成自制 palette”的重复映射。普通插件直接消费框架语义 token；现有 `theme::*` 可以短期作为兼容适配，返回当前框架主题，不继续维护另一套 RGB 表。
3. 先采用一套成熟的亮暗 JSON 主题。产品几何、窗口布局和持久化保持独立；主题库不应重排工作区。
4. 缩略图上的固定深色覆盖信息、用户标签色、评分、格式色等内容角色仍在公共桥接层集中适配，显式配对 foreground/background。库无法自动识别所有自绘 Div 究竟是图片覆盖层还是普通面板。
5. 需要动态种子色时，再把 MCU 生成结果填入同一个 ThemeConfig 管线。对比检查作为验收保留，避免演变为另一套手写颜色引擎。

判断：原生主题注册表的接入成本最低，主要工作是替换颜色消费路径；Material 的生成能力更强但需额外适配；Radix/Catppuccin 更适合作为配色来源或预设。shadcn/next-themes 等网页方案的运行机制面向 CSS/DOM，对现有原生 GPUI 不能直接安装生效；GPUI 默认 JSON 已复用了 shadcn 的设计来源，可优先使用这层现成适配。
