## 2026-10-18 本地收藏取代置顶

已实现持久收藏、普通删除/清理/淘汰保护、编辑去重保留收藏、旧 pinned 数据兼容，以及独立确认删除收藏。12 项数据库测试、shell check、独立 exe 编译、三次独立进程 MCP 收藏/重启/取消/删除验证及原面板 91 步回归通过。标准 exe 正在运行被锁定，未替换用户窗口；使用 `target/debug/fileatlas-clipboard-favorites.exe` 完成验证。截图、准确命令、进程证据和限制见 [收藏验证](validation-ui/2026-10-18-clipboard-favorites/README.md)。

# 界面重构验证记录

## 2026-10-20：WorkCrate 项目/画板首个 UI 纵切

此节为首个两层纵切的历史记录，当前三层界面、A3 默认值和修正后的验证见
[WorkCrate 三层记录](validation-ui/2026-10-20-workcrate-three-level/README.md)。

新增生命周期面板和侧栏入口，完成项目创建、任意文件引用、三列响应式画板目录、JPG/JPEG/PNG 放置入口、缩放预览和 PDF 导出入口。画板数据/导出仍由分开的 WorkCrate 服务插件提供。WorkCrate 五个 crate 的 7 项测试、Shell locked offline check/build、fmt 和 UI theme/refresh、architecture、feature-ledger 检查通过。同进程 MCP 确认命令注册并打开面板，使用原生 UI 创建项目和画板；目录截图确认空白画板预览与 595×842 pt 元数据。首次查看发现编辑器页面被视口裁切，已按物理比例缩放并提高预览栅格密度，但修正后的编辑器截图尚未验证：该次 MCP pointer 输入没有激活画板卡片。图片对话框、放大清晰度与 PDF 输出的运行时验证也仍待完成。环境、命令、参考对照和截图位置见 [WorkCrate UI 验证记录](validation-ui/2026-10-20-workcrate-ui/README.md)。

## 2026-10-20：文件视图快捷复制与主目录拖放

FileManager 焦点下新增 Ctrl+C/ Ctrl+V：按栏保存选择，Ctrl+V 根据鼠标命中子文件夹框架或空白主网格，调用现有 FileOperations 复制服务。子文件夹分组下卡片拖动交给应用内 GPUI 路由，未命中分组框架时落到当前主目录；父目录悬停使用主题 selection 状态。活动概念图及布局清单见多栏工作区参考说明。

Rust 1.97.1 MSVC、150% DPI、隔离 dev-test profile，通过 `scripts/fileatlas-mcp-debug.cjs` 启动 shell 与 GPUI 同进程。生产扫描后 MCP `ctrl-c` 将缓冲计数设为 1；鼠标位于 beta 子文件夹时 `ctrl-v` 实际复制 `alpha.png`；主目录无直连文件组时，从 alpha 卡片拖到网格空白处实际移入主目录，再将 beta 文件 Ctrl+V 至主网格空白处成功。悬停与完成截图已打开检查，场景脚本及准确命令、截图位置、覆盖范围见[快捷复制与主目录拖放验证](validation-ui/2026-10-20-file-keyboard-parent-drop/README.md)。物理键盘输入和平台 OLE 拖出仍未覆盖。

## 2026-10-19：项目文件夹侧栏页签

更正早先浮动面板方案：独立 `fileatlas-plugin-project-tree` 现通过生命周期视图贡献注册为 Navigation“项目”页签，与文件夹/标签共用同一 rail 和侧栏内容列；点项目树目录会导航主窗活动栏并保持项目页打开。Rust 1.97.1 locked offline Shell 构建通过。同进程 MCP 已从隔离 profile 添加测试根目录，验证 `docs/notes`、`src/deep` 完整展开、点击 `deep` 导航及版本 1 JSON 保存。窗口截图已检查；窄窗口、移除后重启、不可读目录和大树性能尚未验收。详情及重现命令见[项目页验证记录](validation-ui/2026-10-19-project-folders/README.md)。

此前只清理过时用户文案的“文件管理窗口/媒体库”语义清理现由完整命名迁移取代：主内容 crate、Cordis 身份、Rust 类型、MCP 控制工具、状态键、UI contribution/slot 及开发文档已统一为 `fileatlas-plugin-file-manager` / `FileManager` / `file_manager`。媒体实体仍按数据库领域模型称为 media。Preview 与 Media Playback 的空目录清理及默认路径数据库 API/共享 DB 架构决定保持不变。

## 2026-10-20：项目树内部命名统一

原 Navigation“项目”页统一命名为“项目树”，只承担项目根目录与递归目录导航；新项目管理面板是独立功能。内部 crate、Cordis 插件/服务 ID、Navigation 状态 key、UI slot 和 view ID 均采用 `project-tree`。旧 `projects` 设置项、Sidebar 状态 key、JSON 存储位置和 `projects` 字段可读兼容；清单迁移后写入 `%LOCALAPPDATA%\\FileAtlas\\project-tree\\project-tree.json`，保留旧文件。

## 2026-10-20：主文件管理窗口命名统一

将主工作区内容视图统一命名为文件管理窗口：crate 从 `fileatlas-plugin-gallery` 改为
`fileatlas-plugin-file-manager`；Rust plugin/service/view 类型、Cordis 服务 ID、UI
contribution/slot、MCP 控制工具及 UI 状态键统一使用 `FileManager` / `file_manager`。
MCP 仅注册 `control_file_manager`，`get_ui_state.file_manager` 是主内容视图状态。
前期开发不做旧字段/旧客户端兼容，旧状态允许清理后重新创建；数据库媒体实体保持其
媒体语义。上游源码引用 `vendor/photag-source/src/ui/gallery.rs` 和历史验证材料里的旧
产物路径保持原样，因为它们指向实际存在的上游文件及既有验证产物。

验证：Windows、Rust `1.97.1-x86_64-pc-windows-msvc`；`cargo +1.97.1 check
--offline -j 2` 更新锁文件后，`cargo +1.97.1 check --locked --offline -j 2`、
`cargo +1.97.1 fmt --all -- --check`、UI theme/refresh、architecture、feature-ledger
和 layering 守卫均通过。用 `scripts/fileatlas-mcp-debug.cjs` 通过同进程 FileAtlas MCP
启动 `fileatlas-shell --mcp`，工具列表确认 `control_file_manager`，状态返回
`file_manager`；窗口 `valid/visible/active` 均为 true，截图已查看。截图及隔离 profile
在 `.local/validation/profiles/fileatlas-file-manager-rename-20261020/`，该目录为本地验证产物。

## 2026-10-20：项目更名为 FileAtlas

产品名统一为 **FileAtlas**，内部名前缀为 `fileatlas`。所有 workspace crate/package、源码导入、
Cargo 路径、shell/bin、调试客户端、`FILEATLAS_*` 环境变量、应用标题、默认数据目录、
测试夹具目录和项目文档已更名。未提供 PhoTag 名称的环境变量、旧默认目录、旧数据库或旧会话
迁移；当前前期开发可直接清理旧数据并以 FileAtlas 路径重新创建。`vendor/photag-source/`
及 `SOURCE_PINS.md` 中的 PhoTag 上游地址、提交和许可证归属保留为来源事实。

验证记录：Cargo 离线检查更新了全部 workspace package 名称及依赖锁文件；FileAtlas shell 重新构建通过。格式检查、UI theme/refresh、architecture、feature-ledger 和 layering 守卫通过。新的 `fileatlas-mcp-debug.cjs` 以隔离 profile 启动 `fileatlas-shell --mcp`，tools/list 与窗口状态读取通过；窗口标题显示 FileAtlas。证据和本地截图位于 `.local/validation/profiles/fileatlas-mcp-rename-20261020/`。

## 2026-10-19：主工具栏布局下拉与子文件夹切换图标

最新用户反馈：子文件夹开启态增加持久主题强调色底框，关闭恢复普通 ghost。默认蓝/Sage Green 两种强调色的同进程 MCP 点击、移开鼠标、悬停、关闭恢复及静止 revision 检查通过，最新效果见[开启态验证](validation-ui/2026-10-19-toolbar-dropdown/README.md)的 `filled-*` 截图。

后续用户反馈修正：布局入口已从缩放之后移至网格/列表旁，使用共享 ghost 图标按钮；单/双/四栏分别为方框、左右分栏和 2×2 图标，子文件夹按开关状态显示 ToggleLeft/ToggleRight。Windows 同进程 MCP 已点击验证三种布局、活动栏 2/4、各栏独立网格/列表和子文件夹配置、回切恢复及 1500 ms 静止 revision；原截图关于图标/位置的结论由 `corrected-*` 截图覆盖。当前截图、场景与未验状态见[修正记录](validation-ui/2026-10-19-toolbar-dropdown/README.md)。

顶部栏位布局切换合并为一个图标下拉；单/双/四栏与多栏活动栏切换均保留，移除工具栏数字焦点按钮。子文件夹操作改用 ToggleRight 图标，原开关行为不变。同进程 MCP 验证双栏选择、活动栏 2 路由和子文件夹切换；Windows 150% DPI 截图已对照检查。构建/守卫、截图及未验状态见[工具栏验证](validation-ui/2026-10-19-toolbar-dropdown/README.md)。

## 2026-10-18：剪贴板参考还原与分类修复 v2

用户指出上一版视觉未还原参考且分类不正确；结合EcoPaste、CopyQ、Ditto一手文档/截图重做。
按 `reference-ui/concepts/clipboard-workbench-layout-2026-10-18-v2.png` 实现深蓝三栏、Lucide图标、
卡片元信息/分组徽标、多行编辑和即时分组。修复浏览模式持久化清单遗漏剪贴板表的实际根因。
Windows150%DPI、Rust1.97.1：fmt、locked shell check/build、类型1项/Database11项测试以及四项治理检查通过。
91步同进程GPUI验证覆盖中文输入、分组创建/即时归属/筛选/重命名/删除、搜索及检查器跟随、自动类型、面板重开、
暂停/恢复与静止revision；另用三个独立子进程确认历史/分组恢复及删除不复活。
调试客户端已强制dev-test和显式profile数据库，修复早期core运行的隔离缺口；误入默认库的本轮fixture已逐条核对清理。
概念、实测截图、精确命令、隔离更正和OS行为未验范围见[v2记录](validation-ui/2026-10-18-clipboard-workbench-v2/README.md)。

## 2026-10-16：原始文件查看改为 QuickLook

用户要求最简外部查看：空格调用本机QuickLook，缺少时仅提示；独立图片/音频/视频
查看器、工具栏设置和旧MCP控制已删除。主界面继续沿用sidebar-pages-preview-v2及
inspector-unified-edit概念，没有新增视觉设计；旧独立查看器mockup由本次用户决定覆盖。
文件管理窗口/视频缩略图、右侧预览/信息/历史/标签/评分和文件操作保留；AI建议独立归FileManager。
fmt、locked Shell check/build、36相关测试、refresh guard通过；同进程MCP9组检查及
只读重启保护通过，静止日志228→228。首次定位约1.7s，缓存后的桌面派发约20ms，
不是QuickLook首帧指标。未卸载用户QuickLook验证缺少安装的真实系统状态；Store、
外部QuickLook渲染/焦点/codec继续未验。命令、截图、完整报告及范围见
[QuickLook证据](validation-ui/2026-10-16-quicklook/README.md)。

2026-10-13 起项目进入界面重新设计阶段：界面以实现
[最终设计稿 v5](reference-ui/final-v5/README.md) 为目标，不再以原版 egui 界面为对照标准。本文件记录每个界面改动的实际验证情况。

## 记录格式

每次界面改动追加一节，包含：

- 对应的 `docs/reference-ui/final-v5/` 设计稿文件名
- 实际执行的命令与运行环境（工具链、启动方式、数据库档位）
- 已验证的交互路径与观察结果
- 设计稿中存在、当前应用仍缺失的控件或状态
- 未验证项（Windows 原生对话框、拖放、Shell 集成、编解码等在完成实测前一律标注未验证）

编译通过或无头测试通过不作为界面可用的证据，只有实际启动窗口操作过才记为已验证。

## 记录

### 2026-10-13 阶段启动

- 尚未执行新的界面实现改动，因此本文件暂无验证条目。
- 迁移阶段的构建与测试结论保留在[验证报告](validation-report.md)，作为后端行为的历史资料。

### 2026-10-13 S0/S1/S2 中途状态：仅构建观测，界面未验证

- 对应设计稿：`final-v5/00-main-library.png`、`05-filters-menus.png`、
  `08-states-feedback.png`（实现进度见
  [改造方案第 7 节 S2 与第 10 节台账](interface-redesign-plan.md)）。
- 执行命令与环境：`cargo +1.97.1 check --locked -p fileatlas-plugin-file-manager -j 2`
  （Windows，MSVC 工具链 1.97.1，`--locked`）。
- 结果：失败，11 个错误，全部来自新写入但尚未接线的
  `crates/fileatlas-plugin-file-manager/src/view/inspector.rs`
  （缺 `FileManagerView.surfaces` 字段、`ensure_exif_loaded`、`remove_context_tag`、
  `restore_selection`/`format_bytes`/`format_timestamp` 可见性、
  `ButtonVariants` 未导入）。`view/filter_row.rs` 尚未在 `view.rs` 声明，
  因此其问题还没进入编译器视野。
- 未执行：`cargo fmt --check`、`check -p fileatlas-shell`、任何 crate 测试、
  以及应用启动。本阶段所有界面（顶栏三段式、卡片网格、月组头、空态、筛选
  chip 行与弹层、检查器右栏）在窗口中渲染成什么样、控件是否可用，**均未验证**。
- 仍缺失的设计稿控件/状态：筛选弹层的包含/排除切换无处理器；检查器未注册到
  `file_manager.inspector` 槽；状态栏与工具菜单未按 6.5-6 重排；空态 “Add folder”
  的 `navigation/add-folder` 无接收方；`filter_row.rs`/`inspector.rs` 的英文
  原文尚未登记 i18n 键（界面会显示英文而非稿面中文）。
- Windows 原生对话框、拖放、文件操作、编解码、Shell 集成：未验证。

### 2026-10-13 S2 接线及启动更新弹窗：插件 MCP 局部实机验证

- 设计稿：`00-main-library.png`、`05-filters-menus.png`；更新策略依本次用户
  指示覆盖旧的自动弹窗行为。已阅读两稿及 v5 concepts companion notes。
- Windows MSVC 1.97.1；`cargo +1.97.1 fmt --all -- --check`、
  `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` 通过。
- `cargo +1.97.1 test --locked -p fileatlas-plugin-file-manager -p fileatlas-plugin-navigation
  -p fileatlas-plugin-i18n -p fileatlas-ui-bridge -j 2`：16+4+4+3=27 项通过。
  后续 `test --locked -p fileatlas-plugin-settings -p fileatlas-plugin-mcp
  -p fileatlas-plugin-file-manager -j 2` 通过（Settings/MCP 当前无单元测试，不声称覆盖）。
- 普通 `build --locked -p fileatlas-shell -j 2` 被既有进程 PID 14400 占用
  `target/debug/fileatlas-shell.exe` 挡住（OS error 5）；未停止该进程。
  使用 `cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2
  -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-s2b.exe` 成功构建
  独立验证二进制（仅 output 名称与 out-dir 警告）。
- 实机客户端：Node stdio MCP，启动上述二进制 `--mcp`；
  `FILEATLAS_DEV_PLUGINS=dev-test`，`FILEATLAS_DEV_TEST_DIR=target/mcp-s2-validation`，
  `FILEATLAS_MCP_ALLOW_MUTATIONS=1`（仅隔离样例扫描与 UI 状态请求所需）。
  隔离 `settings.json` 明确设置 `auto_check_updates=true`、language 为 zh-CN。
  不触碰普通 FileAtlas 数据；验证后停止本次 MCP 子进程。
- 执行工具：initialize/tools/list、get_ui_state、dev_test_scan_fixture、
  dev_test_status、search_media、screenshot_window、control_ui。
  生产扫描处理 9 项；选择 media ID 8 后对应卡片 selection=1；检查器显示
  文件名/真实尺寸/大小。`inspector_close` 后状态 false，`inspector_open`
  后 true，截图显示右栏收起。服务请求证明状态与渲染，不证明点击命中区域。
- 自动更新回归：启动检查完成后 `update_checking=false`、
  `update_available=true`、`update_offer_open=false`；真实主窗口截图无更新
  遮罩。`settings_check_updates` 手动请求完成后 `update_offer_open=true`。
  未下载、安装或重启更新。
