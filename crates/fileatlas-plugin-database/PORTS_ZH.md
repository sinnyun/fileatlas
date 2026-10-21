# Database plugin ports / 数据对接

## 剪贴板历史（2026-10-18）

- 新增的 `clipboard_categories` 和 `clipboard_history` 表由现有数据库初始化器
  创建；现有的媒体/库表与列保持不变。
- `DatabaseService::clipboard_snapshot`、`clipboard_capture`、
  `clipboard_add_manual`、`clipboard_mark_used`、`clipboard_update`、
  `clipboard_add_favorite`、`clipboard_remove_favorite`、`clipboard_assign_category`、类别 CRUD、`clipboard_delete` 以及
  `clipboard_clear_unpinned` 是唯一的生产入口点。SQL 与
  连接加锁仍保留在本 crate 内部。
- 历史内容唯一以用于去重。收藏（legacy pinned 行）不参与
  自动保留清理；服务将临时行上限设为 500。
- 新的剪贴板表是位于数据库服务所选的同一个 FileAtlas 数据库中的
  本地应用数据。浏览器配置分离遵循现有的
  DatabaseService 配置/路径规则。
- 浏览器存储持久化现在在 `clipboard_history` 之前先包含 `clipboard_categories`，并加上子/父删除协调。较早的浏览器构建在内存中创建了这些表，但将其遗漏在持久存储白名单之外。浏览器模式回归测试覆盖重新捕获去重、指派、重命名、重启、删除后归入未分类、以及已删除历史的重启行为。
- 删除类别时会先在同一个事务中显式清空 `category_id`，再移除类别本身；文本内容保留。该功能是新的工作台行为，在锁定的仅照片源码中没有对应实现，并使用现有的共享持久化边界。
- 剪贴板历史现在新增了 `content_type`、`image_path`、`image_width` 和 `image_height` 列。现有行按文本迁移且不改写其内容；图像字节仍保存在剪贴板插件拥有的 FileAtlas PNG 缓存中，而不是共享的 SQLite 记录里。

当前浏览器运行时已取代下文中较早的索引库描述；参见末尾的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：已实现源码兼容的 SQLite 服务；逐查询的广泛回归覆盖仍待完成。权威来源：`src/lib.rs` 与 `src/db/`。

## 生命周期与依赖

- Cordis 插件名：`fileatlas.database`；服务：`fileatlas/database`（`DatabaseService`）。
- 无插件依赖。`DatabaseConfig.path = None` 打开 `%LOCALAPPDATA%\FileAtlas\fileatlas.db`；`Some(path)` 覆盖该路径。
- Cordis 发布范围限定在插件生命周期内。消费者通过 `Context::try_service::<DatabaseService>()` 获取。

## 输入 / 输出

- 首选接口：针对文件夹、媒体、FileManager 快照/分页、标签与评分变更、扫描批次、watcher 协调、浏览器缓存更新、元数据导出以及统计的类型化 `DatabaseService` 操作。复合读写将其 SQL 及锁/事务范围保留在 database crate 内部。
- 兼容/调试接口：`with_reader` 和 `with_writer` 仍保留，用于数据库自有操作与专项测试。生产特性 crate 不得传入任意 SQL 回调；`scripts/check-layering.cjs` 会拒绝在 database crate 之外使用它们（内联单元测试模块除外）。
- 查询门面：`fileatlas_plugin_database::queries` 仍为公开，用于源码兼容的查询/领域类型以及 database crate 内部实现。新的生产行为应调用类型化服务方法，而不是在消费者 crate 中执行查询函数。SQLite 连接与 `db` 实现模块为私有。类型化方法返回 `Result<T, DatabaseAccessError>`；SQLite 与中毒锁（poisoned-lock）失败都会被暴露。
- 持久契约：现有 `fileatlas.db`、表、列、迁移、WAL 设置以及独立读连接。在没有 legacy fixture 与重新打开测试的情况下，不要更改 schema 或行语义。

## 消费者规则 / 调试

保持闭包简短；将耗时的扫描/元数据工作移出 GPUI 渲染线程。遇到服务缺失错误时，先检查启动顺序与 Cordis 插件生命周期；遇到数据库错误时，记录操作与 SQLite 错误，但不要记录媒体内容。

`MediaQuery::external_search_paths` 可选地提供来自
Everything 的候选完整路径。数据库仍然应用当前生效的文件名/路径
子串范围与标签搜索，以及所有 FileAtlas 自有的过滤器。
该字段只是查询优化，不改变持久 SQLite schema。FileManager
负责提供方选择与回退；数据库代码绝不调用 Everything。

