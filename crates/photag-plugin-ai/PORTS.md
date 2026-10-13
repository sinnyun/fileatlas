# AI plugin port

Status: Cordis service is registered in the root workspace and shell. The
Preview plugin now requests suggestions using the persisted mandatory groups,
polls worker results, honors `auto_analyze`, and applies/removes labels through
the Tagging service. Windows inference, model download, and database writeback
have not been runtime-validated.

## Cordis identity and startup

- Plugin: `photag.ai` (`AiPlugin`); service: `photag/ai` (`AiService`).
- `AiPlugin` has no startup dependencies. The shell starts it when Gallery,
  MCP, all plugins, or the explicit `ai` development plugin is enabled. It
  publishes `Arc<AiService>`; consumers must resolve that service from Cordis.
- Preview resolves `AiService`, `SettingsService`, and `TaggingService` from
  Cordis and passes the current `Settings::ai` snapshot into `suggest_tags`.
  Shell starts these providers before Preview.
- Integration dependencies: `cordis`, `photag-plugin-settings`, `dirs`,
  `image`, `ndarray`, `tract-onnx`, `ureq`, and `thiserror`. All are declared
  in this crate's manifest and represented in the checked-in lockfile. No
  `tokio` API is used.
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
- The source flow in `vendor/photag-source/src/app.rs` (`poll_ai` and
  `request_ai_suggestions`) and `vendor/photag-source/src/ui/preview.rs` is
  connected through `crates/photag-plugin-preview/src/ai_suggestions.rs`.
  Auto-analysis runs only when enabled and the model already exists; manual
  Analyze downloads a missing model and then starts inference. Image identity
  comes from `PreviewSnapshot::media_id`; video and audio extensions are
  excluded. Suggested labels are created under `AI Suggested` when missing,
  then assigned through `TaggingService`, preserving its metadata rewrite
  behavior. Successful AI assignments record the exact direct label IDs in
  `HistoryService` as source-compatible `UndoEntry::AssignedTags`; failed DB
  assignments do not create a history entry. Removal removes a matching
  assigned tag from the current image.
- Inference receivers are discarded when preview selection changes, so late
  results are ignored and cannot mutate a different image. The current engine
  API has no cancellation token; the detached worker can finish computing in
  the background, and the Preview panel tells the user when this happens.
- To investigate failures, inspect the receiver result and then
  `src/engine.rs` for model access/inference or `src/imagenet.rs` for label
  mapping. Source reference: `vendor/photag-source/src/ai.rs` at PhoTag commit
  `36cee242a78c0a8cd9ac6b0d63b64265aa0add90`; retain its license in
  `vendor/photag-source/LICENSE`.
