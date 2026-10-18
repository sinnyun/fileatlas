# 程序启动台：图标布局与交互验证（2026-10-17）

## 当前设计

用户的桌面快捷方式截图覆盖旧版宽卡片。使用内置 imagegen 生成并审阅的[图标布局 v3](../../reference-ui/concepts/program-launcher-2026-10-17-v3.png)和[提示/设计说明](../../reference-ui/concepts/program-launcher-2026-10-17-v3.md)作为本轮视觉依据。

实现为左侧分类、中部大图标网格、右侧信息编辑区。图标64逻辑像素，名称最多两行，默认透明底；悬停浅亮底，选中青色边框。图标单元不铺管理按钮或文件路径。单击选中，双击启动，右键复用GPUI Kit菜单。右侧固定预留空间，避免选中后重排导致双击目标移动；名称/分类/隐藏状态先编辑草稿，再统一保存。排序支持名称、安装时间、系统登记大小及升降序；未知值始终在末尾。

置顶可设为分类顶部或全部程序顶部，可同时置顶多个程序；星标以实心代表全局置顶、空心代表分类置顶。置顶级别优先于当前排序字段，同一级内继续按用户选择的排序排列。

截图：

- [图标网格和信息编辑区](icon-grid-inspector-v3.png)
- [真实程序右键菜单](right-click-menu-v3.png)
- [排序选项](sort-options-v3.png)
- [950×660逻辑像素面板，三列图标](narrow-layout-v3.png)
- [两个全部置顶和一个分类置顶](pinned-programs.png)

对照结果：上述分区、图标/名称层级、选中和悬停、右键菜单、排序、信息编辑均已显示。现有宿主标题栏沿用原样；分类采用现有字体字形，程序图标由Windows Shell提供。2个真实条目未取得图像，保留分类回退图标。原型中的侧栏隐藏开关当前以可点击的勾选按钮表达。路径和安装信息位于可滚动信息区，安装信息只读。

## 环境和命令

