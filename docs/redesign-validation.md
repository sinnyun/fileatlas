# 界面重构验证记录

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
- 执行命令与环境：`cargo +1.97.1 check --locked -p photag-plugin-gallery -j 2`
  （Windows，MSVC 工具链 1.97.1，`--locked`）。
- 结果：失败，11 个错误，全部来自新写入但尚未接线的
  `crates/photag-plugin-gallery/src/view/inspector.rs`
  （缺 `GalleryView.surfaces` 字段、`ensure_exif_loaded`、`remove_context_tag`、
  `restore_selection`/`format_bytes`/`format_timestamp` 可见性、
  `ButtonVariants` 未导入）。`view/filter_row.rs` 尚未在 `view.rs` 声明，
  因此其问题还没进入编译器视野。
- 未执行：`cargo fmt --check`、`check -p photag-shell`、任何 crate 测试、
  以及应用启动。本阶段所有界面（顶栏三段式、卡片网格、月组头、空态、筛选
  chip 行与弹层、检查器右栏）在窗口中渲染成什么样、控件是否可用，**均未验证**。
- 仍缺失的设计稿控件/状态：筛选弹层的包含/排除切换无处理器；检查器未注册到
  `gallery.inspector` 槽；状态栏与工具菜单未按 6.5-6 重排；空态 “Add folder”
  的 `navigation/add-folder` 无接收方；`filter_row.rs`/`inspector.rs` 的英文
  原文尚未登记 i18n 键（界面会显示英文而非稿面中文）。
- Windows 原生对话框、拖放、文件操作、编解码、Shell 集成：未验证。

### 2026-10-13 S2 接线及启动更新弹窗：插件 MCP 局部实机验证

- 设计稿：`00-main-library.png`、`05-filters-menus.png`；更新策略依本次用户
  指示覆盖旧的自动弹窗行为。已阅读两稿及 v5 concepts companion notes。
- Windows MSVC 1.97.1；`cargo +1.97.1 fmt --all -- --check`、
  `cargo +1.97.1 check --locked -p photag-shell -j 2` 通过。
- `cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-navigation
  -p photag-plugin-i18n -p photag-ui-bridge -j 2`：16+4+4+3=27 项通过。
  后续 `test --locked -p photag-plugin-settings -p photag-plugin-mcp
  -p photag-plugin-gallery -j 2` 通过（Settings/MCP 当前无单元测试，不声称覆盖）。
- 普通 `build --locked -p photag-shell -j 2` 被既有进程 PID 14400 占用
  `target/debug/photag-shell.exe` 挡住（OS error 5）；未停止该进程。
  使用 `cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2
  -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-s2b.exe` 成功构建
  独立验证二进制（仅 output 名称与 out-dir 警告）。
- 实机客户端：Node stdio MCP，启动上述二进制 `--mcp`；
  `PHOTAG_DEV_PLUGINS=dev-test`，`PHOTAG_DEV_TEST_DIR=target/mcp-s2-validation`，
  `PHOTAG_MCP_ALLOW_MUTATIONS=1`（仅隔离样例扫描与 UI 状态请求所需）。
  隔离 `settings.json` 明确设置 `auto_check_updates=true`、language 为 zh-CN。
  不触碰普通 PhoTag 数据；验证后停止本次 MCP 子进程。
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

- 用户要求所有实机调试/测试经PhoTag插件MCP，并明确暂停实现、整理给小模型。
  所有Luna子智能体停止；本轮MCP子进程停止，保留隔离数据与截图。
- 最后成功构建并截图：`cargo +1.97.1 rustc --locked -p photag-shell
  --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-v5-iteration.exe`。
  用Node stdio MCP启动`--mcp`，profile=`target/mcp-s2-validation`，dev-test，
  UI状态/样例扫描所需mutations=1。主稿00、六个05弹层、02/03设置六页、01预览
  的真实截图已保存，具体文件和失败观察见完整交接。
- 已观察：全量AllAssets图标呈现、单行筛选、ListState不同高度网格；MCP六个
  filter popover均可实际打开。未由此宣称鼠标传播、框选/跨页/压力库已验证。
- 失败截图：设置六页正文空白且Gallery透出；日期弹层中文标签溢出与英文占位；
  Preview舞台半透、信息长表与旧底部浮动工具条。随后已写修复的部分尚未重拍。
  `v5-audio-first.png`仅主界面：MCP audio分支仍报不支持，不能声称音频播放已验。
