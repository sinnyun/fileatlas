# WorkCrate 右侧检查器卡片与字段保存流畅度同进程实测

日期：2026-10-22。环境：Windows x64，Rust 1.97.1（显式 `cargo +1.97.1`），官方
VectorCraft 0.14.0，基线提交 `46c22b3`，dev profile EXE
`target/debug/fileatlas-shell.exe`。

活动参考：[2026-10-21 装配工作台用户稿](../../reference-ui/concepts/workcrate-studio-user-2026-10-21.png)
与其[检查器概念](../../reference-ui/concepts/workcrate-inspector-2026-10-21.png)，
登记于[界面参考入口](../../reference-ui/README.md)的「WorkCrate 装配工作台」行；
本轮要求写在同一[三层清单](../../reference-ui/concepts/workcrate-three-level-2026-10-20.md)
的「Material header / page fields follow-up」一节。

## 结论

1. **右侧检查器按用户稿分成两张卡**：「页面信息 · 画板规格」包含页面名称、尺寸标记
   与画板/像素信息行；「页面内容 · 同步物料栏」包含项目名称（含继承状态、全局设置、
   恢复继承）、物料名称、尺寸、数量、附加参数与「＋ 新增物料参数行」。卡片有独立
   底色、边框、标题圆点与右侧说明，字段标签与可点击值框水平对齐。
2. **编辑不再整页跳动**：每个字段行固定 32 px 高，草稿在值所在行内展开（标签 +
   输入框 + 保存/取消），输入框自动聚焦，Enter 直接保存，行高与下方字段的 y 坐标
   在编辑前/编辑中/保存后保持一致；面板底部状态行改为固定 24 px，保存前后不改变
   面板高度。
3. **首次保存自动预热官方会话**：浏览页面时不启动官方程序；第一次字段保存由
   `canvas_open_quiet` 静默打开 headless 会话并保留，页面仍处于预览态
   （`canvas.mode == "preview"`、`live == false`），之后的字段保存复用同一会话。
4. **不再在 UI 线程解码整页位图**：官方保存帧的解码与选中对象查询在后台线程完成，
   UI 收到后只替换该页卡片图像与 revision，不排队第二次渲染、不重载项目，其它页面
   的预览文件不变。
5. **面板不再整项目重载**：保存期间 `busy` 始终为 `false`，只有该行显示「保存中」，
   左侧页面栏、草稿行和滚动位置保留。首次保存与后续保存均在预览态完成。

## 实现要点

- `WorkCrateBoardsService::canvas_open_quiet` 与 `canvas_open` 共用
  `canvas_open_inner`，仅跳过预热帧的渲染；字段保存先判断 `canvas_holds(page)`，
  不持有才预热，避免每次保存重复启动官方进程。
- `save_field` 串行化字段保存（`field_saves` 非空时拒绝新任务），后台线程依次执行
  「预热会话 → 读页 → 应用草稿 → `update_info` → `canvas_frame` 解码」；无变化草稿
  若由本任务预热则立即关闭会话，避免遗留在后台。成功后 `apply_field_save` 只补该页
  卡片；失败保留草稿并给出中文提示。
- 会话生命周期：`perform`、`open_page`、`toggle_canvas_mode`、`canvas_job` 都在
  `field_saves` 非空时拒绝新工作，`leave_canvas` 在保存中直接返回；保存产生帧后启动
  画布探针轮询，官方程序侧的改动仍能回到该页；离开页面/窗口时由既有 Drop/leave
  路径提交并关闭会话。
