# 子文件夹分组：当前目录优先（2026-10-14）

用户要求：第一组显示当前根目录直属文件，然后显示内部子文件夹。分组服务新增arrange_in_scope，FileManager传入MediaQuery.path_prefix作为当前目录；未选目录时优先所有已登记根目录的直属文件组。随后子目录组按路径名称排列，组内保持所选排序。空直属组不伪造。Windows扩展路径前缀、分隔符、大小写差异仅作排序比较规范化，不改写数据库路径或组标题。原标题背景/虚拟行/缓存/会话保持。

源对照：vendor/photag-source/src/ui/gallery.rs build_file_manager_layout及调用处1047–1063只提供日期分组；本次是本会话新分组插件的需求细化。没有改SQLite、上游筛选、文件和标签语义。

Windows MSVC Rust1.97.1，150%DPI，1942×1256；同进程FileAtlas MCP启动target/debug/fileatlas-shell-root-first.exe --mcp。独立profile target/mcp-root-first-validation。隔离fixture target/mcp-root-first-fixture包含a-library/a-root.txt、z-root.txt、inside/m-child.txt、inside/deeper/b-deep.txt和z-library/other-root.txt，每个内容为root-first grouping fixture。正常FileAtlas资料、原窗口未改。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-grouping -p fileatlas-plugin-file-manager -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-navigation -j 2
cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-root-first.exe
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-root-first.exe --profile target/mcp-root-first-validation --plan docs/validation-ui/2026-10-14-root-first/plan.json --allow-mutations
```

结果：fmt/locked check/独立完整Shell编译通过；28自动测试通过（FileManager17、Grouping6、Navigation5）。新增回归覆盖全部根目录优先、根内文件排序稳定、选中嵌套目录作为当前根、扩展路径比较，以及筛选不生成空组。Navigation原有测试仍假定隐藏目录被排除，与之前全文件策略不一致；更新断言为可见.hidden及nested后5项全部通过，没有修改隐藏目录产品行为。

19步同进程MCP通过并对最终报告作计数/路径断言：

- 全部文件：a-library直属2、z-library直属1、a-library/inside1、inside/deeper1，两个根目录均在子目录之前。
- 选a-library：先直属2，再inside1、deeper1；名称降序下根内z-root排在a-root前。
- 选inside：当前目录直属1，再deeper1。
- 搜索b-deep：只有deeper1，不伪造无匹配的当前目录组；清除筛选恢复根目录优先。
- 已查看selected-root-first.png，保留上一轮蓝灰标题背景和列表结构。

实测同时发现MCP SelectPath/ExpandPath消费时重复canonicalize，给普通路径添加Windows扩展前缀，导致已登记普通路径被筛成空列表。请求队列原先已经按canonical路径验证归属；消费端现在保留调用者提供的已索引路径写法，与原生目录点击一致。修复后同一fixture选根目录/内部目录均返回正确文件数。没有放宽已登记目录范围验证。

本轮不重新宣称大库/网络文件系统/全部系统交互验收；其他排序及缓存行为由既有自动回归覆盖，截图针对名称降序和5个隔离文件。