- 最后工作树：`cargo +1.97.1 fmt --all`与`fmt --all -- --check`通过；
  `cargo +1.97.1 check --locked -p photag-shell -j 2`失败19错，完整日志
  `target/development-handoff/check-current.log`。错误位于bulk_actions的MediaItem/
  section_title、filters的StyledExt、tagging_picker的ScrollableElement。
  最新源码未生成可运行EXE、未跑新相关测试。
- 新辅助脚本`node --check scripts/photag-mcp-debug.cjs`通过；以最后成功EXE在
  `target/mcp-handoff-client-smoke`新隔离档案完成initialize/get_ui_state/
  screenshot_window，退出0；默认mutations关闭。它证明客户端可用，不证明
  最新Rust源码可运行。
- 交接入口：`docs/development-handoff-2026-10-14.md`，含逐稿缺失状态、i18n欠账、
  公开API实/未实现区别、恢复编译与MCP迭代步骤。旧成功测试与截图均按版本区分。

## 2026-10-14：Luna 并行样式实现与主线程 MCP 验收

本节覆盖上文旧构建/19错快照。设计依据是 docs/reference-ui/final-v5 的 00–08 图及 concepts 说明。用户明确要求多个 GPT-6 Luna 子代理：Gallery、Navigation、Settings 分别实现，主线程合并检查并通过同进程 PhoTag MCP 截图迭代。普通数据库/设置未使用；原工作树保留，未提交。

环境：Windows MSVC，已检查 rustup toolchain list/rustc +1.97.1 --version；Rust 1.97.1，150% DPI。常规外窗1942×1256对应约1280×800内容，最小外窗1462×956对应约960×600内容。最新EXE为 target/debug/photag-shell-accepted-20261014.exe。所有运行截图由 scripts/photag-mcp-debug.cjs 启动该EXE --mcp取得，没有另外启动GUI或OS输入注入。

静态验证命令与结果：

- cargo +1.97.1 fmt --all -- --check：通过。
- cargo +1.97.1 check --locked -p photag-shell -j 2：通过，target/accepted-check.log。
- cargo +1.97.1 build --locked -p photag-shell -j 2：通过，target/accepted-build.log；复制到上述独立EXE名。
- cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-navigation -p photag-plugin-settings -p photag-plugin-preview -p photag-plugin-media-playback -p photag-plugin-metadata-export -p photag-plugin-mcp -p photag-plugin-i18n -p photag-plugin-indexing -p photag-plugin-tagging -p photag-plugin-dev-test -j 2：36通过/0失败/1 ignored，target/accepted-tests.log。
- cargo +1.97.1 test --locked -p photag-plugin-indexing extracts_a_thumbnail_from_a_valid_generated_video -j 2 -- --ignored：另1通过，target/final-ffmpeg-test.log。
- node --check scripts/photag-mcp-debug.cjs：通过。客户端现在支持 expect_error 和 until.at_least，实际预期错误保留在报告。

MCP运行模板：node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-accepted-20261014.exe --profile target/mcp-v5-validation --plan target/final-accepted-plan.json --allow-mutations。profile下LOCALAPPDATA、DB、settings、session和缓存全部隔离。改变UI/隔离数据时启用mutations，不涉及用户普通库。当前全部MCP子进程已退出。

| 参考稿 | 本轮修改及实际检查 | 截图（target/mcp-v5-validation/screenshots） |
|---|---|---|
| 00/01 Gallery | 中性背景、紧凑筛选、无缩略图占位、侧栏树/计数、文件信息折叠、选中/无结果、侧栏与Inspector折叠、最小窗 | 00-main-accepted.png、00-min-final.png、00-min-both-closed-final.png、08-no-results-final.png |
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

最后进程检查：既有PID33036仍运行 target/debug/photag-shell-handoff.exe（旧版），未终止；本轮MCP子进程均已退出。查看当前新样式应使用 photag-shell-accepted-20261014.exe，旧窗口不能代表本轮修改。

## 2026-10-14：两侧分页/固定预览 v2 实施与验收

用户当前参考 docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png 优先于final-v5的主窗口两侧布局。配套说明已新增同名.md；其他界面保持前轮设计。沿用用户授权的Luna代理协作实现Navigation与Inspector，主线程完成双侧分隔条、会话兼容及MCP实测。主媒体网格/筛选/后端语义保持，选中勾点移除，蓝边保留。

