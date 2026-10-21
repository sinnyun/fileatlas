# UI bridge 对接 / UI 托槽数据对接

## 原生 GPUI 主题网关（2026-10-19；当前）

GPUI Kit 的 ThemeRegistry/ThemeConfig/Theme 是唯一调色板解析器。UI Bridge
`theme::request_theme(mode: &str, json: Option<String>) -> Result<(), String>` 校验并排队一个原生 ThemeSet；None 选择原生默认值。`apply_components(&mut App)`
安装一个应用生命周期的事件监听器和原生 Theme 观察者，通过注册表应用排队的 JSON，
并且仅在应用了新请求时刷新窗口。它不执行任何
文件系统 IO 或定时器轮询。它是宿主基础设施，不是第二个插件管理器。

`theme::roles` 将现有自定义控件的角色名映射到原生解析后
ThemeColor 的字段。一个只读、原子替换的投影支持无法访问 GPUI App 的
辅助函数/MCP 线程。它在每次原生 Theme 更新时刷新；它不拥有任何
颜色配置，也不含任何模式调色板。新视图可以直接使用 ActiveTheme。内容对比度辅助函数选择现有的原生
前景角色；它们不再混合或插值出新的 UI 调色板。共享 nav_button
使用原生 primary/selected 变体，由 Kit 负责悬停/聚焦/禁用的呈现。
此前所有固定 RGB 常量和 theme/components.rs 覆盖表都已移除。
`theme_name()` 和 color_mode() 报告实际已应用的框架主题，而非某个请求。

Material 生成是一个独立的 Cordis 插件，产出标准 ThemeSet JSON；
UI Bridge 不实现 Material、不读取其数据文件、也不选择种子。Settings
拥有 mode/theme_file 偏好以及操作时/启动时加载。缺失/无效 JSON
在那里连同诊断信息一起回退恢复为原生默认值。几何值仍位于 theme.rs；
元数据 schema 与生命周期目录保持不变。

`components::WorkbenchStyle` 将分区、可选卡片和导航应用到
任意 `Styled + InteractiveElement`，并保留 ID/操作。`section()`、
`action(id, label, primary)`、`icon_button(id, IconName)` 和 `section_heading(label)`
是无状态构建器。它们不注册任何服务/事件，也不执行 IO。
`text_input(&Entity<InputState>) -> Input`、`search_input(&Entity<InputState>) -> Input`
和 `textarea(&Entity<TextareaState>) -> Textarea` 保留原生编辑/聚焦行为，
并带通用内缩背景。Kit `Theme.colors.input` 是边框令牌；
所有插件输入现在都使用这些构建器。主题映射还覆盖表头/列表头与交替行、
手风琴、语义状态按钮和图表颜色。
Clipboard、Launcher、FileManager 检查器/统计/批量操作标题和 Settings 使用
这些构建器；其他 Kit 控件继承单一主题。`panels` 保留
浮动生命周期/几何所有权。参考/开发契约：
[`docs/ui-theme.md`](../../docs/ui-theme.md)。用
`node scripts/check-ui-theme.cjs` 和同一进程的 MCP 截图诊断主题漂移；
令牌所有权并不证明每个运行时状态都经过了视觉验证。下方的
源码调色板笔记描述的是历史兼容 API；当前调色板是工作台深蓝色。

状态：生命周期拥有的元数据与 GPUI 视图工厂目录已存在；shell 挂载侧栏、内容、辅助和浮动模态槽位。本 bridge 提供视图摆放与面板可见性状态；精确的原生窗口化与视觉一致性仍是宿主职责。

该 bridge 还从 `theme` 暴露共享的 FileAtlas 设计令牌与强调色状态，以及从 `winfx` 暴露 Windows 外观原语。这些是给消费者的契约；它们不会自动重新美化已挂载的 GPUI 视图。Shell 对其原生 HWND 应用 `winfx::apply_backdrop`，并将 `Backdrop::is_translucent()` 传给 `theme::set_backdrop_active`；Shell、Navigation、Playback 和 FileManager 消费带背景感知的基础/卡片填充色，而 Settings、FileManager 对话框与 Preview 消费模态填充色。

## 生命周期与输入/输出

