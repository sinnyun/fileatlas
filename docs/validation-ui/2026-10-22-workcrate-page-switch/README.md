# WorkCrate 编辑区内切页保持官方连接与预览同步

日期：2026-10-22。环境：Windows x64，Rust 1.97.1（显式
`cargo +1.97.1-x86_64-pc-windows-msvc`），官方 VectorCraft 0.14.0
（`C:\Program Files\VectorCraft\vectorcraft.exe` 与同目录 `vectorcraft-cli.exe`），
基线提交 `46c22b3`，dev profile EXE `target/debug/fileatlas-shell.exe`。

用户要求：进入页面编辑区后切换页面，新页面仍停在编辑态；预览同步；后端画布连接在
切页时保持，不重复连接；只有退回页面管理、项目管理或关闭面板才退出会话。

## 结论

1. **切页不再重连**：一个画布会话属于整份原生文档。面板切页时对目标页调用
   `canvas_open`，Boards 复用仍绑定同一 `.vectorcraft` 的会话，只执行
   `select.none` 并把 `board_id`/`native_id`/`artboard`/`preview` 移到新画板；
   `CanvasSnapshot.session_id` 保持不变。
2. **编辑态与预览同时跟随**：切页不改变 `canvas_mode`；官方帧落地后只补目标页卡片，
   `boards[].canvas_revision` 与预览位图即该页新帧。预览态（保留显式会话）切页同样
   生效，模式仍为预览。
3. **只有退出编辑区才关闭**：返回页面列表、返回项目管理、选择另一个工程或关闭面板
   走原有提交/关闭路径；新增原生页面属于目录事务，仍会先保存并关闭预览会话，随后
   进入编辑时再建新会话。
4. **在途请求收敛**：切页遇到渲染或字段保存占用会话时先记录选择，帧落地
   （`apply_canvas_frame`）或保存完成时补做；进入编辑模式也会向当前选中页收敛。
5. **选择不跨页泄漏**：旧页的官方 selection 在切页时清空，新页对象面板为空。

## 实现要点

- `crates/fileatlas-plugin-workcrate-boards/src/editing.rs`：
  `CanvasState.session_id` + `CanvasSnapshot.session_id`；`canvas_open_inner` 在
  `holds_document` 为真时调用 `switch_to`，否则丢弃旧会话重建；服务用
  `AtomicU64` 分配进程身份，`canvas_grant` 重启也会得到新 id。
- `crates/fileatlas-plugin-workcrate-ui/src/view.rs`：选择监听区分“编辑区内切页”
  与“离开编辑区”；`open_page` 在导航后调用 `switch_canvas_page`。
- `crates/fileatlas-plugin-workcrate-ui/src/view/canvas.rs`：`switch_canvas_page`、
  `apply_canvas_frame` 的滞后页收敛、`toggle_canvas_mode` 进入编辑时向选中页收敛。
- `crates/fileatlas-plugin-workcrate-ui/src/view/render.rs`：页面栏条目导出
  `workcrate.control.rail-<board_id>` 几何，供真实指针点击验证；工具栏动作在字段
  保存期间禁用。
- `crates/fileatlas-plugin-workcrate-ui/src/view/actions.rs`：字段保存完成后再补做
  在途切页。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-plugin-workcrate-ui -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui
