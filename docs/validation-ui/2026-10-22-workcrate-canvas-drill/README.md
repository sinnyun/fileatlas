# 2026-10-22 WorkCrate 画布点选钻取（组内直接选中子图层）

## 验证目标

修复分组功能引入的回归：在编辑界面画布中点击会选中整组（内容组），导致整组
一起变换。要求画布中选择时能直接选中组内相应图层文件。

## 官方接口事实（先探测后使用）

- `vectorcraft-cli.exe run --cmd document.json` 探测（wc-drill 保存的文档）：
  `document.json` 的节点**没有 bounds**，不能作为命中判据；
  `document.inspect {"depth":16}` 的 `layers` 树携带每个节点的
  `bounds/{x,y,width,height}`、`kind`（Group/Type/Image/Layer）、`locked`，
  并同帧返回 `selection`/`selectionBounds`——钻取判据以它为准。
- 按压会**重新命中测试**：即使文字已被选中（`select.set [9]`），
  `pointer_gesture [down,drag,up]` 之后选中变成内容组 `[10]`——任何以手势
  实现的移动/缩放都会重新抓走整组，因此移动改用 `object.move`（按节点 id）、
  缩放改用 `object.setBounds`（`reference:0`）提交。
- 无头会话的套索（空白按下拖拽）手势不产生选择（实测 `marqueeSelects:null`，
  官方行为原样转发）；实时 GUI 保持官方套索。

## 实现要点

- `native.rs`：`drill_click`（点击后沿 inspect 树按 z 序找包含点击点的最深层
  子节点并 `select.set`；落在锁定组内则 `select.none`）、`run_drag`
  （拖拽规划：显示选区内按子图层 → `object.move`；显示组/多选 → 原样转发；
  选区外 → 合成点击拾取后移动、空白转发套索、物料栏吞掉）、`locate`/
  `deepest_at`/`bounds_contains` 纯函数 + 单测
  `click_drill_picks_the_topmost_child_under_the_pointer`。
- `editing.rs`：`canvas_gesture` 用 `gesture_points` 区分点击/拖拽/不可识别批次；
  `mcp.rs` 的 `NativeProtocol::request` 增加 `pointer_gesture` 路由。
- workcrate-ui：`CanvasDrag::is_handle/predicted`；`canvas_surface.rs` 的
  `canvas_up` 对角点拖拽改发 `canvas_set_bounds`（预测框即最终框），正文拖拽
  仍走 `canvas_gesture`。

## 同进程 MCP 实测（profile `wc-drill`，scenario `.local/validation/validate-canvas-drill.cjs`）

结果 `profiles/wc-drill/canvas-drill-result.json` + 截图
`screenshots/canvas-drill.png`（标注文字被独立选中，角点手柄齐全，对象面板
显示"类型 文字"，缩放后 233×84.4）：

- 点选钻取：点击内容组内文字后选区为 `[9]`（文字），不再是内容组 `[10]`
  （`drillPicksText`）；空白点击取消选中。
- 真实窗口路径：`control_ui` pointer_down/up 经界面映射点击同一文字，
  对象面板显示"标注文字/文字"（`overlayClickPicksText`）。
- 拖动已选文字：选区保持 `[9]`，选框位移 (20,40)（`dragMovesOnlyText`）。
- 空手抓取未选中文字：合成点击拾取后仅移动该文字（`grabMovesUnselectedChild`）。
- 套索与显示组：空白起手的拖拽原样转发（无头不选中，`marqueeSelects:null`），
  显示组时的拖拽转发为整组移动（本次运行无头未选中组，步骤自动跳过）。
- 真实角点拖拽：`pointer_down/drag/drag/up` 在窗口坐标拖动右下角手柄，
  文字宽 112→233，位置 x 不变（`handleScalesChild`），选区仍是文字。
- 物料栏点击：选区为空（`bandSelection:[]`），不会选中任何组。
- 结构复核：官方 CLI 读回保存文档，内容组 `content_root` 子节点仍含该文字
  id（`contentGroupChildren:[9]`）。

## 静态检查

- `cargo +1.97.1 fmt --all -- --check` 通过；
  `cargo test -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui`
  35+10 通过。
- `check-ui-theme` / `check-ui-refresh` / `check-architecture` /
  `check-feature-ledger` 全部通过。

## 已知边界

- 无头会话套索不选中（官方行为）；实时 GUI 中套索/整组移动保持官方路径。
- 组选区（套索后）整体移动、组内"空洞"点选命中该组本身，均为设计行为。
- 通过 MCP 直接改选区（revision 不变的纯选择变化）不会主动推送到对象面板，
  为既有行为，与本修复无关。
