# FileAtlas 更新器插件端口

状态：源实现更新器的发布版本检查、数字版本号比较、分阶段
资产下载、SHA-256 校验、可执行文件替换与回滚、重新启动，
以及启动清理，都由一个 Cordis 功能 crate 暴露。Shell 注册
以及 Settings 持有的“检查、自动发布提示、下载、
进度与安装”界面已经接通。Windows 上的可执行文件替换与
重新启动尚未验证。

## 生命周期与依赖

- Cordis 插件：`fileatlas.updater`；服务：`fileatlas/updater`
  （`UpdaterService`）。`UpdaterPlugin` 发布该服务，且没有运行时的
  插件依赖。
- `UpdaterService` 暴露同步与后台的最新版发布检查、
  带原子字节进度的后台下载、直接安装、由 shell 调度的
  安装（`schedule_install`/`take_pending_install`），以及启动清理。
- 外部依赖：`ureq = "2"`（HTTP；在主版本 2 内解析）、
  `sha2 = "0.10"`、`dirs = 5`（工作区）、`serde_json = 1`（工作区）、
  `cordis-rs = 0.11.1` 与 `thiserror = 2`（工作区）。已存在的上游
  FileAtlas 清单同样声明了 `ureq = "2"` 与 `sha2 = "0.10"`。
- Shell 会在 Settings 之前启动 updater 插件。应用 updater
  插件会执行尽力而为的启动清理并发布该服务。Settings
  通过 Cordis 获得该服务，并负责更新界面，包括
  源风格的浮动提示，以及会话范围内“暂不处理”的忽略行为。

## 输入、输出与行为

- `check()` 通过 HTTPS 请求 `OleksandrShabaldas/FileAtlas` 的最新发布版本，
  优先选择 `-windows-x64.exe`（其次任意 `.exe`），当该发布版本
  不比本 crate 的包版本更新时返回 `None`。
- `UpdateInfo` 包含规范化版本号、说明文本、发布页面、可执行文件
  资产元数据，以及可选的 `SHA256SUMS.txt` URL。`notes_plain()` 保留了
  源实现有限的 Markdown 清理逻辑。
- `download()` 会把 `.part` 文件存放在 `%LOCALAPPDATA%/FileAtlas/updates` 下，
  通过 `AtomicU64` 报告字节进度，在校验和已发布时校验匹配的 SHA-256
  条目，随后提升为 `fileatlas-<version>.exe`。没有校验和清单的发布版本
  仍被接受，与源实现一致。
- `install()` 会把正在运行的可执行文件重命名为 `.old`，把暂存的
  可执行文件复制到原位置，重新启动，并移除暂存内容。复制失败时
  会尝试恢复原始二进制文件。重新启动失败会保留已安装的新
  二进制文件，并报告需要手动启动。
- `cleanup_previous()` 以尽力而为的方式移除 `.old` 二进制文件以及更新暂存
  文件夹中的 `.part`/`.exe` 文件；只应在成功启动之后调用。
- 异步操作返回标准的 MPSC 接收器。丢弃接收器会忽略
  结果但不会取消工作线程；源实现没有取消令牌。

## 集成与调试

请对照 `vendor/photag-source/src/update.rs` 的实现细节，
以及 `vendor/photag-source/src/main.rs` 中的 Settings 更新流程
（`SettingsTab::Updates`、`start_update_check`、`show_update_offer` 与
`on_exit`）。遗留界面会在安装前保存会话状态，并在退出过程中
安装。当前的 GPUI Settings 动作使用显式的两步
确认，并先刷入最新的 `SessionService` 快照，随后调度
暂存路径并退出 GPUI 应用。在 `application().run`
返回之后，shell 才安装更新，因此旧进程会在
可执行文件被替换之前退出。安装失败会写入
`%LOCALAPPDATA%/FileAtlas/update-error.txt`。此实现使用本包的
`CARGO_PKG_VERSION`；宿主包版本必须跟随应用
发布版本。Windows 的替换与重新启动需要真实的 Windows 验证。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与既有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报以墙上时钟计算的 span 时长与错误。领域服务契约、事件负载与源实现的操作顺序均未改变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。