- Cordis 服务：`fileatlas/ui-bridge`，由 shell 在功能插件启动之前发布。
- `UiSurfaceState` 在 shell 宿主与其所属插件之间共享浮动面板的打开/关闭、快捷键捕获以及一次性 Escape 抑制状态。面板所有者更新自己的视图并注册对应命令；宿主将 `workspace.modal` 视图挂载在内容之上。状态仅限于本应用运行时，并随 bridge 一起重建。
- `register_for_plugin(context, owner, contributions)` 注册 Layout/Dock/Panel/Slot/Widget/Command/MenuItem/Keybinding 元数据；重复/空 ID 会以原子方式拒绝整批注册。Cordis 处置（disposal）会注销该所有者的全部条目。
- `register_views_for_plugin` 在槽位 ID 下注册可执行的 GPUI 视图工厂；返回的 `OwnedUiView` 实例由 shell 挂载。视图目录清理跟随所有者生命周期。
- Shell 已挂载的槽位按贡献 ID 加注册代（generation）进行调和。Navigation 对其嵌套的 launcher/add-on 槽位做同样处理；Settings 在其面板内调和 `settings.language`。缓存了已构造 `AnyView` 的宿主必须在目录条目消失或其代号变化时丢弃它们。
- `UiCommandInvoked` 通过 Cordis 将命令 ID 传递给插件处理器。
- `ContributionKind::Command { label, needs }` 是生命周期拥有的命令
  注册表条目。`CommandNeeds` 声明调用策略；`UiCatalog::commands()`
  将命令条目与其拥有的菜单项和键绑定连接起来。条目随其
  插件所有者一起移除。该注册表是元数据与分发契约；
  功能处理器仍负责校验操作时的状态并拥有执行权。
- `theme` 导出源码调色板值（`0xRRGGBB`）、带显式 alpha 的源码预乘控件/边框颜色、尺寸、文字大小、Segoe 家族名、MDL2 字形、七种强调色预设，以及原子的当前强调色。`PremultipliedColor::to_rgba()` 将源码颜色转换为 GPUI 的直接 alpha `0xRRGGBBAA` 格式；`base_rgba()`、`modal_rgba()` 和 `panel_rgba()` 应用源码的 Mica/Acrylic alpha（基础 0xDE、模态 0xF4、卡片 0xE8）。Shell/基础层、Settings 与 Preview 模态、FileManager 对话框/卡片、Navigation 和 Playback 面板都消费这些辅助函数。Settings 消费者在恢复或更改已保存预设后调用 `set_accent_name`；GPUI 视图在渲染时使用 `accent()` 和共享令牌常量。该令牌目录不安装字体，也不能证明 GPUI Kit 的命名文字大小能解析为源码字体面。
- `winfx::system_accent()` 读取 Windows AccentPalette/DWM 注册表值，并通过按比例提升所有通道直到最亮通道达到 150 来提亮颜色，与 FileAtlas 的 `ensure_min_brightness` 一致；黑色映射为中性灰。`theme::normalize_accent()` 为传入颜色的调用者保留同一规则。`winfx::font_availability()` 报告源码字体文件的可用性；GPUI 在文字排版时仍自行解析字体家族名与回退。
- `winfx::apply_backdrop(hwnd)` 应用深色标题栏，并在支持时返回 Mica（Windows 11）或 Acrylic（Windows 10）。它是原生窗口创建之后的 shell 启动操作，不是插件 UI 贡献。非 Windows 调用会安全地返回无强调色/无背景材质。

## 集成规则 / 调试

使用全局唯一且稳定的贡献 ID 和稳定的槽位名；菜单/键绑定条目引用命令 ID。只在 bridge 发布之后注册，并且不要跨插件代（generation）缓存视图实例。如果视图缺失，检查所有者名、槽位名、重复注册、bridge 启动顺序和目录清理。

外观调试：确认设置消费者调用了 `set_accent_name`，用 `font_availability` 确认已安装的系统字体家族名，并检查针对真实 HWND 返回的 `Backdrop`。注册表探测、字体解析、标题栏材质与透明度仍需 Windows 运行时验证。共享 bridge 尚未提供按插件的主题清理，因为令牌与强调色是应用级的 shell 状态。

## 翻译与面板值契约（2026-10-13）

