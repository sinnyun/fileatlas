# AI plugin port

Status: Cordis service is registered in the root workspace and shell. Its
inference and download paths are adapted from the preserved PhoTag source; the
Cargo lockfile still needs dependency resolution, and no Windows runtime or
model download has been validated here.

## Cordis identity and startup

- Plugin: `photag.ai` (`AiPlugin`); service: `photag/ai` (`AiService`).
- `AiPlugin` has no startup dependencies. The shell starts it when Gallery,
  MCP, all plugins, or the explicit `ai` development plugin is enabled. It
  publishes `Arc<AiService>`; consumers must resolve that service from Cordis.
- The consumer passes persisted settings into `suggest_tags`; this avoids a
  service dependency on the settings plugin lifecycle. The settings type is
  `photag_plugin_settings::AiSettings`.
- Integration dependencies: `cordis`, `photag-plugin-settings`, `dirs`,
  `image`, `ndarray`, `tract-onnx`, `ureq`, and `thiserror`. These are declared
  in this crate's manifest; `ndarray` and `tract-onnx` are not yet present in
  the checked-in lockfile, so resolve dependencies before a `--locked` shell
  check can succeed. No `tokio` API is used.
- Model path: `%LOCALAPPDATA%/PhoTag/models/mobilenetv2-7.onnx` (with the
  platform data-local directory fallback used by `dirs`).

## Inputs and outputs

- `model_exists()` reports whether the model file exists at the source path.
- `download_model(progress)` synchronously downloads the ONNX model. Use
  `download_model_async()` for the source-compatible worker-thread API; it
  returns `mpsc::Receiver<Result<(), String>>`.
- `suggest_tags(image_path, top_n, settings)` starts inference on a worker
  thread and returns `mpsc::Receiver<Result<Vec<TagSuggestion>, String>>`.
  `suggest_tags_with_groups` accepts only the mandatory groups for callers
  that do not hold a complete `AiSettings` snapshot.
- Suggestions contain `label`, `confidence`, and an optional mandatory-group
  `group_name`. Mandatory group winners precede neural-network suggestions,
  matching PhoTag behavior.
- Inputs are local image paths and settings snapshots. Videos and audio are
  unsupported, as in the source preview flow.

## Events, limits, and failures

- The service publishes no Cordis events. Progress and completion use standard
  `mpsc` channels; inference failures and download failures are returned as
  strings. Plugin publication failures use `AiPluginError::Publish`.
- The model is fetched from the same ONNX Model Zoo URL as the source. PhoTag
  has no pinned model checksum, and this port does not invent one; the download
  is written to a temporary file and published only after the stream completes.
- The port preserves MobileNetV2 preprocessing, the ImageNet label mapping,
  image brightness/warmth/saturation analysis, mandatory-group scoring, and
  worker-thread behavior.
- The source caller is `vendor/photag-source/src/app.rs` (`poll_ai` and
  `request_ai_suggestions`) and `vendor/photag-source/src/ui/preview.rs` (the
  suggestion panel and apply actions). The shell now registers the service,
  but the native preview does not yet request suggestions, poll results,
  apply tag assignments, or honor `AiSettings::auto_analyze`. The preview
  currently has only a path, not the indexed media ID required by the tagging
  service; these caller behaviors remain incomplete.
- To investigate failures, inspect the receiver result and then
  `src/engine.rs` for model access/inference or `src/imagenet.rs` for label
  mapping. Source reference: `vendor/photag-source/src/ai.rs` at PhoTag commit
  `36cee242a78c0a8cd9ac6b0d63b64265aa0add90`; retain its license in
  `vendor/photag-source/LICENSE`.
