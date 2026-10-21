# 标签数据插件对接 / Tagging plugin ports

当前浏览器运行时已取代下文中较旧的索引库描述；请参阅末尾的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：由数据库支撑的指派/关联/合并/改父（reparent）服务已可用。常规
标签操作仅涉及数据库；自动 IPTC/XMP 文件写入已按 2026-10-17 的用户决定
禁用。显式的元数据写入 API 仍供请求文件写回的集成方使用。
FileManager 检查器提供了内联的可搜索、多选添加控件。

## 生命周期与依赖

- Cordis 插件名称：`fileatlas.tagging`；服务：`fileatlas/tagging`（`TaggingService`）。
- 需要 `DatabaseService`；在 `file_manager.selection-actions` 贡献一个标签控件。

## 输入 / 输出

- `assign(media_ids, tag_ids)` 应用所请求的标签、单向关联以及祖先标签，同时保留 `auto_applied` 的区分。常规指派不会修改媒体文件，也不会创建 XMP sidecar。
- 成功的面向用户的指派会按源顺序（最新在前，最多八个 ID）更新 `recent_tags()`；`restore_assignment` 供 History 重放使用，不会改变最近使用顺序。
- `apply_ai_suggestions(media_id, labels)` 以不区分大小写的方式解析已存在的名称，在 `AI Suggested` 下创建缺失的标签，然后通过常规服务路径委托指派。只有在数据库指派成功后它才返回直接的标签 ID，以便 Preview 能记录源兼容的 History；失败的指派返回错误且不会生成历史条目。
- `remove(media_ids, tag_ids)`、`clear(media_ids)`、`for_media(media_id)` 和 `associated_with(tag_id)` 暴露对应的源查询。
- `set_associations(tag_id, associated_ids)` 更新关联边；`merge(source_id, target_id, reparent_children)` 保留源的合并行为。操作返回 `Result<_, TaggingError>`；数据库失败会向上传播。
- Navigation 的源“合并到…”流程现在在用户从源子树之外选择了一个经过滤的目标后使用此 API。它让用户选择源的子节点是改挂到目标之下（默认）还是被删除。合并成功后会刷新 Navigation 与 FileManager 视图。
- `move_tag(source_id, drop_position)` 返回隐式祖先指派发生变化的媒体 ID。
- `tag_tree()` 返回当前递归的数据库标签树，供 UI 选择器使用。
- `writeback_failures()`、`retry_writeback_failure(path)` 和
  `dismiss_writeback_failure(path)` 通过 `TaggingService` 暴露保留的元数据写入结果；
  消费者不得直接访问私有的 worker 模块。
- `schedule_xmp_sync(media_ids)` 是显式的集成入口点；常规用户操作不会调用它。若被显式请求，JPEG/JPEG 文件会把 IPTC 关键词嵌入 APP13，其他扩展名则生成 `.xmp` sidecar。失败快照与重试/忽略方法仍仅限于这些显式请求。
- `xmp::write_metadata(path, tag_paths)` 同样公开，供显式集成使用。写入后会尽力恢复原始的访问/修改时间。

## 消费者与调试

使用数据库服务中稳定的整数 media/tag ID。FileManager 检查器的标签区可搜索标签层级并内联添加多个选中标签；FileManager 批量操作与 Preview 保留其更广泛的多选选择器。FileManager/Navigation 暴露关联编辑以及合并/删除流程。变更后，消费者重新加载其视图并将操作记录到 HistoryService。调试意外标签时请校验祖先与关联标志。源参考：`vendor/photag-source/src/xmp.rs` 会自动写入 JPEG IPTC 并为非 JPEG 生成 sidecar；FileAtlas 现在有意将常规标签管理保留在 SQLite 中，仅在集成方显式请求时才写入文件元数据。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动以及现有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；已插桩的公开操作会上报挂钟时间的 span 时长与错误。领域服务契约、事件负载与源操作顺序保持不变。捕获范围与限制参见 fileatlas-plugin-debug/PORTS.md。


## 浏览器运行时更新 — 2026-10-15

浏览器指派在持久化关联之前会捕获当前的本地大小/时间与图片缩略图。识别图像会被复制到 fileatlas-tags.db 旁边的 **tag-previews/** 目录，并由 tag_previews 引用，独立于可重建的浏览缩略图。现有的层级与关联语义保持不变；自动 XMP/IPTC 写回如上所述已禁用。

`missing_records()->Result<Vec<MediaItem>,String>` 列出已打标签但元数据返回 NotFound 的路径，并用保存的识别预览替代。权限失败不算缺失。`relink_record(i64,&str)` 要求旧路径当前确实缺失且目标为显式指定的普通文件；它拒绝已有标签或评分的目标，同时在刷新路径/大小/时间/缩略图时保留原始标签。`delete_missing_records(&[i64])->Result<usize,String>` 会再次检查，仅删除记录，绝不删除本地源文件。错误会传播到 FileManager 操作反馈/MCP。相机/编辑器互操作性不在本次变更范围内。

## MCP 贡献

当共享的 `McpToolRegistry` 服务可用时，本插件注册 `tagging.list_tags`、`tagging.assign`、`tagging.remove`、`tagging.list_missing_records`、`tagging.relink_missing_record` 和 `tagging.delete_missing_records`。Schema、校验、变更分类与 handler 位于本 crate 中并调用 `TaggingService`；registry 守卫由 Cordis 拥有，插件停止时会移除这些工具。指派/移除接受 1–100 个正整数 media ID 与 1–50 个正整数 tag ID。删除缺失行需要显式确认，并在操作时重新检查源路径。变更操作受宿主 MCP 变更门控约束。
