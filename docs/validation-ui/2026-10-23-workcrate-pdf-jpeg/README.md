# WorkCrate PDF JPEG 压缩（2026-10-23）

用户要求新的 PDF 配置接近 JPG 输出。沿用官方 PDF 导出，彩色/灰度图片设为 JPEG high，
超过 300PPI 时双三次缩减至300PPI，压缩文字，继续关闭原生编辑文档嵌入。
文字和矢量保持 PDF 原生内容；透明图片由官方处理，其 JPEG 编码要求图片不透明。
这不是将整页变成 JPG，也不保证任意素材的 PDF 与 JPG 体积一致。无设置文件格式变化。

已读取官方0.14.0 `document.exportPdf` 参数目录确认上述选项。WorkCrate 图查询无结果后
定位共享 operations::pdf_export_params；所有 PDF 入口复用该处。未修改第三方源码。

Windows，Rust1.97.1 MSVC，正常 target/debug/fileatlas-shell.exe；同进程 MCP，隔离Full
profile及官方headless。用可复现的1200×800不透明PNG渐变噪声样本，真实导入原生画布后
分别导出单页、多页PDF及JPG，PyMuPDF解析并渲染PDF后实际查看画面。

```powershell
rustup toolchain list
node .agents/skills/vectorcraft/scripts/vc.cjs params document.exportPdf --full
# 修改前：仅默认预设 + 关闭编辑保留
$env:FILEATLAS_PDF_BASELINE='1'
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-pdf-jpeg-before-2026-10-23 --access-level full --scenario scripts/validate-workcrate-pdf-jpeg.cjs
Remove-Item Env:FILEATLAS_PDF_BASELINE
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards export_params_only_use_official_tool_arguments -- --test-threads=1
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-pdf-jpeg-after-2026-10-23 --access-level full --scenario scripts/validate-workcrate-pdf-jpeg.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-pdf-jpeg-text-2026-10-23 --access-level full --scenario scripts/validate-workcrate-pdf-defaults.cjs
```

以上通过，既有LNK4098仍在。修改前PDF图片为FlateDecode；修改后为DCTDecode，确认实际
JPEG编码。单页2,740,123→319,448字节，双页2,741,049→320,374字节；同页JPG2,074,006字节。
这是固定样本的结果，非普遍压缩比例承诺。两个PDF均无附件/原生文档，页数正确且文字可提取。
PDF预览图片和物料内容完整。另重跑纯文字单/多页场景通过。

所有生成数据、PDF、JPG、截图和解析报告保存在上述隔离profile目录，仅本地保留。
本轮未专门验证透明、灰度、高有效PPI图片或官方GUI实例；这些仍遵循官方导出实现，
不将该不透明RGB样本的结果扩展为全部素材覆盖。
