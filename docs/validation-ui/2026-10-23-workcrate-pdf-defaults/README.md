# WorkCrate PDF 默认导出配置（2026-10-23）

用户要求使用默认 PDF 设置并关闭保留编辑能力。官方 VectorCraft 0.14.0
`document.exportPdf` 参数目录确认内置 `VectorCraft Default` 默认开启 preserveEditing；
选项覆盖预设。现有官方 MCP export 顶层不接受任意参数，因此通过 options 传入
`preset: "VectorCraft Default", preserveEditing: false`。其他配置未覆盖，所有 PDF 入口
共用 Boards 的 pdf_export_params。没有设置模型迁移或界面变化。

Windows、Rust 1.97.1 MSVC；正常 target/debug/fileatlas-shell.exe；同进程 FileAtlas MCP
隔离 Full profile + 官方 headless。验证命令：

```powershell
node .agents/skills/vectorcraft/scripts/vc.cjs params document.exportPdf --full
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
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-pdf-defaults-2026-10-23-pass --access-level full --scenario scripts/validate-workcrate-pdf-defaults.cjs
```

以上均通过，构建仅有既存 LNK4098。实际生成单页与双页 PDF，PyMuPDF 解析确认页数正确、
PDF 1.7、文本内容存在、attachments 为空、无 /AF 或 /EmbeddedFiles/EmbeddedFile 对象。
将双页 PDF 第一页渲染后实际查看，页首物料编号及物料内容清晰，纸面布局保持。
这验证关闭的是原生文档嵌入；PDF 本身仍保留官方默认的可搜索文本与矢量内容。

初次解析器 pypdf 因此机缺少 xml.dom.minidom 无法启动，改用已安装的 PyMuPDF；首次
文本断言未考虑 PDF 提取插入空格，改为空白归一化后重跑全部场景通过，无产品代码变更。
生成记录、原生文档、PDF、解析结果和预览均保存在
`.local/validation/profiles/wc-pdf-defaults-2026-10-23-pass/`。本轮未重测官方 GUI 实例导出路径。
