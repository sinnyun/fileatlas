# 分组标题背景（2026-10-14）

按用户本轮要求，PDF等所有分组标题增加整行不透明蓝灰色背景#343e49、细边框、4pt圆角，hover背景#404d5c。共用Gallery标题适配器覆盖类型/月份/目录分组、网格/列表、展开/折叠。保留40pt高度、原组计数和折叠事件，虚拟行几何与插件数据契约未变。

设计依据：现有sidebar-pages-preview-2026-10-14-v2及final-v5主界面结构；标题背景为本轮用户明确新增设计。未改变其他屏幕设计或补充其他待验控件。

环境：Windows MSVC Rust1.97.1，150%DPI，外窗1942×1256；独立EXE target/debug/photag-shell-grouping-header.exe。通过sqlite只读backup复制上一轮隔离248文件profile至target/mcp-grouping-header-validation，正常用户数据库/设置及现有窗口未修改。MCP与GPUI同进程；权限仅为验证UI变更显式开启。

命令：

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-grouping-header.exe
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-grouping -j 2
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-grouping-header.exe --profile target/mcp-grouping-header-validation --plan docs/validation-ui/2026-10-14-group-header/plan.json --allow-mutations
```

fmt、locked check、独立Shell编译通过（仅rustc -o多输出提示）。14步同进程MCP完成：真实文件类型标题、hover、实际GPUI标题点击折叠PDF、折叠后PNG/TXT相邻分组、列表及修改月份分组截图。已查看header-grid/header-collapsed，标题背景与网格明显区分，箭头/数量可读。不是完整应用或其他DPI/系统操作验收。

相关自动测试21项通过（Gallery17、Grouping4），0失败。
