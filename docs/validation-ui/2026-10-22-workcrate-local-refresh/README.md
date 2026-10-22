# WorkCrate 编辑页局部刷新同进程实测

日期：2026-10-22。环境：Windows x64，Rust 1.97.1（显式 `cargo +1.97.1`），官方
VectorCraft 0.14.0（静默 headless），基线提交 `46c22b3`，dev profile EXE
`target/debug/fileatlas-shell.exe`。

本轮没有界面形状变化，活动参考不变；验证目标是页面编辑区打开期间，普通编辑动作
不再触发整目录重载。

## 结论

1. **剩余整面板刷新有三个真实来源**，都已定位并改为局部更新：
   - 画布内“放置设计文件”与“更新/重新链接素材”会调用 `projects.add_references`，
     项目索引因此唤醒面板；旧监听器不分来源一律 `reload()`。
   - 检查器“全局设置”的项目改名与“导出本页图片”复用了 `perform`，`perform`
     会关闭画布并整目录重载。
   - MCP 的页面/目录刷新修订（`request_board_refresh` / `request_catalog_refresh`）
     在编辑区打开时同样走整目录重载。
2. **项目索引变化只补丁工程卡片与字段预设**：`deliver_project_change` 重读
   `projects.projects(true)` 与 `field_presets()`，无变化返回 `false` 不发布；
   有变化只更新 `items` / `preset_fields`，不重建页面卡片、不清预览、不关会话。
3. **画布帧自带页面记录**：`CanvasFrame` 现在同时返回该页的 `WorkCrateBoard`，
   落帧时只补对应卡片，不再为了拿到页面记录额外读一次目录。
4. **仅页面内容变化时只刷新可见页**：监听器区分“切页”“目录修订”“普通帧刷新”；
   编辑区打开且只有页面刷新修订变化时走 `refresh_visible_board`，复用同一个官方
   会话重读该页，其他卡片与预览队列不动。
5. **改名、导出、放置均不再整目录重载**：项目改名只改 `items` 中该行；导出只关闭
   画布并提示“导出完成”，不重载目录；放置设计文件只补目标页卡片。
6. **`ui.reloads` 计数器提供可断言证据**：整目录重载才自增。实测字段保存（冷/热）、
   项目改名、链接放置、重新链接、编辑区内切页全程保持同一计数、同一 `session_id`、
   同一页面 ID 列表，且未触碰页面的预览保持就绪。

需要说清的边界：`WorkCratePanel` 是单个 GPUI 视图，任何 `cx.notify()` 都会重建该
视图的元素树；本轮去掉的是“整目录数据重载”（快照重读、全部卡片重建、全部预览
重新排队解码），不是把单视图拆成多个可独立绘制的子视图。字段输入框是独立实体，
输入本身不会通知面板，所以编辑时的卡顿来源已消除。真正的元素级增量绘制需要把面板
拆成多个 GPUI 子视图，本轮未做。

## 实现要点

- `view.rs` 监听器：记录 `(selected_project, selected_board)` 与两个修订号；切页
  优先、目录修订清画布、普通页面刷新走 `refresh_visible_board`，只有选择变化或
  目录变化才 `reload`。项目监听器改为 `deliver_project_change`，返回 `false` 时
  `refresh::listen` 不通知。
- `CanvasFrame` 增加 `board: Option<WorkCrateBoard>`；`apply_canvas_frame` 用
  `boards.board(&snapshot.board_id)` 的结果直接写回该卡片，保留图像、预览就绪和
  错误状态，并补做在途切页。
- `refresh_visible_board` 对当前选中页调用 `boards.canvas_open`，命中暖会话时只
  把官方进程移到该画板并取一帧，落帧仍走上面的单卡片补丁。
- `management::save_project_name` 走 Projects 服务改名后在原地更新 `view.items`；
  提示为“正在保存项目名称…”/“已保存：项目名称”。
- `perform_export` 不再复用 `perform`：有在途字段保存时拒绝，只清空并提交已打开
  的画布，然后导出，不重载目录。
- `place_selected_file` 在原生选择器返回后复核选中页，调用 `boards.place_file`
  后重读该页记录并只补这一张卡片（含 `canvas_revision` 与预览队列条目）。
- `reload()` 自增 `reloads`；状态 JSON 新增 `"reloads"`，供场景断言“局部更新不
  移动整目录计数”。
- 仍使用 `perform` 的动作：新建工程/页面、关联原生文档、同步工程、工程级
  归档/复制/封面/删除、离开画布与画布错误退出。它们确实改变目录或需要关闭会话，
  保留一次完整重载是有意的。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-plugin-workcrate-ui -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-boards -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-local-refresh4 --access-level full --scenario scripts/validate-workcrate-local-refresh.cjs
