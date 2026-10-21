# WorkCrate 离线浏览与官方 headless CLI 全链路复验

日期：2026-10-21。环境：Windows x64，Rust 1.97.1，官方 VectorCraft 0.14.0
（`C:\Program Files\VectorCraft\vectorcraft.exe` 与同目录 `vectorcraft-cli.exe`），
FileAtlas 基线 `39cea6b`，dev profile EXE `target/debug/fileatlas-shell.exe`。

## 结论

离线浏览链路（插件设置持久化 → 选择文件夹创建原生工程 → 后台 CLI 画板/物料/放置/导出 →
内部封面与目录/预览缓存 → 程序不可用时照常浏览 → 静默 UI → 关闭 FileAtlas 后重开仍走缓存）
在同进程 MCP 场景里完整跑通，官方 GUI 进程数量全程为 0。复验同时暴露并修复了三处缺陷。

## 缺陷与修复

1. PDF 导出参数被官方 CLI 拒绝（阻塞级）。离线重构后导出走 headless MCP `export` 工具，
   该工具 schema 为 `additionalProperties:false`，只接受 `artboard/format/options/outlineText/
   path/range/scale/selection`；代码仍按 GUI 命令目录发送 `useArtboards:true`，官方返回
   `-32602 unknown argument \`useArtboards\``，即离线重构之后每一次 PDF 导出都失败。
   现 PDF 只发 `range`（1 基页号，官方语义为每画板一页），栅格只发单 `artboard` + `scale`。
   参数键由 `operations.rs` 的 `OFFICIAL_EXPORT_ARGS` 固定，并有非忽略单元测试
   `export_params_only_use_official_tool_arguments` 守卫，无需官方程序即可发现同类漂移。
2. 重新关联原生文件必须要 GUI。`attach_document` 走 TCP 客户端，端口未监听时 2 秒超时，
   离线状态下报 `Io(TimedOut)`，与"只有画布内容编辑启动/使用 GUI"的边界冲突。现改为只读
   headless 会话（`open_file` + `doc_inspect` 路径校验），并让 GUI 与会话两条打开路径共用同一
   份 `native.rs` 打开报告策略。headless 的自动化读根目录只有原生文件所在文件夹，因此
   `missingLinks` 只有在链接目标确实不在磁盘上时才阻断；`warnings`、`modifiedLinks` 与真实缺失
   仍阻断并记录诊断，等待成功重关联后清除。
3. `official_native_round_trip` 长期失效。原用例假定"连接 GUI 后 `create_page`"，与现流程不符，
   依次失败于 `NoDocument`、`document.json … no document open`、`Io(TimedOut)`。现改为
   `create_project_document` 建文档、原生命令读写全部走 headless 会话、放置物料改用稳定副本
   （不再引用会被缓存回收删除的 `native-previews` 文件）、重新关联后用 `refresh_project`
   重建目录。该用例现在完全不需要 GUI，且不再泄漏 `vectorcraft.exe` 进程。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui
$env:FILEATLAS_VECTORCRAFT_EXE='C:\Program Files\VectorCraft\vectorcraft.exe'
$env:WORKCRATE_NATIVE_TEST_ROOT='F:\AI\geren\pho_cordis\.local\validation\native-roundtrip-run4'
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards --lib -- --ignored --nocapture official_native_round_trip
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-offline-20261021-r3 --plugins dev-test --access-level full --scenario scripts/validate-workcrate-offline.cjs
$env:WORKCRATE_OFFLINE_REOPEN='1'
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-offline-20261021-r3 --plugins dev-test --access-level full --scenario scripts/validate-workcrate-offline.cjs
node scripts/check-architecture.cjs; node scripts/check-layering.cjs; node scripts/check-feature-ledger.cjs; node scripts/check-ui-refresh.cjs; node scripts/check-ui-theme.cjs
```

结果：fmt 与五个守卫通过；WorkCrate 五个 crate 18 项通过、1 项按条件忽略、无失败；
`official_native_round_trip` 1 项通过；离线场景首轮与重启复浏览轮均 `PASS`，
7 项断言（设置持久化不启动程序、选定文件夹存原生文件、后台创建与导出、内部封面/目录/预览、
缺程序仍可浏览、缓存 UI 静默、无官方进程）全部成立。运行前后
`Get-Process vectorcraft,vectorcraft-cli,fileatlas-shell` 计数均为 0。
场景要求开始时没有任何官方进程，因此本轮由用户先手动关闭 VectorCraft/FileAtlas；
脚本不会结束用户的官方实例。

实测产物：原生文件 216,571 B；PDF 244,723 B（`%PDF-1.7`，2 页，
`MediaBox [0 0 1190.5511 841.8898]`，即 A3 横向）；PNG 5,868,027 B（4961 × 3508，8 bit RGB，
即 300 ppi）；`boards-cache.json` 1,705 B 与两张 `native-previews` PNG。
官方往返用例另产出 2 页 A3 PDF（313,002 B）与 4961 × 3508 PNG（1,010,633 B）。

## 证据位置（本地，未提交）

- 最终 profile：`.local/validation/artifacts/validation-profiles/workcrate-offline-20261021-r3/`，
  含 `screenshots/offline-plugin-settings.png`、`offline-projects-no-program.png`、
  `offline-pages-no-program.png`、`offline-preview-no-program.png`、`artifacts/headless.pdf`、
  `artifacts/headless.png`、`user-project-folder/离线原生工程.vectorcraft`、
  `workcrate/projects/<uuid>/boards-cache.json` 与 `native-previews/`、
  `offline-result.json`、`offline-reopen-result.json`。
- 官方原生往返：`.local/validation/artifacts/validation-profiles/native-roundtrip-run4/`，
  含 `documents/Native <ts>.vectorcraft`、`documents/placed-reference.jpeg`、
  `projects/<uuid>/{pages.pdf,page.png,replacement.vectorcraft}`。
- 官方 CLI 自述快照：`.local/vc-tools.json`（32 个 MCP 工具与 `additionalProperties:false` schema）、
  `.local/vc-commands.json`（792 条命令目录）、`.local/vc-export-pick.txt`。

## 范围与限制

本轮以 CLI headless 路径为准：创建、同步、物料栏、放置、导出、重新关联均无 GUI 参与；
`open_editor` 的 GUI 跳转/聚焦与 GUI 内画布编辑需要官方窗口在前台，不在本轮覆盖内。
原生文件选择器、覆盖确认弹窗、其它官方版本、复杂字体、跨进程同时切换文档
竞态仍未覆盖。本轮记录时后台写入仍用 `refuse_if_gui_has_document` 做尽力检查
（GUI 未运行即跳过，不保证捕获"官方窗口里同一文档未保存"的所有情况）。

> 后续更新（2026-10-22）：该拒绝分支已随画布编辑轮删除，改为 `official_session` 路由——
> 官方 GUI 在配置端口可达时写入落到该 GUI 自己的引擎，不可达时才用静默 headless；
> "GUI 与后台写入冲突拒绝"不再是产品行为。见
> [WorkCrate 画布编辑复验](../2026-10-21-workcrate-canvas-editing/README.md)。