- 证据：`target/mcp-s2-validation/report.json` 与
  `01-startup-quiet.png`、`02-selected-inspector.png`、
  `03-inspector-closed.png`、`04-manual-update-offer.png`。
  初次 `00-empty.png` 受到旧更新弹窗遮挡，不能作为空库验收证据。
- 与 00 稿相比仍缺：S3 面包屑/侧栏分区；默认宽度下筛选行末项折到第二行；
  主稿媒体右键菜单顶部信息卡、完整重排未完成。检查器和卡片主体已呈现，
  不据此宣称全部状态或视觉验收完成。
- 未验证：六个筛选 popover 的真实打开/复选/包含排除/日期/评分滑杆与清除，
  空库添加目录、无结果清除、标签移除、EXIF 展开、多选面板、原生对话框、
  拖放、文件操作、编解码及 Shell 集成。按用户方向需扩展插件 MCP 对这些
  控件的测试能力后再验证；没有使用通用 UI 自动化替代。

### 2026-10-14 额度交接：MCP截图迭代与最新中间态

- 用户要求所有实机调试/测试经FileAtlas插件MCP，并明确暂停实现、整理给小模型。
  所有Luna子智能体停止；本轮MCP子进程停止，保留隔离数据与截图。
- 最后成功构建并截图：`cargo +1.97.1 rustc --locked -p fileatlas-shell
  --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-v5-iteration.exe`。
  用Node stdio MCP启动`--mcp`，profile=`target/mcp-s2-validation`，dev-test，
  UI状态/样例扫描所需mutations=1。主稿00、六个05弹层、02/03设置六页、01预览
  的真实截图已保存，具体文件和失败观察见完整交接。
- 已观察：全量AllAssets图标呈现、单行筛选、ListState不同高度网格；MCP六个
  filter popover均可实际打开。未由此宣称鼠标传播、框选/跨页/压力库已验证。
- 失败截图：设置六页正文空白且FileManager透出；日期弹层中文标签溢出与英文占位；
  Preview舞台半透、信息长表与旧底部浮动工具条。随后已写修复的部分尚未重拍。
  `v5-audio-first.png`仅主界面：MCP audio分支仍报不支持，不能声称音频播放已验。
- 最后工作树：`cargo +1.97.1 fmt --all`与`fmt --all -- --check`通过；
  `cargo +1.97.1 check --locked -p fileatlas-shell -j 2`失败19错，完整日志
  `target/development-handoff/check-current.log`。错误位于bulk_actions的MediaItem/
  section_title、filters的StyledExt、tagging_picker的ScrollableElement。
  最新源码未生成可运行EXE、未跑新相关测试。
- 新辅助脚本`node --check scripts/fileatlas-mcp-debug.cjs`通过；以最后成功EXE在
  `target/mcp-handoff-client-smoke`新隔离档案完成initialize/get_ui_state/
  screenshot_window，退出0；默认mutations关闭。它证明客户端可用，不证明
  最新Rust源码可运行。
- 交接入口：`docs/development-handoff-2026-10-14.md`，含逐稿缺失状态、i18n欠账、
  公开API实/未实现区别、恢复编译与MCP迭代步骤。旧成功测试与截图均按版本区分。

## 2026-10-14：Luna 并行样式实现与主线程 MCP 验收

本节覆盖上文旧构建/19错快照。设计依据是 docs/reference-ui/final-v5 的 00–08 图及 concepts 说明。用户明确要求多个 GPT-6 Luna 子代理：FileManager、Navigation、Settings 分别实现，主线程合并检查并通过同进程 FileAtlas MCP 截图迭代。普通数据库/设置未使用；原工作树保留，未提交。

环境：Windows MSVC，已检查 rustup toolchain list/rustc +1.97.1 --version；Rust 1.97.1，150% DPI。常规外窗1942×1256对应约1280×800内容，最小外窗1462×956对应约960×600内容。最新EXE为 target/debug/fileatlas-shell-accepted-20261014.exe。所有运行截图由 scripts/fileatlas-mcp-debug.cjs 启动该EXE --mcp取得，没有另外启动GUI或OS输入注入。

静态验证命令与结果：

- cargo +1.97.1 fmt --all -- --check：通过。
- cargo +1.97.1 check --locked -p fileatlas-shell -j 2：通过，target/accepted-check.log。
- cargo +1.97.1 build --locked -p fileatlas-shell -j 2：通过，target/accepted-build.log；复制到上述独立EXE名。
- cargo +1.97.1 test --locked -p fileatlas-plugin-file-manager -p fileatlas-plugin-navigation -p fileatlas-plugin-settings -p fileatlas-plugin-preview -p fileatlas-plugin-media-playback -p fileatlas-plugin-metadata-export -p fileatlas-plugin-mcp -p fileatlas-plugin-i18n -p fileatlas-plugin-indexing -p fileatlas-plugin-tagging -p fileatlas-plugin-dev-test -j 2：36通过/0失败/1 ignored，target/accepted-tests.log。
- cargo +1.97.1 test --locked -p fileatlas-plugin-indexing extracts_a_thumbnail_from_a_valid_generated_video -j 2 -- --ignored：另1通过，target/final-ffmpeg-test.log。
- node --check scripts/fileatlas-mcp-debug.cjs：通过。客户端现在支持 expect_error 和 until.at_least，实际预期错误保留在报告。

MCP运行模板：node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-accepted-20261014.exe --profile target/mcp-v5-validation --plan target/final-accepted-plan.json --allow-mutations。profile下LOCALAPPDATA、DB、settings、session和缓存全部隔离。改变UI/隔离数据时启用mutations，不涉及用户普通库。当前全部MCP子进程已退出。

| 参考稿 | 本轮修改及实际检查 | 截图（target/mcp-v5-validation/screenshots） |
|---|---|---|
| 00/01 FileManager | 中性背景、紧凑筛选、无缩略图占位、侧栏树/计数、文件信息折叠、选中/无结果、侧栏与Inspector折叠、最小窗 | 00-main-accepted.png、00-min-final.png、00-min-both-closed-final.png、08-no-results-final.png |
| 01 filters | type/tags/extensions/date/rating/sort六弹层；真实类型9照片/1视频/2音频、rating≥4匹配2项、日期请求、清空筛选 | filter-*-final.png |
| 02 Preview | 实色全窗舞台、底部工具条、命名filmstrip、More/Tags popover、真实信息/评分、旋转/重置/缩放/filmstrip隐藏 | preview-final.png、preview-info-final.png、preview-more-final.png、preview-no-strip-final.png |
| 03 Settings | 六页正文重排、导航图标/激活边、圆形accent、toolbar两列、方形checkbox、F1–F9完整宽度、AI/Tags/Updates组 | settings-*-final.png、settings-shortcuts-accepted.png |
| 04 Tags | create/edit/delete/merge/move标题/X、颜色22点/图标16项、编辑两列、删除单选项、真正tag picker双栏+预览+footer | tag-*-final.png、tag-assign-final.png、tag-assign-min-final.png |
| 05 Playback | 视频适配舞台、固定播放footer、受限音量/速率、Tags/More；生成8秒H264/AAC视频和PCM音频，暂停、seek2秒、speed1.25、volume0.4状态 | video-fit-accepted.png、video-paused-final.png、video-tags-final.png、audio-paused-final.png、audio-more-final.png |
| 06 Bulk | 操作/来源两列、AND筛选、完整匹配计数、独立滚动和固定footer；跨页CurrentSelection真正显示2项 | bulk-final.png、bulk-min-final.png、100k-cross-page-bulk.png |
| 07 Stats/Export/Updates | 真实统计柱形和标签图；JSON12项成功/目录拒绝访问失败；更新offer实色双栏、版本/说明/真实状态、固定footer | 07-statistics-final.png、07-export-success-final.png、07-export-failed-final.png、07-manual-update-accepted.png |
| 08 Feedback | 写回真实锁文件os32错误、精确路径重试、忽略清除；失败卡在右下320pt，标题/文件/错误/按钮完整 | 08-writeback-failure-accepted.png、08-writeback-retry-accepted.png |

报告：report-preparation.json、report-interactions-before-writeback.json（前91步；后续由分段计划完成）、report-writeback-export-success.json、report-feedback-rest.json、report-final-accepted.json、report-writeback-position-accepted.json，均在profile目录。对应计划保留在target/final-*.json与target/writeback-position-plan.json。更新手动检查访问真实release，不下载/安装；启动update_available=true时update_offer_open=false，手动检查后true。实际release notes为远端原文，保留英文。

压力测试通过：100000虚拟+12实际=100012记录，scroll index99999后window_offset=99812/loaded_count=200；select_many[15,16]跨首末页，bulk CurrentSelection匹配2项，未执行删除。清理100000虚拟后重新选择All files，12实际记录仍在。报告report-stress-accepted-before-refresh.json保存成功滚动/跨页/清理证据；report-final-accepted.json确认All files12。生产逻辑修复：GPUI未测量远处行没有累计高度，scroll_to_reveal_item不能用于远跳，改用逻辑ListOffset；已加载页变化后通知发布snapshot。

本轮曾失败的检查与结论：最初13项假设错误，同名validation.png为视频poster不单独索引，正确总数12；dev-test扫描/清理会选fixture folder，clear_filters保留scope，需All files才能断言12。最初invalid JPEG写回未报错与源实现的跳过语义相同，故改用独占文件锁产生真实失败。首次十万滚动失败已修复重测；失败卡scrollbar包装曾丢失外层absolute定位，已将定位与滚动容器分开并重拍。子代理更新offer括号错误已修复check；没有将这些初次失败算通过。

仍未验：原生保存/目录选择/覆盖与回收站确认的实际指针路径、鼠标框选/拖拽/键盘逐键、长列表滚动恢复、扫描失败卡交互、播放声音听感及全codec覆盖、更新下载/校验/重启安装。现有MCP请求验证状态/服务和渲染，不代表这些原生路径通过。无实际每记录进度或音频波形接口，导出用indeterminate spinner、音频保持真实舞台，不伪造设计示例数据。V1通过，V2主要界面范围通过，V3部分通过，V4启动静默/手动检查通过；不能声称全产品完成。

最后进程检查：既有PID33036仍运行 target/debug/fileatlas-shell-handoff.exe（旧版），未终止；本轮MCP子进程均已退出。查看当前新样式应使用 fileatlas-shell-accepted-20261014.exe，旧窗口不能代表本轮修改。

## 2026-10-14：两侧分页/固定预览 v2 实施与验收

用户当前参考 docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png 优先于final-v5的主窗口两侧布局。配套说明已新增同名.md；其他界面保持前轮设计。沿用用户授权的Luna代理协作实现Navigation与Inspector，主线程完成双侧分隔条、会话兼容及MCP实测。主媒体网格/筛选/后端语义保持，选中勾点移除，蓝边保留。

最新运行文件：target/debug/fileatlas-shell-sidebar-v2.exe。环境Windows/150%DPI/Rust1.97.1 MSVC；rustup toolchain list和rustc +1.97.1 --version确认，使用已安装工具链。隔离profile target/mcp-sidebar-v2（复制上一隔离测试库及其WAL，未接触正常FileAtlas数据/会话）；12媒体仍在，选择02-mixed-formats匹配3。

执行命令：cargo +1.97.1 fmt --all -- --check；cargo +1.97.1 check --locked -p fileatlas-shell -j 2；cargo +1.97.1 build --locked -p fileatlas-shell -j 2；cargo +1.97.1 test --locked -p fileatlas-plugin-session -p fileatlas-plugin-navigation -p fileatlas-plugin-file-manager -p fileatlas-plugin-mcp -p fileatlas-plugin-i18n -j 2。最终均通过，25 tests/0failed；包含旧session缺少inspector_width时保留原folder/media/Tags选择并round-trip的新兼容测试。日志target/sidebar-v2-check.log、sidebar-v2-build.log、sidebar-v2-tests.log。

MCP：node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-sidebar-v2.exe --profile target/mcp-sidebar-v2 --plan target/sidebar-v2-acceptance.json --allow-mutations，32步骤全部执行；target/sidebar-v2-pointer.json另16步骤；target/sidebar-v2-restoration.json默认只读5步骤。报告分别profile/report-acceptance.json、report-pointer.json、report-restoration.json；对应target/sidebar-v2-*.log。全部窗口由同MCP child process创建，截图为真实FileAtlas画面，没有另开GUI或OS指针注入。

| 用户要求 | 实现与实测 | 证据（profile/screenshots） |
|---|---|---|
| 左侧文件夹/标签/扩展独立页 | 48pt竖rail；MCP页切换、GPUI down/up实际点击Tags/Folders；每页独立内容；底部Settings沿rail | v2-tags-page.png、v2-tags-clicked.png、v2-folders-clicked.png、v2-extensions-page.png |
| 文件夹缩进/图标/数量/选中 | LucideFolder/FolderOpen、右对齐真实count，紧凑overlay菜单与标题+Add/More；搜索mixed、noresults、clear；All files12 | v2-folder-search.png、v2-folder-no-results.png、v2-restored-final.png |
| 父子层级连接线 | 懒展开folder1及03-nested；浅灰竖线+横短线，正确02-mixed-formats选择/3记录；两层真实目录level-one | v2-final.png、v2-restored-final.png |
| 右侧固定预览/下方分页 | 固定220pt画布、文件名/评分，点击预览路由原服务；信息保留基本/标签/EXIF/路径/文件菜单、独立滚动；History/Extensions切换预览仍固定 | v2-preview-info.png、v2-history-page.png、v2-inspector-extensions.png、v2-minimum-history.png |
| 两侧常驻可拖动分隔条 | 8pt命中/24pt把手/列调整光标；root move/up/out-up；MCP通过Window.defer派发GPUIPlatformInput down/drag/up，左264→304，右300→340；第二次启动恢复304/340 | v2-sidebar-dragged.png、v2-both-dragged.png、v2-restored-final.png |
| 缩略图只留蓝色边框 | 去掉右上checkbadge；边框显式RGBA，folder/rail selectedfill同步修正颜色打包；实际medium.webp选中7 | v2-readonly-final.png |
| 小窗口及折叠 | 960×600附近外窗1462×956，220/260面板，信息区保留footer且下方滚动；left/right折叠可恢复 | v2-minimum-window.png、v2-minimum-narrow.png、v2-sidebar-closed.png、v2-inspector-closed.png |

MCP扩展：control_ui navigation_tab/inspector_tab、sidebar_width/inspector_width；pointer_down/drag/up坐标为逻辑client x,y且受mutationsguard。CaptureWindowHandle单请求mailbox由Shell deferred派发，避免在render lease中重新update当前实体；这是项目FileAtlas MCP真实GPUI事件通路，不是OS注入或仅设置宽度模拟。只读重启时pointer_down被正确拒绝(FILEATLAS_MCP_ALLOW_MUTATIONS)，宽度保持304/340。

发现并修正：首次展开子目录panic“cannot read SidebarView while already being updated”，渲染改为直接传selected_directory；SelectPath原普通slash路径与DB canonical前缀不一致导致0匹配，canonicalize后正确3；菜单Button追加hover与Kit默认hover重复panic，采用标准ghosthover+不透明底色；source theme.accent_alpha返回RGB而非RGBA，修正本轮选中fill/border打包。早期失败保留于stderr和report-before-path-normalization.json，不算最终通过。

边界：History使用既有全局undo/redo摘要，当前无媒体历史时显示实情；没有新增单文件持久日志。扩展预留专用slot，无贡献显示暂无可用扩展，未引入插件商店或假按钮；当前无第三方贡献，不能声称加载/卸载外部扩展已验。信息区域实际鼠标滚轮、OS窗口外捕获、原生文件对话框及其他前轮未验项不在本轮证据内。GPUI内拖动/点击、尺寸与重启恢复已验；正常用户旧PID窗口保持不动。

## 用户确认的开发基线（2026-10-14）

用户认可当前界面并要求建立开发基线。基线标记dev-baseline-2026-10-14；关键截图/报告/计划已归档到docs/validation-ui/2026-10-14-baseline，不再仅依赖target本机输出。下一阶段功能操作优化与功能扩展的入口为functional-development-plan.md。上文未覆盖的行为继续保留，未因布局验收被标记为通过。

## 2026-10-14 基线后的通用弹出面板／列表／菜单优化

用户指定的四项优化已实现：设置及媒体预览共用UI Bridge浮动框架、主界面点击隔离、Cordis-owned Single/Multiple PanelCatalog与各定义默认尺寸、Grid/List按钮及会话偏好、保持卡片按钮菜单触发实体。设计对照final-v5/01/02/03/05与baseline sidebar-pages-preview-v2；新的窗口几何与列表按本轮用户指令。完整141步MCP参数／结果／截图、准确命令、样本及未覆盖边界见[本轮运行证据](validation-ui/2026-10-14-panels/README.md)。

