# PhoTag v5 开发交接：按任务范围执行的开发与验证文档

更新时间：2026-10-14，Asia/Shanghai。工作目录：`F:\AI\geren\pho_cordis`。
用户因额度不足要求暂停实现、整理交接。本文件是接手时的当前状态入口；
`interface-redesign-plan.md` 仍提供设计和阶段细节，但其中早期交接快照已经过时。

## 1. 接手目标与用户最后确认的要求

### 执行规则：只执行用户本轮明确分配的任务范围

**每次切换模型或交接任务时，用户会明确指定执行哪一部分。执行者不应根据模型大小、
身份或本文件标题推断职责；只完成用户本轮指定的章节/任务，完成后汇报并等待下一项分配。**

**职责由用户每轮通过任务范围分配，不由模型身份决定。**

- 用户分配 C0–C4 编码任务时：只实施所分配的代码任务并更新实现记录；本轮不运行格式、
  编译、测试、应用或 MCP。允许编写测试源码，但不执行测试。交付标注“代码已编写，未验证”。
- 用户分配 V1–V4 验证任务时：执行所分配的验证步骤；运行时调试、界面测试和截图必须使用
  PhoTag 插件 MCP。根据结果修复或记录差异，按本轮范围完成。
- 用户同时分配编码与验证时：按明确范围执行。未分配的任务留待用户后续指定。
- 不得把未执行的验证写成通过，也不得用 TODO、空 handler 或假实现代替功能。
- 本文件中的编码清单和验证清单均供用户逐次分配；章节标题不代表特定模型角色。


1. 编码任务按用户 v5 参考图实现界面；验证任务通过真实截图迭代验收。
2. 执行运行时调试、界面测试、截图时，**必须使用项目自己的 PhoTag 插件 MCP。**
   不用通用 computer-use、OS 鼠标/键盘注入或另开的无 MCP 窗口替代。
   缺少控制能力时扩展 `photag-plugin-mcp` 和功能插件的公开请求接口。
3. 启动时的自动更新弹窗必须被屏蔽；后台检查可保留，设置页手动检查、下载、
   校验和明确安装入口保留。此项已通过真实 MCP 验证，不要改回自动弹窗。
4. 用户曾授权用 Luna 子智能体开发其他任务、由主线程对照界面。本轮三个 Luna
   子智能体已全部停止；后续是否拆分任务由用户每轮指定。
5. 中文文案经 UI Bridge `tr/tr_args` 输出并登记两个 Fluent 目录；保留用户数据、
   Cordis 生命周期、GPUI Kit、既有服务和后端行为。不要恢复 egui 风格。

先读 `AGENTS.md`。设计依据依次是用户本次要求、`final-v5` 图与 concepts notes；
`vendor/photag-source` 用于核对数据/后端行为，不是当前外观规格。

## 2. 最重要的状态：工作树和二进制不是同一版本

| 对象 | 当前真实状态 |
|---|---|
| Git 工作树 | 大量未提交修改与新增文件，包含用户交接前的修改；禁止 reset/clean 或整批覆盖 |
| 当前 `cargo fmt --all -- --check` | 已于交接最后运行，使用 `+1.97.1`，通过 |
| 当前 Shell check | **失败：19 个错误**，集中于新批量操作/标签分配模块；尚未修复 |
| 完整编译日志 | `target/development-handoff/check-current.log` |
| 最后成功构建并实际截图的版本 | `target/debug/photag-shell-v5-iteration.exe`，2026-10-13 23:55:42 构建 |
| 上述二进制与现在源码的关系 | **落后于最新工作树**，不含最后的日期日历、设置正文尺寸修复、第二轮侧栏、04/06 最新布局 |
| 较早二进制 | `photag-shell-s2.exe` / `photag-shell-s2b.exe`；更旧，不作为当前验收版本 |
| 新运行实例 | 本轮启动的 MCP 子进程已全部停止，截图和隔离目录保留 |
| 既有普通实例 | 曾有 PID 14400 占用 `target/debug/photag-shell.exe`；没有停止它。验证负责人运行前需重新检查 PID，不能认定仍是同一个进程 |

普通 `cargo build` 曾因旧 EXE 被占用报 OS error 5。不是 Rust/项目不兼容，
不要擅自结束用户实例；用独立输出名构建，详见第 4 节。

## 3. 当前编译阻塞：先修这些，不要重新设计

最后诊断有 19 个错误，但多数是同一个 trait 缺失的连锁错误。

