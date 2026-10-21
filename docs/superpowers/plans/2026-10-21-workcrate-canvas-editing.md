# WorkCrate 可切换画布编辑（官方语义转发）交接文档

创建 2026-10-21。供接力实现使用：先读本文档的"已实测事实"和"锚点"，再按阶段复选框推进并回写进度台账。
AGENTS.md 的 WorkCrate 边界条款已获用户授权改写（阶段 0）。

## 目标（用户原话拆解）

在页面编辑区把 VectorCraft 的画布编辑能力映射进 FileAtlas 现有 UI 框架：

1. 编辑态与文件态双向同步——在 FileAtlas 里改和在 VectorCraft 里改，结果指向同一个 `.vectorcraft` 文档。
2. 只做六件事：添加图片、链接图片、更新图片、替换图片、输入文本、更改文本文字；外加拖拽移动与缩放。
3. 连接官方程序，前台（GUI 在跑）和后台（GUI 没跑）都要能连；**禁止出现"必须先关掉 GUI / 必须后台运行"这类前置条件**。
4. 编辑语义一律由官方执行，FileAtlas 不自行实现命中测试、选区、变换矩阵或文本排版。

## 已实测事实（本轮探针，全部有输出文件）

探针脚本 `.local/vc-probe3.cjs`、`.local/vc-gesture-steps2.json`；原始输出 `.local/vc-probe-live.txt`、`.local/vc-probe-headless.txt`、`.local/vc-g2.txt`。

| 事实 | headless | 实时 GUI（--connect 127.0.0.1:7979） |
|---|---|---|
| 工具集 | 32 个，与实时模式同名同序 | 同 |
| `pointer_gesture` down/up 命中 | 真选区：`selection:[2]`，`revision` 递增，`selectionBounds` 给出官方 200,200,300×200 | 未写测（用户文档 dirty，见下） |
| 拖拽移动 | 官方 history 追加 `Move`，选区界框位移 = 手势位移 (+100,+60) 精确；官方在 `drag` 采样上移动，只给 `down`+`up` 不移动 | 同路径 |
| 句柄拖拽缩放 | 从 `selectionBounds` 右下角 (300,220) 拖到 (360,260) → history `Scale`，界框 200×120 → 260×160 | 同路径 |
| `object.setBounds` 数值变换 | 生效；默认 `reference` 是中心点，直接给 x/y 会偏移半尺寸，传 `reference:0` 后 x/y 即界框左上角，与叠加层完全对齐 | 同 |
| `text.setText` / `text.getRange` / `undo` | setText 生效（history `Typing`），`undo` 回滚为原文本 | 同 |
| `type_text` 就地输入 | **不可用**：`type_text needs the desktop app` | 可用 |
| `file.place {link:true, at}` | 生效，`linked:true`，Image 节点带 bounds；官方只接受一个折叠后的读/写根，因此新文件夹要靠重启会话扩权（实现用 `at` 中心点，不用 `rect`） | 可用（根由启动 GUI 时自己声明，`--connect` 不接受根参数） |
| `links.update` | 生效（无变化时 `updated:[]`） | 同 |
| `document.save {path}` | 8ms 写盘，`warnings:[]` | 同 |
| `render_preview` | 1200×900 用 17ms | 900×695 用 62ms |
| `doc_inspect` | 0–1ms | 4–28ms |
| `screenshot {window:true}` | 报错 | 398ms，3840×2088 |
| `inspect_ui` | 报错 | 2ms，给 `canvasRect`、`documents[].dirty`、`activeDocument`、`perf.fps` |
| 反向同步签名 | `doc_inspect` 含 `revision`/`dirty`/`selection`/`selectionBounds`/`history`/`path`/`objects`；官方无 `resources/subscribe`，只能轮询 | 同 |

其它已确认：`command_run` 的参数名是 `id`（不是 `command`）；`transform` MCP 工具的 `origin:[x,y]` 语义不是文档坐标（曾把 300×200 矩形缩成 4.5×3），**不要用于画布缩放**；`document.activate {index}`、`document.open`、`view.goToArtboard` 存在，用于把目标文档/画板切到激活位。

