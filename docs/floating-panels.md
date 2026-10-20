# 通用弹出面板

2026-10-14，按本轮用户要求，在现有 `fileatlas-ui-bridge` 中提供可复用组件和公共面板目录。无需新建插件管理器；定义、视图工厂和实例继续由 Cordis 插件生命周期拥有。

## 新功能插件按需打开

在插件 `apply` 中注册定义，工厂返回该插件真实的 GPUI `AnyView`：

```rust
use std::sync::Arc;
use fileatlas_ui_bridge::{UiBridgeService, panel_catalog::*, panels::PanelOptions};

register_panel_for_plugin(&context, "my.feature", PanelDefinition {
    options: PanelOptions {
        id: "my.feature/details".into(),
        title: "Details".into(),
        default_size: (760., 540.),
        min_size: (420., 300.),
    },
    multiplicity: PanelMultiplicity::Multiple,
    factory: Arc::new(move |window, app| make_details_view(window, app)),
})?;

let bridge = context.try_service::<UiBridgeService>()?;
let instance_id = bridge.panels.open("my.feature/details")?;
// 关闭指定实例：bridge.panels.close(&instance_id)?;
```

`Single` 重复打开返回相同实例 ID 并将它置前；`Multiple` 每次创建新的内容实体和独立几何状态。实例 ID 为 `definition_id@sequence`，不要自己猜序号。关闭会注销实例视图；插件停止会删除该插件的定义和所有实例。每个定义的默认／最小尺寸均由插件单独设置，单位为逻辑像素。注册拒绝空 owner/ID、重复定义、非有限或非正尺寸。

Host 根据 UI View Catalog 的登记代次增量装载，避免插件停止再启动时复用旧实体。点击可见面板或打开实例会将它置前；标题栏支持拖动，四边及四角支持缩放，右上角支持最大化／还原和关闭。尺寸与位置限制在当前 GPUI 窗口内；拖动期间顶层捕获区持续接收事件。所有打开的面板共用一层阻挡主界面的遮罩，各面板仅在自身矩形内遮挡其他面板。

## 接入已有服务驱动的视图

设置、图片预览、音视频播放已有自己的服务和打开／关闭流程，使用 `panels::mount(content, options, surfaces, visible, close, app)` 包装原内容；其 `UiViewContribution` 调用 `.with_panel_id(id)`。`visible` 读取该插件的真实状态，`close` 调用原关闭行为。不要在 host 里复制产品逻辑。独立实例须使用独立 ID、内容实体和状态提供者。

组件发布 `panel.<id>.bounds`（相对工作区的 frame 几何）、`client_bounds`（GPUI 测量边界）和 `content_bounds`；`panels::content_bounds` 提供内容实际客户区坐标，预览缩放和右键菜单定位必须减去这个原点。几何目前保留于当前实体生命周期，不写入用户设置；重开同一常驻视图保留尺寸，程序重启采用插件默认尺寸。文件管理窗口布局偏好独立保存为 `SessionState.file_manager_list_layout`，旧会话默认为网格。

## 验证与边界

MCP `get_ui_state` 提供 `panels`、`panel_instances`、`open_panel_ids`、`active_panel` 和 `file_manager.layout`。`control_ui panel_open/panel_close` 使用定义／实例 ID；只可操作已注册定义。`dev-test` 注册单实例和多实例的真实开发任务状态面板，供隔离验证使用。无 `dev-test` 时不存在这些定义。

这是应用内部浮动面板，不是独立 OS 窗口。跨窗口拖出、几何重启恢复和多媒体多会话解码不属于当前组件承诺；现有 Preview/Playback 服务各保持单媒体会话，新插件的多实例内容由其自身服务设计决定。当前运行证据见 `validation-ui/2026-10-14-panels/README.md`。
