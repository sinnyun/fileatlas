# Thumbnail plugin ports / 缩略图数据对接

当前的浏览器运行时已经取代了下文较早的索引库描述；参见文末的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：原始图片缩略图流水线已被封装为一项服务，缓存复用有一个合成图片回归测试。完整格式支持与 Windows 行为对等性尚未确立。

## 生命周期与依赖

- Cordis 插件名：`fileatlas.thumbnails`；服务：`fileatlas/thumbnails`（`ThumbnailService`）。
- 不依赖任何 Cordis 服务。`ThumbnailConfig.directory = None` 会选择 `%LOCALAPPDATA%\FileAtlas\thumbs`；也提供路径覆盖以用于隔离使用。

## 输入 / 输出

- 请求：`generate_image(source: AsRef<Path>)`。
- 结果：`Option<(cache_path: String, width: u32, height: u32, micro_thumb: Option<Vec<u8>>)>`；`None` 表示生成过程无法产出图片结果。按照所保留的实现，缓存命中会上报 `(0, 0)` 尺寸，并且仍可能返回微缩略图字节。
- `directory()` 返回缓存根目录。缓存文件名与图片处理与 FileAtlas 保持源兼容。
- `video_thumbnail_path(source: AsRef<Path>)` 返回由该服务持有的视频封面缓存路径；`generate_micro_thumbnail(&DynamicImage)` 返回既有的紧凑 RGB 字节。底层的 `thumbs` 实现模块是私有的。
- 在生成路径上，返回的 `micro_thumb` 是一个 8×8 的 RGB 字节数组（192 字节）；FileManager 在 GPUI 加载缓存 JPEG 期间把它用作低分辨率占位图。已存在的数据库缓存命中若没有存储的 blob，可能不返回微缩略图数据。

## 消费方与调试

Indexing 调用该服务来生成图片缓存，并补齐缺失的缩略图行。不要在 GPUI render 中同步调用它。缩略图缺失时，请检查源路径可访问性、解码器结果、缓存目录权限，以及索引到的 `thumb_path`；不要从这个图片 API 推断视频解码是否成功。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动以及既有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报挂钟时间跨度的时长与错误。领域服务契约、事件载荷与源操作顺序保持不变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。


## 浏览器运行时更新 — 2026-10-15

派生缓存键现在包含源路径、大小以及纳秒级修改时间。缓存图片命中会返回原始头部尺寸，而不是零尺寸。复用遗留的 webp 要求缓存文件不早于源文件。`generate_image` 每 256 次访问检查一次派生的 JPEG/webp 预算：按最旧修改时间顺序最多保留 5000 个文件和 512 MiB（这不是真正的访问 LRU），并保护当前结果。这项维护运行在发起请求的 worker 上，而不是 GPUI render 上。持久的标签预览单独存放并被排除在外。视频封面需求使用相同的带版本命名与缓存维护预算。
