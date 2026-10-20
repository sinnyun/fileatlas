# Everything 搜索与搜索栏提示 — 2026-10-17

## 当前实现

- 顶部文件名/路径搜索和 MCP `search_media` 经 Cordis EverythingSearchService 调用官方 SDK 3.0.1.12；标签、日期、评分、类型和分页仍遵守 FileAtlas 查询语义。
- 当前目录搜索限定目录；收藏集合使用已登记根目录；没有根目录时搜索 Everything 全索引。
- 真实候选路径通过数据库公开 `cache_search_files` 进入临时浏览缓存，从未浏览/扫描过的文件也能显示。普通/扩展 Windows 路径复用行 ID，标签/评分和固定历史记录保留，搜索清单不写入持久注释数据库。
- SDK 正则模式转义用户文本和根目录，保留原 SQL LIKE 子串及 %/_ 通配语义。中文、空格、方括号与前置 ! 不会被误当作 Everything 搜索操作符。
- 搜索操作在需要时以官方 `-startup -first-instance` 参数准备已安装的 Everything 1.5 后台客户端，不显示独立搜索窗口。软件检查页仍只读；程序不负责下载、安装或升级 Everything。
- 安装/版本检测通过 FileManager ReadModel 在后台读取真实 EXE 版本。搜索栏后：缺失显示“安装 Everything”，1.4 等不兼容版本显示“安装兼容版 Everything”，点击只打开官方下载页；相邻按钮可重新检查。索引加载中显示加载状态，真正就绪后刷新当前搜索。
- 首次/恢复查询也进入后台 ReadModel。忙管道预检避免 SDK 默认 30 秒等待；RPC 看门狗通过官方 ShutdownClient 取消长时间无响应的客户端，查询上限 15 秒、就绪探测上限 2 秒。
- 成功查询在 FileManagerSnapshot/MCP 结果中保留 search_provider=everything；异常及超过 250,000 候选时回退 SQLite，不能把检测到已安装当作搜索成功。

## 更正先前诊断

原先唯一的 PID 38460 实际上是 Windows 服务 Everything.exe -svc，只负责低层索引，不是可供 SDK 查询的搜索客户端。此前“1.5.0.1423 因 Session 0 不兼容而必须升级”的结论没有依据，已更正。已安装的 1.5.0.1423 搜索客户端启动后，真实 SDK 搜索能返回 small.png 和未浏览过的中文文件。另一个适配错误是 SDK 3.0.1.12 两个 viewport setter 总是返回 FALSE；源码确认其状态实际已更新，现通过 getter 验证。

## 验证环境及入口

Windows / 150% DPI；Rust 1.97.1 Windows MSVC；Everything 1.5.0.1423。

- 构建：cargo +1.97.1 build --locked -p fileatlas-shell -j 2。
- 检查：cargo +1.97.1 fmt --all -- --check；cargo +1.97.1 check --locked -p fileatlas-shell -j 2。
- 回归：cargo +1.97.1 test --locked -p fileatlas-plugin-everything-search -p fileatlas-plugin-database -p fileatlas-plugin-file-manager -p fileatlas-plugin-mcp -p fileatlas-plugin-i18n -j 2。
- 同进程搜索：node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-everything-final --plugins dev-test --allow-mutations --scenario scripts/validate-everything-search.cjs。
- 提示状态：scripts/validate-everything-status.cjs，隔离 profile target/mcp-everything-status。FILEATLAS_EVERYTHING_EXE 指向该 profile 的临时 EXE 路径，按缺失、官方 1.4.1.1032 EXE、真实安装的 1.5.0.1423 EXE 顺序检查并手动重新检测。两个版本文件只用于读取版本资源，测试不执行、不安装它们。
- 全部实时交互/截图使用 FileAtlas 同进程 MCP；没有用通用 UI 自动化替代。

## 最终验证结果

- 上述 locked Shell 构建、fmt、40 项自动化测试通过（数据库 9、Everything 3、FileManager 24、i18n 4；MCP 当前无单元测试）。最后修复的软件页按钮重复 ID 已重新构建并完成同进程运行验证。
- 搜索脚本完整 8 组检查通过：真实 provider=everything；样例扫描 12 项；未浏览子目录中的中文/方括号/前置感叹号文件；目录约束；分页；文件类型；无匹配；顶栏实际结果；范围弹层与时间开关；静置无额外 snapshot revision；软件检查页。
- 提示脚本 3 组通过：不存在的 EXE、真实 1.4.1.1032 版本资源、真实 1.5.0.1423 版本资源。手动重新检查按 missing → incompatible → ready 切换，静置不重复刷新。
- `node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`、`node scripts/check-layering.cjs` 和 `git diff --check` 通过。架构脚本仍报告已有 preview/media-playback 遗留空目录，不影响本轮通过。
- 已实际检查截图：搜索后的范围弹层四项完整可见；缺失/版本不兼容时搜索栏后的琥珀色安装提示清楚可见；软件页同时显示 QuickLook、Everything、版本及官方/GitHub 链接，保持已接受的主窗布局。

