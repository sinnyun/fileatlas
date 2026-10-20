# 声明式设置页公共契约

设置页共用 `fileatlas_ui_bridge::settings_page`，视觉源是
[十页完整概念](reference-ui/concepts/settings-complete-2026-10-18.md)。
插件声明内容，不自行设计字号、颜色、分区或设置行。主题仍由既有 `theme` 所有。

## 最小接入

在插件 `apply` 中，将下面的 contribution 交给已有
`register_views_for_plugin(&context, OWNER, [page])`。闭包捕获该插件的服务/弱引用。

```rust,ignore
use fileatlas_ui_bridge::settings_page::{
    self, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
};
let read = service.clone();
let write = service.clone();
let page = settings_page::contribution(
    "my-feature/settings", "My feature", "Feature preferences",
    move || {
        let saved = read.snapshot()?; // 只读内存，不执行文件/数据库 IO
        Ok(SettingsPage::new("", "", vec![
            SettingsSection::new("General", vec![
                SettingRow::new("capture", "Capture enabled", SettingControl::Toggle(saved.enabled))
                    .description("Capture new entries automatically"),
            ]),
        ]))
    },
    move |id, value| match (id, value) {
        ("capture", SettingValue::Bool(enabled)) => write.set_enabled(enabled),
        _ => Err("Unsupported setting".into()),
    },
);
```

例中 service 是功能自己的已有服务；示例不定义新持久化层。当前内置插件的启动方式
来自已有运行时能力声明，由设置中心直接生成；插件专用贡献附加在该插件页内。
没有可启停运行时记录的设置贡献也会自动生成导航入口。

## 模型和行为

- `SettingsPage { title, description, sections }`，`SettingsSection { title, rows }`。
- `SettingRow { id, label, description, control, disabled, error }`。
- 控件：Toggle、InlineToggle、Select、Radios、Segments、Swatches、Action、Text、Path、ReadOnly、Status、Custom。
- 编辑事件为 `SettingValue::Bool/Choice/Invoke`。字段 ID 在页内唯一，选项值不得重复；
  错误声明直接显示诊断，避免写入错误字段。
- `Text` 接受已有 InputState，订阅/校验由所有者负责；`Path` 展示路径并触发所有者的
  选择动作，渲染器不会自己打开文件、读磁盘或启动进程。异步动作/外部状态更新、
  快捷键录入、标签组和进度使用自有 Entity，调用同一 schema/公共组件渲染即可。
- `contribution` 负责简单表单的 Entity、重绘与提交失败提示。读取失败和提交失败可见，
  不吞掉错误。所有者必须在 action-time 验证目标，并只在成功后发布新状态。
- 复杂页面可用 `UiViewContribution::with_settings_page(title, description)` 注册已有
  view factory，其正文仍使用同一 schema；宿主负责标题和导航，正文使用 `render_body`。
- `Custom` 是复杂编辑器的局部插槽，不是重新画整页主题的入口。

## 所有权、刷新和存储

复用 `UiViewCatalog`，不另建设置插件管理器。`settings.pages` 的元数据和工厂与其他
view 走相同 Cordis 注册/清理，停止移除，重新注册 generation 变化时重建 AnyView。
设置中心以 ChangeGate 观察注册代次和运行状态，静止时不通知；外部异步表单由其所有者
按 refresh/ReadModel 契约提供更新，基础 contribution 不自动轮询业务数据。

公共 schema 是运行时 Rust 声明，不序列化为另一份全局 JSON 配置。现有设置继续走
SettingsService；插件业务配置沿用自己的公开服务和已确定的 JSON/SQLite 路径。
本改动没有数据格式迁移，不复制原配置，不引入新的配置文件或 SQLite 连接。

## 当前接入

外观的强调色、标签匹配/范围/合并、常规开关、内置插件启动策略已使用声明式模型。
其他页和复杂编辑器复用公共标题、分区、行、导航及原生控件；快捷键录入与冲突状态、
AI 标签组、外部软件 ReadModel、更新下载/安装继续保留各自业务状态机。
后续常规插件设置使用声明即可；复杂交互仍需功能逻辑，但无需单独设计整页样式。

公共几何与排字由 `theme::SETTINGS_*` 令牌集中管理：导航224、正文水平40、标题24、
分组标题16、控件36；设置页使用系统的 Microsoft YaHei UI 中文无衬线字体。
InlineToggle 用于色点网格下方的跟随系统选项；Segments 用于边框/阴影，避免插件自行绘制。