Rust1.97.1 MSVC fmt/check/标准locked shell build通过；相关26项测试通过，最后UI Bridge5项清理回归通过。真实GPUI设置拖动100×50、缩放900×680→800×580、tab点击与外层隔离；菜单进入和预览ID6、单实例复用、多实例独立移动／关闭；真实图片/WAV/MP4解码及面板max/restore；List实际选择ID9与下一进程恢复；100009项列表末页offset99809/cache200；只读拒绝。正常用户数据未用于测试。

SQLite与既有后端动作语义不改；源Session serde-default与FileManager grid/context-action语义已追踪，映射追加于source-feature-audit.md；file_manager_list_layout字段为兼容的新增偏好。新组件使用说明见[floating-panels](floating-panels.md)。七个未逐项实测resize把手、窗口外捕获、不同DPI、列表修饰键/框选/OS拖出、全codec及听感、现有服务多媒体并行仍未覆盖，不将这些记为通过。

## 2026-10-14 添加文件夹点击退出修复

通过项目同进程MCP pointer_down/up(215,74)复现旧版Sidebar +点击退出：main线程在gpui-pre/app/async_context.rs:66报告RefCell already borrowed。同步rfd文件夹选择器的Windows消息循环重入GPUI App借用。已改为AsyncFileDialog的独立STA/native循环，选择完成再this.update→原add_folder_path路径，取消保持无操作；单实例guard和pending禁用避免重复打开。

Rust1.97.1 Windows MSVC locked shell check/build、fmt check及Navigation/MCP相关5项测试通过。15步隔离profile MCP实测：真实按钮点击、native_open=true、原生选择器PrintWindow截图、主窗口持续get_ui_state响应、cancel/reset、第二次重新打开/取消、library始终为空；修复日志无panic。准确命令/参数/截图见[运行记录](validation-ui/2026-10-14-folder-picker/README.md)。设计仍为sidebar-pages-preview-v2的相同标题按钮；Windows选择器委托系统，不改变页面布局。实际native accept及窗口创建期间卸载竞态未覆盖；不能将service测试等同native选择成功。

## 全文件卡片与格式角标（2026-10-14）

本轮沿用 sidebar-pages-preview-v2 及 final-v5 主界面结构，按用户指示将卡片右上More改为非交互格式角标，Grid/List与Inspector无缩略图时使用文件类型图标。Windows同进程FileAtlas MCP隔离验证45步：16文件全部可见、photo筛选5、file查询9；未知/无扩展名/隐藏文件、视频封面独立行、文档/归档图标、真实右键评分子菜单、增量扫描保留ID/评分、watcher未知格式创建/删除及目录排除。fmt、locked shell check、shell备用bin完整编译通过；27相关自动测试通过、1原有FFmpeg测试ignored。证据/命令/环境/启动诊断/未覆盖状态见[验证说明](validation-ui/2026-10-14-all-files/README.md)。正常用户profile没有修改；旧默认exe被运行中窗口占用，新可执行文件为`target/debug/fileatlas-shell-all-files.exe`。

最终补验：缓存命中增量扫描保留图片尺寸/micro，Indexing回归再次通过；重编译后32步FileAtlas MCP重测保留ID/评分及24×16尺寸，见同证据目录cache-retention-report.json。

## 独立分组插件（2026-10-14）

保持sidebar-pages-preview-v2/final-v5主布局，按用户新要求把按月规则/分组排序/虚拟分段迁入fileatlas-plugin-grouping，扩展类型、修改月份、创建月份、登记根目录及实际父目录分组。26自动测试、fmt、locked shell check、独立Shell完整编译通过；57步同进程FileAtlas MCP验证248文件的全部分组、200行缓存末页、实际菜单/标题鼠标点击、选择保留及只读重启恢复。另修复标题框选冒泡与工具菜单滚动容器定位。环境、命令、截图、报告及网络/大库/横向滚动未验边界见[本轮验证](validation-ui/2026-10-14-grouping/README.md)。默认EXE被旧窗口占用，运行构建是target/debug/fileatlas-shell-grouping.exe，未停止旧窗口或改普通用户profile。

## 分组标题背景增强（2026-10-14）

按用户要求给所有分组标题行加入不透明蓝灰背景、边框、圆角及更亮hover，保留40pt几何、计数与折叠。fmt/locked Shell check及独立Shell编译通过；14步同进程FileAtlas MCP验证网格、悬停、标题实际点击折叠、列表及月份标题。依据原v2主结构加本轮明确设计要求；[截图与命令](validation-ui/2026-10-14-group-header/README.md)。新EXE为target/debug/fileatlas-shell-grouping-header.exe，普通用户profile未修改。

## 子文件夹分组根目录优先（2026-10-14）

按用户要求先显示当前目录直属文件，后显示子目录组，保留蓝灰背景。28相关自动测试及19步同进程MCP验证全部/选中根/选中嵌套目录/筛选后无直属文件的顺序和数量。源代码、命令、截图、调试路径拼写修正及边界见[根目录优先验证](validation-ui/2026-10-14-root-first/README.md)。新构建target/debug/fileatlas-shell-root-first.exe。

## 不分组目录浏览与文件夹卡片（2026-10-14）

依用户新指示在原v2/final-v5主结构内增加文件夹卡片，None显示当前目录直属文件/子目录。36相关自动测试通过、1原FFmpeg项ignored；fmt/locked check/独立Shell编译通过。82步最终构建同进程MCP验证网格/列表单双击、上级、空目录、混排框选/图片预览、245文件末页及分组/搜索兼容，另两次只读MCP重启通过，共84步。[命令/截图/报告/边界](validation-ui/2026-10-14-folder-browser/README.md)。独立构建target/debug/fileatlas-shell-folder-browser.exe；未修改普通用户profile或停止原窗口。

## 运行时调试信息插件（2026-10-15）

新增 Cordis-owned `fileatlas-plugin-debug`。主界面底栏提供调试按钮，点击展开可收起的调试抽屉；记录时间、级别、分类、目标、动作、消息、耗时、线程及span关联信息。支持分类/级别/关键词筛选、分页和页长、选中详情、暂停查看/跟随最新及清空。插件订阅应用 tracing/log 与 panic hook，覆盖主要插件操作和数据库/索引耗时；MCP提供共享状态读取与受变更保护的控制接口。每 crate 的 `PORTS.md` 和源行为映射表已更新。

静态验证：Rust1.97.1 MSVC；`cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-debug-panel.exe` 通过；`cargo +1.97.1 test --locked -p fileatlas-plugin-debug -p fileatlas-ui-bridge -j 2` 通过10项；本轮新增 Rust 文件 `rustfmt --edition 2024 --check ...` 通过。当前全工作区 `cargo +1.97.1 fmt --all -- --check` 受到同一工作区中并行的 FileManager/Navigation 图标改动格式差异影响，未对这些文件执行自动格式化，以免覆盖无关改动。

Windows同进程 FileAtlas MCP隔离运行验证46步，并以最终窗口构建补验原生分类菜单的滚动边界。验证真实索引记录与110ms扫描耗时、真实文件改名失败及错误详情、过滤/分页/暂停/跟随/清空、最小窗口和抽屉打开关闭。准确命令、报告、日志、截图、隔离配置及限制见[调试插件运行证据](validation-ui/2026-10-14-debug-panel/README.md)。本轮设计沿用 `docs/reference-ui/final-v5/00-main-library.png` 的底部工作区结构，仅增加底栏入口和向上展开的调试抽屉。

边界：缓冲区最多保留最近10000条、进程重启后清空；不替代系统/原生组件日志或崩溃转储，后台线程的span父子上下文并非处处传播。为避免重复捕获自身的查询响应，RMCP传输层DEBUG/TRACE载荷不纳入记录；MCP应用动作和拒绝请求仍有记录。未加入磁盘日志导出或SQL正文捕获。

## 全界面 gpui-kit 图标统一（2026-10-15）

界面内的状态、操作、媒体类型、评分、导航和空状态图标统一改为 gpui-kit 图标目录；图标尺寸通过 UI Bridge 主题 `small_icon` / `icon` / `large_icon` / `empty_state_icon` 配置。标签图标保留旧数据库值兼容解析，旧 MDL2 字符与未知表情显示对应 gpui-kit 图标，不再直接绘制字符。替换文件管理窗口月份分组的手绘箭头和设置复选框的手绘勾号。界面结构与控制项不变，本次未新增或删减设计状态。

静态验证环境：Windows MSVC Rust 1.97.1。`cargo +1.97.1 fmt --all -- --check` 通过；`cargo +1.97.1 check --locked -p fileatlas-shell -j 2` 通过。扫描 crates Rust 源码确认 `Icon::new` 仅留在主题尺寸 helper，未发现 PathBuilder、SVG 图标绘制或 MDL2 主题字体调用。当前会话没有 FileAtlas MCP 工具入口，因此未启动同进程窗口或做运行时 UI 检查；界面呈现留待项目 FileAtlas MCP 可用时验证。

### 调试日志重复刷新问题修复（2026-10-15）

用户截图中的重复 `database/with_reader` 来自文件管理窗口虚拟列表重绘：列表每次构建行都调用 `media_paths_by_ids` 重查当前多选项的拖拽路径；右侧 Inspector 对不在当前200行数据窗口中的已选媒体也曾在渲染函数内同步查 `media_by_id`。已选媒体滚出窗口后选择仍保留，这是跨滚动/多选行为，右侧信息仍应继续显示。现改为：Inspector 在选中媒体缺少于已加载窗口时后台读取一次媒体与标签，完成后缓存并通知刷新；多选拖拽路径在全部路径不在已加载窗口时后台读取一次，按选择ID缓存。绘制过程仅读取快照/缓存，不再查数据库。选中状态仍决定右侧显示，当前虚拟列表窗口不再决定能否显示。

