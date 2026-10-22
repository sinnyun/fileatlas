# WorkCrate 后台画布打开修复

范围：页面编辑区启动官方画布会话的路径。不改画板内容、布局、持久化格式或官方
自动化语义；本记录也解释本轮报告的第一条错误。

## 第一条错误的成因

用户看到的原文是：

```text
VectorCraft: doc_inspect: command `document.inspect` is not available right now: no document open
```

`canvas_session` 选中可用通道后调用 `McpSession::open_document`。旧实现会先调用官方
`doc_inspect` 查询当前文档，再决定是否打开目标文件。官方会话确实存在、但里面没有
打开任何文档时（例如 GUI 被自动启动后文档打开被拒，或用户关闭了文档），官方返回上面
这句错误；WorkCrate 把它当作失败原样抛到界面，所以看起来像“VectorCraft 未启动”。

这与“没有安装/没有启动 VectorCraft”不是一回事：控制端口无应答时 FileAtlas 使用
静默 headless 会话，而无头引擎启动时自带一个未命名文档，不会产生该错误。出现该
错误说明当时有一个官方会话在应答，但它没有活动文档。

## 修改

- `McpSession::inspect_optional` 把“没有打开文档”归为空状态；`active_path` 使用它，
  `open_document` 因此继续执行 `open_file`，而不是先报错。
- `McpSession::inspect_bound` 用于画布探测和快照：实时 GUI 关闭已绑定文档时自动重新
  打开该文件。
- `canvas_session` 在实时 GUI 无法让目标文档进入可编辑状态时，丢弃该通道并改用静默
  无头会话打开已保存文档；后台编辑器不再依赖可用的 GUI 窗口。
- 文档最终仍未成为活动文档时返回中文提示，不再显示官方英文协议文本。
- 单测 `an_official_session_without_a_document_is_an_empty_state` 固定错误分类。

## 实测

环境：Windows、Rust `1.97.1-x86_64-pc-windows-msvc`、隔离 profile。

```sh
cargo +1.97.1 test -p fileatlas-plugin-workcrate-boards --locked -j 2
cargo +1.97.1 build -p fileatlas-shell --locked -j 2
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
  --profile .local/validation/profiles/wc-canvas-session --access-level full \
  --scenario scripts/validate-workcrate-canvas-session.cjs
```

结果：

- boards 测试 26 通过、3 项需要官方 CLI/`WORKCRATE_NATIVE_TEST_ROOT` 的集成测试保持
  ignored。
- 无 GUI 场景 exit 0：`canvas.live=false`，生成了 1600×1131 的预览，
  `canvas_state` 返回同一画板。最终二进制在
  `.local/validation/profiles/wc-canvas-session-final` 重新跑过同一脚本，结果相同。
- `check-ui-refresh`、`check-architecture`、`check-feature-ledger` 均通过；boards
  crate 的 clippy 只余既有告警。
- 空 GUI 直达路径：用隔离配置启动
  `vectorcraft.exe --control 18979 --automation-read-root … --automation-write-root …`，
  启动后 `document.inspect` 返回与用户报告相同的 `no document open`；此时
  `workcrate-ui.canvas_open` 成功，返回 `canvas.live=true` 和预览，证明该错误已被
  自动打开文档修复。
- 回落路径：再启动一个读根不包含工程目录的隔离 GUI（`--control 18980`）。实时通道
  无法打开文档，`canvas_open` 自动回落到静默会话，返回 `canvas.live=false` 和预览。
- 两个测试 GUI 都用官方 `app.quit` 正常退出；退出后没有遗留 `vectorcraft.exe`。

## 覆盖与限制

- “GUI 已启动但无文档”与“GUI 拒绝读取文档”两条通道均已实测；实时 GUI 在编辑过程中
  关闭文档的情况由 `inspect_bound` 自动重开，但没有单独做长时间交互测试。
- 面板最终渲染未截图；MCP 返回的快照与预览就是面板 `canvas_open` 使用的同一服务
  路径。界面状态文案“后台官方会话”对应 `live=false`。
- 本地产物：`.local/validation/profiles/wc-canvas-session/`、
  `.local/validation/profiles/wc-canvas-live/` 及同目录下的隔离 GUI 配置，未提交。
