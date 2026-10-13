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