按同进程 FileAtlas MCP 回归：profile `target/mcp-inspector-async-final` 注入1000条隔离虚拟记录，选择排序末尾 ID1（首屏窗口实际显示virtual-000999起），右侧展示 `virtual-000000.jpg`、缩略图及基本信息。两次相隔2.5秒的 `get_debug_info` 均为 `latest_id=125,total=17`，重复读取数没有继续增加。最终报告/操作计划/真实窗口截图见[选择与日志回归证据](validation-ui/2026-10-14-debug-panel/README.md#follow-up-selected-inspector-and-render-triggered-database-reads)。相关 FileManager19、i18n4、Debug5测试通过，独立 Shell 编译和本轮Rust格式化通过。

### 全项目渲染读写审计与预览首帧优化（2026-10-15）

按用户反馈审计所有 feature crates 的数据库访问点并沿 Codebase Graph / `src/view` 调用链区分被动渲染与显式动作。审计发现同类重复绘制读取集中于 FileManager/Preview/Playback 的标签、选中媒体、预览信息和胶片带；导航标签关联及 FileManager 键盘、文件操作中的查询由明确动作触发。重复查询已改为快照复用，或以媒体 ID/路径为键的后台一次加载缓存；无效迟到结果按当前身份丢弃。调试span保留，修的是调用路径，不隐藏日志。项目级审计范围、通用规则和尚存的非身份变换首建文件生成边界见[UI渲染性能审计](ui-render-performance-audit-2026-10-15.md)。

FileManager 双击预览现在直接使用当前已加载 `MediaItem`，当前200行胶片带立即入视图，完整过滤结果的有序路径在后台解析；预览路径和胶片带不再逐条同步检查文件。预览尺寸、文件属性、EXIF/视频详情、索引详情和变换图像异步加载，方向信息按路径缓存；变换完成前先显示原图。文件管理窗口选中文件详情也将打开次数与文件属性合并为一次后台读取。Preview标签弹窗仅在打开时加载，Playback 标签面板也按活动媒体缓存。MCP新增 `control_file_manager action=preview`，调用同一 FileManager 预览处理函数以覆盖相同的数据/胶片带路径。

静态验证：Windows MSVC Rust 1.97.1；`cargo +1.97.1 fmt --all -- --check` 通过；`cargo +1.97.1 test --locked -p fileatlas-plugin-file-manager -p fileatlas-plugin-preview -p fileatlas-plugin-media-playback -j 2` 通过（FileManager19、Playback3、Preview0）；`cargo +1.97.1 check --locked -p fileatlas-plugin-mcp -p fileatlas-plugin-file-manager -j 2` 通过；独立 Shell 编译命令 `cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-render-audit.exe` 通过。

最终版本运行时通过同进程 FileAtlas MCP、隔离 profile `target/mcp-render-audit-final`、fixture目录 `test-data/fileatlas-fixtures` 扫描12项。`control_file_manager preview(media_id=5)`走 FileManager预览路径，真实计时 `preview_media` 约3.84 ms；打开信息浮层后，媒体详情仅发生一次后台索引查询。窗口截图见 `target/mcp-render-audit-final/screenshots/gallery-preview-info.png`。检查标签弹窗时，保持关闭产生0次 Tagging `for_media/tag_tree` 读取；打开后每项恰好读取一次（各自一对 start/completed span），之后两次相隔2.5秒的读取均为 `latest_id=152,total=77`，没有持续增加。标签弹窗截图见 `target/mcp-render-audit-final/screenshots/preview-tags-open.png`。该试验覆盖小型本地样例图和服务调用，不代表大型相机RAW、网络盘、媒体损坏/拔除时文件错误展示或指针双击命中测试；这些路径仍需相应实机验证。

### 图片持续刷新与实际首帧/全图延迟补验（2026-10-15）

上面的3.84 ms仅为打开处理函数耗时，不能代表图片显示速度。继续追查发现 AI 面板每150 ms无条件notify、静止缩放仍推进revision，以及调试抽屉观察a11y日志形成重绘反馈。现均改为状态变化才推进绘制，a11y记录仍保留。图片阶段按原生全图/渐进图/文件管理窗口缩略图就绪状态选择，取消500 ms过期规则；普通EXIF方向由原生加载器处理，移除重复全尺寸PNG；开发构建优化解码及像素转换热路径。

标准 `target/debug/fileatlas-shell.exe` 已重新完整构建。Windows11/Rust1.97.1同进程MCP19步、真实GPUI双击事件、Debug展开且原生无障碍启用：6000×4000 JPEG缩略图31.95 ms/全图400.30 ms，同图优化前全图9806.41 ms；3200×2400 TIFF为26.18/74.58 ms；同进程JPEG重开20.24 ms。150%缩放及淡出结束后，两次间隔5秒的render_count均37；两次间隔5秒的日志total/latest_id均167。fmt、locked shell check/build、相关30项测试通过。设计沿用v2/final-v5和现有浮动预览布局，未新增缺失设计控件。详见[完整证据及计时边界](validation-ui/2026-10-15-preview-feedback/README.md)：这是绘制提交时机，未作GPU物理呈现、全部格式或网络盘性能保证。

### 全量静止刷新实测与公共机制统一（2026-10-15）

真实同进程 MCP 首轮审计发现旧静态审计遗漏的 Settings 快捷键/AI、统计、批量匹配
绘制时数据库读取，扩展测试又复现了 EXIF 在缓存检查前读路径，以及静止鼠标事件
重复重启预览淡出的错误。已将这些根因修复，并按用户要求把所有 GPUI 定时轮询
统一到 UI Bridge `refresh::poll`，用 `ChangeGate` 进行变化检测；被动数据读取统一
使用 `ReadModel`，覆盖 FileManager/Preview/Playback/Settings，缓存成功及失败、合并
最新请求、丢弃过期结果。扫描和开发任务只在进度/结果变化时重绘，播放按位置变化
正常更新。架构规则写入 AGENTS 和公共约定，独立 UI 定时器检查加入现有 CI。

标准调试 EXE 完整重建后，Windows/Rust1.97.1、隔离 profile
`target/mcp-unified-refresh-final`、Debug 展开且原生无障碍响应开启，两轮 MCP
共136场景：134个静止状态最终日志与预览渲染计数无增长，2个真实音视频播放场景
位置正常前进且无重复数据库读取，无持续/阻塞项。覆盖1000行滚动、窗口外保留选择、
EXIF切换/清空/缺失源、真实评分写入后的统计/匹配更新、Watcher增删、导出失败/恢复、
面板生命周期及调试控制。4个初始样本发生真实输入或有限GPUI过渡，原始记录及随后
完全静止的复查均保留；最终两轮没有最小化中断。

本次6000×4000 JPEG的缩略图/完整图绘制提交为32.64/486.80 ms，小PNG为21.13/58.95 ms。
相关54项测试和全工作区89项测试通过（原有3项忽略），locked shell check/build、fmt、
差异空白检查和新增架构检查通过。保留 v2/final-v5 布局，检查了 MCP 真实预览、统计、
错误和调试叠加截图；本次没有新增缺失的设计控件。
[完整证据、可重放命令及未覆盖边界](validation-ui/2026-10-15-idle-audit/README.md)，
[统一调用约定](ui-refresh-contract.md)。这些结果不构成全部编解码器、原生破坏性文件
操作、AI下载推理或更新安装的功能验收。

### 调试抽屉跳过鼠标悬停重绘提示（2026-10-15）

依用户新要求，抽屉默认隐藏 GPUI DEBUG/TRACE 无障碍树重绘记录，分页前筛选，
保留告警、错误和实际操作。原始缓冲/MCP全量审计保持可用。沿用现有Debug抽屉布局，
无新增控件。7项相关测试、locked shell check/build、fmt及架构检查通过。
实际同进程 MCP 指针悬停产生10条重绘记录，筛选后不显示；正常数据库记录、搜索、
分页通过并检查真实截图。详见[证据与接口边界](validation-ui/2026-10-15-debug-hover/README.md)。

### 图片加载诊断细节（2026-10-15）

按用户截图中“耗时难以对应文件/阶段”的反馈，新增后台任务ID、源/缓存路径、格式/
字节数、源/输出尺寸、任务类型、缓存状态、解码器、排队及分阶段耗时、失败阶段和
原因。首帧/完整图日志补充路径、generation及计时范围。沿用现有抽屉的点击详情与
滚动，未新增设计控件，鼠标重绘隐藏规则保留。实际同进程MCP对24MP JPEG首次加载、
缓存命中、损坏JPEG、真实日志详情和任务结束后停止记录进行验证；具体结果、命令、
构建产物及限制见[诊断证据](validation-ui/2026-10-15-decode-details/README.md)。

### 实际原始目录冷缓存复测（2026-10-15）

按用户要求通过同进程MCP直接添加实际sanli0515目录，使用全新空缓存隔离档案
而非合成/复制图片。生产索引11文件/生成7图缩略图8.678秒，7图各首次打开和
重开共14次均实际绘制。截图目标PNG首帧26.7ms、完整663.3ms，重开21.7ms；
14997×4800 PNG完整8045.4ms，其预览/96任务均耗时约7.7秒解码，读取约1.8ms。
这明确存在首次PNG解码慢项，不据此宣称优化完成。5秒静止后preview记录152→152。
全11文件SHA256/大小/mtime前后一致；未改正常FileAtlas缓存、DB和原始媒体。
完整原始报告和截图仅在target/mcp-real-folder-empty-20261015保留，避免把真实
用户图片复制到版本化文档；[测量表、复现命令与边界](validation-ui/2026-10-15-real-folder/README.md)。

### 图片打开前三项优化完成（2026-10-15）

沿用final-v5/01-preview-playback和已接受的共享浮动框架，无布局/设计控件变更。
仅增加dev解码依赖优化、文件管理窗口缩略图复用及按格式/需求的渐进预览调度与请求取消。
最终标准fileatlas-shell.exe通过同进程MCP实际目录冷应用缓存14/14绘制：目标PNG
663.3→78.9ms、72MP PNG8045.4→422.1ms，重开17–19ms；截图核对图像、工具栏、
胶片栏及选中卡片均可见。另测24MP/坏图、前后导航、缩放旋转fit及快速开关；
坏图render12→12、raw日志701→701。workspace94通过/3忽略，最终Preview11通过，
locked构建/格式/刷新归属检查通过。精确复现、构建hash、初次测试脚本失败原因和
未验证的指针/格式/写权限范围见[优化证据](validation-ui/2026-10-15-preview-optimization/README.md)。

### 启动窗口尺寸自适应（2026-10-15）

沿用已接受的dev-baseline-2026-10-14布局，未改任何面板结构。启动窗口不再固定
1280×800，改为取主屏可用工作区（`visible_bounds`，已排除任务栏）的85%，下限
1280×800、上限不超过工作区本身（`fileatlas-shell/src/main.rs::startup_window_extent`，
常量在`fileatlas-ui-bridge/src/theme.rs`）。`FileAtlasShell.viewport_width`初值改用实际
启动宽度，不再写死1280。MCP `get_ui_state.window_status`新增只读`bounds`
（`GetWindowRect`物理像素），用于核对真实窗口尺寸。

本机主屏2560×1440、工作区2560×1392、窗口缩放150%（`GetDpiForWindow`=144；本条原记为100%有误，
2026-10-16 的 `window_status.geometry` 探针已给出确定值）。fileatlas-shell.exe（locked构建）
经同进程MCP实测`window_status.bounds`为3286×1831物理像素（含Windows不可见
缩放边框，即GPUI逻辑约2176×1203），窗口居中、标题栏/侧栏/筛选行/预览栏/
状态栏均按现有设计正常绘制，见`target/mcp-windowsize/screenshots/main.png`。
`cargo +1.97.1 fmt --all -- --check`、`check -p fileatlas-shell`、
`test -p fileatlas-shell --bin fileatlas-shell`（4项尺寸推导用例）与
`node scripts/check-ui-refresh.cjs`均通过。多显示器、非100%缩放下次启动的
记忆（窗口尺寸/位置未持久化）仍未验证。

### 侧栏 Explorer 风格分区（驱动器 / 快速访问 / 分组收藏夹，2026-10-16）

实现设计稿 `1792074142274.png`，其余布局沿用已接受的 dev-baseline-2026-10-14：左侧图标栏、
Folders 页与中部“文件夹收藏”库树结构不变，新区块按“驱动器/快速访问在上、分组收藏夹在下”插入。
持久化只加到 settings JSON 的 `sidebar_folders` 区块（`quick_access`、`favorite_groups`、
`show_network_drives`），未动 SQLite。驱动器枚举与用量（盘符+卷标+已用/总量 GB+进度条，网络盘
只显盘符+用量+地球图标、不显 UNC）走 `read_model::ReadModel` 被动读取，未新增计时器。

为让侧栏可被同进程 MCP 直接调试，新增语义通道 `NavigationUiRequest::Sidebar`（14 种请求，
在 `NavigationService::request_ui` 校验目标后调用与渲染控件完全相同的 handler）、只读快照
`get_ui_state.sidebar_folders`、`control_ui{action:"sidebar_request"}`，以及
`window_status.geometry`（`scale_factor` + 客户区原点，给出
`pointer = (screenshot_px - offset) / scale` 的确定换算，替代像素猜测）。

隔离 dev-test profile `target/mcp-sidebar-sections` 实测：32 步语义场景全通过（网络盘开关往返、
快速访问清空/新增/改名、分组创建→加/改名/删收藏文件夹→改分组名→删分组、编辑器取消、区块折叠、
盘内子目录展开、行选中、添加到库，以及 4 项非法目标被拒绝），结果同时落盘到该 profile 的
settings.json；指针场景逐一打开 6 类“…”菜单（驱动器标题网络盘开关含 ✓ 状态、系统行、用户行、
分组行、收藏文件夹行、驱动器行），并点击菜单项把 `show_network_drives` 真实改为 false 再改回 true。
`cargo +1.97.1 fmt --all -- --check`、`test -p fileatlas-plugin-navigation`（7 项）、
`build -p fileatlas-shell`、`node scripts/check-ui-refresh.cjs` 均通过。
截图、复现脚本、命令与仍未验证项（原生文件夹选择器、拖拽放置、编辑器键盘输入、watcher 对新区块
联动、非 150% 缩放）见[侧栏分区证据](validation-ui/2026-10-16-sidebar-sections/README.md)。


## 2026-10-16 浏览模式与其他 AI 侧栏改动审查

在 1177e4e 与其后的 Explorer 分区改动上继续修复，保留新布局。发现并处理快速访问键不一致、箭头展开状态重复、别名覆盖、分组状态未发布/保存、同步盘容量校验、被动目录 IO、保存旧版本覆盖/错误吞没、编辑器错误污染导航快照、虚假目录计数等问题。另补浏览模式未收藏目录重启恢复、会话自身监听循环、持续分块使快照交付饥饿三个遗漏。具体复现/修复映射、设计对照与真实 GPUI 点击截图见 [审查报告](validation-ui/2026-10-16-sidebar-review/README.md)。

Windows/150% DPI，Rust 1.97.1 MSVC，locked/-j2：fmt/check/build、UI refresh guard 通过；13 个相关 crate 65 测试通过。全新隔离 MCP profile 验证侧栏编辑/失败重试/静止日志、稳定后重启、四次箭头 pointer、浏览自身 profile 和真实文件事件。十万真实文件扩展回归（含枚举结束前部分结果断言）、标签重启和基本缺失重链回归通过。最终一轮首批发布 793ms（8224 项，仍在加载），完整 13332ms，峰值工作集 247332864 bytes；为本机 debug 热缓存样本，不是像素首帧或通用性能保证。持久 media/media_tags/tag_previews 各 1 行，未保存十万普通浏览记录。原生选择器/完整键盘焦点/断网和可移动盘等未验项详见报告；不以静态测试代替运行验收。


## 2026-10-16 进入目录及恢复窗口重复刷新

已移除 FileManager 焦点恢复无条件重扫和注册后根伪事件；后台等待监听注册后再枚举，背景快照保留现有列表/选择/滚动与 rich window。23 相关测试、locked check、独立输出构建、fmt 和 UI refresh guard 通过。同进程 MCP 实际三次最小化/恢复：目录 generation 1→1、查询 revision 2→2、窗口 offset 250→250；真实外部新增/删除仍生效。基本 8 项和十万文件扩展 11 项回归通过。常用 exe 因正在运行被占用，验证输出为 target/debug/fileatlas-shell-refresh.exe，未终止用户窗口。命令、截图、范围和剩余边界见 [重复刷新记录](validation-ui/2026-10-16-refresh/README.md)。


## 2026-10-16 用户指定顶部面包屑地址栏

后续真实目录延迟优化：按用户要求只读浏览D盘两个目录及图片子目录，参考Yazi固定
源码移除多段轮询等待，公共Wake交付就绪事件，仍按真实变化通知。最终常用debug构建
10次内容发布中位722.0→67.4ms；真实GPUI文件夹双击563.5–783.6→129.5–192.5ms。
42测试、fmt/check/build/refresh guard、11项地址与4项刷新及2项自身监听回归通过。
十万项首批159ms、全量17775ms，未宣称全量加速；计时不是像素首帧。布局沿用本节
用户地址栏设计。源文件元数据不变，浏览未新增持久媒体记录；详见
[真实D盘证据](validation-ui/2026-10-16-navigation-latency/README.md)。

完整根至当前目录面包屑、路径编辑、后退/前进/上一级已集成 Navigation 既有贡献。历史统一目录/Home/标签范围，窗口内最多200项，真实路径在动作时验证；当前层级不重载，非法路径保留状态。窗口启动只恢复一个对应范围事件，避免空路径清空标签/收藏。34项测试、locked check、独立输出构建、fmt、UI refresh guard通过；同进程MCP 11项语义/实际Enter-Escape-Ctrl+L、4项真实pointer、文件夹及标签只读重启验证通过，重复刷新回归通过。设计来源为用户圈定截图；截图、命令、接口与未覆盖网络/输入系统边界见 [地址栏证据](validation-ui/2026-10-16-address/README.md)。


## 2026-10-16 用户标注四项界面改动：区域区分/分组间隙/收藏区域/导轨配色

设计来源是用户 2026-10-16 的标注截图（浏览模式，深色石墨主题），四条标注即本次范围；整体布局沿用
已接受的 dev-baseline-2026-10-14，只改区域边界、分组间隙与侧栏收藏容器，未新增面板或改动结构。

- 各区域明显区分 + 可切换边框/阴影：`fileatlas-ui-bridge/src/theme.rs` 建立表面阶梯
  `SURFACE_RAIL`(0x171a1d) → `SURFACE_SIDEBAR`(0x1f2328) → `SURFACE_REGION`(0x1a1d21) → 内容列 →
  `SURFACE_CARD` 检视器，并把区域边界收敛为唯一原语 `separate_region(div, RegionEdge)`：边框模式画
  `REGION_EDGE`(0xffffff2e) 亮细线（`SEPARATOR` 比所有表面都暗，深色主题下不可见，故新增亮色细线），
  阴影模式改用偏移 ±10/6、模糊 18、alpha 0.72 的投影。Shell 的标题带/logo 区/侧栏列/检视器/状态栏/
  设置入口与 Navigation、FileManager 全部走该原语。配置项 `region_separation`（默认 border，兼容旧 JSON）
  进入 Appearance 页“区域区分方式”下拉；`SettingsService::persist` 成为唯一写入口，落盘成功后统一
  重发 accent 与 region-depth 绘制令牌，视图侧新增 `cx.refresh_windows()` 与语言切换同一全窗口重绘路径。
- 分组标题与卡片间隙：`GROUP_HEADER_GAP`(10) + 非首组 `GROUP_TOP_GAP`(18)，`file_manager_row_height` 与
  橡皮筋命中测试同步这两个偏移，避免选中框错位。
- 收藏区域：`render_favorite_groups()` 把自定义分组并入同一个 `sidebar_region()`，分组间
  `mt/pt 8px + border_t_1` 隔断，标题行按 `group_tint(index)` 上色（星形图标 + 透明底纹）。
- 最左导轨与可折叠侧栏使用不同表面令牌并各有右边界。

静态与自动回归：`cargo +1.97.1 fmt --all -- --check`、`check --workspace --locked --offline -j 2`、
`node scripts/check-ui-refresh.cjs`（UI timer ownership: passed）通过，49 项测试通过
（navigation 20、ui-bridge 12、file_manager 10、i18n 4、settings 3）。顺带修复既存问题：
`fileatlas-ui-bridge/Cargo.toml` 缺 `tokio` 的 `time` dev-dependency，使 `refresh.rs` 测试自 e0932a3 起无法编译。

同进程 FileAtlas MCP（隔离 dev-test profile `target/mcp-region-sep`，未触碰 `%LOCALAPPDATA%\FileAtlas`）实测：
6 条语义用例全通过——启动即应用 JSON 中的 shadow 令牌、按修改时间分组得 3 个月份带且标题与卡片有间隙、
侧栏只有一个内含 3 个自定义分组的收藏区域、真实指针点击把 border 改为 shadow 并写盘（先用
`window_status.geometry.screenshot_to_pointer` 校准 `pointer = (screenshot_px - (11,45)) / 1.5`）、
非法值被拒绝不写盘、切换生效 3 秒后调试日志 `latest_id` 282→282 无重复 IO/重绘。边框与阴影两模式截图
肉眼可分。MCP 新增只读 `get_ui_state.region_separation` 与受 mutation 开关保护的
`control_ui{action:"region_separation"}`。

仍未验证：MCP 直接写服务不经过 GPUI 视图，场景靠开合 Settings 面板触发重绘，未单独证明非视图写入也能即时
重绘；浅色主题、检视器/浮动预览在两种模式下的全部边界、列表布局与胶片条分组标题、非 150% DPI、最小化恢复
重绘、拖拽与原生选择器；分组颜色只按索引循环取 6 色，用户自定义分组配色尚未提供。截图、脚本、命令与边界见
[区域区分证据](validation-ui/2026-10-16-region-separation/README.md)。

## 2026-10-16 标签侧栏层级样式

先生成标签树胶囊概念图，再据图将每个标签实现为独立圆角胶囊与细边框，明确区分相邻标签；父子缩进由胶囊位置体现，连续树线穿过子项间隙，媒体数位于胶囊内徽标。通过 `cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-plugin-navigation -j 2` 和 `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`。FileAtlas MCP 同进程隔离 profile 中创建3个父标签及7个子标签，展开后截图检查通过。截图、profile和未覆盖项见 `validation-ui/2026-10-16-tag-sidebar/README.md`。本轮未覆盖媒体计数非零和实际标签筛选。


## 2026-10-16 标签侧栏反馈修订

根据用户指出的计数未右对齐、菜单按钮不可见、标签颜色未体现，继续修订侧栏：计数放入胶囊右侧固定宽度区域并右对齐；“…”按钮保持常显；胶囊底色及边框取自各标签自身颜色，选中态增强同一标签色。基于带色胶囊概念图完成修改，MCP 实测截图显示多色测试标签和真实可打开的操作菜单。格式、Navigation 检查与 Shell 构建通过。详见 `validation-ui/2026-10-16-tag-sidebar/README.md`。隔离测试数据的计数为0，非零计数与筛选未验。

## 2026-10-16 右侧信息面板程序与标签上下文

按用户提供的程序信息参考图生成深色概念稿 [`right-inspector-program-dark-2026-10-16.png`](reference-ui/concepts/right-inspector-program-dark-2026-10-16.png)，并生成标签信息/编辑的深色概念图作为面板层级参考。用户确认程序是浏览程序，没有媒体库概念后，程序状态改为“浏览概览、当前浏览位置、程序状态、使用提示”；当前结果数来自活动查询，文件夹数来自当前可见子目录，不显示媒体库容量/占用或扫描任务，也不展示具体标签名。标签模式增加内嵌名称输入与保存、颜色快速色板、缩略图显示切换，写入沿用 `NavigationService`。

验证命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、`node scripts/check-ui-refresh.cjs`，均通过；使用 `cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-inspector-contexts.exe` 构建 MCP 验证可执行文件。程序信息通过 `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-inspector-contexts.exe --profile target/mcp-right-inspector-program --plugins dev-test --plan target/mcp-right-inspector-program-plan.json` 在隔离配置同进程启动，截图 `target/mcp-right-inspector-program/screenshots/program-default.png` 显示真实浏览概览、当前位置空态、当前浏览状态；计数为隔离空 profile 的实际0值，没有展示具体标签名、媒体库容量、磁盘占用或扫描任务。

历史页使用隔离 dev-test profile 的1000条虚拟数据选中 `virtual-000000.jpg`，指令 `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-inspector-contexts.exe --profile target/mcp-right-inspector-history-v3 --plugins dev-test --allow-mutations --plan target/mcp-right-inspector-history-plan.json` 完成运行态截图 `target/mcp-right-inspector-history-v3/screenshots/history-selected.png`。确认当前文件工作副本主线、下方无历史节点空态以及主次创建/切换按钮可见；文件名、大小和时间降为底部小号辅助信息。`dev_test_status` 确认虚拟记录不对应磁盘文件。标签与文件夹上下文及标签色板操作随后在 `target/mcp-inspector-matrix-20261016` 的同进程 MCP 实测，见本节补充记录。

版本历史快照数据与创建/切换行为按用户要求待后续定；当前两项控件禁用并说明快照服务尚未接入。历史概念图里的多个分支/缩略图是目标样式示例，运行界面仅显示实际当前文件节点，不创建伪版本。

## 2026-10-16 右侧信息面板模拟数据实测

使用全新隔离 profile `target/mcp-inspector-matrix-20261016`，同进程启动
`target/debug/fileatlas-shell-inspector-contexts.exe`。模拟数据包含“风景 / 城市 / 海岸”3个根标签及4个子标签；将5张小型 JPG、PNG、GIF、WebP 图片复制到 profile 内的 `simulated-file_manager`，验证本地文件夹浏览和缩略图；另由 dev-test 注入1,000条虚拟记录用于文件/历史详情。未改写仓库原始样例图片或 `%LOCALAPPDATA%\FileAtlas`。标签选择和展开、颜色按钮使用项目 MCP 的 `control_ui` 同进程指针操作。

本轮可复核截图：`program-matrix.png` 显示真实快照中的7个标签、1个可见子文件夹和当前结果数；`folder-context.png` 显示样例目录的5个子目录、README及文件夹路径/已索引媒体数；`simulated-folder-content-clear.png` 显示隔离目录的两个真实图像缩略图与文件夹详情；`tag-select-probe.png` 显示选中标签后的父级/媒体数/子标签信息和名称编辑、保存、色板、显示控制；`tag-color.png` 显示色板点击后的红色标签色，`list_library` 返回颜色 `#E36B6B`；`tags-expanded.png` 显示父子行的层级、缩进、树线和胶囊布局；`file-virtual.png` 显示 PNG 类型、8.2 KB、257×192等文件信息及信息/历史/扩展页签；`history-virtual.png` 显示工作副本节点与空历史状态。

运行态核对：点击“风景”后 `file_manager.included_tags=[1]` 且面板切换为标签信息；点选红色色板后数据库快照颜色变为 `#E36B6B`。展开“城市”后“建筑 / 夜景”子标签可见。截图发现中文标签编辑面板的色板提示误用英文；已将其接入已有“选择色板”本地化词条，重建后截图确认中文生效。名称保存与“在缩略图上显示”切换控件已核对可见，实际切换/改名未在本轮验证。选中文件使用的是 `virtual-000001.png`，`get_media` 标记 `missing=true`，因此右上预览是缺失文件占位图，文件信息取自虚拟记录；隔离目录的真实 JPG/PNG 缩略图已由浏览画面验证，但浏览模式不索引媒体，不能把它们伪装成历史文件节点。标签媒体数为0，因为给虚拟记录写标签会因其虚构路径不存在而被正确拒绝；非零关联数与写回行为未验证。历史快照服务仍未接入，所以没有旧版本节点，创建/切换按钮保持禁用。这些边界保留到历史数据接入及非虚拟媒体标签流程验证时处理。

命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、`cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-inspector-contexts.exe`、`node scripts/check-ui-refresh.cjs`、`git diff --check`。格式、Shell 检查、UI refresh guard 与 diff whitespace 检查通过。完整截图及 JSON 状态证据见 `target/mcp-inspector-matrix-20261016/`。

## 2026-10-16 右侧信息面板概念图对照与模拟态补全

对照 `docs/reference-ui/concepts/right-inspector-contexts-2026-10-16.png`、程序深色稿和历史树稿，基于右侧面板实际截图调整层级：程序统计卡加清楚的细边框、扩大数值字号和卡片留白；各类详情行增加可见分隔；标签的子标签节点使用标签自身颜色绘制独立浅色卡片。文件历史树保留真实工作副本；新增**显式视觉验收开关** `FILEATLAS_DEMO_INSPECTOR=1`，仅在启动时有该环境变量时显示 v1/v0 主线与 v0.1 分支模拟节点，节点上标注 `DEMO` / `Visual sample · not saved`。普通启动仍展示真实空历史态，创建/切换按钮继续禁用，不会把演示数据写入数据库。

使用 profile `target/mcp-inspector-matrix-20261016`，通过 `scripts/fileatlas-mcp-debug.cjs` 同进程 MCP 复查程序、标签、文件夹、文件信息与历史。程序计数和路径来自当前隔离 profile；标签树来自7个实际插入测试库的父子标签；文件夹来自 fixture 与隔离样例目录；文件信息是 `virtual-000001.png` 的开发虚拟记录。历史 demo 的缩略图环境变量已尝试指向隔离 JPG，但当前 GPUI 截图仍显示类型图标占位，因此图像缩略图实际解码/绘制未验通过；不要把这些图标说成照片缩略图。真实旧版数据仍未接入，创建/切换操作和非零标签关联仍未验。

复核截图：

- `target/mcp-inspector-matrix-20261016/screenshots/program-polished.png`：统计卡、当前位置、程序状态和使用提示；无具体标签名、媒体库或磁盘容量。
- `target/mcp-inspector-matrix-20261016/screenshots/tag-polished.png`：当前选中标签信息与编辑控件。
- `target/mcp-inspector-matrix-20261016/screenshots/folder-polished.png`：真实 fixture 根目录的文件夹信息。
- `target/mcp-inspector-matrix-20261016/screenshots/file-polished.png`：开发虚拟文件信息与字段分隔。
- `target/mcp-inspector-matrix-20261016/screenshots/history-demo-polished.png`：显式演示开关下的主线、侧分支和缩略图占位状态。

检查：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、独立输出 Shell 编译、`node scripts/check-ui-refresh.cjs`、`git diff --check` 均通过。运行环境 Windows、150% DPI、隔离 dev-test profile；没有访问用户 FileAtlas 档案。

## 2026-10-16 右侧完整概念 v2：布局、有效内容及真实交互补齐

用户指出上一轮与概念不一致且信息不足后，重新用内置生图生成并审阅[完整状态概念v2](reference-ui/concepts/right-inspector-complete-2026-10-16-v2.md)，提示明确五状态的视觉层级、深色表面、标签配色、卡片间隙、字段对齐及历史分支。按图重排程序2×2统计、标签直接编辑/彩色子项、文件夹实际浏览信息、文件EXIF与路径、紧凑历史树及详情。原始同进程窗口截图、完整命令、脚本与结构化结果见[本轮证据](validation-ui/2026-10-16-inspector-complete/README.md)。

隔离profile为 `target/mcp-inspector-complete-20261016`：根目录18个真实合成JPG、5个子目录各3张图；3根5子共8标签，代表标签关联18文件、5子标签，代表文件4星并关联5标签，有有效缩略图与测试EXIF。程序面板无具体标签名或媒体库；目录显示当前结果及子目录数，无索引/扫描伪字段。测试EXIF仅验证字段解析显示，不代表相机互操作。未触碰用户FileAtlas档案。

修正了历史缩略图槽挤掉文本、历史大预览挤掉详情、空白取消选择仍显示文件夹、同目录重选无上下文事件、标签保存被旧快照回填、分支筛选选中信息不匹配、EXIF别名重复等问题。上轮缩略图“解码未通过”的原因实际是引用路径不存在；本轮有效JPG已正常绘制，不再保留该结论。新代码沿用Navigation公共事件、FileManager选择和ReadModel刷新契约；目录更新保留明确取消选择的程序状态。

95步同进程MCP全部通过：真实指针改名/恢复、色板与显示开关、子项菜单、四上下文、背景取消、同目录重选不重扫、真实文件增删18→19→18、模拟版本选择/切换/创建3→4节点/分支筛选，以及360/300宽度和短窗口。静止日志969→969。最终EXIF展示清理后另用无演示变量启动，20步普通运行验证通过，演示关闭且无模拟节点，静止日志426→426。截图均为MCP原始截图，无拼接。

普通运行保留真实工作副本与无历史状态；实际版本存储/恢复按用户此前决定暂缓。模拟历史必须同时启用显式环境变量、dev-test和隔离profile，创建/切换仅在内存生效并标明模拟。扩展页验证真实无贡献空态，没有添加假扩展。

`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、FileManager及i18n共25项测试、`node scripts/check-ui-refresh.cjs`、`git diff --check`通过。默认 `target/debug/fileatlas-shell.exe` 已更新，独立检查输出 `target/debug/fileatlas-shell-inspector-complete-final.exe`。仍未覆盖非150%DPI、全部滚轮操作、原生Explorer/复制路径、真实非空扩展贡献及全部相机/格式；不将此轮算作整个F0完成。

## 2026-10-16 用户追加：页签预览一致与标签集中确认

先使用内置image_gen生成并审阅[概念图](reference-ui/concepts/right-inspector-unified-edit-2026-10-16.md)，再按用户顺序展开名称、颜色、图标、显示开关和底部确认按钮。颜色/图标/开关改为草稿，点击一次确认才调用Navigation既有写命令；移除独立名称保存和折叠更多设置。历史恢复与信息/扩展完全相同的预览、标题、文件名和评分，所有页签只替换正文。覆盖上一轮“紧凑历史身份栏”的设计选择。

隔离同进程MCP62步通过：真实pointer一次确认保存四字段、确认前完整标签对象未变化、统一恢复、子标签显示传播、三文件页签及300宽度/1080窗口；静止日志798→798。三页原始截图的顶部右栏RGB逐像素差异为空。无演示最终构建另20步通过、静止448→448，普通历史不含模拟节点。FileManager/i18n25测试、fmt、locked Shell check/独立输出编译、refresh guard、diff whitespace通过。默认exe被正常运行实例锁定，os error5；未终止该实例，使用独立输出`target/debug/fileatlas-shell-inspector-unified.exe`完成实际验证。截图、提示、命令、首次指针请求排队修正与边界见[本轮完整证据](validation-ui/2026-10-16-inspector-unified-edit/README.md)。未覆盖写入失败/其他DPI/真实版本后端，未宣称全F0完成。

## 2026-10-16 空格查看延迟改为 QuickLook 管道派发并删除自动启动

用户反馈"按下到 QuickLook 界面出现约 1-2 秒"，并要求依据项目源码与官方文档定位、不许猜测。
根因是每次调用 `QuickLook.exe <路径>` 走的是 QuickLook 第二实例启动器，必须先冷启 .NET/WPF
再经管道转发（实测 1.06-1.12 s）；第一实例本就监听 `\\.\pipe\QuickLook.App.Pipe.<SID>`。
改为直接写一行 `Toggle|<路径>|\r\n` 后，同进程 MCP 实测派发 outcome 全部
`pipe_toggle_sent`，dispatch 15-33 ms，按键到"标题匹配本次文件"的窗口出现 37-254 ms；
QuickLook 刚启动或首次使用某类插件时为 600-755 ms，属 QuickLook 自身加载插件。

用户随后要求删除"QuickLook 未运行时自动启动"：FileAtlas 端不再有启动逻辑，`launch()`、
`launch.ps1`（桌面 EXE 发现/启动、Store AppsFolder 激活）、`Launch` 错误与
`desktop_request_sent`/`store_request_sent` 全部移除，该 crate 内已无 `Command::new`/`spawn`。
无监听实例、被拒绝且非提权或写入失败时返回新的 `NotRunning`，走 FileManager 既有 GPUI Alert 提示
用户自行启动 QuickLook；提权判定只在管道返回访问拒绝时执行。QuickLook 关闭状态下实测真实
空格 6 ms 即弹窗，`windows=[]` 且进程表无 `QuickLook.exe`。同轮修掉 `keys.json` 中
`"{count} selected"` 的重复映射（调试面板新增项覆盖了文件管理窗口既有项，导致 i18n 目录测试失败）。
命令、逐条数字、`window_ms` 口径修正、本轮调试误用空连接导致用户 QuickLook 失去监听（已由
用户重启恢复）的记录与截图，见[本轮完整证据](validation-ui/2026-10-16-quicklook/README.md)。

检查：`cargo +1.97.1 fmt --all -- --check`、`clippy --locked -p fileatlas-plugin-quicklook
--all-targets -j 2`、`build --locked -p fileatlas-shell -j 2`、
`test --locked -p fileatlas-plugin-quicklook -p fileatlas-plugin-i18n -p fileatlas-plugin-file-manager --lib -j 2`
（23+4+5 全通过）、`node scripts/check-ui-refresh.cjs`、`git diff --check`。用户启动 QuickLook
后复跑：完整 10 组用例（含"用同一入口关闭本轮窗口"的收尾）通过，两次运行后只读观测均为
`instance_listening: true`、`windows: []`。外部窗口首帧/解码/焦点/DPI、Store 版运行时派发、
QuickLook 忙碌时的重试表现与真实未安装机器仍未验证。

## 2026-10-17 真实文件 Lore 回退后历史消失修复

直接核对 F:\AI\geren\newworkspace 的 InspectorPanel.vue、版本前端 service、VersionInfo graph_view、Lore activate 与 projection 原始源码。已按完整持久化目录保留历史版本，行与操作绑定稳定 versionId；目录重扫的空/部分/失败结果不再清空文件选择，完整结果仍验证真实存在性。现有 Inspector 概念结构不变。

环境 Windows / 150% DPI / Rust 1.97.1。cargo +1.97.1 fmt --all -- --check、cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-versions -p fileatlas-plugin-version-info -p fileatlas-plugin-file-manager（27 项）、cargo +1.97.1 build --locked -j 2 -p fileatlas-shell、Node 语法与三项治理守卫均通过。FileManager 旧测试夹具的私有 queries 引用已改公开 DatabaseService；插件生命周期夹具补实际 Versions/VersionInfo/Thumbnail 依赖。架构基线仅新增测试用 FileManager→VersionInfo 边，83 edges / 711 debt；两处旧 orphan 目录警告仍在。

FILEATLAS_LORE_TEST_FILE=D:\桌面文件夹\测试\临时文件\00.png；执行 node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-lore-real-file-complete-20261017 --plugins dev-test --allow-mutations --scenario scripts/validate-lore-history-click.cjs。同一 MCP/GPUI 子进程对原路径创建两个版本，实点 History、旧版行、Switch、新版行、Switch；文件字节、列表数量/顺序、当前标记、选中项和页面可见性均通过；settled snapshot revision 9→9。文件已恢复起始字节/时间戳，SHA-256 相同。

证据与原始源码逐文件对照：docs/validation-ui/2026-10-17-lore-version-history/README.md。参考项目完整 dirty/generation 投影故障恢复协议仍未全部移植，本轮不声称其全覆盖。

2026-10-17 后续：从历史01编辑创建03的 shared-store BranchAdvanced 已按官方 API 流程修复。Windows / 150% DPI / Rust1.97.1；四项 Versions/VersionInfo 测试、locked Shell 构建和治理检查通过。原路径00.png的同进程MCP再次验证01/02实点往返、停01编辑创建03、三份内容恢复、历史页可见及quiet代次22→22。当前用户修改已恢复，SHA-256前后一致。官方API对应、精确命令与证据见 validation-ui/2026-10-17-lore-version-history/official-api-rewind-save.md 及同目录README。没有重新设计界面。

## 2026-10-17 文件管理窗口未保存版本提示

在已确认的sidebar-pages-preview v2/Inspector complete v2布局上增加小范围提示。
Grid左上琥珀点，List文件名前琥珀点（避免小缩略图格式标重叠）；悬停显示所选基准，
真实点击打开History手动创建。历史树的“当前文件”改称“版本基准”，
当前内容匹配其他已保存版本时说明两者，不提示重复保存。
VersionsService比较固定Lore不可变revision内容，FileManager后台ReadModel缓存、
Watcher/HistoryChanged失效，新增只读MCP状态。实际00.png经同进程MCP跑完整
01/02/回退01/编辑03、网格/列表、悬停/点击/静置流程，文件最终恢复原始字节。
31相关自动化测试、fmt、shell check和治理脚本通过。运行中用户EXE被占用导致
Cargo最后复制失败，已使用成功链接的产物完成实测并另存
`target/debug/fileatlas-shell-version-status.exe`，未重启用户窗口。
命令、截图、JSON、未覆盖范围见 validation-ui/2026-10-17-version-status/README.md。

## 2026-10-17 Everything 搜索及安装提示

按用户要求在已接受的 sidebar-pages-preview v2 主窗上局部增加顶栏搜索范围按钮、
Everything 安装/兼容性提示和重新检查入口；设置软件页保留既有浮窗结构，
只读检查 QuickLook/Everything 并提供官方与 GitHub 链接。标签、时间范围、评分
仍由现有 SQLite 语义过滤；真实 Everything 候选可包含从未浏览或扫描的文件。

Windows / 150% DPI / Rust 1.97.1 Windows MSVC / Everything 1.5.0.1423。
`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、
`cargo +1.97.1 fmt --all -- --check`、
`cargo +1.97.1 test --locked -p fileatlas-plugin-everything-search -p fileatlas-plugin-database -p fileatlas-plugin-file-manager -p fileatlas-plugin-mcp -p fileatlas-plugin-i18n -j 2`
通过（40 项测试）。check-ui-refresh、check-architecture、check-feature-ledger、
check-layering 和 diff --check 通过；保留 SDK 静态库 LNK4098 链接警告。

同进程运行命令：
`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-everything-final --plugins dev-test --allow-mutations --scenario scripts/validate-everything-search.cjs`。
8 组场景通过：真实 SDK provider、未浏览中文特殊文件名、目录限制、分页、类型、
无结果、顶栏搜索、范围弹层与时间开关、空闲不重复刷新及软件检查页。
先前软件页复用链接按钮 ID 引发 GPUI a11y 崩溃，已按软件分配独立 ID，
最终重建后完整场景通过。

状态场景使用同一脚本入口，profile `target/mcp-everything-status`、
scenario `scripts/validate-everything-status.cjs`；环境
`FILEATLAS_EVERYTHING_EXE=F:\AI\geren\pho_cordis\target\mcp-everything-status\portable\Everything.exe`、
`FILEATLAS_EVERYTHING_GOOD_FIXTURE=C:\Program Files\Everything\Everything.exe`。
隔离测试文件按不存在→官方 1.4 EXE→本机 1.5 EXE 切换，只读取版本资源、不执行安装。
三组状态、重新检查及静置检查通过。截图已逐一对照目标局部控件及既有布局，
未出现裁切；安装 URL 已核对，未运行下载或安装。最终长时间冷索引自动恢复、
网络盘/离线盘、超长路径、大规模上限仍未实测。

截图、精简报告和更正前期服务/版本误诊见
[Everything 验证记录](validation-ui/2026-10-17-everything-search/README.md)。

## 2026-10-18 快捷键参考图纠偏（本轮）

用户复核后按 `reference-ui/concepts/shortcuts-settings-2026-10-18.png` 重做横向插件行，
落实范围徽标、对齐的按键/操作列、蓝色录入框、行内真实冲突、完整宽度的 F1–F9 选择器，
调整默认浮窗至 1120×840 逻辑像素。实际同进程截图已打开逐项核对；默认正常/冲突页
均可看到 F9，小窗口通过新 MCP 滚轮输入到达 F9 并实际点开菜单，页脚保持可见。

fmt、locked shell check、四项治理及差异检查通过。shell 链接成功，主 EXE 因旧窗口占用
导致 Cargo 最终复制失败（os error 5），使用新链接产物另存的
`target/debug/fileatlas-shell-shortcut-layout.exe` 做 MCP 验证，不将本次 Cargo build 记为完整通过。
同进程场景的录入/冲突拒绝/无修饰键拒绝/合法保存/禁用/恢复/取消/切页关闭清理、
静置和滚轮边界拒绝均通过。真实 OS 全局派发、超长第三方名称及多插件规模状态未覆盖。
完整命令、前后截图、逐项对照和剩余适配差异见
[复核记录](validation-ui/2026-10-18-shortcuts-reference/README.md)。

新增 [活动参考入口](reference-ui/README.md)、[按图实现流程](ui-reference-workflow.md)
及该画面的配套清单，并在 AGENTS.md 中设为界面任务必读；明确编译/JSON 断言不能代替
视觉验收，“不逐像素复制”不能用于更改列布局或省略设计状态。以下旧结论明确撤回。

## 2026-10-18 快捷键设置页布局与样式（上轮记录，视觉结论已撤回）

更正（同日，用户复核后）：本节上轮“完成”结论撤回。上轮实际截图的操作按钮被代码
固定放在下一行，并非按窗口宽度自然换行；默认浮窗中 F1–F9 大部分不可见，选择器固定
窄宽，录入/冲突状态也没有落实参考图。编译和保存场景通过只能证明相应功能，不能证明
视觉验收通过。后续按图修正及逐项证据见 [快捷键参考图复核](validation-ui/2026-10-18-shortcuts-reference/README.md)。

以下保留上轮实现和检查记录，不能当作当前布局规范。
概念参考：`reference-ui/concepts/shortcuts-settings-2026-10-18.png`。沿用已接受的
设置浮窗与石墨灰/蓝色强调色，将插件命令整理为细分隔行：命令名称与插件/范围在左，
按键胶囊在右，修改/恢复/禁用操作置于下一行；使用中文标签，并为启动台注册项提供中文名。
插件快捷键与 FileAtlas 原有快捷键分区之间增加清晰的层级标题。录入态提示按键及 Esc 取消，
禁用态以明确文字显示。保留原设置导航、媒体键、排除修饰键、评分和 F1–F9 标签控制。

验证环境：Windows，150% DPI，Rust 1.97.1 MSVC。`cargo +1.97.1 fmt --all -- --check`、
相关 crate locked check、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`，
`check-ui-refresh.cjs`、`check-architecture.cjs`、`check-feature-ledger.cjs`、
`check-layering.cjs` 和 `git diff --check` 通过。保留既有 LNK4098 链接警告；
architecture guard 报告的 media-playback/preview orphan 目录为既存债务。

同进程 MCP 使用隔离 profile `target/mcp-shortcut-final`、`core,dev-test` 与
`scripts/validate-shortcuts.cjs` 重开设置页并检查已保存按键。截图：
`target/mcp-shortcut-final/screenshots/settings-shortcuts-updated.png`。该轮截图仅证明插件条目
和按键可显示；按钮另起一行及默认高度不足与参考图不符，不能作为视觉通过证据。
MCP 合成录入态和设置持久化已有前节验证；
真实系统级按键分发仍未由 MCP 覆盖。

### 搜索位置切换追加（2026-10-17）

按用户方向对已接受顶栏作局部更新，不新建整体视觉概念。
当前文件夹（含全部子文件夹）/整台电脑按钮并排位于搜索框、字段范围和
Everything 状态之间。当前目录模式用 SDK 根路径与 SQL 递归目录保持一致；
全机模式移除登记目录限制，使用 Everything 已有索引配置。清空搜索恢复原浏览。

环境：Windows / 150% DPI / Everything 1.5.0.1423 / Rust 1.97.1 MSVC。
`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、
`cargo +1.97.1 fmt --all -- --check`、四项治理脚本与 diff --check 通过。
本次不新增/重跑单元测试。运行命令：
`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-everything-location --plugins dev-test --allow-mutations --scenario scripts/validate-everything-location.cjs`。
两个目录三个唯一命名真实文件实测：本地含子目录 2 项、全机含未登记相邻目录 3 项、
GPUI 指针实际点击切回 2 项；静置 revision 不变；清空恢复原目录；设置保存与同进程
MCP 重启读取均通过。已查看完整窗口截图，顶部新按钮无裁切，主窗结构保持。
边界：全机只包含 Everything 已索引位置；当前未选择目录时，本地沿用收藏集合登记目录。
截图与摘要：validation-ui/2026-10-17-everything-search/location-report.json 及同目录 README。

## 2026-10-18 插件快捷键设置与系统注册

设置页 Shortcuts 页保留打开媒体键、排除修饰键、评分说明和 F1–F9 标签键，
并从 UI Bridge 自动列出各 Cordis 插件登记的命令快捷键。每行显示插件、范围、
当前组合键，支持录入、恢复默认和禁用。组合键覆盖按稳定贡献 ID 写入设置；
同范围冲突会拒绝，系统快捷键需包含修饰键。Shell 将应用级绑定触发为
`UiCommandInvoked`；`fileatlas-plugin-shortcuts` 按设置覆盖动态注销/重注册全部
系统级贡献，并在捕获键盘时暂时停用全局注册。启动器 Ctrl+F 和剪贴板
Ctrl+Shift+V 用于验证多插件自动注册。

运行环境：Windows、150% DPI、Rust 1.97.1 MSVC。验证 profile 为
`target/mcp-shortcut-final`，启用 `core,dev-test`；用户 `%LOCALAPPDATA%/FileAtlas`
设置和数据库未用作 fixture。
命令：`cargo +1.97.1 fmt --all -- --check`、
`cargo +1.97.1 check --locked -p fileatlas-plugin-settings -p fileatlas-plugin-shortcuts -p fileatlas-shell -p fileatlas-plugin-mcp -j 2`、
`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`，以及
`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、
`node scripts/check-feature-ledger.cjs`、`node scripts/check-layering.cjs`、
`git diff --check`。构建通过，保留已有 Windows LNK4098 链接警告。

同进程 MCP 场景运行两次：
`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-shortcut-final --plugins core,dev-test --scenario scripts/validate-shortcuts.cjs --allow-mutations`。
第一次打开 Shortcuts 页，将启动台键改为 Ctrl+Alt+Q；命令目录即时显示有效绑定，
设置写入隔离 JSON，截图确认行状态。第二次用同一 profile 重启，命令目录确认
覆盖值已恢复，设置页仍显示保存后的按键。运行记录与截图位于
`target/mcp-shortcut-final/report.json` 和 `target/mcp-shortcut-final/screenshots/`。

MCP 只能通过同进程合成 GPUI 按键，不能把真实系统级按键发往另一个前台应用；
因此 OS 前台切换、真实全局键分发和与其他软件的抢占冲突仍需 Windows 实机验证。

## 2026-10-17 程序启动台桌面图标交互

用户指定的大图标+名称布局按 `reference-ui/concepts/program-launcher-2026-10-17-v3.png` 与对应md实现，覆盖早期宽卡片/v2设计。左侧分类、中部图标网格、右侧编辑区复用共享浮动面板和GPUI Kit菜单；单击选中、双击启动、右键管理；安装日期/系统登记大小/名称支持双向排序。修正通用面板字符输入转发，补充同进程MCP文本输入和手动添加公开入口。

环境Windows 150% DPI，Rust1.97.1 MSVC。`cargo +1.97.1 fmt --all -- --check`、locked/offline shell check/build、Database10和i18n4项测试以及layering/architecture/feature-ledger/ui-refresh脚本通过。构建仍有已有LNK4098提示。旧库升级、重启和自定义字段兼容性通过新增数据库回归。

MCP命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-launcher-icons-validated --plugins dev-test,launcher --scenario scripts/validate-program-launcher.cjs --allow-mutations`。398真实项+1隔离无窗口EXE；396原生图标，3回退（含样例）；45日期/100大小。实际控件验证中文名称/分类/隐藏保存、单击不启动、双击样例写标记、右键菜单、三字段六方向排序、扫描变化、247→247静止绘制及重新打开图标缓存。缩至950×660后图标三列，保留分类与编辑滚动区。截图、完整命令、差异和边界见[运行记录](validation-ui/2026-10-17-program-launcher/README.md)。原生文件选取、真实应用启动及卸载/提权路径仍未验；用户常规数据未打开。
# 2026-10-18：顶部图标化及筛选栏右侧视图控件

后续用户反馈静置Lore统计日志：已修复可见文件版本提示对无关全库写入修订的缓存失效，
无需打开History也会进行初次被动检查。真实文件变更提示保留；无关写入不再重读。
同进程MCP静置/真实变化及29项回归证据见
[2026-10-18-version-idle](validation-ui/2026-10-18-version-idle/README.md)。界面布局无改动。

按用户本轮标注图做局部调整：顶部操作文本改图标并保留本地化提示，网格/列表/
子文件夹/缩放移至中部筛选栏右端，移除状态栏标签/更多。统一图标组件，原有服务与
持久化处理器保持。用户图与清单位于 reference-ui/concepts/main-toolbar-icons-2026-10-18.md。
Rust1.97.1 locked Shell build、fmt、FileManager24项测试及治理检查通过。Windows150% DPI
同进程MCP隔离实点验证布局切换、子文件夹1→2→1、缩放保存、搜索位置、范围弹窗、
工具入口与静置；查看默认/选中/tooltip/1510×1000小窗截图。
完整命令、截图和未验项见[本轮验收](validation-ui/2026-10-18-toolbar-icons/README.md)。

# 2026-10-18：独立收藏分组与当前目录快捷添加

按本轮用户两图，将原文件夹收藏/库文件夹/收藏集合复合区替换为多个平级独立分组。
与驱动器、快速访问共用 UI Bridge sidebar_section/sidebar_header；每组＋直接添加主窗
已提交目录，无目录时禁用，重复不覆盖别名，失效目录显示错误。登记库路径一次性迁入
常用组，保留既有自定义组，删除收藏不影响登记/扫描，重启不复活。小窗整列滚动已修复。

用户参考、ImageGen 概念及编码前清单位于 reference-ui/concepts/sidebar-favorite-groups-2026-10-18.md。
Rust1.97.1 locked Shell build、fmt、33项相关单测及全部治理检查通过；Windows 150% DPI
同进程 MCP 隔离 profile 验证真实指针＋/菜单/折叠、当前子目录、重复、空态/错误、长路径、
小窗滚动、静置和新进程持久化。完整命令、截图、差异及未验项见
[验收记录](validation-ui/2026-10-18-favorite-groups/README.md)。主窗工具栏的小窗横向裁切为
既有问题，本次未宣称解决；原生登记管理及网络权限未实测。

# 2026-10-18：剪贴板 v2 基准的全局统一主题

用户要求以当前剪贴板面板统一全软件及各插件，并建立未来开发约束。已将深蓝主题、
字体/间距/圆角、组件状态集中到 UI Bridge，抽取共享卡片/导航/操作/标题/输入组件；
移除功能 UI 的颜色硬编码，剪贴板和启动台共用组件，所有 Input/Textarea 经公共构造器。
相同控件的输入凹槽、边框与焦点环一并修正；业务布局与既有服务/配置路径保持。

`cargo +1.97.1` fmt、locked Shell check/build 通过，UI Bridge/Settings/i18n 共20项
测试通过；theme 正反探针、refresh/architecture/layering/feature-ledger 守卫通过，
主题守卫接入 CI。内置 ImageGen 概念已检查并保存；实际同进程 MCP/150% DPI 验证
主窗、历史空态、统计、批量操作、导出、剪贴板、启动台、设置全部分页、蓝→灰青→蓝
真实切换、hover/选中/焦点、静置及小窗滚动。24张最终截图和精简运行证据见
[完整记录](validation-ui/2026-10-18-unified-theme/README.md)。统计为零索引、历史为空态，
不能扩展为所有分支/错误/原生系统对话框/Debug状态均已验收；具体差异见该记录。

后续开发必须读 [统一主题规范](ui-theme.md)、[活动参考](reference-ui/README.md)，
执行 `node scripts/check-ui-theme.cjs` 和真实截图对照；AGENTS.md 已添加同样要求。
本次用户指令覆盖历史 v5 的石墨灰样式，各页功能布局及控件关系继续按活动参考。
# 2026-10-18：Shotoshot 截图启动管理器

按用户纠正后的范围集成 Shotoshot 完整程序：FileAtlas 只提供启动器、程序路径和截图目录管理，
选区、编辑标注、复制与保存由上游界面负责。用同进程 MCP `command_run` 打开面板，随后
`get_ui_state` 和 `screenshot_window` 检查程序路径、独立输出目录与快捷键；界面截图保存于
`target/mcp-screenshot-check/screenshots/screenshot-manager.png`。MCP `FILEATLAS_DEV_TEST_DIR`
隔离 profile 下真实桌面截图启动被阻止，输出目录和 Shotoshot 设置限定在 profile。

命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、
`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、`pwsh -File scripts/build-shotoshot.ps1`、
`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-screenshot-check --plugins core --plan target/mcp-screenshot-plan.json`，
以及 theme/refresh/architecture/layering/feature-ledger 守卫和 `git diff --check`，均通过。
Shotoshot 外部捕获/标注流程未在桌面上启动或验证；需由用户在实际使用中验收该上游窗口。

# 2026-10-18：截图集成启动修复与回归

用户反馈无法正确启动截图。定位到 Shotoshot 单实例行为：已有 resident 时，第二次 `--capture` 原先弹“已运行”并退出，FileAtlas 快捷启动因此只在 Shotoshot 未启动时有效。修改固定版本的 Shotoshot 启动入口，将重复启动请求发送到正在运行窗口的 `CAPTURE_MESSAGE`；新增 Windows 消息路由回归测试。

验证：`cargo +1.97.1 test --locked --manifest-path vendor/shotoshot/Cargo.toml -p shotoshot-app --lib second_capture_process_forwards_to_the_existing_window -j 2` 1 项通过；`... --test capture --test output -j 2` 9 项通过。真实 FileAtlas MCP（未启用 dev-test 隔离，因隔离模式会拒绝启动真实截图）检查 `command_list` 中 `Ctrl+Shift+S` 已注册，并调用 `command_run(screenshot-tool/capture)`；运行中的 Shotoshot resident 收到转发后，其预启动 editor 窗口获得可见主窗口。`get_ui_state` 显示输出目录为 `%LOCALAPPDATA%\FileAtlas\Screenshots`，目录存在且已同步进 Shotoshot `settings.txt`。验证时没有自动操作外部窗口；目前等待用户手动选区、加标注并保存，以确认实际文件落盘。

FileAtlas `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` 与 `build --locked` 通过。Shotoshot 原 target exe 当时被正在运行的两个进程锁定，故为 patched exe 使用独立 `target/shotoshot-integration` 编译目录；未终止用户进程。临时 FileAtlas MCP 进程与调试脚本均位于 `target/`。

## 2026-10-18：截图启动边界修订

用户指出应把它作为 Shotoshot 的中转插件：FileAtlas 启动时检测一次并缓存 Shotoshot 路径，
截图时再将请求转给 Shotoshot；未安装时由用户自行安装。检测结果加入“设置 → 软件”及截图管理器，
缺失状态提供上游官网入口。安装位置扫描不在每次渲染或每次快捷键触发时重复执行。Shotoshot
最终 PNG 仍保存到 FileAtlas 专用 `%LOCALAPPDATA%\\FileAtlas\\Screenshots` 文件夹。

附图中的提示持续出现，之前的根因是测试只升级了独立 Shotoshot EXE，而截图中 FileAtlas 仍指向
`target/shotoshot/debug/shotoshot.exe` 旧版本。现由 FileAtlas 插件直接向 Shotoshot resident 窗口发送
`WM_APP+2` 捕获消息；Shotoshot handler 收到后请求 editor 开始捕获，因此已有 Shotoshot 无需重装。

验证命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、
`cargo +1.97.1 build --locked -p fileatlas-shell -j 2 --target-dir target/screenshot-integration-build`、
`node scripts/check-ui-theme.cjs`、refresh/architecture/feature-ledger/layering 守卫及 `git diff --check` 均通过。
独立目标目录构建成功（现有默认 FileAtlas 进程占用了 `target/debug/fileatlas-shell.exe`）。隔离同进程 FileAtlas MCP
打开管理面板和“设置 → 软件”页、截取了 `target/mcp-screenshot-final/screenshots/screenshot-manager.png`、
`target/mcp-screenshot-software/screenshots/settings-software.png`；MCP 状态显示已检测到 Shotoshot、快捷键注册和
截图目录存在。缺失场景在 `target/mcp-shotoshot-missing` 验证，状态为 `executable_available=false`，
且界面将禁用启动并显示官网安装入口。

非隔离同进程 FileAtlas MCP 曾使用原有 `target/shotoshot/debug/shotoshot.exe` 触发一次
`screenshot-tool/capture`。当时仅用 `PostMessageW` 返回成功作为证据；用户反馈没有编辑器出现，说明此前
验证结论过度。现在改为同步 `SendMessageTimeoutW`（3 秒超时），在管理器保留请求状态，并记录请求、交付和
错误到 Debug drawer 及 `get_ui_state.screenshot_tool`。这项最新桥接改动已通过格式检查和 shell 编译，尚未通过
非隔离 MCP 重新验证真实 Shotoshot 编辑器响应、选区、标注及 PNG 落盘。测试使用的截图程序路径此前已恢复为
用户原有的 `target/shotoshot-integration/debug/shotoshot.exe`；截图目录仍为 `%LOCALAPPDATA%\\FileAtlas\\Screenshots`。
旧 FileAtlas 进程不能热更新，须关闭后启动新构建才能使用本次修复。

## 2026-10-18：剪贴板图片历史显示

在隔离 MCP profile 下用合成 PNG 记录验证图片历史，不读取或改写 Windows 当前剪贴板。通过同进程 MCP `get_clipboard_state` 确认记录类型、可用状态和 320×180 尺寸；等待 GPUI 图片资源加载后检查历史缩略图，选中记录后检查详情预览、分组、收藏和“复制图片到系统剪贴板”入口。截图：`target/mcp-clipboard-images/screenshots/clipboard-image-loaded.png`、`target/mcp-clipboard-images/screenshots/clipboard-image-selected.png`。新增捕获实现只调用 `get_text` / `get_image` 读取格式；不会清空或写入系统剪贴板。只有用户显式点击复制图片/文本时才调用 arboard 写入。

`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、独立目标目录 `cargo +1.97.1 build --locked -p fileatlas-shell -j 2 --target-dir target/clipboard-image-build` 及 theme/refresh/architecture/feature-ledger/layering 守卫通过。既有 `validate-clipboard-workbench.cjs` 在分组输入坐标断言失败（`category_name_length 0 !== 2`），因此不把该分类交互记为通过；图片卡片和详情由 MCP 隔离合成数据单独验证。Windows 原生剪贴板的真实图像采集、文件列表及应用专有格式仍未验证/支持；完整数据格式覆盖目前限于 arboard 可读的文本和解码图像。实现只写 FileAtlas 专用 `ClipboardImages` PNG 缓存，不会清理 Windows 系统剪贴板缓存。

## 2026-10-18：移除 Shotoshot 源码副本

用户确认 FileAtlas 应调用用户自行安装的 Shotoshot，而不把其应用源码纳入 FileAtlas 仓库。已从当前分支删除 `vendor/shotoshot/`（196 个上游文件）和 `scripts/build-shotoshot.ps1`，移除 `shotoshot-app` / `shotoshot-core` 本地依赖并清理锁文件。上文对临时 vendored 版本的构建和测试仅保留为历史记录，不代表当前仓库仍包含或构建 Shotoshot。当前插件只检测/启动外部 EXE、向运行实例发送请求，并在锁保护下更新 Shotoshot 设置文件中的输出目录字段。验证：`cargo +1.97.1 fmt --all`、`cargo +1.97.1 check --offline -p fileatlas-shell -j 2` 通过。


## 2026-10-18：插件设置中心

新增程序设置/插件设置分组，运行时插件总览及启动台、剪贴板、截图工具独立页；同进程隔离 MCP 验证 Fiber 停启与快捷面板呈现。详情、命令、截图和当前未覆盖项见 [插件设置验证记录](validation-ui/2026-10-18-plugin-settings/README.md)。

修复停用插件后侧栏按钮仍残留：Navigation 原先只在侧栏首次创建时实例化插件托槽，并长期缓存 GPUI 视图。现按 UiBridge 注册 ID 与 generation 同步两个导航槽，Cordis 注销会释放旧视图，再注册会创建新视图；Shell 的注册变化通知唤醒侧栏刷新。`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-plugin-navigation -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2` 均通过（链接器有既存 LIBCMT/LIBC 冲突警告）。同进程隔离 MCP 场景 `scripts/validate-plugin-settings.cjs` 完整通过：剪贴板注销时命令和侧栏托槽均消失、重新启用后恢复；最终停用截图为 `target/mcp-plugin-settings-final/screenshots/clipboard-disabled-sidebar-final.png`。

## 2026-10-18：恢复 QuickLook 与 Everything 软件检查页

用户发现拆分插件设置后，程序设置中的 QuickLook 与 Everything 检查不可见。检测器仍保留两项，但呈现页只筛选 Shotoshot，且程序设置侧栏丢失 Software 项。现恢复程序设置 Software 页并显示 QuickLook/Everything 两张检测卡；截图插件详情仍只显示 Shotoshot。`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-plugin-settings -p fileatlas-plugin-mcp -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2` 通过。默认 EXE 当时未发现对应运行进程，使用标准目标目录构建成功；同进程隔离 MCP 场景 `scripts/validate-software-check.cjs` 通过，实际环境识别 QuickLook 4.5.0 与 Everything 1.5.0.1423，检查页面展示版本、路径和项目/下载链接，没有启动或查询这两项软件。截图：`target/mcp-software-restored-20261018/screenshots/settings-software-check.png`。

随后完成 UI 托槽缓存审计：Shell 主挂载已按 ID/generation 复用有效视图并释放注销/换代视图；PanelCatalog 停用 owner 时移除面板定义、实例和对应挂载；插件元数据贡献在 Shell 渲染时读取实时 catalog；`settings.language` 原先由设置面板初始化时单次快照创建，现改为按 ID/generation 实时同步，并将此槽纳入 Shell 注册变化信号。Navigation 的内嵌槽也按 ID/generation 同步。`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-plugin-settings -p fileatlas-plugin-navigation -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2` 通过；主题、刷新、架构和 feature-ledger 守卫通过。新版通过同进程隔离 MCP 插件设置场景。现有 MCP 插件停启控制只覆盖启动台、剪贴板、截图工具，不能停用 i18n，因此语言托槽换代/注销未做运行时停启演练；此路径已静态检查并纳入统一同步逻辑。


## 2026-10-18 “仅浮窗”独立原生窗口修复

用户纠正：独立打开插件窗口，不拉起主界面。旧实现仅隐藏主窗内容，留下空白大窗，本轮移除该逻辑。
使用现有 PanelCatalog/Cordis 实例和生命周期，Shell组合独立GPUI Kit Root；快捷键不再无条件恢复主窗。
截图管理器正文关闭也改走公共close_definition。

最终64步同进程FileAtlas MCP通过，隔离target/mcp-native-panels，Windows/150%DPI；
主窗最小化保持、重复恢复同一独立窗、两工具并存、真实指针暂停/恢复、较小窗、
截图正文关闭/WM_CLOSE、Escape、停用清理/重启和模式切换通过。最终截图已打开核对。
fmt/locked Shell check/独立命名构建通过，UI Bridge19测试通过；theme/refresh/architecture/ledger通过。
layering仍有既存launcher/clipboard→settings层级错误，无新增依赖边。原用户EXE仍运行未覆盖，
新构建为target/debug/fileatlas-shell-native-panels.exe。OS真实系统快捷键派发、改色/菜单/全部长内容未复验；
较小窗沿用Clipboard隐藏分组创建区的既有响应逻辑，具体差异和尺寸见[完整证据](validation-ui/2026-10-18-native-panels/README.md)。

## 2026-10-18 独立浮窗前台激活更正

旧独立浮窗验收没有检查真正的前台窗口。已复现新建visible=true而active=false；现在首次创建和重复唤起都在Root就绪后走统一GPUI平台激活。三个工具36步前台断言及原64步回归通过，主窗保持最小化，quiet-state通过。默认EXE增量构建、fmt、locked check、theme/refresh/architecture/ledger通过。真实OS全局按键派发仍未验；详见[前台激活证据](validation-ui/2026-10-18-native-foreground/README.md)。

2026-10-19 初版（历史记录，现由下条三档实现取代）：MCP 插件设置中曾使用二态 `mcp_allow_mutations` 开关；该字段、截图和验证场景仅对应当时版本。详见[历史二态验证记录](validation-ui/2026-10-19-mcp-settings-toggle/README.md)。

2026-10-19 更新：按用户要求将二态开关改为三个分段阶段：关闭、常规控制、完全控制。常规控制含 UI 控制和普通增删改；完全控制额外开放插件 MCP 元数据标记为高影响的操作，高影响调用仍需每次 `confirm=true`。字段迁移为 `mcp_access_level`，旧 `mcp_allow_mutations` 布尔值保留兼容（false→关闭、true→完全控制）。设置页同进程 MCP 指针切换、普通写入、高影响拒绝/确认、缺失字段默认关闭和旧字段 true 迁移均验证通过。主题、刷新、架构与 feature-ledger 守卫通过；具体命令和截图见 [MCP 分阶段权限验证](validation-ui/2026-10-19-mcp-access-stages/README.md)。

## 2026-10-19 多栏/程序工作区

新增独立 Cordis 分栏与程序工作区插件。顶部筛选/排序/分组/视图操作作用于激活栏，搜索/功能按钮独立成组，每栏保留独立地址导航和完整文件视图；支持单栏、双栏、四栏。程序工作区保存启动布局及每栏地址、历史和浏览/视图状态，不存媒体数据。Everything 设置仍由设置界面管理。

Rust1.97.1 MSVC fmt、locked offline shell check/build 与 theme/refresh/architecture/feature-ledger/layering 守卫通过。同进程 FileAtlas MCP 使用隔离 dev-test profile 验证双栏与四栏显示、独立地址历史、保存当前双栏、重启恢复以及默认工作区重新应用；工作区管理器截图确认中文翻译。极窄窗口滚动、长列表选择/滚动位置恢复、所有管理器 CRUD 点击及全部主题交互状态未在本轮覆盖。完整命令、截图与边界见[多栏/程序工作区证据](validation-ui/2026-10-19-columns-foundation/README.md)。

## 2026-10-19 Windows 拖出文件导致进程关闭

用户反馈拖动文件时 FileAtlas 自动关闭。10:03:42 的新 WER 转储经本地 PDB 符号化后确认：GPUI Windows 窗口过程在 OLE `DoDragDrop` 的嵌套消息循环重入时触发 fail-fast；仅 `Window::defer` 仍处于原消息分发周期，未消除重入。现将阻塞的 OLE 操作移到独立线程，并保留 CF_HDROP 格式/HGLOBAL 校验。Rust 1.97.1 fmt、相关 crate locked offline check、Shell locked offline build 通过。重建后 FileAtlas 仍能运行，未出现新的 WER；但 Computer Use 的拖动坐标受限于当前窗口，无法把文件送出窗口，故 Explorer 和其他目标程序的实际投递及长时间拖动仍未验证。详见[拖放崩溃跟进记录](validation-ui/2026-10-19-drag-drop-crash/README.md)。

后续用户实测反馈该工作线程方案无法拖动。一个中间迭代将自写 COM 数据对象替换成 `drag` 2.1.1，但仍在 GPUI 回调同步调用 OLE，并临时屏蔽源 HWND 消息；虽然通过构建和启动检查，实际拖出没有得到验证，现已由下面的事件派发重构取代，不能视为有效修复。

本轮先检查 FileAtlas 原始参考项目：其 `egui` FileManager 也在卡片拖动回调同步启动自写 `DoDragDrop`，保留多选路径与 copy-only 语义，但并未提供适合 GPUI 的事件循环方案。参考 Zed/GPUI、Winit、`drag-rs` 与原生 Windows 拖放实现后，改为保留 GPUI 卡片手势和路径选择，由当前 UI 线程将请求投递至临时消息窗；原始输入回调返回后再启动 `drag-rs` OLE 拖动，结束时销毁临时 HWND。该方案不拦截 GPUI 源窗口消息，也不在线程间转移鼠标手势。fmt 与拖放/FileManager crate locked offline check 已通过；Shell 构建、治理检查、同进程 MCP 和 Windows 外部目标实际投递的验证状态见[拖出重构验证记录](validation-ui/2026-10-19-drag-drop-crash/README.md)。

用户确认该重构现在可正常拖动并投递到其他窗口。后续将拖动图像限制在 96×96 像素内，并为不可解码预览的文件生成带扩展名和类型颜色的文件剪影。格式检查、相关 crate locked offline check、FileAtlas Shell locked offline build 通过；新剪影的实际系统拖动外观仍待用户确认。用户提供的 `0x80040064` 来自 GPUI Windows `IDropTargetHelper::DragEnter` 返回值记录，与本插件预览解码是不同路径，详见[拖动图像尺寸和回退验证](validation-ui/2026-10-19-drag-drop-crash/README.md)。

2026-10-19 程序启动台整理：按现有 v3 图标网格参考保留面板结构，在程序信息区增加一句话说明，并完善自动分类和无关安装/帮助快捷方式筛除；右键卸载入口仅对应 Windows 注册的卸载命令且需要确认。隔离同进程 MCP 扫描 292 个主机程序，确认无卸载器/安装器目标混入、所有项目均有说明、Photoshop 信息说明和卸载菜单可见。随后正常用户配置启动并完成扫描，启动台 JSON 从 298 项清理到 295 项，零无效安装/卸载器目标、零空说明、“其他”类别零条目；三条旧自动分类项目也完成归类。Rust 1.97.1 shell 检查/构建及治理检查结果、截图和说明来源见[启动台整理验证](validation-ui/2026-10-19-launcher-classification/README.md)。真实卸载器执行/提权和未知产品的精确说明仍未在本轮验证。

2026-10-19 程序启动台整理：按现有 v3 图标网格参考保留面板结构，在程序信息区增加一句话说明，并完善自动分类和无关安装/帮助快捷方式筛除；右键卸载入口仅对应 Windows 注册的卸载命令且需要确认。隔离同进程 MCP 扫描 295 个主机程序，确认无卸载器/安装器目标混入、所有项目均有说明、Photoshop 信息说明和卸载菜单可见。Rust 1.97.1 shell 检查/构建及治理检查结果、截图和说明来源见[启动台整理验证](validation-ui/2026-10-19-launcher-classification/README.md)。真实卸载器执行/提权、未知产品的精确说明和用户常规配置迁移仍未在本轮验证。

## 2026-10-19 低饱和度语义配色与亮色对比修正

按用户要求统一全插件 palette 并修复前景/背景遮盖。UI Bridge theme 现在提供暗亮两态的低饱和度表面、文字、强调和状态语义，并按实际填充色计算按钮文字；状态/强调文字通过对比度辅助函数适配中性凸起表面。全部 UI 消费 crate 不再直接读取暗色兼容颜色常量。同进程 MCP 两态对比校验通过，亮色弱化正文首次 4.30:1 的边界值已修正为 4.5:1 以上。fmt、locked shell check/build 和 theme/refresh/architecture/feature-ledger 检查通过。运行窗口主视图、已选文件和浅色设置页已截图打开核对；菜单/键盘焦点/禁用状态/小窗口/其他强调色的实画状态仍未全验。完整边界、命令与截图见[低饱和度语义主题验证](validation-ui/2026-10-19-muted-semantic-colors/README.md)。
# Theme modes and Shell action ownership — 2026-10-19

## Material native color palette — 2026-10-19

The seed text field is replaced with GPUI Kit's existing open-source ColorSelect.
Same-process MCP exercised palette selection, draft-only edits, Generate, dark/light,
hover cancellation, smaller-window rendering and persisted restart. Formatting,
locked shell check/build and governance checks passed. Exact scope and screenshots:
[native seed color picker](validation-ui/2026-10-19-material-color-picker/README.md).

## Native GPUI theme convergence / Material generator — 2026-10-19

Current implementation supersedes the handwritten palettes described in the historical
theme-mode, muted-color and thumbnail records below. GPUI Kit now resolves and distributes
all theme colors; Material is an independently registered Cordis JSON generator.
Locked shell check/build, generator tests, formatting and governance checks passed.
Same-process isolated MCP verified actual generation/input, invalid-input preservation,
light/dark changes, plugin surfaces, native menu, small-window scrolling and restart.
Exact commands, screenshots, retained reports and coverage limits:
[native Material theme validation](validation-ui/2026-10-19-native-material-theme/README.md).

The dark/light theme setting and plugin-owned rail/toolbar actions were
implemented against the active theme concepts. Static checks and the main-window
dark-to-light toggle were verified through same-process FileAtlas MCP; broader UI
states remain incomplete. See [theme modes and Shell actions](validation-ui/2026-10-19-theme-modes/README.md).

## 2026-10-19 用户截图：缩略图角标与标签对比纠错

用户截图推翻前轮对格式角标/图片标签的充分覆盖：前轮仅验证普通 Theme token，未覆盖实际 HSL 角标、黑色混合标签背景及图片遮罩。
已按实际底色修复角标、标签文字/图标、评分/计数和选中导航。使用隔离真实文件、黑白彩色 PNG、四个彩色标签与四星，通过同进程 MCP 切换亮/暗并打开截图检查；实际角色对比全部达到 4.5:1，静置不重载。
命令、截图、精确范围与未验项见[本轮修复证据](validation-ui/2026-10-19-thumbnail-contrast/README.md)。静态格式/编译/构建及主题、刷新、架构、分层、台账检查通过。

## 2026-10-19 用户反馈：栏目地址面包屑缺少当前文件夹

中间“前进/后退”地址行的上级路径容器 `overflow_x_scrollbar` 会撑满剩余宽度，把当前目录段推到最右端，
面包屑读起来止于上一级。改为上级段按内容自然宽度排布、可收缩滚动，当前目录段紧跟其后，
编辑/打开/刷新按钮仍靠右。同进程 MCP 在单/双/四栏导航到中文深层目录截图确认；
格式、锁定构建、file_manager 测试（顺带修复该测试目标此前无法编译的插件依赖装配）与四项治理检查通过。
范围与未验项见[栏目地址面包屑证据](validation-ui/2026-10-19-column-address-breadcrumb/README.md)。

## Validation artifact storage policy (2026-10-19)

Reusable scenario scripts and concise Markdown findings are kept in the repository.
Generated screenshots, full JSON reports, fixtures, logs and profiles are Git-ignored
under `.local/validation/`; their former tracked attachments were moved to
`.local/validation/artifacts/` while preserving their validation-record subdirectories.
New same-process MCP profiles default to `.local/validation/profiles/`. Historic `target/`
commands below describe past runs and are retained as a record; new tests use the local
validation root. Directory and link rules: [validation data policy](validation-data-policy.md).

## 2026-10-20 图标库使用审计与文字符号修复

审计全部插件后确认：没有内联 SVG、没有插件自带 `.svg` 资源、没有 emoji 私用区字形；唯一违规类是把 Unicode
文字符号直接写进按钮/菜单文本，绕开 `IconName` + `theme::icon` 网关，共 15 处（launcher 11、debug 3、navigation 1）。
已改为图标库常量或组件库自带的 `dropdown_caret(true)` 与 `PopupMenuItem::checked(...)`，并同步 i18n 键值
（去掉符号前缀、删除 4 条无引用键），顺带补齐 `en.ftl` 缺失的 7 条主题标签消息。
`scripts/check-ui-theme.cjs` 新增“文字符号即图标”守卫与 12 条 `--self-test` 探针，防止该类缺陷回归。
格式、受影响 crate 测试、四项治理检查与 workspace 检查通过；同进程 FileAtlas MCP 在隔离 profile
`.local/validation/profiles/icon-glyph-catalog` 实测了空闲/扫描页脚、排序与方向按钮、分类/置顶下拉、
两处 `checked()` 菜单、`Star`/`StarFill` 角标、`Square`/`SquareCheck` 开关、地址行 `Ellipsis` 与 debug 抽屉三处箭头。
命令、截图与未验项（亮色主题、极窄窗口、键盘焦点、菜单悬停态）见
[图标库使用审计证据](validation-ui/2026-10-20-icon-glyph-catalog/README.md)。

## 2026-10-20 子文件夹分组拖放

拖放后的滚动回归：共同文件操作完成回调误用导航重载，真实拖放复现滚动 8842→0。
现改用保留旧内容、滚动和有效选择的背景刷新。760 文件的中部真实 GPUI 指针场景及
精确结果见[拖放刷新保位验证](validation-ui/2026-10-20-drop-scroll/README.md)。

后续整体框架更新：用户指出内容区无法放置，已将 drop handler 从标题迁入共同分组框架，
所有虚拟行保留目录身份及首末边界，共享目标高亮。增加仅隔离 dev-test profile 可用的内部
拖拽验证开关，通过同进程 GPUI 指针实际覆盖标题、内容空白、无标题后续行、卡片与折叠组。
实现、命令、截图检查及物理 OLE 的覆盖边界见
[整体框架证据](validation-ui/2026-10-20-group-frame-drop/README.md)。

子文件夹分组标题现在接收 FileManager 文件拖放，并通过 File Operations 现有批量移动服务处理；
MCP 同步增加 `move_files_to_subfolder` 控制入口用于隔离进程实测。生产索引器扫描的两目录 fixture
通过同进程 MCP 完成实际文件移动和索引路径刷新。功能链路通过；Windows 原生拖放手势和视觉截图验收
仍未覆盖。详细命令、路径和边界见[验证记录](validation-ui/2026-10-20-subfolder-drop/README.md)。

## 2026-10-20 文件暂存篮首版

新增独立 File Staging Cordis 插件、版本化 JSON 路径清单、主题面板/Launcher 入口、FileManager 选区加入事件，以及委托 File Operations 的逐项复制/移动。隔离同进程 MCP 使用真实临时文件覆盖多目录添加与去重、复制、冲突不覆盖及失败保留、移动、失效清理和新进程重启持久化。格式、受影响 crate/shell check/build 与架构、分层、主题、刷新、feature ledger 检查通过。原生文件/目录选择器点击、FileManager 菜单点击、跨卷/权限行为和小窗/浅色状态仍未验证；不支持任务取消/恢复。详见[文件暂存篮验证记录](validation-ui/2026-10-20-file-staging/README.md)。

后续按用户要求改为缩略图/标题网格卡片，并将暂存面板设为非模态，保留显式复制入口；拖放意图为松开即移动，成功路径应从暂存篮移除。发现 Shell 遮罩谓词未排除非模态面板，导致主界面变暗且无法交互；现已仅对模态面板创建遮罩。同进程 MCP 实测暂存面板打开时仍可点击选择 FileManager 子文件夹。并新增 FileManager → 暂存篮拖入：共享文件拖拽载荷通过 GPUI 后台执行器写入清单；同进程 MCP 验证 tiny.jpg 加入暂存且源文件保留。反向暂存篮 → 文件夹拖放仍显示 ghost 和目标高亮，但 pointer-up 未执行移动，仍待修复/验证。编译和主题/刷新/架构/ledger 检查通过，详见[文件暂存篮验证记录](validation-ui/2026-10-20-file-staging/README.md)。

## 2026-10-20 文件操作面板样式与右键复制崩溃

批量操作弹层标题高度改用 UI Bridge 通用浮动面板的 36 逻辑像素规格，并统一紧凑水平留白；布局、筛选/匹配正文和固定页脚保持活动参考。右键“复制到…”单文件和多文件流程中的同步 rfd 调用改为 GPUI 异步文件选择器，确认目标后继续调用原 File Operations 服务，取消不执行操作。锁定 Shell build 通过；同进程 MCP 打开批量操作面板、截图并再次读取状态成功。原生保存对话框的选择/确认尚未通过 MCP 实测。详见[文件操作验证记录](validation-ui/2026-10-20-file-operations-panel/README.md)。

## 2026-10-20 项目树层级与项目切换

项目页仅展开当前项目根目录的子树，根行点击切换当前项目并导航，箭头单独收起/展开；新增根目录后立即选中并展开。子目录通过 GPUI Kit Theme 分隔色点状竖线/横线表达层级。统一 Windows 路径比较以去掉 canonical path 的 `\\?\` 前缀，修复新增根目录无法与 Navigation 当前目录匹配的问题。同进程 MCP 用两个隔离项目实测添加、自动展开、切换和收起，并检查项目根与深层目录截图。格式、锁定 Shell 构建与治理检查通过；超大树性能和窄窗未覆盖。详见[项目树验证记录](validation-ui/2026-10-20-project-tree-switch/README.md)。
# WorkCrate three-level correction — 2026-10-20

The latest user-selected three-level layout and A3 defaults supersede the first
WorkCrate slice. Three inspected concepts were saved before implementation.
Project cards now show only title/main image; page management and the page editor
are separate screens. Real same-process MCP pointer actions cover project entry,
page selection and saved DesignCraft drag; ordered PDF/PNG/JPEG flows and project
management mutations were exercised. See the focused
[three-level record](validation-ui/2026-10-20-workcrate-three-level/README.md)
for screenshots, commands, corrections and unverified states.

## 2026-10-20 WorkCrate 用户反馈后的纠正

上一轮三层页面及后台放图证据不能证明概念还原和可见添加/拖拽已通过。
本轮按原三张概念恢复卡片比例、信息同行、编辑区工具带/侧栏和画布占比；
修复 Windows 路径比较、缺少画布接收器、FileManager 过早切入 OLE、装载过程解码误报。
实际指针添加与正常模式文件拖入的复现、截图位置和未验路径见
[WorkCrate 布局与图片入口纠正](validation-ui/2026-10-20-workcrate-input-layout/README.md)。

## 2026-10-20 固定 A3 简化排版（覆盖旧通用画布语义）

用户已澄清页面只有页内信息条和下方图片区，页外尺寸只是标记。
本轮撤销整页平移和尺寸修改，接入原图图层实时预览、四角等比缩放、可恢复裁剪、
项目一键刷新及独立 PSD 源链接。主界面正常模式拖入、拖动中不写清单、跨两页刷新、
A3 PDF/PNG 和静止状态均经同进程 MCP 实际操作；Photoshop 外部编辑仍未验。
最新记录：[固定 A3 图片排版](validation-ui/2026-10-20-workcrate-simple-layout/README.md)。


## WorkCrate 性能优化（2026-10-20）

持久页面缩略图、按当前页可见密度的显示代理、原图导出和阶段页脚已实现。
本机三页合成大图项目：打开中位数 2451→125 ms，单页 PNG 12180→950 ms，
三页 PDF 26242→4449 ms，PNG 原始 RGBA 完全一致。
[实测记录与范围](validation-ui/2026-10-20-workcrate-performance/README.md)。

## WorkCrate 画布直接输入与文本局部刷新（2026-10-21）

新增文本直接在 A3 内输入，双击已有文本可继续；四角框尺寸/上沿移动和右侧字号实时生效。
文本保存不走整页 reload，实际 MCP 检查确认 load_generation、背景和照片 GPU 图像 ID 均不变。
原生输入默认 rem 行高导致大字行重叠的问题已修正为随字号变化。
[同进程 MCP 截图、命令及覆盖范围](validation-ui/2026-10-21-workcrate-inline-text/README.md)。

## WorkCrate 拖选、原生文本设置与默认信息同步（2026-10-21）

修正双击后拖选与底层对象拖动冲突；已通过真实鼠标拖选、选区替换、选区下划线、
段落对齐、文本框分栏、属性搜索和字距设置。更多文本设置提供字符/段落/框/字体OpenType
入口；原生目录254项，属性数据覆盖不代表每个字体/枚举组合的UI测试。
默认信息有效修改自动保存，回编辑页可选择；连续添加期间保留编辑/添加/删除可用。
[同进程MCP记录、边界与复现命令](validation-ui/2026-10-21-workcrate-typography/README.md)。
# 2026-10-21 WorkCrate inspector and refresh correction

The right inspector now has explicit section dividers and the current preset snapshot
is read when a menu opens. Page-local writes no longer reload the entire board catalog;
repeated same-page external navigation also returns without a reload. Same-process
MCP, screenshots, commands, result boundaries and the refresh-site audit are recorded
in [WorkCrate inspector validation](validation-ui/2026-10-21-workcrate-inspector-refresh/README.md).