- 检查器布局：`section_card` 统一卡片外壳（标题圆点、说明、`surface_card` 背景、
  `separator` 边框）；`field`/`edit_row` 固定 32 px 行高；草稿复用共享
  `edit_input` 并在 `begin_edit` 中聚焦；输入区 `InputEvent::PressEnter` 订阅负责
  提交；底部状态行固定 24 px，避免 `when(busy/ message)` 条件行改变内容高度。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-plugin-workcrate-ui -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -j 2
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-inspector-layout-20261022 --access-level full --scenario scripts/validate-workcrate-inspector-layout.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-header-fields-20261022f --access-level full --scenario scripts/validate-workcrate-field-header.cjs
```

结果：fmt/check/build 退出 0；boards 28 通过 / 3 忽略、ui 9 通过，无失败；四个守卫
退出 0（架构守卫 A4 债务保持基线 17）。新场景输出：

```json
{"inspectorLayout":"passed","inspectorWidth":309.33334,"coldSaveMs":1118,
 "quantitySaveMs":518,"sizeSaveMs":810,"savingMarkerSeen":true}
```

- 首次保存（冷启动）实测 1118 ms，包含静默 headless 官方进程启动与页面渲染；
  第二次 518 ms、第三次 810 ms 复用同一会话。计时包含点击稳定等待与轮询粒度，
  不作为精确基准；相较旧路径的 867–887 ms 加 UI 线程解码与整项目重载，用户可见的
  卡顿来源已移除，但单次保存仍需要一次官方页面渲染。
- 新场景断言：两张卡与全部字段控件已绘制；检查器实测宽 309.33 px；打开草稿后
  输入框 y 与字段行相差 ≤2 px、行高相差 ≤8 px，检查器高度与下方字段 y 不变；保存
  后检查器高度与下方字段 y 不变；首次保存后会话存在且仍指向该页、模式仍为预览、
  `live == false`、卡片 `canvas_revision` 等于会话 revision、`preview_pending == 0`、
  另一页 `preview_ready == true`；进入草稿期间观察到该行自己的「保存中」标记。
- 既有 `validate-workcrate-field-header.cjs` 全流程回归通过：
  `{"headerFields":"passed","savingMarkersSeen":4}`，覆盖复用显式会话、连续保存、
  附加字段、继承/重置、无变化草稿、空名称失败保留、原生文件持久化与静默态。

截图（本地 `.local/validation/`，不入库）：
`profiles/wc-inspector-layout-20261022/screenshots/inspector-cards-before.png`、
`inspector-draft-inline.png`、`inspector-cards-after-first-save.png`、
`inspector-cards-final.png`；结果 JSON
`profiles/wc-inspector-layout-20261022/inspector-layout-result.json`。
全窗口截图为 1920×1080、Windows 深色主题；卡片使用原生 Theme 角色，浅色主题下
同一结构按 Theme 解析。

## 与手绘稿的差距

- 用户稿的「页面信息」卡用两列并排展示物理尺寸与 DPI；本轮保持单列「尺寸标记」+
  一行画板 pt / 像素信息，未做两列并排，原因是点击编辑仍以单行字段为准且 310 px
  宽度下两列会压缩输入区。
- 用户稿的字段值是不带边框的右对齐文本；本轮保留可点击的轻边框值框与预设下拉，
  以便区分「可编辑」并提供预设，未做纯文本外观。
- 检查器仍是固定 310 px 宽的滚动面板，未实现拖拽调宽或折叠分组。

## 未覆盖项

- 实时 GUI 路由：场景强制空闲端口走 headless，用户自己运行的 `vectorcraft.exe`
  上的保存路径未在本轮复测。
- 窄窗口与 310 px 检查器下的超长字段截断/滚动观感：只验证了真实短值，未做小窗口
  截图验收。
- 保存期间的状态行固定高度只由代码与前后几何断言保证；「保存中」瞬间未单独抓取
  几何快照（该标记本身已观察到）。
- 首次保存仍需一次官方页面渲染（0.5–0.8 s 量级的后台耗时）；要再缩短需要官方提供
  局部/批量更新或复用渲染结果，本轮未改动官方协议。
- 键盘 Tab 顺序、屏幕阅读器标签与高对比度主题下的卡片观感未验证。
