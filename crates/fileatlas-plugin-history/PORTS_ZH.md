# History 插件对接端口 / 历史操作对接

当前的浏览器运行时已取代下方较早的“已索引媒体库”说明；请参见文末 2026-10-15 的更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：针对 FileAtlas 源码中实际存在的五种可撤销操作类型，Cordis 服务与可执行的撤销/重做处理逻辑均已实现。FileManager 会记录评分、标签快捷键、标签选择器的指派/移除/清空，以及经过确认的单条目重命名变更。FileManager 处理 Ctrl+Z/Ctrl+Y，而 history 插件则以生命周期归属清晰的方式暴露 Edit 菜单中撤销与重做的命令贡献。

## 生命周期与依赖

- Cordis 插件：`fileatlas.history`；服务：`fileatlas/history`（`HistoryService`）。
- 启动时需要 `DatabaseService`、`TaggingService` 与 `FileOperationsService`。服务的发布生命周期由 Cordis 掌管。
- 历史栈保存在内存中，作用域限于当前服务实例；它们不会跨启动持久化。
- 来源：`vendor/photag-source/src/app.rs`（`UndoEntry`、`push_undo`、`execute_undo`、`execute_redo`、pending action 处理）以及 `vendor/photag-source/src/main.rs` 的键盘分发。

## 公开操作

- `push(UndoEntry)` 会清空 redo 栈，并把 undo 栈上限设为 100 条。
- `cleared_tags_entry(media_ids)` 在清空之前为每条媒体行快照其已指派的标签 ID。
- `undo().await` 与 `redo().await` 返回与源实现兼容的 toast 文本；栈为空时返回 `None`。
- `undo_description()` / `redo_description()` 返回当前栈顶操作的描述。
- `history/undo` 与 `history/redo` 作为命令注册，并带有 Edit 菜单条目。`UiCommandInvoked` 监听器执行对应的栈操作，并在操作非空时发出 `MediaIndexChanged`，以便 FileManager 刷新。
- `UndoEntry` 与源数据模型一致：`AssignedTags`、`RemovedTags`、`ClearedTags`、`RatingChanged`、`Renamed`。
- 标签变更使用 `TaggingService`（包含源实现的 XMP 元数据同步）；评分使用原有的数据库批量查询；重命名使用 `FileOperationsService`，它会发出 `MediaIndexChanged`。
- 撤销/重做通过 `TaggingService::restore_assignment` 恢复标签指派，因此历史回放不会改变与源实现兼容的最近标签列表。

## 源操作映射与语义

- 被锁定的源实现中完整的 `UndoEntry` 枚举恰好有五个变体：
  `AssignedTags`、`RemovedTags`、`ClearedTags`、`RatingChanged` 与 `Renamed`。
  其生产者位于 `vendor/photag-source/src/app.rs` 的 pending-action 处理中
  （标签指派/移除/清空、评分、重命名，以及 AI 建议的采纳）。
- FileManager 产出标签快捷键/选择器的指派与移除、清空、评分，
  以及经过确认的重命名条目。预览面板的 AI 建议“应用”操作同样会在
  指派成功后记录 `AssignedTags`。文件复制/移动/删除以及
  文件夹/标签树的编辑在源模型中不可撤销，绝不能
  被表示为历史条目。

- 与 FileAtlas 源码一致，操作失败属于尽力而为，不会阻止该条目转入对面栈。任何新的 push 都会清空 redo。
- 撤销/重做执行的栈转移不会裁剪目标栈，与源处理逻辑一致。
- 文件操作的成功/失败目前不会作为结果暴露给调用方；FileManager 显示返回的历史状态文本，并在撤销/重做后刷新其查询。
- 文件夹/标签树编辑、删除/复制/移动/批量操作仍处于历史之外，
  与源实现的 `UndoEntry` 模型一致。

## 调试

- 检查服务启动顺序：database、tagging 与 file operations 必须在 history 之前发布；Preview 必须在它所消费的 History、AI、Settings、Tagging 与 MediaPlayback 提供者之前停止。
- 当键盘操作看似无效时，检查 `undo_description` / `redo_description`；`None` 表示对应的栈为空。
- 当 Edit 命令看似无效时，检查 `history/undo` 与 `history/redo` 的命令 ID 以及 `UiCommandInvoked` 的投递。命令操作会通过 `MediaIndexChanged` 刷新订阅者。
- 无法实际执行的重命名仍会转移它的条目，正如最初的调用方所做的那样。检查媒体行、目标文件名冲突、文件系统权限，以及 `MediaIndexChanged` 的投递。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与既有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报以墙上时钟计算的 span 时长与错误。领域服务契约、事件负载与源实现的操作顺序均未改变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。


## 浏览器运行时更新 — 2026-10-15

`push` 会通过 `DatabaseService::pin_history` 钉住最多 100 条撤销/重做操作所使用的会话缓存句柄，因此目录驱逐不可能丢弃撤销所需的未加标签文件。新操作会刷新钉住集合的并集；历史仍然只存在于会话中，不会创建持久的浏览清单。
