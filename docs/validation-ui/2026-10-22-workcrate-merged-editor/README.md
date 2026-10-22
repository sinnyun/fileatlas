# 2026-10-22 WorkCrate 页面编辑器合并为单一界面

## 目标

按用户要求"将预览界面和编辑界面合并成一个界面，然后将预览界面的单独的功能
也都合并到编辑界面的功能内"，移除页面编辑器的 预览/编辑 双模式。

## 实现

- `WorkCratePanel` 删除 `canvas_mode` 字段；删除 `toggle_canvas_mode` 与
  `canvas_mode_button`，移除 `workcrate-canvas-preview` / `workcrate-canvas-edit`
  两个模式切换按钮。
- 新增 `ensure_canvas_session`（crates/fileatlas-plugin-workcrate-ui/src/view/canvas.rs）：
  显示页面即以原"编辑"按钮相同的方式打开官方会话。入口覆盖页面栏点击
  （`open_page`）、快照落地后的监听（view.rs wake 闭包）与放置设计文件完成后的
  重开（`place_selected_file`，该路径会先提交并关闭会话再逐页写文档）。
- 画布工具条常显缩放/撤销/重做，并把原预览专属的 放置设计文件
  （`place-design-file`）与 导出本页图片（`export-current-image`）并入，
  与 在 VectorCraft 中编辑（`workcrate-canvas-handoff`）并列。
- 中央区域始终是 `canvas_area`（官方位图 + 选择覆盖层 + 官方手势 + 拖拽放置），
  删除 `render_editor` 中原本地位图预览分支。
- 右侧栏 `workcrate-editor-rail`（310px，纵向滚动）堆叠 画布对象面板
  （`canvas_object_panel`）与 页面信息/页面内容（`inspector`）；两者改为只渲染
  内部内容。预览模式独有的物料信息字段因此并入编辑界面。
- `workcrate.ui` 状态不再输出 `canvas.mode`；自动同步、字段保存与预览监视器的
  让位条件统一为 `canvas.is_some()`。

## 实测

- 场景：`.local/validation/validate-merged-editor.cjs`（本目录 git-ignored），
  客户端 `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe
  --profile .local/validation/profiles/wc-merged2 --access-level full --scenario …`。
- 进程基线：运行前后 `vectorcraft.exe` / `vectorcraft-cli.exe` 均为 0，无遗留。
- 结果（`.local/validation/profiles/wc-merged2/merged-editor-result.json`）：
  - `open_board` 后未点击任何模式按钮，官方会话自动打开（revision 1），
    `canvas.mode` 不存在于状态 JSON。
  - 合并控件齐全：`place-design-file`、`export-current-image`、
    `workcrate-canvas-handoff`、`workcrate-canvas-zoom-in`、`workcrate-canvas-undo`、
    `workcrate-canvas-redo`、`inspector`、`workcrate-canvas-create-text`、
    `canvas-text-input`；`workcrate-canvas-preview` / `workcrate-canvas-edit` 不再渲染。
  - 合并界面内直接拖放 `small.png`：revision 2，对象 `linked=true`、`kind=image`。
  - 工具条 撤销 经官方会话生效：revision 前进到 3（撤销本身也是一次官方变更），
    画板回到空页，无错误消息。
- 截图：`.local/validation/profiles/wc-merged2/screenshots/merged-editor.png`
  —— 单一工具条、中央官方画板、右侧栏 画布对象 + 页面信息 + 页面内容 堆叠。
- 静态：`cargo +1.97.1 fmt/check/test`（boards 33、ui 10 通过）；
  `check-ui-theme` / `check-ui-refresh` / `check-architecture` / `check-feature-ledger`
  全部通过。

## 未实测项

- 导出本页图片 / 放置设计文件 按钮只断言存在与可命中，未在场景中点击
  （两者分别走 unchanged 的 `export_catalog` / `place_file` 流程，且会弹原生
  文件/导出对话框阻塞场景脚本）；服务级导出与放置由既有 MCP 工具覆盖。
- 在 VectorCraft 中编辑（handoff 启动官方 GUI）未在场景中点击，避免在用户
  桌面拉起程序窗口；该路径未改动（`canvas_handoff_editor`）。
- 实时 GUI 控制端口路径（live 会话叠加）未在本轮验证，与本次改动无关。
