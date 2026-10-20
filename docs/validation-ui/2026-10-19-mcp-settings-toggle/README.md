# MCP 修改权限开关验证（2026-10-19）

> 历史记录：本次最初实现的二态布尔开关已由关闭/常规控制/完全控制三档替代。此页保留当时的验证证据，字段名、截图和场景不能代表当前版本。当前设置语义、可复现步骤及截图请看[分阶段权限验证](../2026-10-19-mcp-access-stages/README.md)。

设置页清单见 [`settings-minimal-2026-10-18.md`](../../reference-ui/concepts/settings-minimal-2026-10-18.md)。页面由 `fileatlas.mcp` 插件注册到现有 Settings 插件设置区，沿用标题、行式开关、说明、分区线与自动保存页脚。

## 可复现步骤

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-plugin-mcp -p fileatlas-plugin-settings
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-settings-toggle-2026-10-19 --plugins all --allow-mutations --scenario scripts/validate-mcp-settings-toggle.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-settings-default-2026-10-19 --plugins all --scenario scripts/validate-mcp-settings-default.cjs
```

Windows，同进程 MCP stdio，隔离 profile 位于 `target/mcp-settings-toggle-2026-10-19`。场景通过 `control_ui` 打开 MCP 插件设置页，保存“允许 AI 通过 MCP 修改 FileAtlas”的勾选状态截图；随后通过同进程 MCP 指针输入关闭开关。`get_ui_state` 立即报告 `settings_plugins.mcp_allow_mutations=false`，`file_manager.set_rating` 的变更请求被路由守卫拒绝，设置页仍显示关闭状态。该场景不改动照片或真实库数据。

## 结果

- 页面呈现与状态：通过。开/关状态截图分别见 [启用](../../../.local/validation/artifacts/2026-10-19-mcp-settings-toggle/enabled.png) 和 [拦截](../../../.local/validation/artifacts/2026-10-19-mcp-settings-toggle/disabled.png)。
- 即时拦截：通过。开关写入现有 `SettingsService` 后，同一进程的下一次修改工具调用读取最新快照并拒绝请求，无需重启。
- 持久化：通过。开关状态保存在既有设置 JSON 的 `mcp_allow_mutations` 字段；新字段使用 serde 默认值 `false`，旧设置文件保持兼容。
- 新配置默认态：通过。字段缺失的隔离 profile 启动后状态为 `false`，修改请求也被拦截。
- 未验证：关闭后的进程由用户通过 FileAtlas 窗口重新打开开关；出于权限边界，MCP 的 `control_ui` 在关闭时也受拦截，因此没有通过 MCP 自行解除拦截。

## 截图和界面差异

MCP 设置页沿用十页设置完整概念的浮窗外框、左侧分组导航、正文标题/细线分区和右侧开关行。该功能页作为后加插件项，没有单独生成新概念图；采用简洁母版规范。实机默认 1120×840 逻辑像素，Windows 缩放 150%。页面正文当前仅一项，剩余区域留白；与母版的短页面排版一致。截图捕获的是同进程 FileAtlas MCP 的窗口。