### 3.1 `view/bulk_actions.rs`

- `matching_results` 返回 `Vec<MediaItem>`，文件没有导入该类型。
  将现有查询导入扩为 `use photag_plugin_database::db::queries::{MediaItem, MediaQuery};`。
- 305/326 附近调用 `section_title`，实现位于子模块 `bulk_actions/filters.rs`，
  当前私有且没导入。将它改为 `pub(super) fn section_title(...)`，并加入父模块
  `use filters::{filter_controls, match_results_list, section_title};`。
- `Input` 导入目前 unused，检查是否仍使用，确认后移除。

### 3.2 `view/bulk_actions/filters.rs`

- `font_semibold` / `h_flex` / `v_flex` 等报不存在：导入
  `use gpui_kit::base::StyledExt as _;`。不要替换这些布局方法或删控件。
- 若后续 `ghost/primary/outline` 报错，导入 `button::ButtonVariants`；
  `overflow_y_scrollbar` 需要 `scroll::ScrollableElement`；
  `when/when_some` 需要 `prelude::FluentBuilder`。只补实际需要的 trait。

### 3.3 `view/tagging_picker.rs`

- 91 附近 `overflow_y_scrollbar` 缺 trait，导入
  `use gpui_kit::component::scroll::ScrollableElement;`。

若本轮分配 C0 编码任务，按现有日志修复上述代码问题，不运行编译器。交付时说明
“已按日志修改，未编译确认”。若本轮分配验证任务，再 check Gallery 和 Shell 并处理
可能被遮挡的后续错误。

## 4. Windows 工具链与验证命令（分配验证任务时执行）

已验证安装：stable 1.94.1（默认，不满足 MSRV）、1.96.0、1.97.1 Windows MSVC。
执行构建验证任务时必须显式选择已安装的 1.97.1；不降 MSRV，不改全局默认，不重复裸 `cargo`。
仅在用户本轮分配相应验证任务时运行本节命令。

```powershell
Set-Location F:\AI\geren\pho_cordis
rustup toolchain list
rustc +1.97.1 --version
cargo +1.97.1 fmt --all
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-plugin-gallery -j 2
cargo +1.97.1 check --locked -p photag-shell -j 2
```

成功后构建新名字（不要拿旧二进制验证新改动）：

```powershell
cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-handoff.exe
```

该命令已以其他输出名实测成功；`-o`/`--out-dir` 输出警告是可预期的。
若普通 EXE 没占用，也可以正常 `build --locked -p photag-shell -j 2`。
不要并行启动多个 Cargo 争用同一个 target 锁；保留增量编译。

相关测试（编译修复后一次运行，失败再定位，不反复跑无关全量）：

```powershell
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-navigation -p photag-plugin-i18n -p photag-ui-bridge -p photag-plugin-preview -p photag-plugin-media-playback -p photag-plugin-settings -p photag-plugin-metadata-export -p photag-plugin-mcp -j 2
```

历史证据：S2 较早版本 Gallery 16、Navigation 4、i18n 4、UI Bridge 3 共 27 项通过。
这不覆盖最新布局/新请求、最新批量筛选和音频扩展。Settings/MCP 在当时命令中
没有单元测试，不能把它们的“0 tests”说成界面测试通过。

Rust 文件尽量 <500、必须 <800 行。交接时 Gallery `view.rs` **791 行**、
Navigation `view.rs` **784 行**、Preview `lib.rs` 接近 800，后续扩展先拆模块。

## 5. MCP 客户端（分配运行时验证任务时使用）

新增辅助脚本：`scripts/photag-mcp-debug.cjs`。Node 标准库，无安装依赖。
脚本**确实通过 stdio 调用 PhoTag 插件 MCP**；不执行 OS 输入注入。
支持保持同一个窗口的交互模式，也支持顺序 JSON 计划、状态轮询、MCP 截图保存。

已验证：`node --check` 通过；使用最后成功的 EXE，在新隔离目录中完成
initialize → get_ui_state → screenshot_window，退出码 0。
这只是客户端的 smoke test，不代表当前 Rust 工作树可运行。