最新运行文件：target/debug/photag-shell-sidebar-v2.exe。环境Windows/150%DPI/Rust1.97.1 MSVC；rustup toolchain list和rustc +1.97.1 --version确认，使用已安装工具链。隔离profile target/mcp-sidebar-v2（复制上一隔离测试库及其WAL，未接触正常PhoTag数据/会话）；12媒体仍在，选择02-mixed-formats匹配3。

执行命令：cargo +1.97.1 fmt --all -- --check；cargo +1.97.1 check --locked -p photag-shell -j 2；cargo +1.97.1 build --locked -p photag-shell -j 2；cargo +1.97.1 test --locked -p photag-plugin-session -p photag-plugin-navigation -p photag-plugin-gallery -p photag-plugin-mcp -p photag-plugin-i18n -j 2。最终均通过，25 tests/0failed；包含旧session缺少inspector_width时保留原folder/media/Tags选择并round-trip的新兼容测试。日志target/sidebar-v2-check.log、sidebar-v2-build.log、sidebar-v2-tests.log。

MCP：node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-sidebar-v2.exe --profile target/mcp-sidebar-v2 --plan target/sidebar-v2-acceptance.json --allow-mutations，32步骤全部执行；target/sidebar-v2-pointer.json另16步骤；target/sidebar-v2-restoration.json默认只读5步骤。报告分别profile/report-acceptance.json、report-pointer.json、report-restoration.json；对应target/sidebar-v2-*.log。全部窗口由同MCP child process创建，截图为真实PhoTag画面，没有另开GUI或OS指针注入。

| 用户要求 | 实现与实测 | 证据（profile/screenshots） |
|---|---|---|
| 左侧文件夹/标签/扩展独立页 | 48pt竖rail；MCP页切换、GPUI down/up实际点击Tags/Folders；每页独立内容；底部Settings沿rail | v2-tags-page.png、v2-tags-clicked.png、v2-folders-clicked.png、v2-extensions-page.png |
| 文件夹缩进/图标/数量/选中 | LucideFolder/FolderOpen、右对齐真实count，紧凑overlay菜单与标题+Add/More；搜索mixed、noresults、clear；All files12 | v2-folder-search.png、v2-folder-no-results.png、v2-restored-final.png |
| 父子层级连接线 | 懒展开folder1及03-nested；浅灰竖线+横短线，正确02-mixed-formats选择/3记录；两层真实目录level-one | v2-final.png、v2-restored-final.png |
| 右侧固定预览/下方分页 | 固定220pt画布、文件名/评分，点击预览路由原服务；信息保留基本/标签/EXIF/路径/文件菜单、独立滚动；History/Extensions切换预览仍固定 | v2-preview-info.png、v2-history-page.png、v2-inspector-extensions.png、v2-minimum-history.png |
| 两侧常驻可拖动分隔条 | 8pt命中/24pt把手/列调整光标；root move/up/out-up；MCP通过Window.defer派发GPUIPlatformInput down/drag/up，左264→304，右300→340；第二次启动恢复304/340 | v2-sidebar-dragged.png、v2-both-dragged.png、v2-restored-final.png |
| 缩略图只留蓝色边框 | 去掉右上checkbadge；边框显式RGBA，folder/rail selectedfill同步修正颜色打包；实际medium.webp选中7 | v2-readonly-final.png |
| 小窗口及折叠 | 960×600附近外窗1462×956，220/260面板，信息区保留footer且下方滚动；left/right折叠可恢复 | v2-minimum-window.png、v2-minimum-narrow.png、v2-sidebar-closed.png、v2-inspector-closed.png |

MCP扩展：control_ui navigation_tab/inspector_tab、sidebar_width/inspector_width；pointer_down/drag/up坐标为逻辑client x,y且受mutationsguard。CaptureWindowHandle单请求mailbox由Shell deferred派发，避免在render lease中重新update当前实体；这是项目PhoTag MCP真实GPUI事件通路，不是OS注入或仅设置宽度模拟。只读重启时pointer_down被正确拒绝(PHOTAG_MCP_ALLOW_MUTATIONS)，宽度保持304/340。

