# WorkCrate 缩略图、显示代理与导出性能

## 范围与环境

Windows，Rust 1.97.1 MSVC，正常 `target/debug/fileatlas-shell.exe`；同进程
FileAtlas MCP，独立 dev-test profile，Full 访问。截图 3286×1831，DPI 150%。
对照可执行文件来自提交 `277adbd` 的功能实现；验证工作区保留了另一个未提交的
project-tree 重命名改动，本次提交仅包含 WorkCrate 性能与相关构建配置。

活动参考：`workcrate-pages-concept-2026-10-20.png`、
`workcrate-editor-concept-2026-10-20.png` 及
`workcrate-simple-layout-2026-10-20.md`。保留三层导航、页面三列卡片、
180 左页面栏、310 右检查器和固定 A3（页内信息条 + 图片区）。仅现有页脚增加阶段信息。
已打开实际参考及本次运行截图逐项比较；不是通过编译推断视觉通过。

## 同条件对照

用 `scripts/create-workcrate-performance-fixture.py` 生成 3840×2160 JPEG，
同一链接图片放入 3 个 A3 页面。每次由项目管理返回页面管理，重复三次。
下表为该合成素材、本机开发版结果，不代表任意照片/磁盘或发布版的速度保证。

| 操作 | 优化前 | 优化后 |
|---|---:|---:|
| 项目页面管理打开（三次中位数） | 2451 ms | 125 ms |
| 单页 PNG 原图分辨率导出 | 12180 ms | 950 ms |
| 三页无损 PDF 导出 | 26242 ms | 4449 ms |

优化后打开的三次值是 124/125/126 ms，每次 3/3 持久缩略图命中。
首张大图到小预览约 1444 ms；同源图片后续导入约 305/308 ms。
没有保存优化前的导入计时，因此不声明导入加速倍数。第一次没有代理时仍须解码原图。
先显示 512 档预览，随后升级到设备像素所需的 2048 档（2048×1152），
最终 GPU RGBA 9,437,184 字节，对照原图完整纹理 33,177,600 字节。
放大图片/窗口会按可见密度请求更大档，已有清晰纹理不会被小预览替换。

PNG 对照前后都是 4267×3017，Pillow 逐字节比较 RGBA **完全相同**。
`pdfinfo`：3 页，1190.64×841.85 pt，识别为 A3。
不是通过降低导出分辨率获得上述加速。PNG Fast 仍为无损，但压缩文件大小可能不同。

## 实际流程与截图检查

| 项目 | 结果 |
|---|---|
| 页面管理布局 | 三列、页信息同一横行、页脚操作保持；暖缓存立即显示完整页面预览 |
| 持久缓存 | 另起新的 MCP/GPUI 进程，原 JPEG 临时改名不可访问，3/3 页面仍完整显示；编辑器未提前上传原图 |
| 当前页清晰度 | 原图恢复后仅所选页载入显示纹理；升级到当前可见密度，保留比例与清晰手柄 |
| 导出运行状态 | 同一 UI 导出任务连续采到第 1/2/3 页及渲染、编码、写入；按钮工作中禁用，页脚完整可见 |
| 完成/失败 | 完成显示 3/3、耗时；不存在的输出父目录明确显示“导出失败”，busy/阶段采样停止，不重新加载页面 |
| 既有编辑流程 | 13 项同进程回归通过：可见图片添加、普通 FileManager 拖放、固定 A3、拖动无逐帧写盘、四角手柄、等比缩放、裁剪/恢复、标记不改纸张、独立 PSD 链接、全项目图片更新、PDF/PNG、静止状态 |

运行截图/原始 JSON（仅本地，不提交）：

- `.local/validation/profiles/workcrate-performance-before/`
- `.local/validation/profiles/workcrate-performance-final-r2/`：performance-result.json、
  pixel-comparison.json、reopen-result.json；screenshots 中 pages-warm-cache、
  export-progress、export-complete、export-failure、reopened-with-original-offline、
  reopened-viewport-resolution。
- `.local/validation/profiles/workcrate-performance-regression/`：simple-layout-result.json，
  selected-handles、image-moving、image-scaled、image-cropped、main-file-dropped 等截图。

后台 MCP 导出使用 `export_pages background:true`，走与可见按钮相同的 UI 导出任务。
本次没有重新操作原生保存/选目录对话框，也没有实际进入 Photoshop；这些边界不因本次
优化变成已验证。未新增取消接口。主题/小窗口/原生浏览对话框本次未重新做全量验证，
正文布局未改；默认大小的新阶段行、禁用和错误状态已截图核对。

## 可重现命令

```powershell
python scripts/create-workcrate-performance-fixture.py
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-export -p fileatlas-plugin-workcrate-ui
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-performance-final-r2 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-performance.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-performance-final-r2 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-cache-reopen.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-performance-regression --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-simple-layout.cjs
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
```

构建通过；现存 LNK4098 默认 CRT 提示仍在，无新的编译错误。相关 9 个单元测试、
主题自检/主题/刷新/架构/分层/台账守卫及 whitespace/fmt 检查通过。
图谱 MCP 返回 Transport closed，本次按 AGENTS 允许的 fallback 直接读取本地 pinned 源码。