`i18n::Translator` 是公开的渲染边界；i18n 插件提供一个
弱实现。`tr(source)` 与 `tr_args(source, named_strings)` 使用它，
不存在时回退到源文本。install/changed/revision 支持插件
清理与 GPUI 刷新而不长期持有提供者。UiSurfaceState 的 value /
set_value 在现有打开/关闭状态之外携带 Settings 的标签页请求。
`theme::apply_components(App)` 将源码的幽灵控件表面与
Segoe 排版应用到 GPUI Kit 控件，避免早先的浅色/灰色默认值。

`theme::apply_accent_components(&mut gpui_kit::App)` 是 GPUI 线程上的
运行时强调色同步入口。在 Settings 中调用 `set_accent_name` 之后调用它；
它只按现有的实色/64-alpha 规则更新 primary/accent/selection。
`Theme::update` 调和组件令牌与 Base 投影并刷新
窗口；该运行时辅助函数不重置排版、几何或其他颜色。

## 浮动面板契约（2026-10-14）

`panels::mount(AnyView, PanelOptions, Arc<UiSurfaceState>, visible, close, &mut App) -> AnyView` 提供一个共享的可拖拽/可缩放框架，带最大化/还原/关闭。选项携带稳定实例 ID、翻译后的标题、独立的默认/最小逻辑尺寸。实际的 GPUI 内容/客户区边界会被发布；Preview 使用 `panels::content_bounds`。宿主提供一层背景遮挡并按 `panel.active` 对 `UiViewContribution.with_panel_id` 视图排序。一个仅用于拖拽的捕获表面绘制在框架之上，使框架遮挡不会吞掉移动/抬起事件。

`UiBridgeService.panels: Arc<PanelCatalog>` 暴露 register/open/close/unregister_owner。`register_panel_for_plugin(Context, owner, PanelDefinition)` 将清理绑定到 Cordis。定义包含选项、Single/Multiple 策略和真正的 GPUI 工厂。Single 打开时返回/聚焦其现有 ID；Multiple 返回新 ID 并在挂载时创建独立的内容实体。关闭会移除该实例贡献；所有者停止会移除定义、实例、打开标志与视图。UiViewCatalog 附加一个单调分配的注册代；宿主挂载时比较 ID 加代号，以避免跨插件重启保留实体。由服务驱动的既有视图使用 mount 并保留其原有服务语义。

窗口内部几何只在每个实体生命周期内持久；不提供原生分离窗口和自动几何持久化。内置 Preview/Playback 仍是单会话服务。运行时证据见 docs/floating-panels.md 和 docs/validation-ui/2026-10-14-panels。

## 运行时诊断（2026-10-14）

运行时调试集成：workspace.bottom 是 shell 底部停靠槽位，workspace.debug-status 是页脚后缀。UiSurfaceState::set_open 只在实际可见性转换时写日志。注册、代号与 Cordis 拥有的清理语义保持不变。

## 以键索引的后台读模型（2026-10-15）

`read_model::ReadModel<K,V>` 是一个共享的展示层工具，不是服务或
插件管理器。`update(key, FnOnce() -> Result<V,String>) -> bool` 应放在
操作/模型轮询器中；`value() -> Option<&Result<V,String>>` 是渲染
边界。它一次只放行一个 worker，将编辑合并到最新键，
忽略过期完成结果，缓存成功与失败，并对
未变化/空通道轮询返回 false。Worker 断开/启动失败会变成
缓存的错误。已经丢弃接收器的视图不会被那次读取所影响；
已在进行的读取可以完成，但不得变更应用状态。
所有者插件/视图仍拥有其任务与失效标识。Settings
使用 Navigation 的标签 revision；FileManager 对话框使用索引 revision 加输入项。
该工具不拥有任何持久化、Cordis 注册或数据库访问。

## 统一刷新契约（2026-10-15）

2026-10-16 就绪性扩展：`refresh::Wake: Clone + Default` 暴露
`trigger()` 与异步 `wait()`，由单个合并的 Notify permit 支持，不带
载荷也没有空闲定时器。`refresh::listen(cx, wake, FnMut(&mut V,
&mut Context<V>) -> bool) -> Task<()>` 通过弱实体采样，只在
发生真实变化时通知。把返回的 task 存入视图；丢弃它
即取消监听。每个信号只有一个消费者；这不是广播。
`ReadModel::set_wake(Wake)` 可选择接收完成/断开提示。结果
发送端会在 panic 提示送达之前断开。现有 poll 标识、
过期结果与缓存失败的规则仍然权威。Navigation/FileManager
保留其 100/200ms 回退采样器。参见 navigation-latency 验证。