可复查证据：[精简报告](../../../.local/validation/artifacts/2026-10-17-everything-search/verification-summary.json)、[搜索范围](../../../.local/validation/artifacts/2026-10-17-everything-search/search-scopes.png)、[软件检查](../../../.local/validation/artifacts/2026-10-17-everything-search/software.png)、[缺失安装](../../../.local/validation/artifacts/2026-10-17-everything-search/missing.png)、[版本不兼容](../../../.local/validation/artifacts/2026-10-17-everything-search/incompatible.png)、[兼容安装](../../../.local/validation/artifacts/2026-10-17-everything-search/ready.png)。报告省略本机全索引命中的非测试路径；完整 MCP 原始调用仅保存在 target 隔离档案。

最终成功场景使用已就绪的客户端，没有观测到首次全盘索引的 loading→ready 分支。后台客户端启动曾实际观测到，但修复后的长时间冷索引自动恢复与 RPC 超时取消分支仍需独立覆盖，不能据本次成功查询宣称已验证。官方下载按钮的 URL 已核对，验证未实际打开浏览器或运行安装器。

## 搜索位置切换补充

按用户追加要求，顶栏搜索框后增加“当前文件夹 / 整台电脑”按钮；原字段“范围”按钮继续控制文件名、路径、标签、时间。

- 当前文件夹：当前路径作为 Everything 查询根目录，SQL 使用同一递归目录约束，包含直属文件和全部子文件夹；不受“显示子文件夹”普通浏览开关影响。
- 整台电脑：清除当前路径/登记目录及 live cache 限制，给 Everything 传空 roots，以其实际索引卷、文件夹与排除规则为范围。保留日期、标签等其他筛选；不会修改 Everything 自身配置或临时扫描未索引磁盘。
- 切换立即重跑搜索并保存 `settings.search_entire_computer`；旧设置默认 false。原导航路径保留，清空搜索后恢复原目录浏览。没有具体目录时，本地模式沿用收藏集合登记目录，悬停提示明确说明。

最终 locked Shell 构建、fmt 和四项治理脚本、diff --check 通过。未新增或重跑单元测试。
同进程 MCP 在 `target/mcp-everything-location/search-location` 创建唯一命名的三个真实文件：当前目录一个、其 nested 子目录一个、相邻 selected-sibling 目录一个。关闭浏览递归时，本地搜索为 2 项；切换全机为 3 项；真实 GPUI 指针点击顶栏按钮后恢复 2 项；静置 revision 不变；清空恢复目录浏览；选择写入设置，重启同一隔离 profile 恢复 true。

精确命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-everything-location --plugins dev-test --allow-mutations --scenario scripts/validate-everything-location.cjs`。
重启读取使用同一入口与 profile、`--plan target/mcp-everything-location/restore-plan.json`，对 `get_ui_state.file_manager.search_entire_computer == true` 断言通过。
截图沿用既有主窗设计，检查按钮、字段范围和 Everything 提示完整并排可见，没有裁切。
[当前目录截图](../../../.local/validation/artifacts/2026-10-17-everything-search/location-current-folder.png)、[整台电脑截图](../../../.local/validation/artifacts/2026-10-17-everything-search/location-entire-computer.png)、[验证摘要](../../../.local/validation/artifacts/2026-10-17-everything-search/location-report.json)。

## 官方参考链接

- [官方 SDK 3 说明、API 与版本](https://www.voidtools.com/forum/viewtopic.php?p=83808)
- [Everything Windows 服务职责](https://www.voidtools.com/support/everything/everything_service/)
- [官方启动参数](https://www.voidtools.com/support/everything/command_line_options/)
- [官方下载：1.4 stable 与 1.5 beta](https://www.voidtools.com/downloads/)

## 边界

依赖 Everything 已有索引；不更改其索引范围或配置。Everything 1.4 不支持本适配器的 SDK 3 接口，会得到兼容版本提示。SDK 3 静态库保留 MIT 授权；不打包 Everything 应用。MSVC 仍有 LNK4098 默认 CRT 警告。网络盘、离线盘、超长路径及 250,000 条上限仍需对应系统规模验证；普通编译和单元测试不能替代这些实测。

