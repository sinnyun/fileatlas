# WorkCrate 原生文档资源诊断中文化与修复提示

范围：仅原生文档打开/关联守卫的用户文案与可操作提示；不改画板内容、布局、
持久化格式或官方自动化接口。

## 触发文档与判断

- 触发文档：`开发中间会话传递/新建项目 1.vectorcraft`（工作区真实文档，不是测试夹具）。
- 缺失链接：`docs/2026-10-21 20-46-04.png`。该文件在 git 中由 `39cea6b` 加入、
  `2d4a956` 删除，当前磁盘上确实不存在。
- 同一文档的另一链接 `docs/reference-ui/final-v5/00-main-library.png` 存在。静默
  headless 会话的读取根位于文档所在目录，官方 open report 仍会把它列入
  `missingLinks`；这是沙箱产物，界面不应把它说成缺失。
- 修复方式：在 VectorCraft 中恢复或重新链接缺失文件并保存，然后回到 WorkCrate
  点击“关联工程”重新选择该文档；`revalidate_open_headless` 收到干净报告时会自动
  删除诊断缓存。

## 实测

环境：Windows、Rust `1.97.1-x86_64-pc-windows-msvc`、隔离 profile。

```sh
cargo +1.97.1 test -p fileatlas-plugin-workcrate-boards --locked -j 2
cargo +1.97.1 build -p fileatlas-shell --locked -j 2
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
  --profile .local/validation/profiles/wc-diagnostics-20261022d --access-level full \
  --scenario scripts/validate-workcrate-diagnostics.cjs
```

结果：

- 25 项 boards 测试通过，3 项需要官方 CLI/`WORKCRATE_NATIVE_TEST_ROOT` 的集成测试保持
  ignored。
- MCP 场景 exit 0；固定项目 `780a7566-4b0e-4647-a5d2-d0ab22588a39` 的关联请求返回：

```text
资源文件缺失：2026-10-21 20-46-04.png。请先在 VectorCraft 中恢复或重新链接这些资源并保存文档，然后回到 WorkCrate 点击“关联工程”重新选择该文档以重新校验；修复前该文档无法打开或重新关联。
```

- 断言确认：只列出真实缺失文件；不出现 `00-main-library.png`、`missingLinks`、
  `\\?\` 等内部细节；关联被拒后项目记录仍在。
- 单元测试另覆盖：缺失文件名超出三个时的总数、改写链接/警告计数、沙箱过滤，以及
  中文提示不含原始 JSON。

## 覆盖与限制

- 未启动 GUI 时的 headless 关联路径已实测；`open_safely` 读取诊断缓存的 GUI 分支需要
  运行中的官方控制端口，本轮未实测，但其文案复用同一 `describe_open_report` 并由
  单元测试覆盖。
- 面板最终渲染未截图；MCP 返回的字符串就是 UI `perform` 路径显示的 `BoardsError`
  文本，但本轮没有视觉证据。
- 触发文档仍保持损坏链接；本轮只修正提示，不替用户恢复或改写该文档。
- 本地产物：`.local/validation/wc-diagnostics-run4.log` 与
  `.local/validation/profiles/wc-diagnostics-20261022d/`，未提交。
