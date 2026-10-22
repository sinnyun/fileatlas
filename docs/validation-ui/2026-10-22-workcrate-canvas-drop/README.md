# WorkCrate 画布拖拽放置图片同进程实测

日期：2026-10-22。环境：Windows x64，Rust 1.97.1（显式 `cargo +1.97.1`），官方
VectorCraft 0.14.0（静默 headless），dev profile EXE
`target/debug/fileatlas-shell.exe`。验证配置
`.local/validation/profiles/wc-canvasdrop3`，场景
`.local/validation/validate-canvas-drop.cjs`（截图补拍
`.local/validation/shot-canvas-drop.cjs`）。

## 功能

页面编辑器（预览与编辑两种模式）的画布视口接受拖放：

- 从资源管理器等外部程序拖入：GPUI 外部拖放的 `ExternalPaths` 值，
  `on_drop::<ExternalPaths>` 进入与内部拖放相同的 `canvas_drop_files`。
- 从 FileAtlas 自身界面拖入：文件卡片的 `FileManagerFileDragPayload`（`allowed`
  且非 ctrl/shift 才落放）与文件暂存区的 `FileStagingDragPayload`。
- 每个图片文件走 `canvas_place(path, at, true)`：会话未开则静默开启，官方
  `file.place {link:true}` 放置，并登记为项目引用；多个文件按 24pt 阶梯错开
  放在画板中心附近。非图片（按扩展名过滤）提示“拖入的内容里没有可放置的图片。”，
  忙碌时提示“画布操作进行中，请稍候…”。
- 悬停高亮：`drag_over` 对三种拖拽值把画布视口边框改为 `accent_hover`。

## MCP 验证通道

GPUI 0.2.2 把 `ExternalPaths` 的构造封在 crate 内（字段 `pub(crate)`、无公开
构造器），进程内无法合成真实 `FileDropEvent`。因此新增语义等价通道：

- `control_ui {action:"canvas_drop_files", request:{"paths":[…]}}` 写
  `workcrate.canvas.drop-request` surfaces 键；
- WorkCrate 面板预览轮询（1.5s）消费并调用真实的 `canvas_drop_files`，
  与 on_drop 处理器共用同一入口；
- `workcrate-ui.state` 新增 `ui.drops` 计数器（每次接受的拖放请求 +1）。

真实 OS 拖放的 on_drop 接线与内部拖放同构（`ExternalPaths` 值类型匹配已在
代码层确认），但鼠标级外部拖放只能由用户手工验证；见“未覆盖”。

## 实测结果（wc-canvasdrop3，全部通过）

1. **预览模式（会话未开）拖入 `small.png`**：静默开启官方会话，官方修订
   2，`ui.canvas.mode` 保持 `preview`，落放对象 `kind=image`、`linked=true`。
2. **非图片拒收**：拖入 `.txt`，提示“拖入的内容里没有可放置的图片。”，
   官方修订保持不变。
3. **编辑模式拖入 `tiny.jpg`**：官方修订 2→3，落放对象 `linked=true`。
4. **`ui.drops` 计数**：3 次请求全部计入。
5. **截图证据**：`profiles/wc-canvasdrop3/screenshots/editor-preview-drop.png`
   ——预览画布中心可见 `small.png`（彩虹渐变）与其后错开 24pt 的
   `tiny.jpg`，均为链接放置后的官方渲染。

## 顺带修正

`boards.canvas_holds` 原在会话未开时返回
`Err("canvas editing is not open")`，与函数名语义相反；现改为
`Ok(false)`。原有两个调用点都先判 `canvas_is_open()`，行为兼容。

## 检查与回归

`cargo fmt --all -- --check`（通过）、`cargo check/test --locked -j 2`
（boards 31、ui 10、mcp 4 全过）、四项治理检查（theme/refresh/architecture/
ledger）通过。场景全程未触碰用户运行的官方程序（用户无 GUI 实例；测试自身
遗留的 headless 子进程按 CommandLine 核实后清理）。

## 未覆盖

- 真实操作系统级拖拽（鼠标按住从资源管理器拖入窗口）无法在进程内合成，
  `drag_over` 悬停高亮与 `on_drop` 的 OS 接线需用户手工拖放确认；
- 从 FileAtlas 文件卡片按住拖到画布的内部拖放（`FileManagerFileDragPayload`）
  未在本场景中走完整指针轨迹，仅代码层与 file-manager 现有落放面一致。

## 补充（同日）：跨卷/跨根拖入自动复制到 materials

用户实测拖入与文档不共享安全文件夹的图片时被官方拒绝：启动器只接受一个读
根，`automation_folder` 找不到非盘根的公共祖先时报
“CLI inputs do not share a safe folder…”，且失败的授权已经关闭了会话。

修复：新增 `boards.canvas_place_dropped(board_id, source, centre)`——先按原
路径走 `canvas_place`；失败于自动化根约束（`Invalid` 且消息以 "CLI inputs"
开头）时把源文件复制到 `<文档文件夹>/materials`（FNV-1a 内容戳去重：同名同
内容复用，同名不同内容加 `名-戳.扩展名`），重开静默会话后放置副本并链接，
副本登记为项目引用。可合法授予的来源保持原地链接不变。MCP `canvas_place`
语义不变。

实测（wc-canvasdrop4）：文档在 F: 盘 profile 内，拖入 C: 盘临时目录的
`small.png`——修订→2、`linked=true`，链接路径为
`…\documents\<id>\materials\wc-drop-….png` 且文件存在。boards 33 测试通过
（新增 `material_target`/`material_stamp` 两个单元测试），四项治理检查通过。
未覆盖：live GUI 会话中指向 GUI 读根之外路径仍按设计保持可见错误。
