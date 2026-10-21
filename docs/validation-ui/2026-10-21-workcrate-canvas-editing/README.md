# WorkCrate 可切换画布编辑（官方语义转发）同进程复验

日期：2026-10-21 起，2026-10-22 收尾。环境：Windows x64，Rust 1.97.1，官方 VectorCraft 0.14.0
（`C:\Program Files\VectorCraft\vectorcraft.exe` 与同目录 `vectorcraft-cli.exe`），
基线 `39cea6b`，dev profile EXE `target/debug/fileatlas-shell.exe`。
活动参考：[2026-10-22 画布编辑态概念图](../../reference-ui/concepts/workcrate-canvas-edit-concept-2026-10-22.png)
（登记于 [界面参考入口](../../reference-ui/README.md) 的「WorkCrate 画布编辑态」行）。

## 结论

页面编辑区现在有预览/编辑两态，编辑态是官方 VectorCraft 的一层语义转发：
面板只采集指针输入、把叠加像素映射成文档点、显示官方 `selectionBounds` 界框与 8 个句柄，
并在拖拽期间移动那个框做视觉预测；选择、命中测试、移动、缩放、图片放置/链接/更新/替换、
文本创建与改字、撤销/重做、数值框写入全部由官方命令执行，FileAtlas 不本地绘制、不本地排版、
不自建历史。实测 9 个阶段全部成立：

- **路由不是前置条件**：官方控制端口未监听时自动走静默 headless 会话（快照 `live == false`），
  端口可达时走实时会话；本轮全程 `vectorcraft.exe` 进程计数为 0，编辑照样完成。
- **双向同步**：每次编辑后官方报告 `dirty` 即写回同一个 `.vectorcraft`；面板之外的客户端
  （MCP 直连同一官方会话）改动的内容被面板轮询捕获并重绘，选区一致；
  离开编辑态/切换页面都会提交并关闭会话；离线 `refresh_project` 从磁盘读到编辑后的 2 页。
- **静默**：空闲面板两次读取（间隔 2.5 s）快照完全相同，无空转重绘。

## 缺陷与修复

1. 面板看不到"别人改的"官方会话（反向同步失效，功能级）。`canvas_probe` 用会话自己记录的
   基线判断变化，因此由其它客户端推进的 `revision` 永远与该基线相等，观察者永不重绘。
   现改为由观察者传入它上次显示的 `revision`（`editing.rs:214`、`canvas.rs:275`），
   集成测试同时断言"用已显示的 `revision` 探测为 `None`"和"陈旧观察者探测为 `Some`"。
2. 离开画布后被丢弃的在途帧把快照装回来。`canvas_job` 回投时不校验自己属于哪一代，
   于是"预览已退出、官方会话已关闭"的面板又显示出一份快照。现按 `canvas_poll` 代数丢弃过期帧
   （`canvas.rs:266`、`canvas.rs:401`）。
3. 切换页面泄漏上一页会话。页面选择变化路径从不关闭会话，官方子进程与文档绑定继续存活。
   现在选择变化时先对被离开的页面执行提交并关闭（`view.rs:155`）。
4. 选中框不透明遮挡画布内容。`theme::fill_transparent_overlay()` 返回的是 alpha 字节为 0 的
   packed 值，`rgb()` 会把 alpha 掩掉，预测框因此被涂成实心块。现按主题约定补上 alpha：
   `rgba(theme::fill_transparent_overlay() | 24)`（`canvas_surface.rs:319`）。
5. 界面泄漏 Win32 扩展长度前缀。官方把 `\\?\F:\…` 原样报给链接文件行。现由
   `object.rs:38` 的 `display_path` 统一去掉前缀，界面只显示普通路径。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui
