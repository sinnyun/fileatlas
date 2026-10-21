# WorkCrate 契约对接

`fileatlas-workcrate-contracts` 是一个普通的共享 crate，不是 Cordis 插件。它
不发布任何服务，也不打开任何文件。它拥有稳定的 WorkCrate 值类型，
以及特性插件之间交换的 Cordis 事件类型。

## 身份与消费方

- ID：`WorkCrateProjectId` 和 `WorkCrateAssetId` 是由
  `fileatlas-plugin-workcrate-projects` 分配的 UUID 字符串。
- 记录：`WorkCrateProject`（name、created_at、archived、可选的 `main_image`）、
  `WorkCrateFileReference`（path、显示标题、order、大小、修改时间、
  可选的 `custom_preview`）以及 `WorkCratePreview`（相对缓存路径、sha256、
  像素尺寸）就是 Projects 所持久化的带版本 JSON 结构。
- `canvas_image_format` / `CanvasImageFormat` 决定一个画板
  可以接受哪些位图来源。Projects 及其 `management` 模块是当前的唯一调用方。
- 请求：`WorkCrateAddReferencesRequest { project_id, paths }`。

## 事件

- `WorkCrateAddReferencesRequested` — 由
  `fileatlas-plugin-workcrate-projects/src/lib.rs:730` 订阅。目前还没有 crate 发布它，
  因此 FileManager 的“把所选内容添加到 WorkCrate”交接仍未接线。
- `WorkCrateProjectsChanged` — 在此声明，目前在整个工作区中既没有被发布
  也没有被订阅。

## 排查位置

类型变更会以编译错误的形式出现在 WorkCrate 插件 crate 中；不存在
运行时注册表。事件投递问题需要通过发出方与订阅方插件日志中的 Cordis 事件
总线来排查，也可以通过 WorkCrate UI 的
操作消息来查看。持久化布局见 `fileatlas-plugin-workcrate-projects/PORTS.md`，
官方程序边界见 `fileatlas-plugin-workcrate-boards/PORTS.md`。
