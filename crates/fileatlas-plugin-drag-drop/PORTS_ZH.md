# Drag-out 插件对接

状态：FileManager 保留 GPUI 的卡片手势与所选文件解析。它的
回调向同一 UI 线程上的一个临时纯消息窗口投递一个请求，然后返回。Windows 会在发起
输入回调展开（unwind）之后派发该请求；派发器随后使用被锁定的
`drag` crate 2.1.1 启动原生会话。当原生会话
返回时，派发窗口被销毁。用户已确认在 Windows 上成功投递到其他窗口；
2026-10-19 的记录覆盖了当前的预览图片行为以及
其余的视觉确认。

## Cordis 身份与生命周期

- 插件：`fileatlas.drag-drop`（`DragDropPlugin`、`DragDropConfig`）。
- 服务：`fileatlas/drag-drop`（`DragDropService`）。它没有插件
  依赖，并在活动状态下发布其服务。Cordis 拥有
  发布权，并在插件停止时移除它；该服务不注册任何
  UI 贡献。它的派发窗口是临时的，归某个排队的拖拽
  请求所有。
- Shell 在本 Cargo workspace 中包含该 crate，并在 FileManager 之前启动
  `DragDropPlugin`。FileManager 在初始化期间解析该服务，
  并从卡片拖拽手势中调用它。

## 公开操作

- `DragDropService::start_file_drag(window, &[PathBuf]) -> Result<(),
  DragDropError>` 使用源窗口句柄排队一个仅复制的原生拖拽请求。该调用在 OLE 开始之前返回；投递的派发器回调
  仍留在 GPUI UI 线程上，并在放下或取消之后返回。空输入
  是一个无操作。第一个路径提供拖拽图像；所有路径按顺序
  投递。可解码的图片预览会被缩小以适配 96×96 像素之内。
  没有可解码图片预览的文件使用一个紧凑的文件剪影，带
  扩展名标签与类型颜色。当已有一个请求排队或活动时，第二个请求会被拒绝。
- `drag` crate 会对路径进行规范化并构造 Windows Shell 数据
  对象。其允许的效果默认为 copy，因此原始文件不会被
  本功能移动。队列/窗口创建失败会以 `DragDropError::Platform` 同步返回；后续原生
  会话中的错误会从派发器窗口过程记录日志。
- FileManager 拥有路径选择，并传递所选媒体路径（与
  FileAtlas 一致：拖拽一个已选中的卡片会导出完整选区；拖拽一个
  未选中的卡片只导出该卡片）。它在
  排队拖拽之前释放对模型的借用（borrow）。

## 事件与失败检视

- 该服务不生产也不消费任何 Cordis 事件。检视
  `src/dispatch.rs` 以了解延迟的同线程派发与临时窗口
  清理，检视 `DragDropError` 以了解队列失败，检视被锁定的 `drag` 2.1.1
  实现以了解 OLE/数据对象行为。
- 该依赖以 `Apache-2.0 OR MIT` 授权，锁定在 `Cargo.toml` 与
  `Cargo.lock` 中。它仅用于运行时拖拽会话，不引入
  任何持久化状态或迁移。

## 源码映射与实现参考

- `vendor/photag-source/src/drag.rs` 包含 PhoTag 最初手写的
  `IDataObject`/`IDropSource` 与同步 `DoDragDrop` 调用。它在
  `vendor/photag-source/src/ui/gallery.rs` 中的调用者解析所选 ID，并在 `egui`
  卡片的 `drag_started` 块内启动拖拽。它定义了
  负载与仅复制行为，但那个同步调用点并不是一个安全的
  GPUI 模板。
- GPUI `gpui-pre` 0.3.8 暴露 `external_drag_payload`，并把该负载
  提升到 `PlatformWindow::start_external_drag`；其 Windows 平台
  实现继承了不支持的默认行为。因此本功能
  保留 FileManager 已有的 `on_drag` 手势，并在不改变 UI 框架的前提下
  提供一个 Windows 适配器。
- `drag-rs` 记录了 Windows 支持，并测试了 Tao、Winit、Wry 与 Tauri。其
  Windows 实现初始化 OLE、创建 Shell 数据对象，并同步
  调用 `DoDragDrop`。纯消息派发器提供了独立库无法为 GPUI 提供的
  事件循环时序。
- 一个单独的 Winit 实现把待处理的原生拖拽数据保存在其
  Windows 事件循环状态中。这证实了拖拽会话的事件循环
  集成是平台适配器的一部分，而不仅仅是数据对象的
  构造。
- 用户确认了跨窗口的 Windows 投递。长按行为、
  取消，以及更新后拖拽图像的视觉确认仍有待
  在 Windows 上检查；仅编译不能验证这些交互。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与已有的错误/诊断输出现在
会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作
会上报挂钟（wall-clock）span 时长与错误。领域服务契约、事件
负载与源码操作顺序保持不变。捕获范围与限制见
`fileatlas-plugin-debug/PORTS.md`。

## 原生主题绘制（2026-10-19）

`start_file_drag(window, paths: &[PathBuf], style: DragImageStyle)` 现在要求
调用方提供从其解析后的原生主题得到的
`DragImageStyle { background, foreground, border, accent: [u8;3] }`。FileManager 提供当前的 UI Bridge/原生角色。回退文档
图标不再包含私有像素调色板或文件格式颜色表。真实图片
预览像素与 OS 行为不变；原生 OLE 拖拽执行未因主题迁移
而重新验证。这个底层服务没有新增任何 UI 栈依赖。

2026-10-20 媒体卡片修正：当一个活动媒体拖拽越过
主窗口视口边界时，FileManager 才排队本服务，而不是在拖拽开始
时。内部功能面板接收者会先得到常规的带类型 GPUI 负载。
派发器与仅复制的 OLE 实现保持不变；边界
鼠标捕获与外部 Explorer 投递仍未验证。
