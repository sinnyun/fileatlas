# AI 插件端口

状态：Cordis 服务已在根工作区与 shell 中注册。
Preview 插件现在会使用持久化的强制分组（mandatory groups）请求建议，
轮询工作线程结果，遵循 `auto_analyze`，并通过
Tagging 服务应用/移除标签。Windows 上的推理、模型下载与数据库回写
尚未经过运行时验证。

## Cordis 身份与启动

- 插件：`fileatlas.ai`（`AiPlugin`）；服务：`fileatlas/ai`（`AiService`）。
- `AiPlugin` 没有启动依赖。当 FileManager、MCP、全部插件，
  或显式的 `ai` 开发插件被启用时，shell 会启动它。它
  发布 `Arc<AiService>`；消费者必须从 Cordis 解析该服务。
- Preview 从 Cordis 解析 `AiService`、`SettingsService` 与 `TaggingService`，
  并把当前的 `Settings::ai` 快照传入 `suggest_tags`。
  Shell 会在 Preview 之前启动这些提供者。
- 集成依赖：`cordis`、`fileatlas-plugin-settings`、`dirs`、
  `image`、`ndarray`、`tract-onnx`、`ureq` 与 `thiserror`。全部声明
  在本 crate 的清单中，并体现在已提交的 lockfile 里。没有使用
  任何 `tokio` API。
- 模型路径：`%LOCALAPPDATA%/FileAtlas/models/mobilenetv2-7.onnx`（附带
  `dirs` 所使用的平台 data-local 目录回退）。

## 输入与输出

- `model_exists()` 报告模型文件在源实现对应的路径上是否存在。
- `download_model(progress)` 以同步方式下载 ONNX 模型。若要使用
  `download_model_async()`，可获得与源实现兼容的工作线程 API；它
  返回 `mpsc::Receiver<Result<(), String>>`。
- `suggest_tags(image_path, top_n, settings)` 在工作
  线程上启动推理，并返回 `mpsc::Receiver<Result<Vec<TagSuggestion>, String>>`。
  `suggest_tags_with_groups` 只接受强制分组，适用于
  不持有完整 `AiSettings` 快照的调用方。
- 建议项包含 `label`、`confidence`，以及可选的强制分组
  `group_name`。强制分组的胜出项排在神经网络建议之前，
  与 PhoTag 源实现行为一致。
- 输入为本地图片路径与设置快照。视频与音频
  不受支持，与源实现的预览流程一致。

## 事件、限制与失败

- 该服务不发布任何 Cordis 事件。进度与完成使用标准
  `mpsc` 通道；推理失败与下载失败以字符串形式返回。
  插件发布失败使用 `AiPluginError::Publish`。
- 模型从与源实现相同的 ONNX Model Zoo URL 获取。FileAtlas
  没有锁定的模型校验和，本端口也不会自行编造一个；下载内容
  写入临时文件，只有在流完成后才发布生效。
- 本端口保留了 MobileNetV2 预处理、ImageNet 标签映射、
  图像亮度/色温/饱和度分析、强制分组打分，以及
  工作线程行为。
- `vendor/photag-source/src/app.rs` 中的源流程（`poll_ai` 与
  `request_ai_suggestions`）以及 `vendor/photag-source/src/ui/preview.rs`
  通过 `crates/fileatlas-plugin-preview/src/ai_suggestions.rs` 接入。
  自动分析仅在启用且模型已存在时运行；手动
  Analyze 会先下载缺失的模型再启动推理。图像身份
  来自 `PreviewSnapshot::media_id`；视频与音频扩展名被
  排除。缺失的建议标签会在 `AI Suggested` 下创建，
  然后通过 `TaggingService` 指派，并保留其元数据重写
  行为。成功的 AI 指派会把确切的直接标签 ID 记录到
  `HistoryService` 中，形式为与源实现兼容的 `UndoEntry::AssignedTags`；数据库
  指派失败不会创建历史条目。移除操作会从当前图像上移除一个匹配的
  已指派标签。
- 当预览选择发生变化时，推理接收器会被丢弃，因此迟到的
  结果被忽略，无法改动另一张图像。当前引擎
  API 没有取消令牌；分离的工作线程仍可能在
  后台完成计算，此时 Preview 面板会告知用户。
- 要排查失败，先检查接收器结果，然后查看
  `src/engine.rs`（模型访问/推理）或 `src/imagenet.rs`（标签
  映射）。源参考：PhoTag 提交 `36cee242a78c0a8cd9ac6b0d63b64265aa0add90`
  处的 `vendor/photag-source/src/ai.rs`；其许可证保留在
  `vendor/photag-source/LICENSE`。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与既有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报以墙上时钟计算的 span 时长与错误。领域服务契约、事件负载与源实现的操作顺序均未改变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。
