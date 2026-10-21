# Watcher 插件对接端口 / 文件监视数据对接

当前的浏览器运行时已取代下方较早的“已索引媒体库”说明；请参见文末 2026-10-15 的更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：由 Cordis 管理的监视器、有界的协调（reconciliation）处理，以及 Linux 的创建/删除集成均已存在。原生 Windows 的事件投递与竞态行为尚未验证。

## 生命周期与依赖

- Cordis 插件名：`fileatlas.watcher`；服务：`fileatlas/watcher`（`WatcherService`）。
- 依赖 `DatabaseService` 与 `IndexingService`；在数据库变更以及启动扫描提交之后发出 `MediaIndexChanged`。
- 插件停止时会向后台协调工作线程发出信号并等待其结束（join）；不要再启动第二个独立的监视循环。

## 输入 / 输出

- `watch(path)` / `unwatch(path)` 安排对文件夹的递归监视。
- `poll_and_reconcile()` 返回 `ReconcileSummary { added_or_updated, removed }`；每次轮询的处理量被限制为 200 个路径。
- 新增/更新的文件会被协调写入 SQLite，缺失的缩略图生成则委托给 Indexing。变更成功后会携带 unit 参数派发 Cordis 事件 `fileatlas/indexing/media-index-changed`。
- 启动时，已有的后台工作线程会先通过 `IndexingService::scan_folder` 扫描每一个已持久化的根目录，然后才开始轮询通知。`library_refresh()` 返回 `LibraryRefreshState { running, total_folders, completed_folders, failures }`。插件停止会设置共享的取消标志并等待该工作线程结束。

## 消费者与调试

Navigation 负责注册文件夹并消费扫描生命周期；file_manager 监听索引变更事件并重新加载。需要检查的是：被监视根目录的注册、防抖队列、事件路径分类、协调错误以及事件路由。除非有一致性证据支持改动，否则保留源实现的 800 ms 防抖与有界抽取（drain）行为。

## 全文件通知（2026-10-14）

创建/删除/重命名路径不再受后缀名或点前缀的限制。协调过程只接受普通文件（`metadata.is_file()`），从而阻止产生目录行。分类逻辑复用 Indexing 公开的 `classify_media_type`，其中包含 `audio` 与通用的 `file`。800 ms 防抖、4,000 事件的抽取上限以及 200 路径的发出上限保持不变。Windows 集成回归测试覆盖了隐藏的未知文件、无扩展名文件、音频、目录排除以及移除。通用的修改事件仍像之前一样被忽略；启动遍历会在下次重启后刷新文件大小与修改时间。这并不构成对竞态/目录重命名的穷尽覆盖。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与既有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报以墙上时钟计算的 span 时长与错误。领域服务契约、事件负载与源实现的操作顺序均未改变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。


## 浏览器运行时更新 — 2026-10-15

浏览器运行时会跳过 StartupRefresh，也不会监视每一个收藏项。`set_browse_directory(Option<&str>,bool)` 替换当前生效的订阅；recursive=false 为默认值，只有在查看后代目录时才为 true。`retry_browse_watch()` 在显式刷新/重新激活时重试注册。`invalidate_browse_directory`/`take_directory_dirty` 与 FileManager 协调完整重读。

notify 的 RecommendedWatcher 在初始化或注册失败时回退到 PollWatcher（2 秒）。订阅操作在线程之外执行，并在修改实际注册路径之前检查代次（generation），从而防止过期的工作线程对较新的位置取消监视。注册时会发出一次协调提示，以覆盖“列举目录”与“建立订阅”之间的空档。文件的 Create/Remove/Rename/Data/Metadata/Any 事件都会与本地 stat 结果比对校验；AccessTime 被忽略。保留 800 ms 防抖、最多 4000 次通道抽取以及 200 个发出路径的上限； incoming 与合并后的路径容量为 4096。溢出时收敛为根目录级失效。外部删除会清除可见的成员列表，同时保留标签记录。错误会记录精确路径以及回退失败情况。原生的创建/删除/覆盖/新建目录都已被实际演练；强制回退以及断开的网络驱动器尚未演练。


## 注册时序 — 2026-10-16

WatcherService::browse_registration_pending()->bool 用于暴露最近一次异步监视注册尝试是否已经完成。FolderWatcher 保留 requested/completed 两个代次原子值，并在同一把原生监视器锁的保护下发布完成情况，因此旧的工作线程无法覆盖较新的完成状态。FileManager 可取消的目录工作线程会在 read_dir 之前等待就绪。注册之后不再发出合成的根事件；真实的 Create/Remove/Modify 事件、800ms 防抖、有界处理与溢出失效仍然保留。这在不进行第二次完整扫描的前提下关闭了“先枚举、后订阅”的空档。原生监视失败时仍会尝试 PollWatcher；终态错误依然只记录日志，而不会阻止目录浏览。