```

结果：fmt/check/build 退出 0；ui 9 通过，boards 31 通过 / 4 忽略；四个守卫退出 0。
场景输出：

```json
{"localRefresh":"passed","passed":true,"project":"4a1c9a2f-…",
 "reloadsBeforeCreate":3,"reloadsAfterCreate":4,"textId":14,"imageId":15}
```

场景步骤与断言：

1. 独立 profile、空闲端口，新建工程与原生文档，再建第二页作为对照；两页都先出
   预览。
2. 冷字段保存（自动预热官方会话）与编辑区内热保存：`reloads` 不变、`session_id`
   不变、页面 ID 列表不变、对照页预览仍就绪。
3. 进入 edit 模式后用官方命令创建文本，记录基线 `{reloads, session, ids, other}`。
4. 项目改名（`projects.wake` 路径）：只补丁工程卡片，计数/会话/页面/对照预览均
   不变。
5. 放置外部设计文件（先复制进文档目录，避免扩根重开会话）与重新链接：计数、
   会话、页面保持，放置页 `canvas_revision` 前进。
6. 在编辑区切到对照页再切回：仍为 edit 模式，`session_id` 与 `reloads` 不变。
7. 切到预览并做一次热字段保存：同样不重载。
8. 新建第三页（目录事务）：`reloads` 由 3 增至 4，画布会话关闭——证明计数器确实
   能观察到完整重载，前面的“不变”不是失效断言。
9. 静默检查：动作结束后不再出现新的状态读取或重载。

结果文件：`.local/validation/profiles/wc-local-refresh4/local-refresh-result.json`
（本地，不入库）。

## 与设计参考的差距

本轮无视觉改动，不涉及参考图差异。

## 未覆盖项

- 实时 GUI 路由：场景用空闲端口强制走 headless；用户自己运行的 `vectorcraft.exe`
  经控制端口的局部刷新路径未在本轮复测。
- 单视图重绘边界：GPUI 仍会在 notify 时重建面板元素树；本轮只证明数据/解码/预览
  队列不再整目录重做，未做元素级增量绘制的性能采样。
- 自动同步仍然整目录换卡片（保留已解码图像、不增加 `reloads`）；官方文档没有
  逐页内容修订号，无法只重解码变化页，本轮未改原生协议。
- 新建页面/关联文档/同步工程等有意保留的整目录重载未逐项计时。

## 追加：保存期间按钮整体置灰的消除（同日第二轮）

用户仍反馈"右侧面板一修改就大面积刷新"。加 `renders` 计数（每次面板 `render`
自增，进入 `workcrate-ui.state`）并配合逐行像素对比（PowerShell
System.Drawing，阈值 8/通道）后定位到真实原因：

- 按键编辑本身只改动被编辑的那一行像素（58/1831 行），GPUI 的 notify 会让整个
  面板视图重建元素树，但祖先重建不产生可见差异——这是框架行为，包子视图无法
  阻止（`Window::mark_view_dirty` 会把 dirty 传播给全部祖先视图）。
- 用户看到的"大面积刷新"其实是字段保存期间 `action()` 把
  `!field_saves.is_empty()` 计入 disabled：保存约 0.5–1 s 内顶部工具栏、中间
  动作区和画布工具栏所有按钮同时置灰（mid-save 像素差异 840/1831 行，几乎
  全宽）。

修复：`action()` 的 disabled 只看 `busy || canvas_busy`；点击时若
`field_saves` 非空则设置提示"正在保存页面内容，请稍候…"并拒绝执行。安全性不
变——所有处理器（`perform`、`open_page`、`toggle_canvas_mode`、`canvas_job`、
`perform_export`、`place_selected_file`、`save_field`）本就在保存期间拒绝新
动作。曾试验把输入框包进子视图宿主（`InputHost`）以隔离重渲染，实测无效果
（祖先传播），已整体回退，仅保留 `renders` 计数。

复测（profile `wc-rerender-diag5`）：

| 阶段 | renders 增量 | 像素差异行 |
|---|---|---|
| 打开草稿 | +7 | — |
| 打字（未保存） | +11 | 58/1831（仅编辑行） |
| Enter 后保存中 | +3 | 74/1831（编辑行 + 底部状态行） |
| 保存完成 | +4 | — |

截图确认保存中所有工具栏按钮保持可用，仅被编辑行的"保存"按钮显示"保存中"，
底部固定状态行显示"正在保存页面内容…"。结果文件：
`.local/validation/profiles/wc-rerender-diag5/diagnose-rerender-result.json`（本地）。

回归：`cargo test -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-boards`
40 通过；四项治理检查（theme/refresh/architecture/feature-ledger）全绿。

## 追加：画布编辑期间按钮整体置灰的消除（同日第三轮）

画布区有同样的模式：`canvas_action`、`canvas_mode_button` 和位置/尺寸数字输入框
都在 `canvas_busy` 时置灰，一次画布命令（更改文字、应用字体与字号、对齐、应用
位置与尺寸）执行期间右侧画布对象面板与画布工具栏全部变灰。按同轮字段保存的
方案修复：

- `canvas_action` 的 disabled 只保留结构性条件（无画布、"更改文字"需要文字对象、
  "链接图片"需要已链接对象）；`busy || canvas_busy` 时点击改为提示
  "画布操作进行中，请稍候…"。
- `canvas_mode_button`（预览/编辑切换）与 `action()`（画布工具栏缩放/撤销/重做、
  顶部工具栏）同样不再因 `canvas_busy` 置灰，点击时提示。
- 位置/尺寸数字输入框不再随 busy 状态禁用——数值提交由"应用位置与尺寸"按钮
  的拒绝逻辑兜底。

复测（profile `wc-canvas-diag2`）：编辑模式下创建文字、填字号并点击
"应用字体与字号"，点击后帧与基线的像素差异为 58/1831 行、单一区块
（右侧字号输入框），截图确认所有面板与工具栏按钮保持可用。headless 的文字
样式命令约 50 ms 内完成，顺序 MCP 调用抓不到 `canvas_busy=true` 的中间帧；
但修复后没有任何视觉 disabled 依赖 `canvas_busy`（grep 全量确认），置灰在
机制上已不可能出现，且同机制的字段保存路径已在前一轮抓到过 busy 中间帧验证。

结果文件：`.local/validation/profiles/wc-canvas-diag2/diagnose-canvas-result.json`
与 `canvas-{baseline,midjob}.png`（本地，不入库）。回归与治理检查同前，全绿。

## 追加：画布角点拖拽等比缩放（同日第四轮）

需求：编辑画布中拖角点缩放图片/文字时保持等比，参考 VectorCraft 按住 Shift
缩放的效果。先探明官方接口事实：

- 安装的 vectorcraft-cli 0.14.0 中 `pointer_gesture` 虽声明 `mods` 参数
  （事件级数组/字符串、`shiftKey`、顶层 `mods` 共四种写法），headless 实测全部
  被忽略（130×90 与自由缩放一致），无法靠官方手势携带 Shift。

因此等比约束在 FileAtlas 侧实现（`constrain_corner`）：角点手柄（0/2/5/7）的
拖拽采样点被引导到过对角锚点的等比路径上（取两轴伸缩比的最大值，双轴同时越过
锚点时整体翻转），边中点手柄保持单轴自由。官方引擎仍然执行真正的缩放与命中
测试——FileAtlas 只选择发送哪些采样点，不做任何变换运算、不解析 artwork。
`predict` 预览框与转发的手势点使用同一约束点，视觉预测与官方结果一致。

实测（profile `wc-proportional3`，场景 `validate-proportional-scale.cjs`）：
编辑模式创建文字（96×34.75 pt，比例 2.762），用 `pointer_drag` 以刻意偏离比例
的目标（+100/+40 pt）拖右下角手柄，官方结果 204.14×74.39 pt（比例 2.744，偏差
0.7%，官方按文字度量取整），左上锚点保持 (100, 72.16) 不动，宽度按主导轴增长。
合成拖拽必须使用 `control_ui` 的 `pointer_drag`——`pointer_move` 不携带按键
状态，GPUI `dragging()` 为假，画布不采样。

单元测试 `corner_drags_sample_the_proportional_path_and_side_handles_stay_free`
覆盖等比路径、释放点引导与边中点自由。回归与治理检查全绿（修复测试中一处
`unwrap` 触发的 A4 债务计数）。

结果文件：`.local/validation/profiles/wc-proportional3/proportional-result.json`、
`wc-dragprobe1/drag-trace.json`（本地，不入库）。

## 未覆盖项（第四轮）

- 实时 GUI 会话（用户自己的 vectorcraft.exe 控制端口）中的 `mods` 行为未测；
  无论其是否生效，FileAtlas 侧约束在两种传输下都成立。
- 角点拖到锚点另一侧的翻转行为只有单元测试，未做画布实测。
- 图片对象的等比缩放未单独实测（与文字共用同一选框手势路径）。
