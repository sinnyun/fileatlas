# System shortcuts plugin / 系统快捷键插件

## Cordis 身份

- 插件：`fileatlas.shortcuts`；服务：`fileatlas/shortcuts`（`ShortcutService`）。
- 依赖 `fileatlas/ui-bridge`；Cordis 清理时会注销全局快捷键，并
  停止命令转发任务。
- Shell 在正常的 `core` 组合中于 launcher 之后启动它。选择
  `FILEATLAS_DEV_PLUGINS=shortcuts` 也会启用 launcher 依赖。

## 快捷键行为

- 使用开源的 `global-hotkey` crate（Tauri 团队，MIT OR Apache-2.0）。
- 每一个生命周期所属、`System` 作用域的 `Keybinding` 贡献都从
  `UiCatalog.commands()` 读取。Settings 中的覆盖值优先级更高；缺失的值
  使用插件默认值，空值则禁用绑定。launcher
  贡献 Ctrl+F，剪贴板工作台贡献 Ctrl+Shift+V。
- 按下某个绑定会发出其注册的 `UiCommandInvoked` 命令 ID。在
  Windows 上，宿主在分发之前会把 FileAtlas 恢复并置于前台。
- `ShortcutService::start()` 和 `poll()` 必须运行在 GPUI 窗口/事件循环
  线程上，这是 `global-hotkey` 在 Windows 上的要求。插件自身拥有
  注册与停止时的清理责任。
- 设置变更后，会在所属的 UI 线程上注销并重新注册系统绑定。Settings 捕获按键期间绑定会暂停。注册冲突
  与操作系统拒绝都会被记录日志；该插件命令对菜单仍然可用。
- GPUI 宿主通过其现有的共享 100 ms 刷新循环
  轮询 `ShortcutService::poll(hwnd, overrides, capture_active)`。

## 限制与诊断

- Windows 是产品目标。该库也记录了 macOS 与 Linux X11 的支持；
  该依赖不支持 Wayland。
- 系统解析支持字母、数字、F1–F12、导航键、Space、
  Enter、Tab、Escape、Backspace 和 Delete。Settings 会阻止同一作用域内出现重复
  绑定，并要求系统级绑定必须包含修饰键。
- 只要 FileAtlas 正在运行，每个已注册的系统快捷键都是一次系统级占用。
  如果另一个应用已经注册了同一个快捷键，请查看快捷键日志。
- 实际的全局按键投递与前台恢复需要 Windows 运行时
  验证。同进程 FileAtlas MCP 目前没有可在另一个应用处于前台时
  按下系统级快捷键的 API。

## 前台路由修正 — 2026-10-18

ShortcutService::poll 为了兼容性保留其 HWND 参数，但不再无条件地恢复
该 HWND。它分发已注册的命令。工具命令选择
`PanelCatalog::open_presented`；宿主激活由此产生的原生窗口，或在嵌入式模式下恢复
主窗口。直接截图捕获不会恢复 FileAtlas。
真实的操作系统全局按键投递仍未验证；MCP 调用的是同一套命令处理函数。