`refresh::poll<V>(Context<V>, Duration, FnMut(&mut V, &mut Context<V>) -> PollResult)` 拥有弱实体定时器，并且每个有变化的采样只发一次通知。`PollResult::until(changed, active)` 在停止前送达最后一次终结变化。`ChangeGate<T>::new/changed/current` 集中管理被动的签名相等判断。

`ReadModel::request` 声明一个需求而不在渲染中接收结果；`poll` 送达它，`update` 为模型轮询器把两者结合。`key/is_pending/value` 暴露状态。每个模型一个运行中的 worker 加一个最新排队的闭包；过期结果不能覆盖当前键，A→B→A 会复用 A，worker 断开时缓存一个错误。两个工具都不输出日志。调用者拥有语义化失效键；不涉及 schema/服务/插件生命周期变更。参见 docs/ui-refresh-contract.md 和 scripts/check-ui-refresh.cjs。

### 浮动面板文本输入（2026-10-17）

面板根节点包含快捷键/导航按键事件，并允许字符事件继续传递给聚焦的输入处理器。这使得同一进程的 GPUI 文本输入路径在被托管的功能面板内保持可用。Escape 仍会关闭当前面板。启动器验证通过 `control_ui input_text` 使用真实聚焦的搜索/名称输入，不使用剪贴板或系统输入注入。
# 共享侧栏区块（2026-10-18）

`components::sidebar_section(Div) -> Div` 应用背景表面、主题区块
分隔、紧凑内边距/圆角和不收缩的框架几何。
`sidebar_header(label, toggle: Button, menu: impl IntoElement, trailing: Vec<Button>)`
返回共享的折叠箭头/标题/菜单/操作行；下拉包装器保留其自身
原生行为。Navigation 为驱动器、快速访问和
每个收藏分组使用这些构建器。构建器不注册资源，也不拥有产品数据。
# 声明式设置页面 — 2026-10-18

`settings_page` 拥有 `SettingsPage`、`SettingsSection`、`SettingRow`、类型化的
`SettingControl`/`SettingValue`，以及通用的头部/分区/行渲染。
`settings_page::contribution(id,title,description,snapshot,apply)` 构建简单
表单而无需插件本地布局。快照仅存于内存；处理器拥有新鲜的
校验与持久化。文本控件使用所有者管理的 InputState；路径与
操作控件调用所有者动作。复杂的异步编辑器使用 Custom/local Entity
加同一个渲染器，而不是第二套主题。

`UiViewContribution::with_settings_page` 提供 `SettingsPageMetadata` 与
`settings.pages` 槽位。注册/移除使用现有的 UiViewCatalog 与
Cordis 效果清理；不引入并行注册表/服务。Settings
按所有者/ID/代调和并丢弃过期视图。Schema 错误与提供者
读/写错误保持可见。不新增持久化文件；完整契约与
示例：`docs/settings-page-contract.md`。


## 独立原生面板 — 2026-10-18

`winfx::restore_and_activate(hwnd)` 只对最小化的窗口使用 SW_RESTORE；
其余情况用 SW_SHOW 以保持最大化几何。它会尝试
SetForegroundWindow 一次并记录被拒绝的情况。Shell 另外使用 GPUI 的平台激活。

`PanelCatalog::open_presented(definition_id, native: bool) -> Result<String,String>`
保留生命周期所有权和 Single/Multiple 语义。`open` 委托给嵌入模式。
`native_panels() -> Result<Vec<NativePanel>,String>` 向 GPUI 宿主暴露实例 ID、PanelDefinition
（工厂/选项）以及单调递增的激活代。原生实例
不会注册为 workspace.modal 视图。重新打开一个 single 原生实例会使其激活计数递增；
切换模式会关闭先前的实例。`close`/`unregister_owner` 同时移除
原生记录与面板。不引入新的存储或插件管理器。`workspace.activate`
只是宿主激活请求，仅用于嵌入模式打开。原生窗口几何是临时的，
使用功能默认/最小尺寸；现有的工作区面板/会话几何保持不变。

`close_definition(definition_id) -> Result<(),String>` 关闭该工具注册的实例而不使用全局活动选择；截图管理器的内部 Close 在两种模式下都使用它。