安全事实：用户当前 GUI（PID 10880，`--control 7979`）里有一个 **未保存的 untitled 文档**（`path:null, dirty:true`，含椭圆作品）。任何实时通道探针都不得在它上面做写操作，也不得结束/重启该进程。另有一个非本会话启动的 `vectorcraft-cli.exe mcp`（PID 25816，auto 模式），归属不明，保持不动。

## 架构决定

- **一个编辑会话 = 一个长生命周期子进程**，跑 `vectorcraft-cli mcp`，传输层二选一：`--connect 127.0.0.1:<cfg.port>`（GUI 在跑且端口可达）或 `--headless --automation-read-root/--write-root`（端口不可达）。两条通道暴露同一 32 工具面，因此上层编辑 API 只有一套。
- **路由不是拒绝**：`refuse_if_gui_has_document` 已删除，改为 `official_session`/`canvas_session` 选通道：端口可达走实时，不可达走 headless。
- FileAtlas 侧只做四件事：采输入、画布像素↔文档点坐标映射、显示官方返回的 `selectionBounds`、拖拽期间移动那个框做纯视觉预测（松手才提交官方手势并重渲染）。
- 画布真图来自官方 `render_preview`，每次提交后重取；不做本地绘制、不做本地裁剪/文本排版。
- 六项图片/文本能力全部转发官方命令：`file.place(link)`、`links.update`、`links.relink`、`text.create`/`text.setText`。官方只在实时通道提供的 `type_text` 就地键入未接线，两条通道统一走对象面板改文本。
- 双向同步：每次编辑一旦官方报告 `dirty` 就走 `save_file` 写盘；外部改动靠 `canvas_probe(known)` 轮询官方 `revision`，变化即重取预览；离开画布或切换页面都提交并关闭会话，目录由 `refresh_project` 重读。
- 会话态（模式开关、选中对象 id）属临时 UI 态，不新增持久化格式；原生真相只在 `.vectorcraft`。

## 锚点（实现后现状）

- 传输抽象 `crates/fileatlas-plugin-workcrate-boards/src/protocol.rs:13` `NativeProtocol{request,execute}`；legacy TCP 客户端 `:26` `Client::connect`（只服务 `connect()`/`open_editor()`）
- 官方会话 `crates/fileatlas-plugin-workcrate-boards/src/mcp.rs`：`McpSession::headless:44`、`McpSession::live:63`、`start:76`（隐藏窗口、隔离 `VECTORCRAFT_CONFIG_DIR`）、`call_tool:189`（命令级 `isError` 不杀会话）、`assert_bound:204`（实时会话每次变更前复核活动文档）、`open_document:286`、`port_is_listening:462`
- 折叠自动化根 `crates/fileatlas-plugin-workcrate-boards/src/lib.rs:190` `headless_client_in`（`automation_folder`）；通道选择 `lib.rs:208` `official_session`；GUI 拉起 `lib.rs:219` `connect()`；只读重关联 `lib.rs:273` `attach_document`
- 画布会话 `crates/fileatlas-plugin-workcrate-boards/src/editing.rs`：`canvas_open:89`、`local_links:118`、`canvas_session:135`、`canvas_grant:157`（扩权重启）、`canvas_probe:214`、`canvas_edit:251`（dirty 即写盘）、`canvas_place:327`、`canvas_set_bounds:294`（`reference:0`）
- 编辑态 UI `crates/fileatlas-plugin-workcrate-ui/src/view/canvas.rs`：`canvas_frame:81`、`CanvasMap:111`（`to_document:126`）、`handle_point:150`、`CanvasDrag`（`MAX_GESTURE_EVENTS:17` 上限）、`request_canvas_probe:275`、`leave_canvas:344`、`canvas_job:388`（代数守卫）；绘制 `view/canvas_surface.rs`；对象面板 `view/object.rs`；切页关闭会话 `view.rs:152`
- 放置/链接/文本能力 `crates/fileatlas-plugin-workcrate-boards/src/operations.rs`；MCP 工具面 `crates/fileatlas-plugin-workcrate-ui/src/mcp/editing.rs:95` `canvas_tools`
- 治理：`docs/ui-reference-workflow.md`、`docs/reference-ui/README.md`、`docs/ui-theme.md`、`docs/ui-refresh-contract.md`；守卫 `node scripts/check-{architecture,layering,feature-ledger,ui-refresh,ui-theme}.cjs`；Cargo 一律 `cargo +1.97.1 ... --locked -j 2`
- 实测：`scripts/workcrate-canvas-scenario.cjs`（九阶段同进程场景）、`boards/src/integration_tests.rs`（`canvas_edits_land_in_the_native_document`、`command_error_keeps_the_headless_session_usable`，需 `FILEATLAS_VECTORCRAFT_EXE` + `WORKCRATE_NATIVE_TEST_ROOT`）

