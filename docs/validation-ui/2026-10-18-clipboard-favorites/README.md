# 剪贴板本地收藏验证 — 2026-10-18

## 行为

置顶改为本地收藏。收藏不参与 500 条临时历史淘汰或清理；普通删除被数据库拒绝。编辑、重复收录不能取消收藏。只有“从收藏中删除”明确确认后才永久删除内容。旧 SQLite pinned 列和旧数据兼容保留，无需重建用户数据库。

## 环境与命令

Windows MSVC，Rust 1.97.1；沿用布局参考 `docs/reference-ui/concepts/clipboard-workbench-layout-2026-10-18-v2.png`，本次为局部语义/控件调整。代码图工具连接关闭，使用源码追踪备用路径。

- `cargo +1.97.1 fmt --all -- --check`：通过。
- `cargo +1.97.1 check --locked -p photag-shell -j 2`：通过。
- `cargo +1.97.1 test --locked -p photag-plugin-database --lib -j 2`：12 项通过，新增收藏保护/旧数据/重启回归。
- `cargo +1.97.1 build --locked -p photag-shell -j 2`：链接完成，但正在运行的标准 exe 被 Windows 锁定，输出安装阶段拒绝访问；未结束用户进程。
- `cargo +1.97.1 rustc --locked -p photag-shell -j 2 --bin photag-shell -- -o target/debug/photag-clipboard-favorites.exe`：通过，使用独立文件完成同进程 MCP 验证。既有 LNK4098 警告，另有显式输出路径相关 rustc 提示。
- `node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`：通过。既有两个无 Cargo.toml 目录警告保留。
- `node scripts/photag-mcp-debug.cjs --exe target/debug/photag-clipboard-favorites.exe --profile target/mcp-clipboard-favorites-verified --plugins core --scenario scripts/validate-clipboard-favorites.cjs --allow-mutations`：连续运行三次通过。core,dev-test 强制隔离，无系统剪贴板写入或 OS 输入注入。
- 相同客户端，profile `target/mcp-clipboard-favorites-layout`、scenario `scripts/validate-clipboard-workbench.cjs`：91 步通过。

## 观察与证据

`favorite-created.png`：真实 GPUI 收藏到本地、收藏筛选、中文分组即时保存。安静状态 1100ms revision 不变。
`restart-restored.json`：第二进程 PID 27940 恢复第一进程 PID 17156 的收藏与分组；普通垃圾桶不再对收藏显示，点击原位置不删除。删除分组保留收藏内容。
`favorite-delete-confirmation.png`：明确的永久删除文案，取消后内容仍在，重新确认后删除。
`restart-deleted-verified.json`：第三进程 PID 34152 确认内容和分组均不复活。
`workbench-favorites.png`：实际渲染保留已接受的三栏布局、深蓝层次和星形图标，旧置顶文案已替换为收藏，右侧单独删除入口可见。未扩大界面重设计范围。

最初一次点击坐标落在取消按钮导致断言失败；按同进程截图校正后，最终三进程场景含明确取消和确认断言全部通过。早期失败仅发生于隔离测试目录。

## 限制

普通运行的旧窗口和标准 exe 仍未更新；新版独立 exe 已构建。实际 OS 捕获、跨应用粘贴和全局快捷键路径未在本次验证。当前收藏仍为文本，不覆盖图片/文件。
