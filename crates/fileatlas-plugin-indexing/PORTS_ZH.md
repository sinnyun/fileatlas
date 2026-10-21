# Indexing 插件对接 / 索引数据对接

当前的浏览器运行时取代了下文较旧的已索引库描述；参见末尾的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：Cordis 服务与原始扫描工作流均已存在。当前测试使用合成 fixture；完整的媒体格式与 Windows 对齐仍待完成。

## 生命周期与依赖

- Cordis 插件名：`fileatlas.indexing`；服务：`fileatlas/indexing`（`IndexingService`）。
- 要求在启动之前已发布 `DatabaseService` 与 `ThumbnailService`。

## 输入 / 输出

- `scan_folder(folder_id, folder_path, cancel)` 接收一个已注册的 FileAtlas 文件夹标识/路径，以及共享的 `AtomicBool` 取消标志；返回 `mpsc::Receiver<Option<ScanProgress>>`。进度值携带扫描更新；对于目录遍历、媒体索引/收尾、缩略图数据库批量写入、或扫描工作线程 panic 失败，会填充一个终止性的 `ScanProgress.error`。`generate_missing_thumbnails` 通过同一个终止字段报告缩略图数据库批量写入错误与工作线程 panic。`None` 仍是完成哨兵值。取消会在不携带失败负载的情况下完成。
- `generate_missing_thumbnails(items, cancel)` 接收 `(media_path, media_type)` 对，并返回相同的进度通道形态。
- `list_subdirectories(path)` 返回直接子项的 `(name, path, has_children)` 元组，包含以点开头的目录。
- 通过数据库服务写入与源码兼容的媒体/文件夹行，通过缩略图服务写入图片缓存。

## 消费者与调试

Navigation 启动/取消扫描；watcher 回填可以请求缺失的缩略图。请让通道抽取（drain）远离 UI 渲染流程。诊断卡住的扫描时，检查取消所有权、接收端抽取、文件系统访问、数据库写入，以及生产者是否到达 `None` 哨兵值。FFmpeg 提取与 Windows 路径需要后续的 Windows/媒体语料验证。

## 全文件索引（2026-10-14）

`classify_media_type(extension: &str) -> &'static str` 是扫描与 Watcher 共用的、不区分大小写的分类器。它返回 `image`、`video`、`audio` 或 `file`；空扩展名与无法识别的扩展名返回 `file`。递归扫描会为每个可访问的普通文件建立索引，包括以点开头的文件/目录以及无扩展名的文件。与某个视频配对的图片文件仍作为独立的行存在，同时继续为其提供海报。只有被识别的图片/视频类型才会请求解码器缩略图；不支持的文件不会保留任何伪造的缩略图。`list_subdirectories` 包含以点开头的目录。遍历错误仍会产生一个终止错误，取消/批量限制依旧生效。对账只有在对某个可用根完成一次完整遍历之后，才会删除离线被删除文件的行；一个不可用的根会保留其缓存行。重复扫描会刷新文件大小与修改时间，同时保留 ID、评分、日期、标签以及缓存的尺寸。Navigation 的 Rebuild/Rescan 保留其显式的清空并重建行为。

重复的图片缩略图缓存命中可能不返回解码出的尺寸/微缩字节。扫描批量更新通过 COALESCE 保留已有的 width/height/micro，而不清空这些值。回归测试与同进程重复扫描查询覆盖了被保留的尺寸。

## 启动刷新（2026-10-14）

Watcher 使用 `IndexingService::scan_folder` 对每个已注册文件夹启动一次可取消的后台刷新。这会添加旧构建所排除的文件类型并移除离线删除项，然后在行被提交时发出 `MediaIndexChanged`，以便 FileManager 在刷新进行时能够重新加载。`WatcherService::library_refresh()` 报告运行状态、文件夹进度与失败；`get_ui_state` 为实时诊断暴露相同的快照。缺失/不可用的根会报告错误并保留其现有数据库行。这是在每次应用启动时进行的一次完整递归遍历。当文件类型、大小或修改时间发生变化时，扫描器会使旧的缓存缩略图失效并重新生成它；未变化的路径复用缩略图缓存。Windows 遍历以及超大库的启动时间仍需用户规模的验证。

## 带检查的即时目录读取（2026-10-14）

`IndexingService::try_list_subdirectories(&self, path: &str) -> std::io::Result<Vec<(String,String,bool)>>` 返回相同的 name/path/has_children 负载，包含以点开头的目录，但会为 FileManager 的浏览器错误状态报告 read_dir/entry/file_type 失败。已有的 list_subdirectories 通过共享读取器保留其尽力而为的跳过/出错返回空的行为；扫描/写入/事件语义不变。子项存在性探测仍为尽力而为。仅用于即时目录卡片，包括空目录。

## 运行时诊断（2026-10-14）

运行时诊断观察被追踪的生产进度流：阶段转换、错误、scan_id，以及带有 duration_ms、failed 与 completed 标志的流完成。即使下游关闭，观察仍会转发所有消息并清理其活动条目；不会伪造扫描结果。流的耗时包含工作/等待，而非 CPU 时间。


## 浏览器运行时更新 — 2026-10-15

Indexing 通过 `ThumbnailService` 获取视频缓存路径与微缩略图生成；它不导入缩略图实现模块。

正常启动、文件夹浏览与收藏点击都不会调用递归索引。`visible_thumbnail(&str,&str)->Option<(String,Option<u32>,Option<u32>,Option<Vec<u8>>)>` 针对当前可见需求复用图片缩略图与 ffmpeg 视频海报管线。派生缩略图目录内部的路径会被排除，以防止缓存自我生成。递归扫描 API 仍保留给隔离的 dev fixture 与遗留回归覆盖，而非浏览器产品入口。