发现并修正：首次展开子目录panic“cannot read SidebarView while already being updated”，渲染改为直接传selected_directory；SelectPath原普通slash路径与DB canonical前缀不一致导致0匹配，canonicalize后正确3；菜单Button追加hover与Kit默认hover重复panic，采用标准ghosthover+不透明底色；source theme.accent_alpha返回RGB而非RGBA，修正本轮选中fill/border打包。早期失败保留于stderr和report-before-path-normalization.json，不算最终通过。

边界：History使用既有全局undo/redo摘要，当前无媒体历史时显示实情；没有新增单文件持久日志。扩展预留专用slot，无贡献显示暂无可用扩展，未引入插件商店或假按钮；当前无第三方贡献，不能声称加载/卸载外部扩展已验。信息区域实际鼠标滚轮、OS窗口外捕获、原生文件对话框及其他前轮未验项不在本轮证据内。GPUI内拖动/点击、尺寸与重启恢复已验；正常用户旧PID窗口保持不动。

## 用户确认的开发基线（2026-10-14）

用户认可当前界面并要求建立开发基线。基线标记dev-baseline-2026-10-14；关键截图/报告/计划已归档到docs/validation-ui/2026-10-14-baseline，不再仅依赖target本机输出。下一阶段功能操作优化与功能扩展的入口为functional-development-plan.md。上文未覆盖的行为继续保留，未因布局验收被标记为通过。

## 2026-10-14 基线后的通用弹出面板／列表／菜单优化

用户指定的四项优化已实现：设置及媒体预览共用UI Bridge浮动框架、主界面点击隔离、Cordis-owned Single/Multiple PanelCatalog与各定义默认尺寸、Grid/List按钮及会话偏好、保持卡片按钮菜单触发实体。设计对照final-v5/01/02/03/05与baseline sidebar-pages-preview-v2；新的窗口几何与列表按本轮用户指令。完整141步MCP参数／结果／截图、准确命令、样本及未覆盖边界见[本轮运行证据](validation-ui/2026-10-14-panels/README.md)。

Rust1.97.1 MSVC fmt/check/标准locked shell build通过；相关26项测试通过，最后UI Bridge5项清理回归通过。真实GPUI设置拖动100×50、缩放900×680→800×580、tab点击与外层隔离；菜单进入和预览ID6、单实例复用、多实例独立移动／关闭；真实图片/WAV/MP4解码及面板max/restore；List实际选择ID9与下一进程恢复；100009项列表末页offset99809/cache200；只读拒绝。正常用户数据未用于测试。

SQLite与既有后端动作语义不改；源Session serde-default与Gallery grid/context-action语义已追踪，映射追加于source-feature-audit.md；gallery_list_layout字段为兼容的新增偏好。新组件使用说明见[floating-panels](floating-panels.md)。七个未逐项实测resize把手、窗口外捕获、不同DPI、列表修饰键/框选/OS拖出、全codec及听感、现有服务多媒体并行仍未覆盖，不将这些记为通过。

## 2026-10-14 添加文件夹点击退出修复

通过项目同进程MCP pointer_down/up(215,74)复现旧版Sidebar +点击退出：main线程在gpui-pre/app/async_context.rs:66报告RefCell already borrowed。同步rfd文件夹选择器的Windows消息循环重入GPUI App借用。已改为AsyncFileDialog的独立STA/native循环，选择完成再this.update→原add_folder_path路径，取消保持无操作；单实例guard和pending禁用避免重复打开。

Rust1.97.1 Windows MSVC locked shell check/build、fmt check及Navigation/MCP相关5项测试通过。15步隔离profile MCP实测：真实按钮点击、native_open=true、原生选择器PrintWindow截图、主窗口持续get_ui_state响应、cancel/reset、第二次重新打开/取消、library始终为空；修复日志无panic。准确命令/参数/截图见[运行记录](validation-ui/2026-10-14-folder-picker/README.md)。设计仍为sidebar-pages-preview-v2的相同标题按钮；Windows选择器委托系统，不改变页面布局。实际native accept及窗口创建期间卸载竞态未覆盖；不能将service测试等同native选择成功。

## 全文件卡片与格式角标（2026-10-14）

本轮沿用 sidebar-pages-preview-v2 及 final-v5 主界面结构，按用户指示将卡片右上More改为非交互格式角标，Grid/List与Inspector无缩略图时使用文件类型图标。Windows同进程PhoTag MCP隔离验证45步：16文件全部可见、photo筛选5、file查询9；未知/无扩展名/隐藏文件、视频封面独立行、文档/归档图标、真实右键评分子菜单、增量扫描保留ID/评分、watcher未知格式创建/删除及目录排除。fmt、locked shell check、shell备用bin完整编译通过；27相关自动测试通过、1原有FFmpeg测试ignored。证据/命令/环境/启动诊断/未覆盖状态见[验证说明](validation-ui/2026-10-14-all-files/README.md)。正常用户profile没有修改；旧默认exe被运行中窗口占用，新可执行文件为`target/debug/photag-shell-all-files.exe`。