## 通用文件类型（2026-10-14）

`MediaItem.media_type` / `MediaQuery.media_type` 在 `image`、`video`、`audio` 之外支持 `file`。现有 SQLite 文本列与无过滤查询已可容纳该值；不需要 schema 或持久文件迁移。解码器支持并不决定行的可见性。图像/视频统计仍专属于媒体；全部文件总数包含通用行。

`queries::scan_upsert_media(conn, folder_id, filename, full_path, media_type, file_size, modified_at, poster_path) -> rusqlite::Result<Option<String>>` 是扫描器专用的 upsert，当源文件元数据变化时返回旧的缓存路径。遇到完整路径冲突时，它会在不替换行的前提下刷新文件夹/名称/类型/大小/修改时间以及所提供的视频封面；它保留 ID、评分、标签链接与拍摄日期。若类型、大小或修改时间发生变化，它会清除缓存的尺寸/微缩略图数据，以便 Indexing 重新生成图像缓存。现有 `insert_media` 与文件操作语义保持不变。这避免了源扫描器在协调持久化索引时因 `INSERT OR REPLACE` 造成的元数据丢失。

## 紧凑分组输入（2026-10-14）

`queries::get_media_grouping_fields(&Connection, &MediaQuery) -> rusqlite::Result<Vec<MediaGroupingFields>>` 使用现有过滤器构建器与已注册文件夹，返回过滤后去重的 id/full_path/modified_at/root_path。请通过 DatabaseService 调用。它与 skeletons 一样忽略分页；不新增 created-at 列，也不做 schema 迁移。分组功能负责文件系统创建时间的读取。FileManager 按排列后的 ID 抓取视口内的富行；SQL 查询/SortOrder 的含义保持不变。

## 直接子目录查询（2026-10-14）

`MediaQuery.direct_directory: Option<String>` 将所有现有查询/skeleton/计数构建器限制在给定路径正下方的普通已索引行。SQL 参数比较经斜杠规范化的字面量前缀，然后要求其中不再出现分隔符；`%` 与 `_` 是目录字面字符。源 path_prefix 递归保持不变。这是内存中的查询字段，不是 schema 变更。FileManager 浏览器投影使用该字段；递归调用方保留 None。245 行缓存/范围查询的回归测试位于 FileManager 的 browser_tests.rs 中。

## 运行时诊断（2026-10-14）

运行时诊断包装器保留独立的 WAL 读/写连接、加锁、SQL 操作顺序与错误类型。with_reader/with_writer 现在记录调用方位置与包含锁等待的挂钟时间跨度；SQL 失败会输出 ERROR。不做 schema 变更或 SQL 重写。日志订阅器属于 fileatlas.debug。

## 内存写版本号（2026-10-15）

`DatabaseService::write_revision() -> u64` 读取一个原子的失效标识，
不记录日志也不查询 SQLite。它在每个 writer 闭包
返回后前进，即使出错也前进，因为闭包可能已执行了部分写入。
它不会因 reader 而前进。这是保守的失效机制，不是
持久事务 ID，也不是变更行数统计。不更改 schema、SQL、锁顺序或
闭包结果；不追踪外部的直接 SQLite 写入。
FileManager 的统计/批量读取模型将其与现有的事件版本号
配合使用，从而在直接服务操作后保持数据新鲜，而不依赖重绘驱动的读取。


## 浏览器运行时更新 — 2026-10-15

运行时现在使用 `DatabaseService::open_browser_at(path)`；插件打开 **fileatlas-tags.db**，不触碰 legacy 的 fileatlas.db。`main` 是内存中的 SQLite 会话缓存，`saved` 是附加的持久数据库。浏览调用 `with_cache_writer`；标注/书签命令调用 `with_writer`，后者只同步已标注/已评分媒体与用户自有表。未变化的持久行不会被重写。不执行旧库迁移。

公开新增：`is_browser() -> bool`、`with_cache_writer<T>(FnOnce(&Connection)->rusqlite::Result<T>)`、`begin_directory_cache(&str)`、`pin_file(&str,&str,Option<i64>)->Result<i64,_>`、`pin_history(&[i64])`、`flush_annotations()`、`forget_records(&[String])->Result<usize,_>`。序列化的浏览器读取器使用同一个内存连接。缓存 ID 是真实的会话行句柄；持久选择使用绝对路径。最近范围预算：四个目录，清理超过 200,000 行的较旧范围；当前目录、已标注/已评分文件、活动预览与历史固定项可超出该预算。`file_versions` 与路径列表仅存在于会话中。`favorite_meta` 存储别名/顺序；`tag_previews` 存储独立的持久识别缩略图路径。隐藏文件夹属主 0 不是收藏。

