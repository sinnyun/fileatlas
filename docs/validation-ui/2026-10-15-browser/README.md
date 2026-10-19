# 浏览模式改造验证（2026-10-15）

本轮按用户确认的“全系统浏览 + 文件夹收藏 + 标签快速跳转”实施。界面结构仍遵循 `sidebar-pages-preview-2026-10-14-v2`；新增顶部浏览/父级/刷新/子目录按钮及重新链接弹层。弹层使用既有通用模态框结构，50条分页；没有把参考图当作尺寸测量。图片、原有设置/播放/批量等未改屏幕的全部状态不由本轮证据重新覆盖。

## 运行模型

- **本地文件是事实来源**，默认只读直属目录；只有“显示子文件夹”才递归，并显示进度/取消。
- **内存SQLite会话缓存**提供现有查询/操作所需的真实行句柄；持久 **photag-tags.db**仅保留收藏、标签、已标记/评分文件。没有普通浏览文件的持久清单；旧photag.db不迁移、不删除。
- 收藏支持别名、排序、移除；点击/路径输入/选择目录都切换浏览位置。标签结果跨目录，支持定位并选择。
- 外部移动/删除只保留缺失记录；重新链接可指定文件或选择候选目录逐项审阅/应用/跳过；删除缺失记录重新核验，不删除本地文件。
- 图片识别快照保存在独立tag-previews；可重建缩略图使用版本键及5000文件/512MiB周期回收。工具菜单可禁用浏览缩略图。
- 后台目录线程和可见窗口读取均有界；刷新不销毁正在工作的ReadModel，旧解码结束后只运行最新需求。当前目录/可见窗口/文件版本共同拒绝旧结果。

## 环境与复现

Windows 11 Pro 10.0.26200，150% DPI；KIOXIA EXCERIA G2 NVMe SSD（F:）。Rust 1.97.1 MSVC，debug构建，locked、-j2。当前可运行文件为 `target/debug/photag-shell.exe`。运行窗口与MCP同进程，所有交互通过生产服务/GPUI事件，未使用通用OS输入自动化。隔离profile在target，普通PhoTag资料不参与运行场景。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
node scripts/check-ui-refresh.cjs
cargo +1.97.1 test --locked -p photag-plugin-database -p photag-plugin-gallery -p photag-plugin-navigation -p photag-plugin-tagging -p photag-plugin-watcher -p photag-plugin-history -p photag-plugin-file-operations -p photag-plugin-thumbnails -p photag-plugin-preview -p photag-plugin-session -p photag-plugin-metadata-export -p photag-plugin-i18n -p photag-plugin-mcp -j 2
```

运行场景：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-browser-smoke9 --allow-mutations --scenario scripts/validate-browser-mcp.cjs
$env:PHOTAG_BROWSER_LARGE_DIR='F:\AI\geren\pho_cordis\target\mcp-browser-extended2\100k'
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-browser-extended6 --allow-mutations --scenario scripts/validate-browser-extended.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-browser-extended6 --scenario scripts/validate-browser-restart.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-browser-extended6 --allow-mutations --scenario scripts/validate-browser-pointer.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-browser-extended6 --allow-mutations --scenario scripts/validate-browser-snapshot.cjs
```

首次不设置PHOTAG_BROWSER_LARGE_DIR时，extended场景会在自身隔离profile内创建10万个真实空文件；后续可指向既有隔离夹具。它不是dev-test虚拟媒体行。重启额外案例把隔离session中的旧selected_media_ids预置为无效值2147483647，保留selected_file_paths，再只读启动，确认忽略旧行号。

## 结果与证据

最终数值与场景清单见 [summary.json](../../../.local/validation/artifacts/2026-10-15-browser/summary.json)，截图见本目录PNG。完整MCP请求/日志/SQLite与exe留在对应target隔离profile，不能当作发布包。

- 静态fmt、locked check/build、刷新归属脚本通过；相关13个crate **62项测试通过，0失败**。包含内存浏览不持久化、标签重启、缓存淘汰/历史保护、重新链接冲突、缩略图版本与配额、原生监听事件及原有查询/分组/预览等回归。
- 未收藏目录直属浏览、明确后代范围、原生新增/删除/覆盖/新子目录、收藏别名/排序/移除保留标签、标签定位、缩略图禁用、缺失面板候选应用和清理通过。
- 10万真实直属文件完整结果、列表最后200行窗口、取消实际生产线程、连续切换到最后目录通过。当前目录保留全量紧凑结果，内存不是固定常量。
- 同路径PNG覆盖60×30→100×80，Preview generation更新并实际呈现full新尺寸，Gallery缩略图随后更新；独立识别快照仍为最初蓝色内容，缺失面板明确标“保存的预览”。
- Debug打开后静止观察2.5秒，原始日志latest_id/total不增长；没有隐藏日志。单次窗口有原生重绘不等于后台重复IO。
- 持久库检查仅1条version.png标签文件，1条标签关联，1条独立预览；10万未标记文件不在持久media表。重启无收藏扫描，收藏/标签仍存在；未标记选择按路径恢复，旧缓存ID无效不影响结果。
- 顶部子目录开关经过真实GPUI pointer down/up（项目MCP，逻辑坐标680,28）验证，开启/关闭与先前分组恢复通过。

性能计时从导航请求到GPUI已发布快照可读，**不是精确的首屏像素呈现时钟**。完整发布包含枚举/缓存/查询排序/模型交付，不能单独解释为原始磁盘吞吐。PeakWorkingSet为Windows进程历史工作集峰值，包含GPUI、插件和缓存，不是仅目录内存。样本为本机debug、热文件系统缓存，不外推为机械盘/网络盘或release性能保证。

## 排查与已修复的问题

- Gallery原有未接入编译的重复plugin模块使实际启动未取得Watcher，现统一实际入口。
- 新相对目录标题曾从正在更新的Gallery实体再次read，真实运行触发GPUI借用panic；改为传入快照范围。
- notify通用Modify事件、切换订阅空窗、长路径规范化和旧订阅线程卸载新位置问题均已处理。
- 缩略图缓存命中现在返回源header尺寸；更新了旧测试的0×0约定。旧监听回归保持原路径约定，浏览运行采用规范路径。
- 早期场景把XMP副文件当作多余项导致数量断言失败，现按真实本地文件计数；程序按要求显示这些文件。
- 旧缓存ID是否恰好变化与枚举顺序有关，测试现明确模拟无效旧ID，按文件路径核验恢复。

## 仍未覆盖的系统边界

原生文件选择器/回收站/跨卷移动的完整交互、全部图片视频codec、强制notify失败后的PollWatcher回退、断开的网络/可移动盘、访问拒绝和复杂junction的实机行为仍未逐项验收。分块取消能终止后续生产，不能强行中断已进入的OS阻塞调用或解码。上述路径已实现处理或沿用既有生产实现，但本轮证据不能证明全部Windows环境均通过。原有F0其他未验项继续保留。


2026-10-16 补充审查：未收藏目录重启与目录快照交付存在遗漏，已在后续[审查修复记录](../2026-10-16-sidebar-review/README.md)修复。上面的 pointer 坐标及截图属于当时窗口；新侧栏的点击证据以新记录为准。
