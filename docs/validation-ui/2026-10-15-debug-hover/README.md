# 调试面板鼠标悬停提示显示筛选（2026-10-15）

按用户要求，调试抽屉默认跳过 GPUI window DEBUG/TRACE 的
`Sending a11y tree update:` 常规重绘记录。共享分类谓词复用现有渲染观察判定，
不会隐藏同框架的 WARN/ERROR、其他 DEBUG 消息或数据库/文件/用户操作日志。
筛选发生在分页前，匹配数量、分页、分类与搜索均遵守该规则。
原始环形缓冲和按ID读取保留；底部“保留”计数仍表示原始缓冲数量。
不新增按钮或改变已接受的抽屉布局，未增加缺失设计控件。

`Filter.hide_ui_refresh` 默认false用于原始查询，抽屉UiState默认true。
MCP `get_debug_info.hide_ui_refresh` 省略/false保留全量原始审计兼容性，true匹配抽屉显示。

Windows/Rust1.97.1 同进程 MCP、隔离 profile `target/mcp-debug-hover-filter`，
原生无障碍provider响应true。实际GPUI指针滑过网格/列表按钮产生10条重绘记录，
原始查询95条，抽屉筛选查询82条且无常规重绘记录；打开统计后仍能读取18条数据库
日志。搜索常规重绘文本返回0，第二页正常返回2条非重绘记录。
真实窗口[截图](../../../.local/validation/artifacts/2026-10-15-debug-hover/hover-filter.png)已检查，保留正常操作与数据库条目。
测试使用MCP指针命令，命令本身的INFO日志仍作为实际调试动作显示。
[原始报告](../../../.local/validation/artifacts/2026-10-15-debug-hover/report.json.gz)包含查询结果、筛选条件和实际EXE SHA-256。

命令：
- `cargo +1.97.1 test --locked -p fileatlas-plugin-debug -p fileatlas-plugin-mcp -j 2`：7项通过。
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、对应check、fmt和共享刷新架构检查通过。
- `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-debug-hover-filter --scenario scripts/validate-debug-hover.cjs --allow-mutations`：通过。

本次仅验证日志显示规则、搜索/分页和真实读取保留；不是对所有产品功能的重新验收。
