# WorkCrate 官方程序重构验收

日期：2026-10-21。环境：Windows x64，Rust 1.97.1；官方 VectorCraft 0.14.0 portable 已核验官方 SHA256。FileAtlas 基线为 `153016d`，开发分支为 `codex/workcrate-vectorcraft`。

## 实现及视觉对照

FileAtlas 保持 Rust / GPUI Kit / Cordis。项目关联真实 `.vectorcraft`，页面来自其原生 Artboard。浏览只读 FileAtlas 自己的目录与预览缓存；创建、同步、文字/形状物料栏、放置、保存和导出走 `vectorcraft-cli.exe mcp --headless`；重新关联同样走只读 headless 会话；只有画布内容编辑启动或聚焦官方窗口。离线 headless 化之后的复验见 [WorkCrate 离线 headless CLI 复验](../2026-10-21-workcrate-offline-cli/README.md)。

检查了原项目 `old/src/components/ProjectListView.tsx`、`PageMatrixView.tsx`、`AssemblyStudioView.tsx`、`studio/PageSidebar.tsx`、`studio/InspectorPanel.tsx`，以及既有三层概念图。实现保留项目卡片、页面矩阵、左侧页面目录、中央大画布、右侧信息栏的组织，使用 GPUI 主题和组件。实际同进程截图确认三层画面、官方图片预览和物料表单完整显示。自建裁切/文字画布编辑器已移除。

## 验证命令及结果

- `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`：通过。链接器存在 LNK4098（LIBCMT 与其他运行库冲突）警告，程序可启动；本次未调整无关的整个工作区链接配置。
- `cargo +1.97.1 test --locked -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui`：14 项通过，官方程序专用测试默认忽略；doc tests 无失败。
- `WORKCRATE_NATIVE_TEST_ROOT` 指向隔离目录，运行 `cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards official_native_round_trip -- --ignored --nocapture`：1 项通过。覆盖原生保存回读、中文物料栏、字段增删重排、官方图片放置、预览、PNG/PDF、重新关联和同路径重新关联后拒绝旧页面标识。
- `cargo +1.97.1 fmt --all -- --check`：通过。
- `scripts/check-architecture.cjs`、`check-layering.cjs`、`check-ui-theme.cjs`、`check-ui-refresh.cjs`、`check-feature-ledger.cjs`：全部通过。
- 新 `scripts/validate-workcrate-vectorcraft.cjs` 经 `scripts/fileatlas-mcp-debug.cjs` 驱动真正的同进程窗口，最终退出码 0。配置 Full，仅声明的高影响工具附 `confirm:true`。新建两张原生画板，放置真实 JPEG，填写中文字段，并通过测量到的控件进行点击、输入、保存；官方回读数量确为“3 套”。验证三层导航、工程复制、归档/恢复/删除、官方编辑器跳转及保存后刷新。预览完成后，1.5 秒间隔的被动 UI 快照完全相同。
- 导出结构以文件字节核验（本机没有 pdfinfo/pdftoppm/mutool，pypdf 导入不可用，因此不使用它们作为证据）：PDF 头 `%PDF-1.7`、2 页、`MediaBox [0 0 1190.5511 841.8898]`（A3 横向）；PNG 4961 × 3508、8 bit RGB，即 300 ppi。窗口画面通过同进程 MCP 截图检查。

## 复现

先设置 `FILEATLAS_VECTORCRAFT_EXE` 为官方程序路径，并确认没有用户自己的 VectorCraft 实例在运行
（场景不会结束官方程序）。官方 GUI 端口 17979 只在 `open_editor` 跳转与画布编辑时使用。

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-offline-20261021-r3 --plugins dev-test --access-level full --scenario scripts/validate-workcrate-offline.cjs
```

测试数据、原始 MCP 报告、截图和导出均只留本地，不提交二进制证据：

- 本轮离线证据：`.local/validation/artifacts/validation-profiles/workcrate-offline-20261021-r3/`，
  其中 `offline-result.json` 与 `offline-reopen-result.json` 记录流程结果，`screenshots/` 保留设置、
  项目、页面与预览截图，`artifacts/` 保留 PDF/PNG，`user-project-folder/` 保留原生文件。
- 官方原生往返证据：`.local/validation/artifacts/validation-profiles/native-roundtrip-run4/`。
- 离线 headless 链路的命令、实测尺寸与未覆盖项：
  [WorkCrate 离线 headless CLI 复验](../2026-10-21-workcrate-offline-cli/README.md)。
- 早期审阅复核记录：backend-review 与 final-review 的字段重叠、异常连接继续写入、关联失效、
  资源诊断丢失、MCP 保存不刷新问题均已修复，无剩余 Important/Critical 问题。

## 范围与限制

旧网页数据和旧 DesignCraft 页面不迁移。官方编辑器为独立窗口；外部修改后需保存并刷新管理器，多项目合并 PDF 不支持。样本 `001.vectorcraft` 在应用打开时报告外部图片链接缺失，相关诊断会阻止静默导出不完整内容；需在官方编辑器中修复资源，再明确重新关联验证。

原生文件选择器、覆盖确认弹窗、不同官方版本、复杂字体与跨进程同时切换文档的竞态未作全面覆盖。本次真实验证使用固定官方版本、隔离工程和已安装的中文字体。原生命令之间无法与人工操作构成跨进程原子事务；检测到目标变化或不确定响应会中止且不自动重放。