Windows / 150% DPI；同一子进程内运行MCP服务和GPUI窗口，窗口截图3286×1831像素。使用已安装Rust 1.97.1 MSVC，离线locked构建，增量编译，-j2。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked --offline -p photag-shell -j 2
cargo +1.97.1 build --locked --offline -p photag-shell -j 2
cargo +1.97.1 test --locked --offline -p photag-plugin-database -p photag-plugin-i18n --lib -j 2
node scripts/check-layering.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-ui-refresh.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-launcher-add-visible --plugins dev-test,launcher --scenario scripts/validate-program-launcher.cjs --allow-mutations
```

最终格式、check/build、四项治理检查通过。Database 10项测试通过；i18n初次未给全量目录格式测试提供新增的programs/categories参数，补齐测试参数后4项通过。针对手动添加文件选择器的后续修复，fmt/check/build和本记录中的扩展同进程场景再次通过。已有LNK4098链接警告和两个无Cargo.toml旧目录提示仍存在，不影响本轮通过项。

## 同进程运行结果

[验证摘要](launcher-validation.json)。完整输入步骤/状态保存在本机 `target/mcp-launcher-add-visible/report.json`，不把target数据当作发布产物。

- 扫描398个真实程序；额外通过公开服务添加一个隔离的无窗口EXE，所以截图和摘要共399项。396项获得原生图标，3项使用回退（含该样例）。45项有系统安装日期，100项有系统登记大小。
- 聚焦搜索输入框，通过MCP原生GPUI文本输入筛选到样例；单击选中并更新右侧信息，没有启动标记。
- 从右侧输入并保存中文名称“00 样例启动器”，选择系统工具分类、保存；隐藏后离开普通网格，恢复后重新出现。通过公开快照核对持久字段。
- 双击样例的真实图标控件，样例EXE运行并写入同目录标记。它不打开窗口、不修改用户程序；未启动任何真实安装应用。
- 右键打开样例和真实Photoshop程序菜单，真实安装程序菜单包含卸载入口；没有运行卸载。
- 通过真实排序下拉框与方向按钮验证名称、安装时间、大小六种方向；核对完整可见ID序列的单调性及未知值在末尾。
- 给两个程序设置“全部顶部”、另一个程序设置“分类顶部”；用真实列表ID顺序验证两层级顺序，再逐项切换名称/安装时间/大小排序及方向，置顶层级均保持优先。星标在实际窗口中显示为全局实心星和分类空心星；关闭重开面板后，三项状态仍在共享数据库快照中。
- 显式重新扫描后服务修订变化，扫描完成；焦点离开输入框后静止1秒，两次服务revision一致，面板render_count为247→247。
- 拖动原浮动面板边角缩至950×660，待布局稳定后截图；网格五列变三列，分类和编辑区独立滚动。关闭再打开，700ms检查已复用全部图标结果，没有重新提取。
- 发现文本调试事件被通用面板根键盘拦截后，修正为字符输入交给聚焦输入框；快捷键/导航事件继续由面板包含。搜索和中文输入的完整流程已通过同进程验证。
- 后续复现了添加程序时同步调用原生文件对话框造成的界面卡死风险，将它改为受跟踪的GPUI异步任务，并在等待时禁用重复点击。通过实际点击“添加程序”打开Windows选择器后，同进程MCP仍成功读取启动台状态，证明GPUI窗口线程仍在响应。
- 添加成功后的界面会返回“全部程序”、关闭隐藏和搜索筛选，并选中与所选路径匹配的新条目或已发现条目。此次同进程运行再次确认选择器弹出期间窗口可响应；实际挑选文件后的桌面路径差异和文件权限尚未做人工实测。

## 数据兼容和边界

所有修改只在上述隔离档案内；没有打开用户常规数据库。安装信息经DatabaseService存入同一数据库的新增app_launcher_metadata表，未改已有媒体/标签或启动台列。旧启动台表无metadata表时的兼容打开、重新发现保留手动修改、信息保存、重启恢复和删除清理由数据库回归覆盖。

安装时间来自有效的Windows InstallDate；大小来自正数EstimatedSize（KiB转字节）。无法对应注册记录或缺少值时显示“未知”，不使用文件修改时间或EXE文件大小冒充。图标失败也缓存，当前插件生命周期内相同路径复用缓存；软件升级后的同路径图标变化要到下一次插件启动才重新提取。

选择器打开及主窗口响应已验证；本次没有选择或取消真实文件。也没有启动真实安装应用、执行卸载确认或提权卸载器。发现范围仍是用户/系统开始菜单和用户/机器卸载注册表，不枚举所有磁盘可执行文件。安装日期和估算大小可能受软件维护/更新影响，不是本程序独立测量的安装事实。

旧首版证据保留为sidebar-launchpad.png；v2网格概念为历史中间稿，当前以v3和用户桌面图标参考为准。

## 图标提取与重启缓存复验（2026-10-18）

排查发现手动添加将Windows路径规范化为 `\\?\` 前缀；Shell 图标工厂对这类路径解析不稳定。提取前现在移除扩展前缀、统一路径分隔符（UNC路径转换为标准UNC），保留数据库里的规范路径。`IShellItemImageFactory` 无法渲染的可执行文件改用 `ExtractIconExW`/`DrawIconEx` 读取内嵌原生图标。成功的原生图标会以PNG持久缓存到共享数据库旁的 `program-icons/` 目录；缓存键包含归一化路径、文件大小和修改时间。进程内仍保留解码图像；失败项不写入磁盘缓存，以便后续重试。启动台MCP诊断报告磁盘命中、Shell提取、失败和手动程序状态。

冷启动验证命令：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-launcher-icon-cache-validation --plugins dev-test,launcher --allow-mutations --scenario scripts/validate-program-launcher.cjs
```

全套GPUI/MCP流程中，手动添加的隔离EXE图标状态为 `shell-extracted`；399项全部成功提取。关闭进程后，使用相同隔离档案再次启动：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-launcher-icon-cache-validation --plugins dev-test,launcher --allow-mutations --scenario scripts/validate-launcher-icon-cache.cjs
```

重启后的第一次状态读取已从数据库显示全部399项，`scanning=false`，图标读取显示后台等待中；随后在1.8秒内命中399个磁盘PNG，Shell重新提取为0，失败为0，手动程序图标来源为 `disk-cache`。运行状态和重启后截图位于隔离档案 `target/mcp-launcher-icon-cache-complete/launcher-icon-cache-restart.json` 和 `screenshots/icon-grid-after-restart.png`。程序发现刷新与图标读取互不依赖；程序列表先显示，缓存图标随后完成加载。
