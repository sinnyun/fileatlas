# Material 主题生成器插件对接

## 身份、依赖与生命周期

- Cordis 插件 `fileatlas.material-theme`，服务 `fileatlas/material-theme`（`MaterialThemeService`）。
- 需要公开的 `SettingsService` 与 `UiBridgeService`；可选的宿主 `McpToolRegistry`。
- 编译期插件在 Settings 之后启动。插件停止后没有定时器或工作线程会存活。
- Settings 视图 `material-theme/settings` 位于 `settings.pages`；元数据标题 Material 主题生成。
  Cordis 拥有的视图注册会在停止时移除它。MCP 注册守卫会移除两个工具。
- 它只产出原生 GPUI ThemeSet JSON。它从不调用 Theme::update/change，也不维护
  单独的 UI 调色板。已有的 UI Bridge/原生 GPUI 基础设施解析每一次绘制。
  生成的 JSON 在生成器停止后可以保持被选中状态：它是宿主读取的用户数据，
  不依赖已停止的生成器的服务或 UI。

## 公开负载

`Preferences { schema_version: u32, seed: String, variant: String, contrast: f64 }`。
版本 1；seed 为 #RRGGBB；variant 为 neutral/tonal-spot/monochrome；有限的 contrast 0..1。
默认 Neutral 为低彩度。Zero 是 Material 的标准对比度，而不是一个承诺，
保证任意用户着色的媒体内容都满足相同的主题角色配对。

`MaterialThemeService::snapshot() -> State` 仅存在于内存中。
`generate(preferences: Preferences, apply: bool) -> Result<State, String>` 校验输入，
对两种模式都使用 MCU DynamicScheme，把角色转换为原生 ThemeConfig JSON，
在单个操作锁下序列化写入，并可选地通过 Settings 选择该文件。
`apply_saved() -> Result<(), String>` 校验已保存的 ThemeSet 并选择它。
`State { preferences, output: PathBuf, error: Option<String>, generated: bool }`。
MCP 将领域错误序列化为 `last_error`，以免它与一次失败的工具调用相混淆。
seed 使用 GPUI Kit 原生的 `ColorSelect` / `ColorPickerState`，而不是文本输入
或插件本地的色板表。其内置的多色阶调色板与 HSLA 滑块
来自已有的开源 `gpui-component 0.7.2` / `gpui-base 0.7.2`
（经由被锁定的 `gpui-kit 0.7.1`，Apache-2.0）。没有添加额外的 GUI 依赖。
在 Generate 之前，选择仅为草稿；悬停预览不会提交或写入。
选取器可能暴露 alpha，但 Material 的 seed 只使用其 RGB，输出为不透明。
`ColorPickerEvent::Change` 订阅属于该视图，并随其一同丢弃。
该视图使用共享的声明式 SettingsPage 起草输入；generate/apply 是显式的
用户操作。Render 不读取文件、不生成颜色、也不改变 Settings。

MCP `material-theme.state {}` 为只读。`material-theme.generate { seed, variant?,
contrast?, apply? }` 默认为 neutral/0/true，属于变更操作且需要 Standard。
两者都不是高影响操作。宿主强制执行访问权限，插件拥有 schema/handler 与清理。
工具不接受任意输出路径，也不改变媒体数据。

## 持久化、兼容性与恢复

选择 JSON 是因为这是一个小型的整文档配置，而非查询密集型数据。
Root = SettingsService.path().parent()/themes；正常情况为 `%LOCALAPPDATA%/FileAtlas/themes`，
MCP profile 为 <profile>/themes。没有 SQLite、独立存储管理器或二进制资源。

- `material-settings.json`：带版本的 Preferences（schema_version=1）。
- `material.json`：原生 GPUI ThemeSet，包含具名的 light/dark ThemeConfig 条目。
  支配该文件的是 GPUI 已有的 schema，而不是私有的插件主题 schema。
- 已有文件被备份为 `.json.bak`；写入先暂存到 `.json.tmp` 再重命名。
  每次文件替换都是原子的；这两个文件不构成一个事务。对于已应用的颜色，ThemeSet 是权威来源；
  失败会被报告，且保留的备份可以被显式恢复。
- Settings 保留 color_mode 并添加可选的 theme_file。缺失该字段时选择原生默认值。
  旧的 accent_name 作为元数据保留，而不是另一个颜色来源。
- 被选主题缺失/无效时回退到 GPUI 默认值，并记录确切问题，而不
  删除用户的 JSON。无效/未知版本的 preferences 会加载默认值并显示一个可见的
  生成器错误，并保留该文件直到下一次显式生成。
- 无效的 seed/variant/contrast 不会替换上一次输出。渲染期间不发生任何 IO。

## 上游与许可

`material-colors = 0.4.2`，crate VCS `3ffe880a21dbcfde28504af021600799d0c5882c`，
https://github.com/Aiving/material-colors。这是 Google Material Color
Utilities（https://github.com/material-foundation/material-color-utilities）的社区 Rust 移植。
被锁定的 0.4.2 支持本 workspace 的 MSRV；0.5.0 需要 Rust 1.97，而 workspace 声明的是 1.95。
MIT OR Apache-2.0；分发时保留所包含的 licenses/ 声明。Workspace 保持 MIT。
桥接层不复制 MCU 算法。状态 seed 色相来自 GPUI 内置的语义
状态颜色；生成器通过 MCU 创建它们的角色配对，没有插件绘制表。

## 调试与限制

检视 Settings → Plugin settings → Material 主题生成、`material-theme.state`、生成的
JSON、宿主 Debug 日志，以及 get_ui_state.theme（framework/name/mode/解析后的角色配对）。
Generate 会在 State 中以及其拥有的视图中报告校验/写入错误；Settings 持久化
错误会被返回。GUI 应用失败会在 UI Bridge 中记录日志；没有静默的备用调色板。
测试覆盖跨极端 seed/variant 的原生 JSON 解析/对比度，以及失败/备份
保留。运行时/视觉证据记录在 docs/validation-ui 之下；编译不是
每一个自定义组件状态的证明。没有引入动态 DLL/商店/云服务。