### 5.1 交互模式：分配 MCP 验证任务时使用

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-handoff.exe --profile target/mcp-handoff --interactive --allow-mutations
```

shell 工具会返回运行 session ID。使用其 stdin 写入**每行一个 JSON**，在同一个
MCP 子进程内连续调试。不是另启动应用再尝试 attach。

```json
{"method":"tools/list"}
{"tool":"get_ui_state","capture":"main","wait_ms":400}
{"tool":"dev_test_scan_fixture"}
{"tool":"dev_test_status","until":{"field":"running","equals":false,"attempts":30,"interval_ms":500}}
{"tool":"search_media","arguments":{"limit":10}}
{"tool":"control_ui","arguments":{"action":"select_media","value":"8"},"wait_ms":400,"capture":"selected"}
{"tool":"control_gallery","arguments":{"action":"popover","value":"extensions"},"wait_ms":400,"capture":"extensions"}
{"tool":"control_gallery","arguments":{"action":"popover","value":"close"},"wait_ms":300}
```

`8` 是旧隔离库里的 small.png 示例 ID；**新库先 search_media，使用它返回的 ID**。
`exit` 单独一行结束客户端，仅停止它自己启动的子进程。

输出目录：`<profile>/screenshots/*.png`、`report.json`、`stderr.log`。
用 `view_image` 查看保存的原生截图，和参考图比较，再改代码、重建、重拍。
截图始终来自 MCP `screenshot_window`，不是编辑器预览或生成效果图。

### 5.2 顺序计划模式

把上述对象写成 JSON 数组到 `target/mcp-plan.json`，执行：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-handoff.exe --profile target/mcp-handoff --plan target/mcp-plan.json --allow-mutations
```

没有 plan/interactive 时输出完整工具 schema。首次先读 schema，不猜参数。
默认 mutations 关闭；只有 `--allow-mutations` 才设置环境变量为 1。
扫描隔离样例、修改 UI 状态和测试标签时按需开启。不要对真实图库测试删除/移动。
隔离目录必须位于工作区 target 下；脚本不覆盖已存在的 settings/language 文件。
设置文件在 `<profile>/settings.json`，不是 `<profile>/local/PhoTag/settings.json`。
后者曾导致启动检查配置没有生效，已经定位清楚。

旧 `scripts/validate-ui-mcp.cjs` 有历史 UI/动画流程，默认 EXE 与部分断言针对旧界面；
不要原样运行后把失败当新 UI 业务回归。优先使用新脚本的小计划。

## 6. 当前 MCP 与公开接口：哪些已实现，哪些只是计划

### 已实现于最新源码，且最后成功版本已包含

- `get_ui_state`：`inspector_open`、`tools_open`、`gallery_popover`、
  `update_checking/update_available/update_offer_open`、`gallery` 真实渲染状态、
  Settings 页、语言、卡片动画与生产扫描进度。
- `gallery` 包含 search/media_type/sort、扩展名/标签包含排除、日期、评分、
  selected_media_ids、matched_count、group_by_month、inspector_sections。
- `control_ui`：原设置/语言/选择等动作，加 `inspector_open`、`inspector_close`、
  `settings_check_updates`。手动更新检查是异步请求。
- `control_gallery`：`popover/search/media_type/sort/rating/extension/tag/tag_mode/`
  `extension_mode/date_from/date_to/clear_filters/inspector_section/tag_picker/`
  `month_grouping/tools_open/tools_close`。具体参数以 tools/list schema 为准。
- `control_navigation`：`create/edit/delete/merge/close`，后面三种目标动作使用
  `tag_id`。**只打开真实弹窗，不提交删除/合并操作**。
- `GalleryUiRequest` 位于新 `gallery/src/ui_requests.rs`，消费在 `view/render.rs`；
  `GalleryUiSnapshot` 由真实 Gallery render 发布，不是另查 DB 假装界面状态。
- `NavigationService::request_ui(NavigationUiRequest)`，enum 为
  `OpenCreate/OpenEdit(i64)/OpenDelete(i64)/OpenMerge(i64)/CloseModal`，有标签存在性/
  队列 busy 错误。由同一个 SidebarView 消费。

`Popover::open` 与 `on_open_change` 共同绑定 `surfaces.value("gallery.popover")`；
真实按钮和 MCP 使用同一开关，Gallery poller检测变化后 notify。
这是实际控件状态验证，但仍不能声称验证了 OS 指针点击传播。

### 尚未实现：不要误以为已经有

- `MediaPlaybackService::open_media(path, Option<i64>)`：**没有落地**。
- `PlaybackSnapshot.media_id`：**没有落地**。
- Playback Tags/More 服务接线：**没有落地**。
- `tools_write.rs::open_media` 的 `audio` 分支仍直接返回“不支持音频预览”；
  即使 Playback 已扩展 audio-only，MCP 当前也进不去。`v5-audio-first.png`
  是主界面，不是成功的音频播放器截图。

MCP 工具可能返回 text 内 JSON `{error:...}` 而非 `isError=true`。新客户端已同时
检查这两种错误；旧聊天内临时客户端曾只看 isError，因此音频 open 的错误一度被漏看。

## 7. 设计稿与代码位置对应

| 稿/任务 | 主要代码 | 当前实现与下一步 |
|---|---|---|
| 00 主界面 | Shell `shell.rs/main.rs`；Gallery `filter_row/inspector/card/render`；Navigation `view/render` | 已有骨架/检查器/卡片/面包屑，继续用当前新版本截图验收 |
| 01 图片预览 | Preview `view/render/toolbar/filmstrip/overlays` | 有新顶栏/More/slider，但信息卡、舞台、工具条尚不一致 |
| 01 视频/音频 | Playback `view.rs/lib.rs/ffmpeg.rs` | modal 与 audio-only 扩展有代码；Tags/More、ID、MCP audio 路由缺 |
| 02/03 设置 | Settings `view.rs/tabs.rs/ai_groups.rs/updates_page.rs` | 六项纵向导航；首轮正文空白，最后尺寸修复未重拍 |
| 04 标签管理 | Navigation `tag_modal/tag_picker/tag_actions` | create/edit/rename/delete/merge/move、层级 parent/target、Esc；未截图完整验收 |
| 04 标签分配 | Gallery `tagging_picker.rs` | 最新430左+220预览、树和recent；当前缺trait导致编译失败 |
| 05 筛选/菜单 | Gallery `filter_row`、`card_menu.rs`、`regions.rs` | 六个真实popover已截；日期溢出已写修复，未重拍；native context menu已接 |
| 06 批量/文件 | Gallery `bulk_actions.rs/bulk_actions/filters.rs`、`file_actions/selection_panel` | 最新两栏与AND过滤未编译；单文件确认/rename完整稿面仍需检查 |
| 07 统计/导出/更新 | Gallery `statistics.rs`、Export `view.rs`、Settings `updates*` | 有分区/状态卡；统计、导出尚未新截图；更新启动静默已验 |
| 08 空态/扫描/错误 | Gallery `empty_state.rs/regions.rs`、Navigation 扫描、Tagging writeback | 空态代码有；失败卡/重试/取消完整稿面仍缺验证与部分实现 |

所有设计图在 `docs/reference-ui/final-v5/`。配套文字：
`concepts/photag-reference-layout-v5-2026-10-13.md` 与
`concepts/photag-v5-companion-interfaces-2026-10-13.md`。
不能把示例山水图、文件名、数据、版本号和生成图的文字瑕疵当业务要求。
卡片保持定义的正方形，不因图中风景横幅改成未经验证的尺寸。

## 8. 已有真实截图：逐张看，不要凭文档猜

目录 `target/mcp-s2-validation/`，均来自本机同进程 MCP。

| 截图 | 已观察事实/限制 |
|---|---|
| `01-startup-quiet.png` | 启动已发现新版但没弹窗；对应较早 S2b |
| `04-manual-update-offer.png` | MCP 手动检查后弹窗仍可打开；没有下载/安装 |
| `v5-main-first.png` / `v5-main-selected.png` | 全量图标、一行筛选、不同高度虚拟列表已显示；仍旧侧栏常显工具，是第二轮修复前 |
| `v5-filter-type/tags/extensions/date/rating/sort.png` | 六个真实 Popover 可由 MCP 打开并截图 |
| `v5-filter-date.png` | 中文 From/To 撑出边界，Select date 英文，无内嵌月历；源码已写下一版修复，未重拍 |
| `v5-settings-appearance/toolbar/shortcuts/tags/ai/updates.png` | 只见nav/header/footer，正文空白且 Gallery 透出；是失败证据，不是验收通过 |
| `v5-preview-info.png` | 新顶栏存在；信息卡仍旧纵向长表、舞台半透、底部浮动旧工具条 |
| `v5-audio-first.png` | MCP audio 路由报错，画面仍为主界面，不能称播放已验 |
| `00-empty.png` | 旧更新弹窗遮挡，不能作为空库验收证据 |

`report.json` 主要记录较早更新/检查器验证，不是全部最新截图的完整报告。
新客户端将每次计划的状态和路径记录在指定 profile/report.json。

## 9. 最后成功截图之后已落盘的修改与待验事项

本节“截图/重拍/运行核对”属于验证任务；执行者仅在用户分配相应任务时执行。

### 9.1 Navigation 第二轮（已格式化，尚未新截图）

- All files 放到 Folders header 上方；Folders header 使用 ghost Plus。
- 路径添加与刷新收进 hover `⋯`，文件夹/子目录旧 Open/Plus 不再常显。
- Tags header 改 Search+Create；搜索/模式/modifier/clear 面板默认折叠。
- 标签行色点、计数、选中条/填充；动作菜单保留完整已有操作。
- breadcrumb、sidebar、modal 共享同一个 Entity；不要另造状态写者。

### 9.2 Settings 正文为空修复（源码已写，尚未重拍）

- 只改 `settings/src/view.rs`：card/header/footer/content 改实色 SURFACE_MODAL，
  左 rail 为 SURFACE_BASE；为 body、rail、divider、scroll viewport 设置确定尺寸。
- 先截图六页，确认真实控件可见。不要以格式检查通过就跳过这一步。

### 9.3 Gallery 日期弹层（源码已写，尚未重拍）

- `view.rs` 新增 `date_calendar: Entity<CalendarState>`，初始化为 Date::Range。
- `filter_row::subscribe_calendar` 订阅 CalendarEvent，实际更新 filter 和两个 DatePicker。
- 弹层304宽；两列标签在日期框上方；新占位 `Select date` 经过 tr；内嵌 Calendar。
- clear filters/date 会同步 calendar；MCP DateBound 的同步代码需要复核。
- Shell 还新增 GPUI Kit locale 与 I18nService 的初始化/切换同步。
- 核对 session 已有日期是否同步进新 Calendar：当前初始化 Range(None,None)，
  不要让旧保存范围在日历里丢失高亮；日历月份显示与中文 locale 也要截图。

### 9.4 Gallery 04/06（当前19错的来源）

- `tagging_picker.rs`：层级复选树，recent_tags 的真实名称，真实选中缩略/文件名；
  左430、右220；regions 中容器690，已写中心 scrim。
- `bulk_actions.rs + filters.rs`：动作/来源/AND筛选左栏、真实匹配预览右栏、
  取消/继续；年龄与大小在 Advanced；原四种scope/native目的地/确认ID流程保留。
- 新 Gallery 字段：`bulk_media_type: MediaTypeFilter`（All）、
  `bulk_tag_ids: Vec<i64>`、`bulk_extension_exclude: bool`，reset_inputs 清理它们。
- 风险：跨页 CurrentSelection 的预览仍可能只从 snapshot.media 找，200条窗口外
  的选中项需调用现有 `GalleryService::media_by_id` 补齐，不能悄悄显示部分结果。
- 动作/来源按钮需核对明显选中态。Older-than/newer-than 是距今更久/更近的天数
  下限/上限约束；目前 `Minimum / maximum age` 文案容易颠倒，按实际字段逻辑校对。
- `regions.rs` bulk 外框860、统计外框800；需要在960最小窗口下 clamp 并验布局。

## 10. 编码任务清单 C0–C4

只有用户本轮分配 C0–C4 时执行对应编码项。按用户指定范围逐项完成；若用户明确分配
完整编码阶段，则按 C0 → C4。编码任务内可以阅读已有截图与日志，但不执行验证命令、启动应用或运行 MCP。

### C0：修复已记录问题并补齐文案

- [ ] 按第3节修复19个错误对应的类型、可见性和 trait 导入，依据现有诊断修改。
- [ ] 接近800行的文件扩展前拆分，不删除已有功能压行数。
- [ ] 登记第11节文案，补全 keys.json/en.ftl/zh-CN.ftl；必要时调整测试源码的参数样本，
  **不运行测试**。
- [ ] 记录修改位置及可能需要后续验证任务重点检查的 API/借用疑点。

### C1：补齐00/02/03/05已知布局代码

- [ ] 对照00图与旧截图，保持侧栏顺序、默认折叠、图标/面包屑、一行筛选、网格间距、
  检查器各区域和状态栏；阅读最后侧栏修复代码，补遗漏的真实处理器。
- [ ] 对照02/03，完善设置确定尺寸、实色背景、导航选中、独立滚动正文与即时保存页脚。
  旧截图正文空白；最后修复是否有效留待后续验证任务确认。
- [ ] 对照05，补齐六弹层包含/排除、单选关闭/多选保留、日期月历、评分和清除代码。
  将 session 已存日期同步到新 Calendar，维护日期框/日历/实际筛选一致的数据流。

### C2：完成04/06/07功能代码

- [ ] 阅读并完善 Navigation create/edit/delete/merge/parent/move 模态及公开请求消费。
- [ ] 完成标签分配层级/近期标签/真实预览/按钮；跨页选中预览通过已有服务查询补齐。
- [ ] 完成06批量两栏、AND过滤、真实匹配列表、来源/动作选中态、取消/继续、确认绑定ID、
  原生目的地流程；现有scope和高级过滤保留。
- [ ] 扩展真实公开 MCP 请求，以便验证任务配置 bulk 控件、打开媒体菜单并读取状态；
  请求应调用与 UI 相同的功能方法，不创建假界面状态。
- [ ] 完成07统计/导出准备、进行、成功、失败状态代码。分类容量无真实数据来源时省略，
  不用前200条样本估算。若 export_metadata 不驱动可见 ExportState，补正确公开入口。

### C3：完成01图片/视频/音频代码

- [ ] Preview舞台改实色 SURFACE_EXTREME；信息/EXIF采用紧凑两列和可折叠组，保留真实值；
  底部固定工具条+filmstrip，已有文件/变换动作收到More，保持设置控制与真实处理器。
- [ ] Playback增加 `snapshot.media_id`、`open_media(path, Option<i64>)`；原 `open(path)`
  兼容委托None。这些接口尚未实现，需要编写。
- [ ] Gallery/Preview/MCP传真实indexed ID；MCP audio和video共同走Playback新入口。
- [ ] Playback从Cordis捕获可选Tagging/FileOperations公开服务，写真实Tags/More功能；
  不依赖Gallery私有模块，不引入反向依赖循环。
- [ ] 按新增依赖调整Shell stop逆序：消费者Playback先停，Tagging后停。
- [ ] 必要时补音频解析/暂停seek等测试源码，不运行；标注音频-only是相对source的扩展。

### C4：完成08反馈与代码交付

- [ ] 完成空库/无结果、添加文件夹、生产扫描进度/取消、失败重扫、writeback重试/忽略
  的真实代码。公开重试API不足时先实现API，再绑定按钮。
- [ ] 编写默认/最小窗口与侧栏/检查器折叠的布局处理，保留分页和跨页选中行为。
- [ ] 同步各crate PORTS与阶段台账，交付修改文件、功能清单、已知疑点及待验证列表。
  每项实现状态写“代码已写（未验证）”，不能写“验收通过”。

### 验证任务清单 V1–V4（仅执行用户本轮分配的项）

#### V1：格式、构建与自动化测试

- [ ] fmt/check/build/相关tests；生成与最新源码一致的新EXE。

#### V2：参考图与主要界面交互

- [ ] 使用第5节插件MCP客户端，在隔离档案逐稿截图→对照→修改→重拍。
- [ ] 核对六页设置正文、六弹层数据流、标签模态、bulk跨页列表、统计与导出状态。

#### V3：媒体与窗口行为

- [ ] 核对图片/视频/音频、暂停/seek/音量/速率、原生对话框及文件确认机制。
- [ ] 核对960×600与1280×800、折叠、分页/框选/滚动恢复/压力库、扫描/写回失败。

#### V4：更新行为与验证记录

- [ ] 保留“启动自动发现新版但无弹窗、手动检查可打开”的既有验证要求。
- [ ] 在 redesign-validation 记录命令、profile、工具、截图、观察与仍未验项。

代码交付不等于最终验收。最终完成标准仍是结构/控件/状态/文案与设计一致且真实
路径可用；概念图示例照片/数据和系统原生标题栏允许不同。

## 11. 最新尚未登记的中文词条

以下20条在交接最后确实不在 keys.json；按 `msg-` + SHA1(英文原文)前12位
登记 keys.json、en.ftl、zh-CN.ftl。先查重，参数使用 Fluent `{ $count }` 等。

| 英文原文 | 中文建议 |
|---|---|
| Date from / Date to / Select date | 从 / 到 / 选择日期 |
| Add folder by path… / Refresh folders | 按路径添加文件夹… / 刷新文件夹 |
| 3. Match all filters (AND) | 3. 同时满足以下条件（AND） |
| Separate with commas | 用逗号分隔 |
| Advanced filters | 高级筛选 |
| Older / newer days | 距今更久 / 更近天数 |
| Minimum / maximum age | 先核对字段语义，勿直接照抄容易颠倒的词序 |
| Size in MB / Minimum / maximum size | 文件大小（MB）/ 最小 / 最大大小 |
| Matching media ({count}) | 匹配媒体（{count}） |
| No matching media preview is available. | 当前没有可显示的匹配媒体预览。 |
| Continue · {action} {count} | 继续 · {action} {count} |
| Assign tags to {count} selected media | 为 {count} 个选中媒体分配标签 |
| Recently used tags / Selected media | 最近使用的标签 / 选中的媒体 |
| and {count} more selected | 另有 {count} 个已选 |
| Select media to see a preview. | 选择媒体以查看预览。 |

还要扫新文件的静态文案：`1. Choose action`、`2. Choose source` 等 helper接收
的字符串也会送入tr，不能只用正则找 `tr("...")` 而漏掉变量参数。
不要全局修改旧 From/To 中文使评分标签正确却破坏日期标签，按语义用新键。
当前i18n catalog测试固定参数数组已增加error/records/path；新模板引入额外
参数时扩测试样本，不能通过删掉目录校验规避失败。

## 12. 已知实现细节与避免重复踩坑

- 全量Lucide `gpui_kit::assets::IconName` 配 **AllAssets**；旧 Assets只含组件
  子集，曾导致Aperture/标签/关闭等图标编译正常但实际看不见。Shell已改AllAssets。
- Component内置IconName只是子集，Preview曾因此枚举编译失败；用assets枚举。
- Popover content不能捕获 `&mut PopoverState` 到静态事件。关闭用
  `let popover = cx.entity(); ... popover.update(cx, |state,cx| state.dismiss(window,cx))`。
- `.on_click` 需要Stateful元素（div先.id）；`.context_menu`产生wrapper，
  应放在on_drag/child等原元素操作最后，不能对wrapper调用on_drag。
- Gallery由等高uniform_list改为不同高度ListState/list，避免月标题高度导致每行
  间距错误。仍保留200-row服务cache、viewport加载、滚动/键盘/框选；这些需要新
  回归验证，尤其large library。不能因9条样例正常宣称100000条已验。
- 坐标框选改用 ListState.bounds_for_item，只枚举可视范围；检查实际padding/gap。
- 缩略图无缓存的broken-image目前可能持续Spinner，不能把它当已验证失败占位。
- 原字幕/视频时长从真实服务取得；没有duration来源就省略，不抄示例02:14。
- 统计音频数通过load(audio)真实查询，但会带最多200详细行和聚合，注意重复render
  的查询成本；不为了图好看改DB或用样本估算。
- 旧 regions::chip 仍用于部分旧模态/失败按钮；controls.rs已删除。不误删仍用helper。
- 接手前的 Preview `view/tag_picker.rs` 有3行用户既有diff，保持，不把它当新回归。

## 13. 按任务范围交付

根据用户本轮分配的任务范围汇报。执行编码任务时记录：

1. 对应参考稿、修改文件与关键符号。
2. 已编写的功能和数据/事件流，必要的接口变更与i18n登记。
3. 仍缺代码、已知 API/借用/布局疑点，以及需要后续验证的项目。
4. 若本轮未分配验证，声明：**代码已编写，未运行格式/编译/测试/应用/MCP。**

执行验证任务时，记录任务清单、命令/环境、MCP工具、截图结果、差异和未覆盖项。
按分配范围编写真实完整代码，不做空 handler、假数据或 TODO 替代。保存工作树改动，
清楚标记已完成和待后续分配的任务。

以下两段提示词已写明完整任务范围。用户将其分别交给编码执行者和验证执行者；
执行者按工作树和本文件的实际状态自动跳过已完成项并继续未完成项，不依赖模型判断自身身份。

### 编码执行者：自动完成全部未完成的 C0–C4

```text
继续 F:\AI\geren\pho_cordis 的 PhoTag v5 改造。先完整阅读 AGENTS.md 和
docs/development-handoff-2026-10-14.md，再查看当前 Git 工作树、C0–C4 清单、阶段台账、
已有诊断与源码。

本轮完整范围是 C0、C1、C2、C3、C4 的全部编码工作。你不需要判断自己属于哪种模型，
也不需要等待用户逐项分配：请根据当前代码、文档记录和 Git diff 自动判断每一项已完成、
部分完成或未完成，跳过已真实完成的部分，直接继续所有缺失的实现，直到 C0–C4 的编码工作完成。
不能只凭旧勾选框判断；如果勾选状态与代码不符，以实际代码为准并修正文档台账。

执行顺序：
1. C0：修复交接文档记录的 Gallery 构建诊断对应代码；检查文件长度；补齐登记的 Fluent
   keys、en.ftl、zh-CN.ftl 与必要的测试源码样本。
2. C1：完成 00 主界面、02/03 设置页和 05 六个筛选弹层的结构、状态与真实数据流代码。
3. C2：完成 Navigation 标签模态、Gallery 标签分配、批量操作、统计/导出和所需公开 MCP
   控制接口，使用真实服务与跨页数据。
4. C3：完成图片/视频/音频预览与播放界面、带媒体 ID 的播放入口及相关服务依赖。
5. C4：完成空状态、扫描/失败/writeback 反馈、窗口布局与 PORTS/阶段台账。

按每项当前实际缺失继续实现，不要重写已经正确完成的代码，也不要用空 handler、假数据或
TODO 代替功能。保留真实后端行为、Cordis 生命周期、用户已有工作树改动和启动时自动更新静默。
编码过程可以阅读已有截图、源代码、日志和设计参考；本轮只写代码与记录，不运行 Cargo
fmt/check/build/test/clippy，不启动应用，不运行 PhoTag MCP，不截图，不扫描测试数据，也不
声称任何编译或运行结果通过。可以补写测试源码和 MCP 接口，但不要执行它们。

进度自动追踪：每完成一个子项，立即更新本文件 C0–C4 清单和 docs/interface-redesign-plan.md
阶段台账；只在代码确实完成时勾选，并将状态标成“代码已编写，未验证”。若部分完成，写清
剩余符号/文件和阻塞原因，然后继续其他可独立完成的项。完成全部编码范围后，汇总修改文件、
功能和数据流、状态台账、未解决问题及待验证清单；明确声明没有运行任何验证。
```

### 验证执行者：自动完成全部未完成的 V1–V4

```text
继续 F:\AI\geren\pho_cordis 的 PhoTag v5 改造。先完整阅读 AGENTS.md 和
docs/development-handoff-2026-10-14.md，再查看 Git 工作树、C0–C4 实现状态、V1–V4 验证清单、
docs/redesign-validation.md 与现有截图/诊断。

本轮完整范围是 V1、V2、V3、V4 的全部验证工作。根据当前代码、验证记录和真实证据自动判断
每项已通过、失败、部分完成或尚未执行；跳过有当前代码对应证据的已通过项，继续所有未完成项，
直到 V1–V4 全部有结论。旧截图、旧二进制、历史构建结果不能证明最新源码通过。不能只凭勾选框
判断；若记录与证据不符，以实际执行结果为准并修正文档。

执行顺序：
1. V1：按 AGENTS.md 选择已安装的 Rust 1.97.1，执行格式检查、locked Shell 构建及相关测试，
   修复发现的问题并重复必要检查，确保运行的 EXE 对应最新源码。
2. V2：通过项目 PhoTag 插件 MCP，在隔离 profile 中启动与 MCP 共享同一进程的应用；对照
   final-v5 00/02/03/04/05/06/07/08 参考稿检查主界面、设置、筛选弹层、标签/批量/统计流程，
   截图、修复差异并重拍。
3. V3：通过 MCP 检查图片、视频、音频、播放控制、预览、原生对话框、窗口尺寸、折叠、分页、
   框选、滚动恢复、扫描、写回失败等真实行为；覆盖清单列出的尺寸与数据状态。
4. V4：确认启动时发现更新但不弹窗、设置页手动检查仍可用；把命令、工具、profile、截图路径、
   观察结果、失败修复和仍未覆盖项写入 docs/redesign-validation.md 与阶段台账。

运行时调试、交互操作和截图必须使用项目 PhoTag 插件 MCP。优先使用
scripts/photag-mcp-debug.cjs 启动 --mcp 同进程应用和截图；所有测试数据使用隔离 profile。
不要使用通用 computer-use、操作系统输入注入或另一个无 MCP 的应用窗口替代。只有测试确实
需要改变隔离测试数据时才启用 MCP mutations；不要触碰用户正常 PhoTag 数据。

遇到失败不要停在报告：在本轮 V1–V4 范围内修复代码，重新运行受影响的构建/测试或 MCP 流程，
直到通过或确认存在无法由代码解决的外部阻塞。每完成一项就更新验证记录和本文件勾选状态；
勾选必须有当前源码对应的命令或 MCP 截图证据。不能把编译成功说成界面验收，不能隐去失败。
完成后汇总通过项、失败/修正、证据路径、未覆盖状态和外部阻塞。
```
