# 验证数据目录

测试脚本是源代码，可以提交；运行产生的数据、中间结果和系统状态不可提交。

- `.local/validation/profiles/<运行名>/`：MCP 隔离 profile、fixture、临时 SQLite、
  Settings/Session、缩略图缓存、stderr、完整请求/响应报告及对应截图。
- `.local/validation/artifacts/<验证记录名>/`：从 profile 导出的可复查截图、原始/压缩
  报告、日志、测量数据和摘要 JSON。此目录也用于没有独立 profile 的工具验证输出。
- `docs/validation-ui/<日期-功能>/`：只保存可读的 Markdown 验证结论与可复现脚本；
  测试输入计划可以留下，但结果数据须迁出。根目录 `.gitignore` 忽略整个
  `.local/validation/`，其中内容不会进入提交。

新 MCP 测试直接使用默认目录：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile .local/validation/profiles/<run-name> --plugins dev-test --access-level standard --scenario scripts/<scenario>.cjs
```

不再省略 `--profile` 时会写到 `.local/validation/profiles/mcp-handoff`。
旧 `target/` profile 仅为重跑既有场景保留兼容，新场景不得新建在 `target/`。

验证 Markdown 可链接到本地 artifact；克隆仓库默认不包含这些大文件。摘要应保留足以
审阅结论的信息：工具链、命令、profile 名、关键断言、真实覆盖路径与未验范围。不要
把完整 MCP transcript、fixture 数据库或运行截图复制回 `docs/validation-ui/`。
