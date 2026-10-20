# 独立分组插件验证（2026-10-14）

依据：用户本轮新增独立分组插件及文件管理器式分组要求；主界面保留 `docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png` 及说明，筛选菜单遵循 final-v5/00 与05的结构。只扩展原分组入口及工具菜单，不进行整体重设计。设计中的其他操作未在本轮重新验收。

## 实现和语义

`fileatlas.grouping` / `fileatlas/grouping` 由 Cordis 管理；Shell只组合启动与停止，FileManager消费分组规则与分段结果。原按月分组迁入新crate；文件类型是大小写不敏感扩展名；修改/创建时间目前按本地月份分组，创建时间读取真实文件系统值，不可用归入未知日期；根文件夹是已登记目录，子文件夹是文件实际父目录。日期分组不再受限于日期排序，组内保留用户选择的排序。

模式/当前折叠组与列表状态保存至原session；旧JSON和英文月份折叠键兼容。切换分组清空旧模式折叠集，选择ID保留，滚动回顶部。200条完整行缓存按分组后ID读取；原数据库查询/预览/标签语义不变。文件类型分组不增加格式解码能力。

## 环境与命令

Windows MSVC，Rust 1.97.1 (`rustup toolchain list`、`rustc +1.97.1 --version` 已检查)，150% DPI；外窗1942×1256及1700×1100。隔离profile `target/mcp-grouping-validation`，样本 `target/mcp-grouping-fixture`，两个根目录、248个真实文件（含嵌套目录，1张PNG、其余文本内容的普通文件）。PDF扩展名样本用于文件分类，不是PDF渲染测试。未修改正常用户FileAtlas数据/设置/会话，未停止既有窗口。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-grouping -p fileatlas-plugin-file-manager -p fileatlas-plugin-session -p fileatlas-plugin-i18n -p fileatlas-plugin-mcp -j 2
cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-grouping.exe
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-grouping.exe --profile target/mcp-grouping-validation --plan docs/validation-ui/2026-10-14-grouping/plan.json --allow-mutations
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-grouping.exe --profile target/mcp-grouping-validation --plan docs/validation-ui/2026-10-14-grouping/native-plan.json --allow-mutations
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-grouping.exe --profile target/mcp-grouping-validation --plan docs/validation-ui/2026-10-14-grouping/restart-plan.json
```

普通 `cargo +1.97.1 build --locked -p fileatlas-shell -j2` 被运行中的默认EXE占用（os error5）；未结束用户进程。使用上面的独立输出路径完成Shell编译，仅有rustc多输出/-o忽略out-dir提示。首次引入本地crate时更新Cargo.lock；没有第三方依赖升级。

所有运行操作均由项目客户端启动 `fileatlas-shell-grouping.exe --mcp`，MCP与GPUI共用子进程。前两轮显式开启隔离profile的MCP修改权限用于扫描/UI动作；最后重启只读。无OS输入注入或通用computer-use。plan中目录需先准备；它们是本机隔离fixture路径，不会创建普通用户资料。

## 结果

- fmt、locked Shell check及完整独立Shell编译通过。26项自动测试通过：FileManager17、Grouping4、Session1、i18n4；MCP无单元测试，不声称其有自动单元覆盖。包含245行分组重排/首屏与末页ID一致、筛选、稳定排序、未知日期、折叠、根与父路径、旧month键转换、旧session新增字段兼容、Cordis服务与UI贡献清理。
- 最终构建同进程MCP：34步全模式/缓存/菜单，22步真实GPUI鼠标事件/选择/工具菜单，1步只读重启，共57步。报告为report.json、native-report.json、restart-report.json。计数/窗口/恢复值另经断言复查。
- 类型：PDF122、PNG1、TXT125。修改月份：2026-10为3、2025-04为122、2024-04为123；创建月份：真实2026-10为248，证明没有使用mtime代替。原按月有效日期与修改月份一致（这些fixture没有拍摄日期）。不分组恢复平铺。
- 根目录246/2；实际父目录83/82/81/2，完整路径避免同名目录混淆。
- 网格/列表、PDF折叠/展开、名称排序下分组、列表末页offset48/cache200通过。GPUI鼠标实际点击菜单“修改时间”切换成功；标题点击折叠2026-10成功；切换子文件夹后选中ID1保留；工具菜单实际鼠标点击文件类型分组成功。
- 最后只读重启恢复file_type、PDF折叠、list、选中ID1、248项。截图restart-restored-final.png。
- 运行检查修复两个现有交互问题：分组标题鼠标按下冒泡至框选导致折叠点击丢失；工具菜单的滚动包装器丢失绝对定位并挤乱主布局。标题阻止冒泡，工具菜单采用有界外层定位、内层滚动；narrow-tools.png确认1700宽窗口仍保持主布局。筛选条增加固定高度的横向滚动，窄窗口也可通过工具菜单访问全部分组。

## 边界

修改时间使用索引mtime，随现有扫描/Watcher更新；创建时间只在重载时读取。真实文件操作后时间变化、网络盘/权限/离线文件系统、大规模创建时间读取延迟、10万真实文件创建时间分组、跨系统时区/夏令时未逐项运行验收。缺失文件创建时间归未知日期有自动回归；Windows实测只覆盖可读取的NTFS样本。未新增今天/昨天/周等时间粒度，界面明确标注月份。横向触控板/滚轮本身未专门验收，窄窗工具入口已经鼠标实测。未重验其余F0待验操作。