最终补验：缓存命中增量扫描保留图片尺寸/micro，Indexing回归再次通过；重编译后32步PhoTag MCP重测保留ID/评分及24×16尺寸，见同证据目录cache-retention-report.json。

## 独立分组插件（2026-10-14）

保持sidebar-pages-preview-v2/final-v5主布局，按用户新要求把按月规则/分组排序/虚拟分段迁入photag-plugin-grouping，扩展类型、修改月份、创建月份、登记根目录及实际父目录分组。26自动测试、fmt、locked shell check、独立Shell完整编译通过；57步同进程PhoTag MCP验证248文件的全部分组、200行缓存末页、实际菜单/标题鼠标点击、选择保留及只读重启恢复。另修复标题框选冒泡与工具菜单滚动容器定位。环境、命令、截图、报告及网络/大库/横向滚动未验边界见[本轮验证](validation-ui/2026-10-14-grouping/README.md)。默认EXE被旧窗口占用，运行构建是target/debug/photag-shell-grouping.exe，未停止旧窗口或改普通用户profile。

## 分组标题背景增强（2026-10-14）

按用户要求给所有分组标题行加入不透明蓝灰背景、边框、圆角及更亮hover，保留40pt几何、计数与折叠。fmt/locked Shell check及独立Shell编译通过；14步同进程PhoTag MCP验证网格、悬停、标题实际点击折叠、列表及月份标题。依据原v2主结构加本轮明确设计要求；[截图与命令](validation-ui/2026-10-14-group-header/README.md)。新EXE为target/debug/photag-shell-grouping-header.exe，普通用户profile未修改。

## 子文件夹分组根目录优先（2026-10-14）

按用户要求先显示当前目录直属文件，后显示子目录组，保留蓝灰背景。28相关自动测试及19步同进程MCP验证全部/选中根/选中嵌套目录/筛选后无直属文件的顺序和数量。源代码、命令、截图、调试路径拼写修正及边界见[根目录优先验证](validation-ui/2026-10-14-root-first/README.md)。新构建target/debug/photag-shell-root-first.exe。

## 不分组目录浏览与文件夹卡片（2026-10-14）

依用户新指示在原v2/final-v5主结构内增加文件夹卡片，None显示当前目录直属文件/子目录。36相关自动测试通过、1原FFmpeg项ignored；fmt/locked check/独立Shell编译通过。82步最终构建同进程MCP验证网格/列表单双击、上级、空目录、混排框选/图片预览、245文件末页及分组/搜索兼容，另两次只读MCP重启通过，共84步。[命令/截图/报告/边界](validation-ui/2026-10-14-folder-browser/README.md)。独立构建target/debug/photag-shell-folder-browser.exe；未修改普通用户profile或停止原窗口。

## 运行时调试信息插件（2026-10-15）

新增 Cordis-owned `photag-plugin-debug`。主界面底栏提供调试按钮，点击展开可收起的调试抽屉；记录时间、级别、分类、目标、动作、消息、耗时、线程及span关联信息。支持分类/级别/关键词筛选、分页和页长、选中详情、暂停查看/跟随最新及清空。插件订阅应用 tracing/log 与 panic hook，覆盖主要插件操作和数据库/索引耗时；MCP提供共享状态读取与受变更保护的控制接口。每 crate 的 `PORTS.md` 和源行为映射表已更新。

静态验证：Rust1.97.1 MSVC；`cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-debug-panel.exe` 通过；`cargo +1.97.1 test --locked -p photag-plugin-debug -p photag-ui-bridge -j 2` 通过10项；本轮新增 Rust 文件 `rustfmt --edition 2024 --check ...` 通过。当前全工作区 `cargo +1.97.1 fmt --all -- --check` 受到同一工作区中并行的 Gallery/Navigation 图标改动格式差异影响，未对这些文件执行自动格式化，以免覆盖无关改动。

