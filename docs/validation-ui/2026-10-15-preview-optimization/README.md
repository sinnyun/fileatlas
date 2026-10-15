# 图片打开前三项优化实测（2026-10-15）

已实现：dev PNG/DEFLATE依赖优化；胶片栏复用数据库现有图库缩略图；JPEG超过150ms仍未完整显示才申请一次渐进预览，PNG跳过重复1600px解码。切换/关闭使旧任务失效，完整图绘制后取消渐进任务。

同进程PhoTag MCP直接添加实际sanli 0515目录，11文件中7张PNG/JPEG各首次打开及重开。每轮启动时使用全新应用缓存，索引完成生成7张图库缩略图后才打开预览。没有清除Windows文件缓存，指纹/扫描会预读文件；这是应用冷缓存，单轮观测而非均值或性能保证。

| 文件 | 原实现完整图 ms | 仅依赖优化 ms | 三项优化完整图 ms | 三项首帧 ms | 三项重开 ms |
|---|---:|---:|---:|---:|---:|
| 2026-08-10 222-26-13.png | 663.3 | 123.1 | 78.9 | 24.6 | 17.1 |
| 01 (10).jpg | 117.2 | 106.3 | 98.4 | 18.2 | 17.7 |
| 01 (3).png | 8045.4 | 451.7 | 422.1 | 21.0 | 17.1 |
| 01 (4).png | 460.5 | 68.3 | 65.7 | 19.2 | 18.0 |
| 01 (5).png | 295.9 | 43.5 | 53.6 | 17.9 | 17.9 |
| 01 (6).jpg | 324.5 | 272.7 | 268.9 | 22.3 | 19.3 |
| 01 (7).png | 1102.6 | 126.3 | 127.3 | 19.6 | 17.7 |

7200万像素PNG从8045ms降到422ms（约19倍）；仅依赖优化已降到452ms，说明主要收益来自原本未优化的PNG/DEFLATE。后两项的收益还体现在消除重复工作，不能把三轮时间的差值全部归因于某一项：存在调度、文件缓存和单次观测波动。

最终14/14完整绘制；5秒结束静止preview记录126→126。19个96px完成记录全部reused=true，read/decode/resize/write均为0；没有PNG1600任务。仅一个JPEG1600任务实际启动，原生全图先完成，任务记录status=cancelled、cancellation_reason=full_image_ready、written=false；已进入解码器的计算仍运行到返回，不能强制中断。完整图不会等它结束。

全11个源文件SHA256/大小/mtime前后一致；未写标签/评分/媒体文件。源01 (6).jpg真实尺寸3584×1536，旧索引结果160×69是另一处索引尺寸问题，本轮不修改其数据语义。测试复用同进程RAM和磁盘缓存，不能把重开结果视为仅磁盘缓存收益。

## 构建及复现

Windows MSVC Rust1.97.1，bounded jobs2，dev profile/incremental；未改release配置或升级依赖。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 test --locked --workspace -j 2
cargo +1.97.1 test --locked -p photag-plugin-preview -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
node scripts/check-ui-refresh.cjs
$env:PHOTAG_REAL_FOLDER = 'D:\桌面文件夹\测试 - 副本\sanli 0515'
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-real-preview-final --scenario scripts/validate-real-folder.cjs --allow-mutations
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-preview-final-details-pass --scenario scripts/validate-decode-details.cjs --allow-mutations
```

重跑使用新的target档案，先写settings.json关闭auto_check_updates并开启show_filmstrip；实际目录脚本拒绝非空图片缓存。包装器设置PHOTAG_MCP_ALLOW_MUTATIONS=1，仅注册测试目录/驱动隔离窗口，不改正常PhoTag数据。

源码产物和原始MCP报告仅保留于target；用户实际图片截图不复制入版本化文档。

- mcp-real-folder-empty-20261015：24724ca0b2a3f7119d77b7cc2e542ca1f79370ab7730d8a9c1923d09835ee4c1
- mcp-real-png-opt-only：d2bea6d50168b3d95715461d58f9d4c2e131b103853ba0351ca2c40f6ffcb44e
- mcp-real-preview-final：f9100b5476cfb3af60f0b5c2ef7caef62ef0233c1f2e4fcd05569b5afdebc061

最终产物target/debug/photag-shell.exe的SHA256与最后一项相同。原始报告路径为target/<profile>/real-folder-report.json；首轮/重开截图为screenshots/first-target.png、reopen-target.png。

## 回归、界面及边界

workspace测试94通过/3既有忽略；最后增加“一次失败不随poll重试”后Preview复测11通过。
格式、locked shell构建、刷新所有权脚本、JS语法和diff空白检查通过。
缓存复用测试使用不可解码原文件，证明不会偷偷解码；另测缓存缺失/过期回退，PNG/BMP/TIFF/WebP像素语义、JPEG缩放和旧请求绘制隔离。

最终同一产物的合成24MP/坏图MCP补测通过：首次/缓存、七项filmstrip/前后导航/缩放/正反旋转/fit状态、四轮快速打开切换关闭、日志详情截图。
坏图静止3.5秒preview记录43→43，render_count12→12；最终3秒原始日志latest_id701→701，preview总数140→140。没有隐藏a11y消息后才计算的安静结果，raw查询显式hide_ui_refresh=false。
报告及合成详情截图：target/mcp-preview-final-details-pass/decode-details-report.json、screenshots/decode-details.png。

第一次合成补测在索引未完成时打开，确实走了缓存缺失回退，测试预期“已生成缩略图”不成立；修正为等生产扫描结束并读取最新thumb_path。第二次只是日志零值格式0.0与测试的0不匹配，修正为数值等价匹配。保留两次原始失败报告于target/mcp-preview-optimization-details及target/mcp-preview-final-details。最后通过记录没有替换/删除这些证据。

沿用已接受preview布局，参照final-v5/01-preview-playback及后续共享浮动框架；实际截图检查图像、底部工具栏和胶片栏可见、当前卡片高亮。没有新增设计控件或重设计布局。本轮不证明鼠标双击命中/拖动、所有媒体格式、强制文件写入拒绝或所有其他页面交互；本轮实际覆盖是上述生产预览路径。新鲜但损坏的已有缩略图不属于mtime检测范围；已运行解码器只能在返回后放弃后续编码/发布。
