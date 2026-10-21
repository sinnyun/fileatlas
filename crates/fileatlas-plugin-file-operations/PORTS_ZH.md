# 文件操作插件对接 / File operations plugin ports

当前浏览器运行时已取代下文中较旧的索引库描述；请参阅末尾的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：源兼容的单项与批量复制/移动/删除、重命名、创建文件夹、回收站
删除、打开、显示位置以及数据库后续处理均已可用。FileManager 负责目标
对话框、删除确认、操作反馈，以及变更后的预览/选区修复。迁移过程中
shell 依赖图已成功编译；文件系统行为仍需 Windows 运行时验证。
测试仍推迟到集中调试阶段进行。

## 生命周期与依赖

- Cordis 插件：`fileatlas.file-operations`；服务：`fileatlas/file-operations`
  （`FileOperationsService`）。
- 代码地图：服务与文件系统流程位于 `src/lib.rs`；公开的按操作结果/错误记录
  位于 `src/model.rs`；服务发布与插件生命周期错误位于 `src/plugin.rs`。
- 需要 `fileatlas/database`，并在数据库插件之后启动。
- 数据库行变更会发出 `MediaIndexChanged`；file_manager 消费该事件并重新加载。
  复制与创建文件夹不会变更索引行；现有 watcher 会处理在被监视根目录下
  创建的文件。

## 输入与输出

- `copy_to(media_id, destination)` 复制到调用方选择的完整路径，并保留源行
  不变。
- `bulk_copy_to_folder(media_ids, destination_dir)` 以当前文件名复制每个已索引
  文件，在单项错误后继续执行，并为每个输入 ID 返回一个
  `BulkCopyOutcome`。它不变更数据行；folder watcher 会在已注册根目录下
  发现成功复制的文件。
- `copy_paths_to_folder(paths, destination_dir)` 和
  `move_paths_to_folder(paths, destination_dir)` 既接受已索引路径，也接受任意
  暂存文件路径，在执行时重新校验每个源，并为每个输入返回一个
  `PathOperationOutcome`（`source`、`completed`、`error`）。
  已存在的目标会被跳过/报告，且路径复制辅助函数使用
  `create_new`，因此并发冲突不会覆盖目标字节内容；跨卷移动的回退路径
  在移除源之前使用同样的不覆盖复制路径。在判定某个暂存源是否已被索引
  之前，会先匹配 Windows verbatim/普通路径。
  已索引的移动保留现有的数据库/根目录协调逻辑并发出一个
  `MediaIndexChanged`；未索引的移动采用先复制后删除的方式，由已注册根目录的
  Watcher 发现移动后的内容。暂存篮调用这些方法。
- `move_to(media_id, destination)` 对跨卷移动使用先重命名、失败后复制/删除的
  回退方式。当目标字符串以某个已注册根目录开头时更新该行，否则删除该行，
  与 FileAtlas 的逻辑一致。
  `move_to_with_outcome` 额外返回移动是否发生；`false` 表示源兼容的命名冲突
  或文件系统移动失败。
- `move_into_folder(media_id, destination_dir)` 保留文件名，拒绝覆盖已存在的
  文件，并在目标位于已注册库根目录下时更新索引路径/根目录。移动到所有
  根目录之外时文件保留在磁盘上并删除其媒体行，与 FileAtlas 一致。
  `move_into_folder_with_outcome` 报告是否移动了文件。
- `bulk_move_into_folder(media_ids, destination_dir)` 对每一项应用相同行为，
  在单项失败后继续执行，并为每个输入 ID 返回一个
  `BulkMoveOutcome`。命名冲突或缺失项为 `moved: false`；
  数据库错误附带在该结果中。
- `rename(media_id, new_filename)` 会裁剪名称，拒绝空名称/未变更或冲突的名称，
  先在磁盘上重命名，然后更新现有 DB 行。
  `rename_with_outcome` 对未变更、空名称或冲突名称返回 `false`，以便 UI 能
  区分无操作与已完成的改名。
