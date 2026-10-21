# Development test 插件对接 / 开发测试插件对接

状态：一个可选启用的 GPUI 面板会扫描已生成的磁盘媒体 fixture，并为 FileManager/SQLite
规模检查注入或移除 1,000、10,000 或 100,000 条虚拟媒体行。它是开发工具，被排除在 `core` profile 之外。
`start-dev.bat` 默认为 `all`，其中包含本插件，并把它的
数据库、settings、session 与缩略图缓存重定向到 `target/dev-test`。

## 生命周期与依赖

- 通过 `FILEATLAS_DEV_PLUGINS=dev-test` 或 `start-dev.bat dev-test` 选择启用。
- Cordis 插件名：`fileatlas.dev-test`；服务：`fileatlas/dev-test`。
- 需要 `DatabaseService`、`IndexingService`，以及 shell 的 `UiBridgeService`。
- 在 `workspace.dev-test` 中拥有一个 GPUI 面板；插件停止时 Cordis 会移除其服务与视图
  贡献。
- 仅当这个可选启用插件运行时，本插件才向共享 MCP 注册表贡献
  `dev-test.status`、`dev-test.scan_fixture`、`dev-test.inject_virtual` 与 `dev-test.clear_virtual`。变更操作遵循宿主
  MCP 变更策略。任务启动是异步的；轮询状态直到
  `running=false`，然后使用常规读取/截图工具检视 FileManager。
- MCP schema：`dev-test.status`、`dev-test.scan_fixture` 与
  `dev-test.clear_virtual` 不接受参数；`dev-test.inject_virtual`
  接受 `count` 为 1000、10000 或 100000。任务工具受变更门控。
- 对该 profile，shell 会启用 FileManager、Navigation、Thumbnails、Indexing、Preview 以及
  它们所需的提供者。

## 面板操作与数据

- **扫描示例目录** 注册 `test-data/fileatlas-fixtures` 并调用
  真正的递归 `IndexingService::scan_folder`；进度与已索引行数
  显示在面板中。`FILEATLAS_TEST_LIBRARY` 可以选择另一个目录。
- **注入 1K / 10K / 100K** 以事务方式仅替换 fixture 文件夹下隐藏的
  `.fileatlas-virtual-benchmark` 路径前缀。各行在 image/video/audio 类型、文件大小、尺寸与
  评分上有所变化。有四分之一的行指向 `.dev-support/preview.png`；虚拟路径从不创建物理文件。
- **清除虚拟行** 仅删除那个保留的路径前缀。真实
  扫描得到的 fixture 行与物理文件保持完好。
- 操作在 GPUI 线程之外运行，并在完成后发出 `MediaIndexChanged(())`，
  以便 FileManager 重新加载数据库。
- 所有数据库工作都使用带类型的 `DatabaseService` 操作。benchmark
  replace/clear 方法把保留前缀的删除与行插入保持在同一个
  writer 事务内。浏览器模式的 benchmark 行使用 session 缓存，且不
  持久化带评分的合成媒体；replace/clear 也会移除由较早运行持久化的保留前缀
  行。进度回调每 10,000 行报告一次。

## 数据隔离与调试

启用时，shell 使用 `target/dev-test/fileatlas.db`、`settings.json`、
`session.json` 与 `thumbs/`，而不是正常的 FileAtlas 用户数据。用 `FILEATLAS_DEV_TEST_DIR` 覆盖
根目录，或仅用 `FILEATLAS_DEV_TEST_DB` 覆盖 DB 文件。
先检查面板状态；然后检视隔离的数据库与缩略图
缓存。fixture 资源与可复现的生成器记录在
`test-data/README.md`。

运行可重复的检查：

```powershell
cargo +1.97.1 test -p fileatlas-plugin-dev-test --locked
```

测试使用生产索引器扫描已签入的生成格式，并
针对一个临时 SQLite 文件执行一次 100,000 行的替换/清理循环。
交互式 GPUI 性能与滚动仍需在 Windows 上以 `dev-test` profile
启动桌面应用。

## 本地化集成（2026-10-13）

可见控件标签使用由 fileatlas.i18n 提供的公开 UI Bridge 翻译器。稳定命令、源码 settings/schema 值、媒体路径与
用户创建的标签数据保持不变。缺失的翻译回退到英文；
OS/后端错误与某些生成的诊断未被完整本地化。
真正的仅 MCP 验证覆盖与剩余的源码布局差异
记录在 docs/validation-report.md；截图不能证明指针
交互或动画的对等。

## 浮动状态面板（2026-10-14）

在已有的可选启用隔离 dev-test profile 中，定义 dev-test/status（Single，默认520×300）与 dev-test/status-multi（Multiple，默认600×360）使用公开的 UI Bridge PanelCatalog 并渲染 DevTestService::status/is_running。它们是真实的开发任务观察者，不是产品插件或伪造状态。每个实例每200ms 轮询一次状态变化；Cordis 所有者 fileatlas.dev-test 在停止时清理定义/实例。MCP panel_open/close 演练了那同一种面向未来功能插件的公开集成契约。

## 全文件 fixture 预期（2026-10-14）

生产扫描回归现在预期 `test-data/fileatlas-fixtures` 中全部 12 个普通文件，包括 README.txt、XMP sidecar 以及 `.dev-support/preview.png`。虚拟记录生成与 profile 隔离保持不变。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与已有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报挂钟（wall-clock）span 时长与错误。领域服务契约、事件负载与源码操作顺序保持不变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。

## 共享刷新所有权（2026-10-15）

StatusPanel 与有限任务监视器使用 UI Bridge refresh::poll 与 ChangeGate 覆盖 status/running。一个正在运行但未变化的任务不再强制重绘；终止性变化会在监视器停止之前发出通知。
