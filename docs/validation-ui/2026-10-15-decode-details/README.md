# 图片逐文件加载诊断（2026-10-15）

点击调试面板中的 `preview decode finished` 行，底部可滚动详情显示全部字段。
源文件、缓存路径、任务角色和job_id帮助分辨当前图片与胶片带预取任务；首帧/完整图
日志也带源路径、打开请求generation和尺寸，可以按路径对应。

| 字段 | 含义 |
|---|---|
| source / cache_path | 源文件与生成缓存的完整路径 |
| format / file_bytes | 扩展名提示的格式、文件字节数；损坏内容不保证符合扩展名 |
| source_dimensions / output_dimensions | 解码器读到的源尺寸、输出缩放后的尺寸 |
| role / max_dim / urgent | 胶片带96或渐进1600任务、目标长边、入队时优先级 |
| cached / written / status | 是否命中有效缓存、缓存是否可用、成功或失败 |
| decoder / fallback_error | JPEG DCT、通用或缓存命中路径，以及快速路径回退原因 |
| queue_ms / cache_check_ms | 等待worker、元信息与缓存有效性检查 |
| read_ms / header_ms / decode_ms | 读取压缩文件、解析头、像素解码 |
| orientation_ms / resize_ms | EXIF读取/方向处理、最终缩放 |
| write_ms | JPEG编码、写缓存与发布文件，合并计时 |
| duration_ms / total_ms | worker处理时间（不含排队）、排队+处理总时间 |
| error_stage / error | 读取、解析/解码或缓存写入的失败阶段与原因 |

毫秒保留到0.001ms。缓存命中会跳过解码，未知尺寸为None，避免为日志增加额外图像IO。
orientation_ms包含方向读取的IO；阶段和可能比处理总耗时少一些记账开销。
JPEG回退时相应阶段累计两次尝试。generic为分离读取计时先读压缩数据，新增压缩输入
缓冲，既有像素与方向语义保持。读取耗时不是物理磁盘吞吐量测量。
首帧/完整图的duration_scope=open_to_paint是打开到绘制提交，与worker耗时不能相加，
也不是GPU物理显示时刻。日志不在轮询或每次绘制时重复发出；每个后台任务一对开始/结果。

验证：Windows、Rust1.97.1、同进程PhoTag MCP、隔离profile
`target/mcp-decode-details-verified`。24MP JPEG首次处理包含完整字段，重开命中缓存，
损坏JPEG产生ERROR并标出header阶段/具体错误。点击实际日志行并检查
[详情截图](decode-details.png)，后台任务结束后3秒观察期间preview记录数不变。
[完整原始报告](report.json.gz)记录结果与实际EXE哈希。

14项相关测试通过，包含PNG/BMP/TIFF/WebP与原open_oriented输出像素一致、JPEG缩放尺寸、
缺失文件读取错误、损坏文件解析错误。强制系统缓存写入拒绝未在本轮MCP中触发。

命令：
- `cargo +1.97.1 test --locked -p photag-plugin-preview -p photag-plugin-debug -j 2`
- `cargo +1.97.1 check --locked -p photag-shell -j 2`
- `cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-decode-details.exe`
- `node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-decode-details.exe --profile target/mcp-decode-details-verified --scenario scripts/validate-decode-details.cjs --allow-mutations`
- fmt、差异空白与UI刷新架构检查通过。

标准`photag-shell.exe`运行中导致构建覆盖被Windows拒绝，未终止用户进程。上述显式-o
构建了独立新版EXE并据此实际测试；Rust关于适配输出类型和忽略out-dir的两条警告由
该显式输出命令产生。标准入口仍为旧进程/旧文件，使用新字段需退出后运行新版。
保留已接受的Debug抽屉布局，未新增设计控件或正常数据/SQLite变更。