`MediaQuery::{local_only,direct_directory,directory_tree,annotated_only}` 将可见的文件系统成员关系与标注查询区分开。`MediaItem::missing` 是展示字段，不持久化。`LibraryStats::total_files` 包含所有类型。SQL 错误在 DatabaseAccessError/tracing 中查看；持久化测试覆盖重启、无未标注清单、字面路径前缀以及历史缓存保留。

## Everything 搜索缓存

Everything 搜索集成新增了 `cache_search_files(&[BrowserFile]) -> Result<Vec<String>, DatabaseAccessError>`。
它把真实的 SDK 命中物化到会话数据库中，而不改变活动目录的成员关系。
常规/扩展的 Windows 路径别名复用行 ID；相同路径/版本集合是空操作，不会
推进写版本号。新的搜索集合会清理旧的派生命中，同时保留目录范围、
评分/标签与预览/历史固定项。`search_paths` 仅存在于内存中；持久标注保存
不会存储搜索清单。目录与搜索写入共用 `search_cache::cache_file`。

## Lore 版本元数据


`versions`、`version_graph` 与 `version_metadata_imports` 是由本服务拥有的持久类型化
表。`VersionInfoService` 委托给类型化方法；
特性 crate 不得打开第二个 SQLite 连接或使用原始 SQL
回调。浏览器模式持久化包含这些表，因此版本历史
可以在进程重启后保留。Legacy 的 `version-info.sqlite3` 会被
幂等导入并作为回滚副本保留。

## 程序启动器表（2026-10-17）

共享的 FileAtlas 数据库 schema 包含 `app_launcher_categories` 与
`app_launcher_entries`；它们与应用其余部分位于同一个数据库服务和连接
池中。浏览器模式的 `fileatlas-tags.db` 持久化会把这些表复制到其持久存储
并同步删除，因此启动器
类别与编辑能在重启后保留，而无需创建插件自有的 DB 文件。
类型化方法 `launcher_snapshot`、`launcher_merge_discovered`、
`launcher_add_category`、`launcher_rename_category`、`launcher_delete_category`、
`launcher_rename_entry`、`launcher_move_entry`、`launcher_set_hidden`、
`launcher_remove_entry` 与 `launcher_add_manual` 构成该功能边界。
Schema 建表在 `db::initialize` 中是幂等的，只添加表；它不会
改动现有的照片行或列。

搜索位置契约（2026-10-17）：`MediaQuery.search_entire_computer: bool`
是 FileManager 的编排标志（默认 false），不是 SQLite schema 字段。
FileManager 在调用 DB 之前会规范化当前生效的搜索目录约束；
SQL 继续应用 `directory_tree`、标签/日期/评分以及候选谓词。
该标志不引入任何迁移或持久清单变更。

### 启动器安装元数据（2026-10-17）

现有数据库拥有新增的 `app_launcher_metadata(entry_id INTEGER PRIMARY KEY REFERENCES app_launcher_entries(id) ON DELETE CASCADE, installed_on TEXT, size_bytes INTEGER, pin_scope INTEGER NOT NULL DEFAULT 0)`。`pin_scope` 使用 0=无、1=类别顶部、2=全部程序顶部。启动时会迁移较早的元数据表，方法是添加带默认值的列。`launcher_set_pin_scope` 通过类型化服务校验并写入；重新扫描会在更新安装元数据的同时保留该值。浏览器模式持久化会连同其表一起同步新列。回归测试检查先前 schema 的迁移、重启后的持久化、范围校验与级联删除。

### 收藏删除保护（2026-10-18）

legacy 的 `pinned` 字段表示本地收藏。现有行保持兼容。`clipboard_delete(id)` 只删除临时行，遇到收藏会报错；`clipboard_remove_favorite(id)` 是删除收藏的唯一方法，遇到非收藏会报错。保留清理与清空都排除收藏。手动 upsert 与内容更新使用 MAX(已有值, 请求值) 来决定收藏状态，因此过期的编辑器/重复捕获无法撤销保护。浏览器持久存储协调会同时持久化编辑和显式移除。回归测试：`favorites_are_durable_and_only_explicit_removal_can_delete_them` 使用生产浏览器模式重新打开、legacy 行、保留清理、清空、普通删除、重复/手动编辑以及专用移除。