## 阶段

### 阶段 0 边界与视觉前置
- [x] 改写 AGENTS.md WorkCrate 条目：删去"WorkCrate implements no second canvas, crop or text editor"式的绝对禁止，改为"画布编辑语义一律由官方执行，FileAtlas 只做输入转发与坐标映射；不实现本地绘制/排版"。
- [x] 按 UI 参考流程生成"页面编辑·画布编辑态"概念图，存 `docs/reference-ui/concepts/workcrate-canvas-edit-concept-2026-10-22.png`，在 `docs/reference-ui/README.md` 登记为活动参考。
- [x] 修 `call_tool` 的 poison 语义，加单测：命令级错误后同一会话仍可继续调用（`command_error_keeps_the_headless_session_usable`）。原 `headless.rs` 与计划的 `live.rs`/`session.rs` 合并为单一 `boards/src/mcp.rs::McpSession`（两条传输同一实现），会话持有在 `boards/src/editing.rs::CanvasState`。

### 阶段 1 传输路由与会话生命周期
- [x] `McpSession::live(cli, port)`：`vectorcraft-cli mcp --connect 127.0.0.1:<port>` 适配器，实现同一 `NativeProtocol` 面；端口不可达由 `official_session`/`canvas_session` 选 headless，不报错。
- [x] 会话持有：`CanvasState` 打开文档、保持热、`dirty` 即 `save_file`、`canvas_close` 结束子进程；实时优先/离线兜底的路由唯一 owner 是 `lib.rs::official_session` 与 `editing.rs::canvas_session`。
- [x] `refuse_if_gui_has_document` 已删除：GUI 可达 → 实时，不可达 → headless；残留风险改为"实时 GUI 保留其启动者声明的自动化根，被拒路径保持可见错误"，记录在 boards PORTS。
- [x] 目标文档激活：`open_document` 先比对 `doc_inspect.path`，实时会话再用 `inspect_ui.documents[].title` 找索引并 `document.activate {index}`，都不匹配才 `open_file`。
- [ ] 单测/集成测试覆盖：无 GUI 静默编辑（`canvas_edits_land_in_the_native_document` 已覆盖）、命令错误后会话存活（已覆盖）；**有 GUI 实时编辑与"编辑中途 GUI 关闭/重开"仍未覆盖**，需要官方窗口在前台。

### 阶段 2 画布编辑态 UI（叠加层）
- [x] 编辑/预览切换控件加在页面编辑区工具栏（`workcrate-canvas-edit`/`workcrate-canvas-preview`），界面全中文、不泄漏内部 id。
- [x] `view/canvas.rs` + `view/canvas_surface.rs`：官方 `render_preview` 位图 + 官方 `selectionBounds` 界框与 8 个句柄；容器固定尺寸、内部滚动，切模式不移动其它面板。
- [x] 坐标映射：叠加像素 → 文档点（y 向下，原点为首画板左上），按返回图实际宽高做 Contain 拟合；映射唯一 owner 是 `canvas.rs::CanvasMap`。
- [x] 输入转发：点击/拖拽/句柄缩放 → 官方 `pointer_gesture`（拖拽期间只移动预测框，松手提交 + 重渲染）；数值框 → `object.setBounds {reference:0}`；`object.move` 作为增量移动保留。
- [x] 撤销/重做转官方 `undo`/`redo`；不在 FileAtlas 造历史。