$env:FILEATLAS_VECTORCRAFT_EXE='C:\Program Files\VectorCraft\vectorcraft.exe'
$env:WORKCRATE_NATIVE_TEST_ROOT='F:\AI\geren\pho_cordis\.local\validation\workcrate-page-switch-native'
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards --lib -- --ignored --nocapture page_switches_reuse_the_same_official_session
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-page-switch-2026-10-22-r2 --access-level full --scenario scripts/validate-workcrate-page-switch.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-canvas-page-switch-2026-10-22-r3 --access-level full --scenario scripts/workcrate-canvas-scenario.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-ui-theme.cjs
node scripts/check-layering.cjs
```

结果：fmt/check/build 退出 0；WorkCrate 五个 crate 48 项通过（boards 31、projects 7、
ui 9、contracts 1），boards 4 项按条件忽略；官方门控用例
`page_switches_reuse_the_same_official_session` 3.48 s 通过；五个守卫均通过
（architecture `edges=124, debt=800`；layering 42 crates / 29 approved peer edges）。

同进程切页场景 `{"pageSwitchScenario":"passed",…}`，项目
`753b8547-be6d-41f5-972d-dd4ff1503e9f`，两页同属文档
`f88c551f-5c44-4d39-b91b-71a280999640`，首轮 `session_id=1`，切页后仍是 1，编辑
后 revision 3：

- 浏览态 `connection=="not_connected"`；真实点击“编辑”后 `mode=="edit"`、
  `state.live==false`（隔离端口走静默 headless）。
- 真实指针点击页面栏 `rail-<board2>`：仍为 `screen=="editor"`、`mode=="edit"`，
  `session_id` 不变，`state.preview` 换成第 2 页自己的路径，卡片 `preview_ready` 为
  真且 `canvas_revision==state.revision`。
- 通过同一会话在第 2 页创建官方文字并反向同步到面板（`revision` 一致）。
- 再点第 1 页：`session_id` 仍为 1，`selection==[]`，卡片 revision 与会话一致。
- 预览态下切到第 2 页：`mode=="preview"` 保持，`session_id` 不变，卡片 revision 与
  会话一致。
- 点击“项目管理”面包屑中的工程名返回页面列表：`canvas.state==null`，
  `workcrate-ui.canvas_state` 返回 `canvas editing is not open`。
- 重新进入编辑得到新 `session_id`（证明退出确实释放官方进程）；离线目录读到 2 页，
  原生文档增大，空闲 5.5 s 快照不变。
- 既有 9 段画布场景回归通过：`{"canvasScenario":"passed","textId":7,"imageId":8}`。
  其中 7b 段改为：新增页面（目录事务关闭旧会话）→ 重新进入编辑 → 打开另一页仍为
  编辑态且 `session_id` 相同 → 在第二页写官方文字 → 切回第一页仍复用同一会话。

## 实测证据（本地，未提交）

- 切页场景 profile：`.local/validation/profiles/wc-page-switch-2026-10-22-r2/`，
  截图 `screenshots/page-switch-preview.png`（预览态停在第 2 页，左右页面栏与中央
  官方帧一致）；结果 JSON `page-switch-scenario.json`、MCP 步骤 `report.json`。
- 画布回归 profile：`.local/validation/profiles/wc-canvas-page-switch-2026-10-22-r3/`，
  截图包括 `canvas-page-switch.png`（切回第一页仍为编辑态）、
  `canvas-catalogue-after-edit.png`（编辑后的两页目录）。
- 官方门控测试产物：`.local/validation/workcrate-page-switch-native/`。
- 场景脚本：`scripts/validate-workcrate-page-switch.cjs`（新增）、
  `scripts/workcrate-canvas-scenario.cjs`（7b 段改写并稳定工具栏点击时序）。

## 范围与限制

- 本轮全部走“控制端口未监听 → 静默 headless”路径；实时 GUI 路由
  （`mcp --connect` 下切页复用同一 GUI 会话）未实测。执行前检查过
  `vectorcraft.exe` 进程数为 0；两个遗留的 `vectorcraft-cli.exe mcp` 测试进程在核对
  命令行后结束，未触碰用户 GUI。
- 通过指针点击验证了页面栏切页与工具栏入口；项目管理/关闭面板路径通过选择监听与
  `Drop` 代码路径覆盖，面板关闭（窗口 X）未单独注入。
- 字段保存在途时的切页收敛由代码路径覆盖（保存完成回调再次调用
  `switch_canvas_page`），未构造保存期间点击页面栏的定时竞态场景。
- 多个页面连续快速切换依赖 `canvas_poll` 代次与 `apply_canvas_frame` 收敛，本轮
  断言了两次普通切页；更密集的连点与窄窗口布局未实测。
