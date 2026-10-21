# QuickLook 查看器集成

## 身份与所有权

- Cordis 插件 `fileatlas.quicklook`，服务 `fileatlas/quicklook`（`QuickLookService`）。
- `Config=()`、`Input=()`。没有启动依赖、视图、工作线程或事件。
- Shell 在 FileManager/MCP 之前启动它，并在那些消费者之后停止它。
- Cordis 拥有服务的发布/清理。FileAtlas 从不启动、终止、
  安装、下载、更新或卸载 QuickLook；它只向一个
  由用户启动的实例发送请求。

## 契约

- `open(path: impl AsRef<Path>) -> Result<(), QuickLookError>` 是一个阻塞的、
  动作时刻的操作。它要求一个已存在的普通文件。FileManager 与 MCP
  在一个阻塞工作线程上调用它。它自身从不读取/解码/变换媒体。
- **仅请求。** FileAtlas 从不启动、重启、终止或安装
  QuickLook；该进程归用户所有。唯一的派发目标是运行中实例的
  `\\.\pipe\QuickLook.App.Pipe.<user SID>` 管道，写入为一行
  `QuickLook.App.PipeMessages.Toggle|<path>|\r\n`。当没有任何东西应答该
  管道时，按键以 `NotRunning` 失败，且 FileManager 显示一个翻译后的提示，
  告诉用户去启动 QuickLook。较早的 `QuickLook.exe <path>` 入口在
  2026-10-16 应要求被移除：它是 QuickLook 的第二实例
  启动器，会先启动 .NET/WPF 运行时（1.06-1.12 秒）再转发，而
  由 FileAtlas 启动的实例会保留其窗口直到用户关闭它，随后
  便彻底不再应答预览。
- 通过该管道在应用内测量（2026-10-16，`target/mcp-ql-pipe-measure` 与
  `target/mcp-ql-pipe-close`）：请求在 15-33 毫秒内离开 FileAtlas，包含 MCP
  往返，而显示同一文件的查看器在按键后 37-254 毫秒出现，
  前提是 QuickLook 与该文件类型的插件已预热。第一次使用某个
  插件类型的按键耗时 600-755 毫秒，那是 QuickLook 在加载该插件，而非派发。
- 请求行总是在管道被打开之前组装完成。QuickLook 的
  读取循环会在一个不携带任何行的连接上崩溃，因此一个半开的客户端
  会让用户的 QuickLook 保持存活，却永远听不见任何
  跨进程请求。
- 管道路径由当前令牌派生，并在首次成功之后被缓存。Win32 要求
  传给 `WaitNamedPipeW` 的名字中包含计算机组件，因此裸形式总是以
  `ERROR_BAD_PATHNAME` 失败。QuickLook 恰好拥有一个服务器实例，所以
  `WaitNamedPipeW`（250 毫秒）为打开操作设卡；打开一个繁忙的管道会阻塞本
  工作线程。一个无人创建过的名字会立即失败，而 wait/open 这一对会重试三次，
  以免一次快速的第二次按键或针对单实例的竞争被报告为不可达。
  诊断探测必须只等待，绝不打开。
- `PrivilegeMismatch` 需要一次 Windows 完整性级别比较，而那个进程
  快照只在 QuickLook 的管道拒绝本客户端（`ERROR_ACCESS_DENIED`）时运行；
  一次成功或实例缺失的派发不执行任何快照。
- 错误：`NotRunning`（没有实例应答，或它拒绝本进程）、
  `MissingFile`（目标被移除）、`PrivilegeMismatch`（以提权运行的 QuickLook）、
  `NotInstalled`（非 Windows 构建；不存在 Windows 安装探测）。
- `snapshot() -> LaunchSnapshot` 返回最后一次尝试的路径与派发
  结果/错误。`pipe_toggle_sent` 仅表示派发；它不断言
  外部渲染、编解码器、当前可见性或焦点。
- Windows MCP 诊断使用 `QuickLookService::inspect()` 与
  `visible_windows()`；Win32 窗口/进程/管道实现保持为本 crate
  私有。`QuickLookInspection` 是只读诊断状态，并不
  声称 QuickLook 已渲染或聚焦了某个文件。

## 消费者与失败检视

- FileManager 为所有
  文件类型把 Space、Enter、其保留的自定义打开键、文件双击、
  上下文菜单 Preview 以及检视器缩略图点击路由到本服务。长按 Space 或带修饰键的 Space 不会反复切换它。
- FileManager 在失败时显示一个 GPUI 提示，已翻译：`NotRunning` 告诉用户
  自行启动 QuickLook（如果缺失则去安装它）。本应用既不
  打开安装程序、不启动查看器，也不回退到另一个播放器。
- MCP 通过 `McpToolRegistry` 从本功能 crate 贡献
  `quicklook.open_media {media_id}`、`quicklook.snapshot` 与
  `quicklook.inspect`；open 需要变更权限并调用同一个服务。MCP 宿主保留
  其图片内容诊断 `screenshot_quicklook`，因为功能工具
  契约当前返回结构化的 JSON 值。
  `control_file_manager {action:"preview",media_id}` 演练 FileManager 的提示流程。
- 检视 `quicklook.snapshot`、`get_ui_state.quicklook` 与
  `get_ui_state.quicklook_notice`，以及 `fileatlas_plugin_quicklook` tracing span。
  弹窗渲染与 UI 静息状态检查使用同进程 FileAtlas MCP。

## 上游参考

- https://github.com/QL-Win/QuickLook/wiki/Develop%2C-build-and-integrate
- https://github.com/QL-Win/QuickLook/blob/master/QuickLook/App.xaml.cs
- https://github.com/QL-Win/QuickLook/blob/master/QuickLook/PipeServerManager.cs

不捆绑任何 QuickLook 二进制、库、插件或源码。已有的
FileManager 缩略图、检视器预览以及 indexing 的 FFmpeg 支持仍
独立于本服务。