- `create_folder(parent, name)` 保留 FileAtlas 的非法字符规则，并且只创建
  一层目录。
- `delete_to_recycle_bin(media_id)` 使用 `trash` crate。它以尽力而为的方式移除
  缩略图，即使提交回收站失败也会删除 DB 行。`DeleteResult` 报告回收是否
  成功及其错误。
- `media_by_id(media_id)` 读取索引行用于操作后的修复。当错误可能来自
  文件系统/数据库变更已提交之后的事件投递时，请在操作出错后调用它。
- `bulk_delete_to_recycle_bin(media_ids)` 先解析各行，尽力将文件和缩略图移入
  回收站，即使提交回收站失败也移除每一行，并为每个输入返回一个
  `BulkDeleteOutcome`。数据库/查询失败按单项处理；成功的批次只发出一个索引
  事件。
- `open_external(media_id)` 使用操作系统默认应用打开；
  `reveal_in_file_manager(media_id)` 在资源管理器中选中/显示该路径（在非
  Windows 构建上为平台文件管理器）。
- 文件系统/数据库工作在 Tokio 的阻塞线程池上执行。方法返回类型化的 I/O、
  数据库、worker 和事件错误。变更后的事件错误意味着操作可能已经完成；
  请在重试前刷新 file_manager。

## 调试与剩余集成

将移动回退、路径前缀根匹配、命名冲突处理、缩略图清理以及回收失败后的
删除行为与 `vendor/photag-source/src/app.rs`
（`do_copy_media`、`do_move_media`、`do_move_media_to`、`do_rename_media`、
`do_create_folder` 和 `do_delete_media`）进行对照。数据库变更使用
`fileatlas-plugin-database` 中原有的查询辅助函数。不要在 GPUI render 中调用
这些同步文件操作；服务已经卸载了文件系统/数据库工作。FileManager 视图
请求原生保存/文件夹对话框并调用这些方法。请查阅
`fileatlas-plugin-file-manager/PORTS.md` 和
`src/view/file_actions.rs` 以了解 UI 流程、确认、反馈以及操作后的
预览/选区处理。本服务将对话框与展示策略留给调用方决定。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动以及现有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；已插桩的公开操作会上报挂钟时间的 span 时长与错误。领域服务契约、事件负载与源操作顺序保持不变。捕获范围与限制参见 fileatlas-plugin-debug/PORTS.md。


## 浏览器运行时更新 — 2026-10-15

运行时文件操作在执行操作时校验本地元数据。已知的由程序发起的移动/重命名会更新批注路径，即使该文件不在收藏夹中也是如此。浏览器行使用 owner 0 而不是书签所有权。成功回收会移除活动成员关系，并为缺失/重新链接维护保留带批注的记录；回收失败不会抹除记录。外部移动不会被自动匹配。遗留测试通过 open_at 保留源语义。真实回收站/原生对话框路径在浏览器模式下仍明确未经验证。

## MCP 功能贡献

`FileOperationsService` 通过 `McpToolRegistry` 贡献
`file-operations.rename_media`、`file-operations.move_media`、
`file-operations.copy_media` 和
`file-operations.delete_media`。定义与 schema 位于本 crate 的 `src/mcp.rs` 中；
四个操作都要求显式确认，批量操作会校验 1–100 个正整数 media ID。结果
保留逐项的成功/错误情况。原先不带命名空间的 MCP 名称在迁移期间仍作为
兼容别名保留。

## 功能属有的源文档链接

`open_linked_file(&self, path: &Path) -> Result<(), FileOperationError>` 会对
已存在的文件进行规范化与校验，然后使用与已索引的 `open_external` 相同的
`open::that` 平台交接方式。它不会创建媒体行，也不会修改文件。
WorkCrate 在 worker 上针对另行链接的 PSD/源文档调用它。
文件缺失与 Windows 关联错误对功能 UI 保持可见。
实际的 Photoshop 编辑/导出由外部编辑器和用户负责。
