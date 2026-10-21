# Everything 搜索适配器

## 生命周期

- Cordis 插件：`fileatlas.everything-search`；服务：`fileatlas/everything-search`。
- Shell 会在 FileManager 之前发布这个可选服务。发布过程与软件检查都不会启动外部软件。
- 文件名/路径搜索会连接已存在的 Everything 1.5 客户端。若客户端不存在，它会定位一个已安装且兼容的可执行文件，并使用官方的 `-startup -first-instance` 参数启动它，且不打开搜索窗口。不执行任何下载、安装、升级、服务/配置更改或进程终止。
- 启动尝试是串行化的，并且每 30 秒至多一次。索引加载中的结果会启动一个有界的就绪工作线程；真正转为就绪状态时会使 `revision()` 递增一次。FileManager 会通过其既有的刷新监视器重试当前查询。服务销毁会取消就绪轮询；Everything 始终是用户自己的外部应用。

## 公开契约

- `search_paths(query: &str, roots: &[String]) -> Result<Vec<String>, EverythingSearchError>`：只在工作线程上调用的同步查询；返回去重后的完整文件路径。当前文件夹搜索使用该文件夹；集合搜索使用已注册的根；roots 为空切片时搜索 Everything 的整个索引。
- `inspect_installation() -> EverythingInstallation`：阻塞式的注册表/文件版本检查。必须通过 ReadModel 调用，绝不在渲染中调用。状态：`missing|incompatible|ready|unsupported`；包含可选的可执行文件路径以及实际的四段式 EXE 版本。SDK 3 要求 Everything 1.5 或更高版本。
- `revision() -> u64` 与 `index_loading() -> bool`：被动的内存读取。就绪工作线程只在完成时改变 revision 一次，而不会在每次定时器唤醒时改变。
- 便携安装可以使用 `FILEATLAS_EVERYTHING_EXE`；显式配置但不存在的路径具有决定性。探测还会检查 App Paths/卸载位置、Program Files、Local AppData 与 PATH。
- 搜索使用 SDK 3 的正则模式，并对文本/root 路径做转义。FileAtlas 的子串语义（包括既有的 SQL LIKE `%|_` 通配符）被保留；以 `!` 开头的合法名称不会被解释为 Everything 运算符。
- SDK 3.0.1.12 的视口（viewport）设置函数尽管更新了状态，却始终返回 FALSE。适配器会用对应的 getter 校验其值。Unicode 输出缓冲区使用 SDK 要求的字节数（包含 NUL），而不是默默接受被截断的路径。
- 忙探测（busy-pipe probe）会避开 SDK 默认的 30 秒忙等待。每个客户端的关闭看门狗把查询 RPC 限制在 15 秒、就绪探测限制在 2 秒；租约（lease）在销毁客户端之前会先与它的看门狗汇合（join）。恢复搜索时会在 FileManager 的 ReadModel 上排队第一次查询，绝不在 GPUI 线程上等待外部 IPC。
- 结果超过 250,000 条、客户端不受支持/缺失/不兼容，或其他 IPC 失败时返回错误。FileManager 会改用 SQLite 回退；绝不根据安装探测推断出“提供者搜索成功”。

## 分发与诊断

- 官方 SDK 3.0.1.12 x64 静态库/头文件与 MIT 声明：`vendor/everything-sdk`。不打包 Everything 应用程序本身。
- Settings > Software 仍然是只读的安装探测。FileManager 会显示安装/版本/索引加载状态与安装链接，位置紧跟在搜索输入框/范围按钮之后。
- 请检查 `FileManagerSnapshot.search_provider`、`search_media.search_provider` 与 `get_ui_state.file_manager.everything`。每个渲染出的快照都保留自己的提供者，因此并发的 MCP 查询不会覆盖它的诊断状态。
- 早先关于 Session 0 不兼容的解释是错误的：PID 38460 是 `Everything.exe -svc`，即索引服务，而不是 SDK 搜索客户端。在用户会话中启动现有的 1.5.0.1423 搜索客户端后 IPC 即恢复正常；不需要升级 Everything。
- 运行时证据：`docs/validation-ui/2026-10-17-everything-search/README.md`。

