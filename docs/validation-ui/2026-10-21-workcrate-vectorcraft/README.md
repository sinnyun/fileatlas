# WorkCrate 官方程序重构验收

日期：2026-10-21。环境：Windows x64，Rust 1.97.1；官方 VectorCraft 0.14.0 portable 已核验官方 SHA256。FileAtlas 基线为 `153016d`，开发分支为 `codex/workcrate-vectorcraft`。

## 实现及视觉对照

FileAtlas 保持 Rust / GPUI Kit / Cordis。项目关联真实 `.vectorcraft`，页面来自其原生 Artboard；预览、文字/形状物料栏、保存和导出经官方程序控制通道完成。完整编辑定位到官方窗口。

检查了原项目 `old/src/components/ProjectListView.tsx`、`PageMatrixView.tsx`、`AssemblyStudioView.tsx`、`studio/PageSidebar.tsx`、`studio/InspectorPanel.tsx`，以及既有三层概念图。实现保留项目卡片、页面矩阵、左侧页面目录、中央大画布、右侧信息栏的组织，使用 GPUI 主题和组件。实际同进程截图确认三层画面、官方图片预览和物料表单完整显示。自建裁切/文字画布编辑器已移除。

## 验证命令及结果

- `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`：通过。链接器存在 LNK4098（LIBCMT 与其他运行库冲突）警告，程序可启动；本次未调整无关的整个工作区链接配置。
- `cargo +1.97.1 test --locked -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui`：14 项通过，官方程序专用测试默认忽略；doc tests 无失败。
- `WORKCRATE_NATIVE_TEST_ROOT` 指向隔离目录，运行 `cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards official_native_round_trip -- --ignored --nocapture`：1 项通过。覆盖原生保存回读、中文物料栏、字段增删重排、官方图片放置、预览、PNG/PDF、重新关联和同路径重新关联后拒绝旧页面标识。
- `cargo +1.97.1 fmt --all -- --check`：通过。
- `scripts/check-architecture.cjs`、`check-layering.cjs`、`check-ui-theme.cjs`、`check-ui-refresh.cjs`、`check-feature-ledger.cjs`：全部通过。
- 新 `scripts/validate-workcrate-vectorcraft.cjs` 经 `scripts/fileatlas-mcp-debug.cjs` 驱动真正的同进程窗口，最终退出码 0。配置 Full，仅声明的高影响工具附 `confirm:true`。新建两张原生画板，放置真实 JPEG，填写中文字段，并通过测量到的控件进行点击、输入、保存；官方回读数量确为“3 套”。验证三层导航、工程复制、归档/恢复/删除、官方编辑器跳转及保存后刷新。预览完成后，1.5 秒间隔的被动 UI 快照完全相同。
- 实际导出以 pypdf、Pillow、pdfinfo 和 Poppler 渲染检查：PDF 为 2 页 A3（1190.55 × 841.89 pt），按要求以第 2、第 1 画板的逆序输出；中文物料文字和数量正确。PNG/JPEG 均为 4961 × 3508 像素。最终 PDF 页面与放大后的物料栏均已视觉检查，图像没有覆盖物料栏。

## 复现

先设置 `FILEATLAS_VECTORCRAFT_EXE` 为官方程序路径，`VECTORCRAFT_CONFIG_DIR` 为隔离的官方配置目录，`WORKCRATE_NATIVE_PORT` 为测试端口。确认测试端口没有用户实例，允许 WorkCrate 启动隔离官方程序，或预先以官方 `--control` 及自动化读写根目录参数启动。

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/vectorcraft-integration/workcrate-ui-acceptance --plugins dev-test --access-level full --scenario scripts/validate-workcrate-vectorcraft.cjs
```

测试数据、原始 MCP 报告、截图和导出均只留本地，不提交二进制证据：

- 最终窗口 profile：`.local/validation/profiles/vectorcraft-integration/workcrate-ui-acceptance/`，其中 `native-validation.json` 记录流程结果，`screenshots/` 保留项目、页面、预览与表单截图，`artifacts/` 保留 PDF/PNG/JPEG。
- 最终后端 profile：`.local/validation/profiles/vectorcraft-integration/backend-final/`。
- PDF 渲染检查图：`.local/validation/workcrate-refactor/acceptance-pdf-preview.png` 与 `acceptance-header-detail.png`。
- 两轮独立审阅及修复复核：`.local/validation/workcrate-refactor/backend-review.md` 与 `final-review.md`。此前字段重叠、异常连接继续写入、关联失效、资源诊断丢失、MCP 保存不刷新均已修复并复核，无剩余 Important/Critical 问题。

## 范围与限制

旧网页数据和旧 DesignCraft 页面不迁移。官方编辑器为独立窗口；外部修改后需保存并刷新管理器，多项目合并 PDF 不支持。样本 `001.vectorcraft` 在应用打开时报告外部图片链接缺失，相关诊断会阻止静默导出不完整内容；需在官方编辑器中修复资源，再明确重新关联验证。

原生文件选择器、覆盖确认弹窗、不同官方版本、复杂字体与跨进程同时切换文档的竞态未作全面覆盖。本次真实验证使用固定官方版本、隔离工程和已安装的中文字体。原生命令之间无法与人工操作构成跨进程原子事务；检测到目标变化或不确定响应会中止且不自动重放。