$env:FILEATLAS_VECTORCRAFT_EXE='C:\Program Files\VectorCraft\vectorcraft.exe'
$env:WORKCRATE_NATIVE_TEST_ROOT='F:\AI\geren\pho_cordis\.local\validation\native-canvas-docs'
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards --lib -- --ignored --nocapture canvas_edits_land_in_the_native_document command_error_keeps_the_headless_session_usable
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-canvas --plugins dev-test --access-level full --scenario scripts/workcrate-canvas-scenario.cjs
node scripts/check-architecture.cjs; node scripts/check-layering.cjs; node scripts/check-feature-ledger.cjs; node scripts/check-ui-refresh.cjs; node scripts/check-ui-theme.cjs
```

结果：fmt 与五个守卫退出码 0；WorkCrate 五个 crate 24 项通过、3 项按条件忽略、无失败
（boards 11、projects 7、ui 5、contracts 1）；两个官方门控用例通过（5.05 s）：
`command_error_keeps_the_headless_session_usable` 证明命令级错误不再误杀会话，
`canvas_edits_land_in_the_native_document` 证明编辑落进原生文件。
同进程场景 `{"canvasScenario":"passed",…}`、`exit=0`，9 个阶段断言全部成立：
预览态不连官方（`connection=="not_connected"`）、真实点击进编辑并选中静默 headless 路由、
面板外官方新建文字被面板同步、4 段指针采样拖拽 = 官方 `Move`（界框位移 +60/+40 pt，容差 2）、
跨文件夹链接放置会扩权并让对象面板跟随（`kind=="image"`、`linked`）、
先改源文件字节再 `links update` 使官方 `revision` 前进、`links relink` 留在同页、
`set_text`/`undo`/`redo` 各进一步、工具栏放大提高 `ui.canvas.zoom`、预览退出并关闭会话、
切页关闭上一页会话、离线目录读到 2 页且原生文件变大、空闲无重绘。
场景要求开始时没有任何官方 GUI 进程，本轮由用户先手动关闭 VectorCraft/FileAtlas；
脚本与测试都不会结束用户的官方实例。收尾时仍存在一个不属于本轮的
`"vectorcraft-cli.exe" mcp`（PID 2520，父进程是本仓库技能脚本 `vc-mcp-launcher.cjs` 的 node 进程），
未启动、未关闭、未使用，与本用例无共享资源。

同轮另有一处**先前就存在、与本功能无关**的破坏：`cargo +1.97.1 check -p fileatlas-plugin-dev-test --all-targets`
失败于 `crates/fileatlas-plugin-dev-test/src/lib.rs:319` 的 `use fileatlas_plugin_database::queries;`
（E0603 module `queries` is private）。它只影响该插件自己的测试目标，WorkCrate 与 shell 构建不受影响；
本轮未修改，留待单独处理。

## 实测证据（本地，未提交）

profile：`.local/validation/profiles/wc-canvas/`，最终一轮项目
`2bb415c2-e9ee-4c62-98fd-a74b82b0df21`。

- 截图（`.local/validation/profiles/wc-canvas/screenshots/`，与概念图逐项对比见下节）：
  `canvas-edit-open.png` 编辑态首帧（工具栏 + 官方位图 + 空选区提示）、
  `canvas-edit-reverse-sync.png` 面板外官方新建文字后面板已选中同一对象、
  `canvas-edit-after-drag.png` 拖拽后官方界框与新位置、
  `canvas-edit-placed-image.png` 跨文件夹链接图片后 8 句柄选中框与对象详情、
  `canvas-page-switch.png` 切页后回到预览态、
  `canvas-back-to-preview.png` 预览退出态、
  `canvas-catalogue-after-edit.png` 编辑后离线目录两页。
- 原生真相：`documents/1792605476989/画布实测1792605476989.vectorcraft` 218,377 B，
  JSON 头 118,354 B + `VCBLOBS`；`format:"vectorcraft" version:4 generator:"VectorCraft 0.14.0"`，
  两个画板（id 1 "Artboard 1" 与 id 2 "未命名页面"，各 1190.5512 × 841.8897 pt = A3 横向），
  节点 id 16 `kind.type:"image"` 名 `placed.jpeg`，其 `link.path` 已指向
  `…\links\replacement.jpeg`（`relative:"../../links/replacement.jpeg"`，即"替换文件"生效），
  文本节点保留重做后的"第二行文字"。
- 编辑态缓存：`workcrate/projects/2bb415c2-…/canvas/1.png` 1,935,764 B、`canvas/2.png` 62,072 B
  （官方 `render_preview` 写回的编辑帧，编辑态专用），
  `boards-cache.json` 1,682 B、`native-previews/`、`cli-config/`（会话专属隔离偏好目录）。
- 门控官方用例产物：`.local/validation/native-canvas-docs/`，其中
  `canvas-documents/Canvas <ts>.vectorcraft` 171,841 B 内含节点 id 16 `placed.jpeg`
  （`type:"image"`，`link` 指向 `../canvas-links/replacement.jpeg`），
  即"放置 → 移动 → 重新链接"确实写进了原生文件；`session-survival/` 为命令错误后仍可用的会话。
- 场景脚本：`scripts/workcrate-canvas-scenario.cjs`（仓库内，可复现）；
  结果 JSON `canvas-scenario.json`、MCP 步骤日志 `report.json` 在 profile 内。

## 概念图与实现差异

结构、控件集合与信息层级按概念图实现；下列差异是本轮的取舍，不是遗漏未报：

- 顶部工具栏为「预览 / 编辑」+「− 100% +」+「撤销 / 重做」+ 会话状态 +「在 VectorCraft 中编辑」；
  概念图上的 添加图片 / 链接图片 / 更新图片 / 替换图片 四个按钮改放在右侧面板：
  前两个在「画板」区（作用于画板中心，无需先选对象），后两个在「链接图片」区
  （只对当前选中的链接对象有意义）。原因是它们的作用对象不同，放在同一排会让人误以为
  都能对任意选区生效。
- 概念图的标尺、对象名标签与拖拽时的「移动 +12, +8」提示未实现：官方 `render_preview`
  位图本身不含标尺，FileAtlas 不本地绘制任何官方未返回的内容。
- 概念图的 🔍（适应窗口）缩放按钮未实现，当前只有 −/＋ 两级步进缩放。
- 右侧「选中对象」的 名称 与 类型 为只读展示（来自官方 `document.node`），
  概念图画成可编辑输入框与下拉；官方没有"重命名对象/改对象类型"的编辑语义可转发，
  因此不做本地假象。X/Y/宽/高 与「应用位置与尺寸」按官方 `object.setBounds`
  （`reference:0` 使 x/y 为左上角）接线。
- 概念图为浅色示例稿；实际配色全部走 GPUI Kit 原生主题角色，深浅随设置变化。
- 概念图暗示的"在画布上直接键入文字"未实现：官方只在实时通道提供 `type_text`，
  本轮两条通道统一用「官方画布文字」输入框 + `text.setText`（新建走 `text.create`），
  未接线就地键盘编辑。

## 范围与限制

- 本轮以"官方控制端口未监听 → 静默 headless"路径完成全部断言。前台路由
  （GUI 在 `--control` 端口可达时的实时会话、`open_editor` 跳转/聚焦、GUI 内画布编辑与
  后台写入冲突拒绝）需要官方窗口在前台，本轮未覆盖，代码路径与文档见
  [boards PORTS.md](../../../crates/fileatlas-plugin-workcrate-boards/PORTS.md)。
- 添加图片 / 链接图片 / 替换文件 走原生文件选择器，指针自动化无法驱动本机对话框，
  本轮通过 `workcrate-ui.canvas_place` / `canvas_links` 走同一服务入口完成实测，
  对话框本身仍未实测。
- 对象面板的文字输入框需要真实键盘输入，当前同进程 MCP 对该面板没有键盘注入设施，
  因此「更改文字」通过 `canvas_set_text` 与面板同一服务路径实测，界面键入未实测。
  官方只在实时通道提供的就地键盘输入 `type_text` 本轮未接线：两条通道统一走
  「官方画布文字」输入框 → `text.setText`，概念图上的画布内直接键入因此仍是差距。
- headless 会话的自动化根是折叠后的单个文件夹，指向新文件夹时由 `canvas_grant` 重建会话扩权；
  实时 GUI 保留其启动者声明的根，被拒绝的路径保持为可见错误，不会静默重启用户程序。
- 原生文件选择器、覆盖确认弹窗、其它官方版本、复杂字体、跨进程同时切换文档竞态、
  PDF/栅格导出（属上一轮离线复验范围）本轮不再重复覆盖。
