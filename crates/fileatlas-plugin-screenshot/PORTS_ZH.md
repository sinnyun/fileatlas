# 截图启动器 / Shotoshot 集成

## Cordis 身份与依赖

- 插件：`fileatlas.screenshot`；服务：`fileatlas/screenshot`（`ScreenshotService`）。
- 依赖 `fileatlas/settings`、`fileatlas/ui-bridge`，以及既有的快捷键分发器。
- 可作为独立工具 `fileatlas.screenshot` 在运行时控制；Settings 可以销毁并重新生成它的 Cordis Fiber，从而停止/恢复它的面板、命令和系统快捷键。
- Shell 会在 `core`、`all`、MCP 或 `FILEATLAS_DEV_PLUGINS=screenshot` 配置档中启用它。
- 它注册一个单实例的通用浮动管理面板、一个侧边栏条目、一个打开面板命令、一个捕获命令，以及系统快捷键 `Ctrl+Shift+S`。插件停止时，UI Bridge 与 Cordis 会移除全部贡献项/监听器。

## 上游应用边界

- 在 FileAtlas 启动时，服务会对配置的可执行文件以及受支持的安装位置检查一次，缓存结果，并把发现到的路径存入既有 settings。渲染与快捷键激活使用该启动时的快照；它们不会重新扫描安装位置。用户可以在管理面板中选择另一个 `shotoshot.exe`。
- 如果没有检测到可执行文件，管理面板会提供 Shotoshot 的官方安装页面，并在用户安装它并选择其可执行文件之前保持捕获功能禁用。捕获功能同样会报告清晰的“路径缺失/无效”消息。
- 捕获命令会把捕获请求发送给一个常驻的 Shotoshot 窗口；若不存在常驻窗口，则以 `--capture` 参数启动所选的可执行文件。编辑器与捕获进程由 Shotoshot 控制。
- Shotoshot 是单实例应用。如果它已经在运行，FileAtlas 会向 `Shotoshot.Window` 发送其现有的 Windows `WM_APP+2` 捕获消息；否则以 `--capture` 启动已安装的可执行文件。
- FileAtlas 中不实现任何截图界面、选区遮罩、标注工具或编辑器行为。
- FileAtlas 不会内联（vendor）或构建 Shotoshot，用户需自行安装。启动器会发现同级目录的可执行文件、`%LOCALAPPDATA%\Programs\Shotoshot\shotoshot.exe`，或 `FILEATLAS_SHOTOSHOT_EXE`；管理面板也可以直接选择可执行文件。

## 设置与输出

- FileAtlas 通过既有的 `SettingsService` JSON 模型，以增量方式持久化 `screenshot_tool.executable` 与 `screenshot_tool.save_directory`。
- `Settings.plugin_launch_modes["fileatlas.screenshot"]` 默认使用其管理面板。`PanelOnly` 会在共享面板后面隐藏 FileAtlas 工作区；`Direct` 会把管理面板快捷键路由到 Shotoshot 捕获。专用的 `Ctrl+Shift+S` 捕获命令始终直接调用 Shotoshot。
- MCP 的 `get_ui_state.screenshot_tool` 会报告缓存的可执行文件、可用性、输出目录、`last_capture_action` 与 `last_capture_error`。调试抽屉会收到显式的请求/结果/错误 tracing 事件。
- 默认输出：`%LOCALAPPDATA%\FileAtlas\Screenshots`。
- 该插件会创建目录，并在持有同一个命名 Windows 互斥量、使用原子写穿（write-through）替换的前提下，只更新 Shotoshot 带版本号的设置文件中的输出文件夹字段。Shotoshot 的其他字段会被保留。如果设置文件尚不存在，请先启动一次 Shotoshot 让它创建默认值，然后重试。
- 目录/路径错误会返回给管理面板。捕获进程的启动错误由 Cordis 命令监听器记录；请检查 `fileatlas.screenshot` 的 tracing 事件。

## 限制

- 应用窗口、工具提示和菜单由 Shotoshot 提供，目前只有英文。
- 默认保存目录是 `%LOCALAPPDATA%\\FileAtlas\\Screenshots`；FileAtlas 会把它同步到 Shotoshot 的原生保存设置，使用户最终的 PNG 存储在 FileAtlas 的截图目录下并可在此管理。这一集成尚未把截图行导入照片库数据库。
- 早先的同进程 FileAtlas MCP 只验证了 `PostMessageW` 接受了一次 `WM_APP+2` 的队列投递；用户报告没有出现编辑器，因此那还不足以证明捕获可用。此桥接现在使用同步的 `SendMessageTimeoutW`（3 秒超时），会报告投递失败，并记录请求/结果/错误诊断。重建宿主后的实时捕获以及用户驱动的选区/标注/保存仍需验证。
- 对于这个个人使用的工作区，GPL-3.0 已被接受。此处不改变工作区声明的 MIT 包许可证；若要分发组合后的可执行文件，则需要遵循 GPL-3.0 及依赖声明义务。

PanelOnly 展示方式会通过 PanelCatalog 在一个独立的原生窗口中打开管理面板。WorkspaceAndPanel 仍保持内嵌；Direct 仍会调用 Shotoshot 而不恢复主窗口。

管理面板主体上的 Close 按钮调用 PanelCatalog::close_definition(PANEL)，纠正了原先不相关的表面标志位，使其能同时关闭内嵌与原生两种实例。