### 阶段 3 六项能力
- [x] 添加图片（嵌入）/链接图片：`canvas_place` → 官方 `file.place {path, link, at}`，链接文件登记为工程引用，下次会话据此授予文件夹。
- [x] 更新图片：`links.update {ids}`；替换图片：`links.relink {ids, path}`；两者先 `canvas_grant` 扩权，保持界框。
- [ ] 输入文本：官方 `type_text` 只存在于实时通道且本轮未接线；两条通道统一用「官方画布文字」输入框 + `text.setText`，新建走 `text.create`。事实限制已写进 ui/boards PORTS 与验收记录。
- [x] 更改文本文字：官方快照选区 → `canvas_object` 读文本 → `text.setText` 写，官方 history 追加步骤，`undo`/`redo` 各进一步。

### 阶段 4 双向同步与验证
- [x] 轮询签名：`canvas_probe(known)` 由观察者传入上次显示的 revision，配 `refresh::listen` + `Wake` + `ReadModel<u64,…>`；`PollResult::until` 保证空闲不空转，无快照时不探测。
- [x] 外部改动的反向同步：面板重取官方快照、重绘位图与选区；离开编辑态/切换页面都提交并关闭会话；`refresh_project` 从磁盘重建目录与预览缓存。
- [x] 同进程 FileAtlas MCP 实测：新增 `workcrate-ui.canvas_open/canvas_state/canvas_gesture/canvas_set_bounds/canvas_set_text/canvas_create_text/canvas_place/canvas_links/canvas_step/canvas_close`，用真实指针输入走完九阶段，截图对比概念图并记入 `docs/validation-ui/2026-10-21-workcrate-canvas-editing/README.md`；`docs/redesign-validation.md` 已追加条目。
- [x] 五项守卫脚本 + `cargo +1.97.1` fmt/build/test 全绿；boards 与 ui 的 PORTS/PORTS_ZH 与新公开签名对齐。

## 进度台账

- 2026-10-21：能力探针完成（headless 选择/移动/缩放/文本/放置/链接/保存、live 只读与延迟、`type_text` 仅实时可用、`transform.origin` 不可信、`command_run` 用 `id`）。代码锚点与传输抽象确认；发现 `headless.rs:139` 命令错误误杀会话。阶段 0 尚未动工。
- 2026-10-22：阶段 0–4 完成（除下表标注未覆盖项）。同进程九阶段场景与两个官方门控用例通过，5 处缺陷修复，验收记录见 `docs/validation-ui/2026-10-21-workcrate-canvas-editing/README.md`。三条实测事实修正，实现以此为准：
  - `object.setBounds` 的 120pt "偏差"不是官方 bug：默认 `reference` 是中心点，传 `reference:0` 后 x/y 就是界框左上角，数值框与叠加层完全对齐（`editing.rs::canvas_set_bounds`、门控用例断言容差 1pt）。
  - 缩放只走句柄上的官方 `pointer_gesture`（对锚点对角，`canvas.rs` 有单元测试），history 里的 `Scale` 来自这个手势；`transform` MCP 工具的 `origin` 不是文档坐标，曾被当成缩放路径，代码里已完全不使用它。
  - 官方引擎在 `drag` 采样上移动对象，只给 `down`+`up`（松手点）不会移动；面板因此按真实输入转发采样序列，实测需要到松手点为止的多段采样。
- 仍未覆盖：有 GUI 在前台的实时通道编辑与"编辑中途 GUI 关闭/重开"、原生文件选择器、面板内键盘输入、就地 `type_text`（未接线，两条通道统一 `text.setText`）。
- 待用户授权：本轮与上一轮的未提交改动（含本文档）都还没有 commit 许可。