Windows同进程 PhoTag MCP隔离运行验证46步，并以最终窗口构建补验原生分类菜单的滚动边界。验证真实索引记录与110ms扫描耗时、真实文件改名失败及错误详情、过滤/分页/暂停/跟随/清空、最小窗口和抽屉打开关闭。准确命令、报告、日志、截图、隔离配置及限制见[调试插件运行证据](validation-ui/2026-10-14-debug-panel/README.md)。本轮设计沿用 `docs/reference-ui/final-v5/00-main-library.png` 的底部工作区结构，仅增加底栏入口和向上展开的调试抽屉。

边界：缓冲区最多保留最近10000条、进程重启后清空；不替代系统/原生组件日志或崩溃转储，后台线程的span父子上下文并非处处传播。为避免重复捕获自身的查询响应，RMCP传输层DEBUG/TRACE载荷不纳入记录；MCP应用动作和拒绝请求仍有记录。未加入磁盘日志导出或SQL正文捕获。

## 全界面 gpui-kit 图标统一（2026-10-15）

界面内的状态、操作、媒体类型、评分、导航和空状态图标统一改为 gpui-kit 图标目录；图标尺寸通过 UI Bridge 主题 `small_icon` / `icon` / `large_icon` / `empty_state_icon` 配置。标签图标保留旧数据库值兼容解析，旧 MDL2 字符与未知表情显示对应 gpui-kit 图标，不再直接绘制字符。替换图库月份分组的手绘箭头和设置复选框的手绘勾号。界面结构与控制项不变，本次未新增或删减设计状态。

静态验证环境：Windows MSVC Rust 1.97.1。`cargo +1.97.1 fmt --all -- --check` 通过；`cargo +1.97.1 check --locked -p photag-shell -j 2` 通过。扫描 crates Rust 源码确认 `Icon::new` 仅留在主题尺寸 helper，未发现 PathBuilder、SVG 图标绘制或 MDL2 主题字体调用。当前会话没有 PhoTag MCP 工具入口，因此未启动同进程窗口或做运行时 UI 检查；界面呈现留待项目 PhoTag MCP 可用时验证。

### 调试日志重复刷新问题修复（2026-10-15）

用户截图中的重复 `database/with_reader` 来自图库虚拟列表重绘：列表每次构建行都调用 `media_paths_by_ids` 重查当前多选项的拖拽路径；右侧 Inspector 对不在当前200行数据窗口中的已选媒体也曾在渲染函数内同步查 `media_by_id`。已选媒体滚出窗口后选择仍保留，这是跨滚动/多选行为，右侧信息仍应继续显示。现改为：Inspector 在选中媒体缺少于已加载窗口时后台读取一次媒体与标签，完成后缓存并通知刷新；多选拖拽路径在全部路径不在已加载窗口时后台读取一次，按选择ID缓存。绘制过程仅读取快照/缓存，不再查数据库。选中状态仍决定右侧显示，当前虚拟列表窗口不再决定能否显示。

