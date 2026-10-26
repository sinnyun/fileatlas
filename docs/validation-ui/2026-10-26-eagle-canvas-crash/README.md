# Eagle 图片缩放/拖动崩溃排查（2026-10-26）

## 结果与修复

用户在 MCP 启动的程序里重现后，FileAtlas 于 2026-10-26 22:34:17（本地时间）退出。stderr 明确记录 `gpui-pre` 构建无障碍树时发现 `Duplicate a11y node id: #1493084778566136873` 并 panic；同一时间 Windows Application Error 记录 `0xc0000409`、参数 7。回溯显示重复节点发生在 FileAtlas 白板界面渲染路径中，经过 GPUI 图片/按钮绘制和插件滚动容器。

根因是 Eagle 属性行复制按钮的 ID 从显示标签生成时对所有非 ASCII 字符都替换成 `-`。因此“所属文件夹”和“缩略图路径”变成相同的 `canvas-copy-----`。当 Eagle 图片被选中、右侧属性行进入无障碍树时，GPUI 检测到重复 ID 并在开发版 panic。该问题出现在属性面板渲染阶段；图片几何缩放算法不是触发点。

修复位于 `crates/fileatlas-plugin-whiteboard/src/view/sidebars.rs`：复制按钮 ID 现在保留唯一标签文本，不再做有损字符替换。

修复后使用相同 Eagle 原图，在同进程 MCP 隔离窗口加载图片、缩放右上角手柄并拖动图片。图片尺寸从 `280 × 159.05` 更新到 `336.61 × 191.21`，位置从 `(380, 280.47)` 更新为 `(415, 273.32)`；场景状态和交互后截图均成功取得，未再出现 panic。

## 运行环境和命令

- Windows，`target/debug/fileatlas-shell.exe`，Rust `1.97.1-x86_64-pc-windows-msvc`；Eagle 保持运行。
- MCP 启动独立的 `fileatlas-shell --mcp` 子进程，profile 位于 `.local/validation/profiles/canvas-crash-repro-20261026c`，AI control 为 Standard。使用项目 MCP 的 `whiteboard.add_images` 和 `control_ui`，没有附着另一个 FileAtlas 进程。
- 构建：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked --offline -p fileatlas-plugin-whiteboard -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked --offline -p fileatlas-shell -j 2` 均通过。`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs` 和 `node scripts/check-feature-ledger.cjs` 均通过。链接阶段有既有 `LNK4098` 警告。
- 设置环境变量 `FILEATLAS_EAGLE_REPRO_IMAGE` 后执行：

  ```powershell
  $env:FILEATLAS_EAGLE_REPRO_IMAGE = '<Eagle 原图路径>'
  node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/canvas-crash-repro-20261026c --plugins whiteboard --access-level standard --scenario scripts/validate-eagle-canvas-resize-crash.cjs
  ```

- 修复后脚本运行：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/eagle-canvas-crash-fixed2-20261026 --plugins whiteboard --access-level standard --scenario scripts/validate-eagle-canvas-resize-crash.cjs` 通过。
- 脚本、画布 Scene 前后状态及同进程截图保存在 `.local/validation/profiles/eagle-canvas-crash-fixed2-20261026/`。最终截图 `screenshots/after-move.png` 显示选中的图片缩放并移动后仍能正常渲染。

## Windows 崩溃记录与覆盖范围

Application Error/WER 中找到三条 `fileatlas-shell.exe` 记录：2026-10-26 22:04:19、22:13:02、22:20:43（本地时间）。三条均为 `ucrtbase.dll`、异常码 `0xc0000409`、异常参数 `7`，进程映像为 workspace 的 `target/debug/fileatlas-shell.exe`。参数 7 对应 `FAST_FAIL_FATAL_APP_EXIT`；该签名说明进程快速失败退出，不能单独确定是变换算法、资源处理还是其他路径触发。归档中只有 `.wer`，没有可用转储或调用栈。现有 MCP 运行记录的 PID 与事件记录的 PID 不同，因此无法把具体一条事件归因到本次验证步骤。

自动化复现的插入使用 `whiteboard.add_images`（与宿主原生拖放共用图片放置回调），没有模拟从 Eagle 窗口真实跨应用拖放。用户报告的原生拖放后崩溃与 Eagle 属性树的重复 ID 根因相符；修复后的真实 Eagle 跨应用拖放仍需用户在新构建程序中复测。
