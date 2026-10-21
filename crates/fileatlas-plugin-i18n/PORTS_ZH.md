# 本地化插件对接

Cordis 插件 `fileatlas.i18n` 以 `fileatlas/i18n` 发布 `I18nService`。
它在启动前需要 `UiBridgeService`；Shell 在特性视图之前启动它，
并在消费方之后停止它。不额外引入独立的插件管理器。

## 公共契约

- `I18nConfig { path: Option<PathBuf> }` 用于选择偏好文件。默认路径：
  `%LOCALAPPDATA%/FileAtlas/language.json`。隔离的 dev-test 配置使用
  `<FILEATLAS_DEV_TEST_DIR>/language.json`；源应用的 settings/session/schema 保持完整。
- `I18nService::open(path) -> Result<Self, I18nError>` 解析内嵌的 Fluent
  资源并加载偏好设置。缺失或无效的偏好会使用英文。
- `language() -> Language` 返回 `English`（`en`）或 `Chinese`（`zh-CN`）。
- `set_language(Language) -> Result<(), I18nError>` 先持久化，然后改变
  可见语言并发布一次 UI 翻译版本更新。保存失败会让
  之前的语言继续生效。不受支持的字符串代码会被
  `Language::parse` 拒绝。
- `Translator::translate(source, &[(&str, String)]) -> String` 对具名
  参数使用 `fluent-bundle` 0.16 进行格式化。英文是回退语言。未知文本
  保持原样。`keys.json` 把英文迁移标签映射到稳定的 Fluent
  ID；翻译本身存放在 `locales/en.ftl` 和 `locales/zh-CN.ftl` 中。

## UI 归属与更新

该插件在 `settings.language` 中拥有 `i18n/language-selector`。Settings 把该贡献
挂载在“外观”面板内部。Shell 不渲染语言控件。
公开的 UI Bridge `i18n::tr` / `tr_args` 边界持有一个弱引用翻译器；
Cordis 析构时会同时移除该绑定和它所拥有的视图。Shell 会观察
翻译版本更新并刷新 GPUI 窗口。特性输入框会更新自己的占位符，
而不重置已输入的值、过滤器或选择项。

语言变化没有对应的 Cordis 事件；消费方观察公开的
版本更新。库文件名、路径、标签名、按键以及存储下来的源枚举值
仍然是数据，而不是翻译键。系统错误、由
GitHub 提供的发布说明、原生操作系统对话框，以及开发诊断信息可以保留其
原始语言。部分生成的状态/错误文本仍需纳入目录覆盖。

## 校验与失败

四个测试覆盖每个内嵌消息/参数集合、默认值/持久化、
保存失败，以及 Cordis 服务/视图/绑定的清理。MCP 校验实际使用了
两种语言、六个 Settings 标签页和真实的 FileManager/Preview 截图；
语言选择器的鼠标点击本身在仅使用 MCP 的那一轮中尚未测试。
证据与视觉限制见 `docs/validation-report.md`。目录解析错误会导致插件启动失败。偏好保存失败会出现在
语言面板中并返回 `I18nError`；MCP 通过 `control_ui` 报告它们。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动以及既有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报挂钟时间跨度的时长与错误。领域服务契约、事件载荷与源操作顺序保持不变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。