按同进程 PhoTag MCP 回归：profile `target/mcp-inspector-async-final` 注入1000条隔离虚拟记录，选择排序末尾 ID1（首屏窗口实际显示virtual-000999起），右侧展示 `virtual-000000.jpg`、缩略图及基本信息。两次相隔2.5秒的 `get_debug_info` 均为 `latest_id=125,total=17`，重复读取数没有继续增加。最终报告/操作计划/真实窗口截图见[选择与日志回归证据](validation-ui/2026-10-14-debug-panel/README.md#follow-up-selected-inspector-and-render-triggered-database-reads)。相关 Gallery19、i18n4、Debug5测试通过，独立 Shell 编译和本轮Rust格式化通过。

### 全项目渲染读写审计与预览首帧优化（2026-10-15）

按用户反馈审计所有 feature crates 的数据库访问点并沿 Codebase Graph / `src/view` 调用链区分被动渲染与显式动作。审计发现同类重复绘制读取集中于 Gallery/Preview/Playback 的标签、选中媒体、预览信息和胶片带；导航标签关联及 Gallery 键盘、文件操作中的查询由明确动作触发。重复查询已改为快照复用，或以媒体 ID/路径为键的后台一次加载缓存；无效迟到结果按当前身份丢弃。调试span保留，修的是调用路径，不隐藏日志。项目级审计范围、通用规则和尚存的非身份变换首建文件生成边界见[UI渲染性能审计](ui-render-performance-audit-2026-10-15.md)。

Gallery 双击预览现在直接使用当前已加载 `MediaItem`，当前200行胶片带立即入视图，完整过滤结果的有序路径在后台解析；预览路径和胶片带不再逐条同步检查文件。预览尺寸、文件属性、EXIF/视频详情、索引详情和变换图像异步加载，方向信息按路径缓存；变换完成前先显示原图。图库选中文件详情也将打开次数与文件属性合并为一次后台读取。Preview标签弹窗仅在打开时加载，Playback 标签面板也按活动媒体缓存。MCP新增 `control_gallery action=preview`，调用同一 Gallery 预览处理函数以覆盖相同的数据/胶片带路径。

静态验证：Windows MSVC Rust 1.97.1；`cargo +1.97.1 fmt --all -- --check` 通过；`cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-preview -p photag-plugin-media-playback -j 2` 通过（Gallery19、Playback3、Preview0）；`cargo +1.97.1 check --locked -p photag-plugin-mcp -p photag-plugin-gallery -j 2` 通过；独立 Shell 编译命令 `cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-render-audit.exe` 通过。

最终版本运行时通过同进程 PhoTag MCP、隔离 profile `target/mcp-render-audit-final`、fixture目录 `test-data/photag-library` 扫描12项。`control_gallery preview(media_id=5)`走 Gallery预览路径，真实计时 `preview_media` 约3.84 ms；打开信息浮层后，媒体详情仅发生一次后台索引查询。窗口截图见 `target/mcp-render-audit-final/screenshots/gallery-preview-info.png`。检查标签弹窗时，保持关闭产生0次 Tagging `for_media/tag_tree` 读取；打开后每项恰好读取一次（各自一对 start/completed span），之后两次相隔2.5秒的读取均为 `latest_id=152,total=77`，没有持续增加。标签弹窗截图见 `target/mcp-render-audit-final/screenshots/preview-tags-open.png`。该试验覆盖小型本地样例图和服务调用，不代表大型相机RAW、网络盘、媒体损坏/拔除时文件错误展示或指针双击命中测试；这些路径仍需相应实机验证。

### 图片持续刷新与实际首帧/全图延迟补验（2026-10-15）

上面的3.84 ms仅为打开处理函数耗时，不能代表图片显示速度。继续追查发现 AI 面板每150 ms无条件notify、静止缩放仍推进revision，以及调试抽屉观察a11y日志形成重绘反馈。现均改为状态变化才推进绘制，a11y记录仍保留。图片阶段按原生全图/渐进图/图库缩略图就绪状态选择，取消500 ms过期规则；普通EXIF方向由原生加载器处理，移除重复全尺寸PNG；开发构建优化解码及像素转换热路径。

标准 `target/debug/photag-shell.exe` 已重新完整构建。Windows11/Rust1.97.1同进程MCP19步、真实GPUI双击事件、Debug展开且原生无障碍启用：6000×4000 JPEG缩略图31.95 ms/全图400.30 ms，同图优化前全图9806.41 ms；3200×2400 TIFF为26.18/74.58 ms；同进程JPEG重开20.24 ms。150%缩放及淡出结束后，两次间隔5秒的render_count均37；两次间隔5秒的日志total/latest_id均167。fmt、locked shell check/build、相关30项测试通过。设计沿用v2/final-v5和现有浮动预览布局，未新增缺失设计控件。详见[完整证据及计时边界](validation-ui/2026-10-15-preview-feedback/README.md)：这是绘制提交时机，未作GPU物理呈现、全部格式或网络盘性能保证。

### 全量静止刷新实测与公共机制统一（2026-10-15）

真实同进程 MCP 首轮审计发现旧静态审计遗漏的 Settings 快捷键/AI、统计、批量匹配
绘制时数据库读取，扩展测试又复现了 EXIF 在缓存检查前读路径，以及静止鼠标事件
重复重启预览淡出的错误。已将这些根因修复，并按用户要求把所有 GPUI 定时轮询
统一到 UI Bridge `refresh::poll`，用 `ChangeGate` 进行变化检测；被动数据读取统一
使用 `ReadModel`，覆盖 Gallery/Preview/Playback/Settings，缓存成功及失败、合并
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
全11文件SHA256/大小/mtime前后一致；未改正常PhoTag缓存、DB和原始媒体。
完整原始报告和截图仅在target/mcp-real-folder-empty-20261015保留，避免把真实
用户图片复制到版本化文档；[测量表、复现命令与边界](validation-ui/2026-10-15-real-folder/README.md)。

### 图片打开前三项优化完成（2026-10-15）

沿用final-v5/01-preview-playback和已接受的共享浮动框架，无布局/设计控件变更。
仅增加dev解码依赖优化、图库缩略图复用及按格式/需求的渐进预览调度与请求取消。
最终标准photag-shell.exe通过同进程MCP实际目录冷应用缓存14/14绘制：目标PNG
663.3→78.9ms、72MP PNG8045.4→422.1ms，重开17–19ms；截图核对图像、工具栏、
胶片栏及选中卡片均可见。另测24MP/坏图、前后导航、缩放旋转fit及快速开关；
坏图render12→12、raw日志701→701。workspace94通过/3忽略，最终Preview11通过，
locked构建/格式/刷新归属检查通过。精确复现、构建hash、初次测试脚本失败原因和
未验证的指针/格式/写权限范围见[优化证据](validation-ui/2026-10-15-preview-optimization/README.md)。

### 启动窗口尺寸自适应（2026-10-15）

沿用已接受的dev-baseline-2026-10-14布局，未改任何面板结构。启动窗口不再固定
1280×800，改为取主屏可用工作区（`visible_bounds`，已排除任务栏）的85%，下限
1280×800、上限不超过工作区本身（`photag-shell/src/main.rs::startup_window_extent`，
常量在`photag-ui-bridge/src/theme.rs`）。`PhoTagShell.viewport_width`初值改用实际
启动宽度，不再写死1280。MCP `get_ui_state.window_status`新增只读`bounds`
（`GetWindowRect`物理像素），用于核对真实窗口尺寸。

本机主屏2560×1440、工作区2560×1392、窗口缩放150%（`GetDpiForWindow`=144；本条原记为100%有误，
2026-10-16 的 `window_status.geometry` 探针已给出确定值）。photag-shell.exe（locked构建）
经同进程MCP实测`window_status.bounds`为3286×1831物理像素（含Windows不可见
缩放边框，即GPUI逻辑约2176×1203），窗口居中、标题栏/侧栏/筛选行/预览栏/
状态栏均按现有设计正常绘制，见`target/mcp-windowsize/screenshots/main.png`。
`cargo +1.97.1 fmt --all -- --check`、`check -p photag-shell`、
`test -p photag-shell --bin photag-shell`（4项尺寸推导用例）与
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
`cargo +1.97.1 fmt --all -- --check`、`test -p photag-plugin-navigation`（7 项）、
`build -p photag-shell`、`node scripts/check-ui-refresh.cjs` 均通过。
截图、复现脚本、命令与仍未验证项（原生文件夹选择器、拖拽放置、编辑器键盘输入、watcher 对新区块
联动、非 150% 缩放）见[侧栏分区证据](validation-ui/2026-10-16-sidebar-sections/README.md)。


## 2026-10-16 浏览模式与其他 AI 侧栏改动审查

在 1177e4e 与其后的 Explorer 分区改动上继续修复，保留新布局。发现并处理快速访问键不一致、箭头展开状态重复、别名覆盖、分组状态未发布/保存、同步盘容量校验、被动目录 IO、保存旧版本覆盖/错误吞没、编辑器错误污染导航快照、虚假目录计数等问题。另补浏览模式未收藏目录重启恢复、会话自身监听循环、持续分块使快照交付饥饿三个遗漏。具体复现/修复映射、设计对照与真实 GPUI 点击截图见 [审查报告](validation-ui/2026-10-16-sidebar-review/README.md)。

Windows/150% DPI，Rust 1.97.1 MSVC，locked/-j2：fmt/check/build、UI refresh guard 通过；13 个相关 crate 65 测试通过。全新隔离 MCP profile 验证侧栏编辑/失败重试/静止日志、稳定后重启、四次箭头 pointer、浏览自身 profile 和真实文件事件。十万真实文件扩展回归（含枚举结束前部分结果断言）、标签重启和基本缺失重链回归通过。最终一轮首批发布 793ms（8224 项，仍在加载），完整 13332ms，峰值工作集 247332864 bytes；为本机 debug 热缓存样本，不是像素首帧或通用性能保证。持久 media/media_tags/tag_previews 各 1 行，未保存十万普通浏览记录。原生选择器/完整键盘焦点/断网和可移动盘等未验项详见报告；不以静态测试代替运行验收。
