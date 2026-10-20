# 侧栏与浏览模式审查修复（2026-10-16）

审查基准：`1177e4e` 浏览模式快照，以及其后其他 AI 尚未提交的 Explorer 侧栏、设置、MCP、窗口尺寸修改。保留这些功能和布局。设计对照：项目根目录 `1792074142274.png`，以及已接受的 sidebar-pages-preview-2026-10-14-v2。逐项检查了调用、事件、持久化、后台读取和真实 GPUI 点击；没有把原记录中的通过结论直接当作本轮证据。

## 发现并修复

| 问题 | 修复与证据 |
| --- | --- |
| 快速访问折叠使用 `quick`，语义请求使用 `quick_access` | 统一键；截图与状态一致。 |
| 普通路径箭头调用驱动器展开逻辑，重复展开状态错位 | 删除重复状态，统一目录展开；真实 pointer down/up 连续四次通过。 |
| 重复添加快速访问覆盖自定义名称，未变化的编辑仍保存或不关闭 | 保留别名；无变化不发修订，确认后关闭编辑器。 |
| 分组折叠未进入发布签名，分区和分组状态未保存 | 完整路径/分组签名与 serde-default 会话字段；稳定后重启复核。 |
| 驱动器点击校验同步枚举所有盘容量，普通收藏编辑重复触发驱动器读取 | 校验仅用 GetLogicalDrives 位图；容量和已知目录均用后台 ReadModel，独立刷新版本，聚焦或手动刷新。 |
| 系统快速访问在 render 里检查路径是否存在 | 移至后台已知目录读取，渲染只消费快照。 |
| 盘容量失败显示 0/0，磁盘已用容量使用当前用户可用量 | 显示不可用错误；已用量使用总容量减实际剩余容量。 |
| 异步保存可能旧配置覆盖新配置，失败被吞掉 | 版本事件、串行保存，成功后才更新内存；旧事件拒绝，失败可重试并在界面显示。 |
| 非法编辑器路径把整个导航快照变成错误 | 错误留在编辑行；原收藏数据保持可读。 |
| 浏览模式未统计目录媒体数却显示 0；长路径选中判断不一致 | 隐藏未知数值；纯词法统一 Windows 扩展路径/UNC/大小写比较。 |
| 展开的当前侧栏目录未跟随本地新增子目录 | 当前目录事件/刷新重新排队读取；实际 mkdir 后 2→3 项。 |
| 旧恢复逻辑要求目录在资料库根内，任意浏览目录重启后退回收藏集合 | 浏览模式允许任意仍存在目录；重启等待稳定后再次验证。 |
| 相同会话反复写盘，刷新期间先写空选择再恢复，可能触发自身监听循环 | 相同状态不写盘，失败不改内存，刷新仅保存最终选择；浏览自身 profile 后稳定，外部文件变化仍刷新。 |
| 每个目录块替换异步快照键，首批结果可能一直失效 | 在途查询期间合并块变化，先交付完成的部分结果，再申请新版本；十万真实文件测试显式检查枚举结束前发布部分结果。 |
| 新 render 文件超过 800 行 | 分出 quick_access/favorites/editor，现有导航 view/render 均低于 800 行。 |

Shell 的工作区比例初始化和 MCP 截图坐标换算继续保留；本轮实际点击使用其 geometry。没有发现需要撤销这两项的证据。正常 `%LOCALAPPDATA%\\FileAtlas` 未用于验证。

## 命令和结果

Windows、150% DPI、Rust 1.97.1 MSVC、debug、增量编译。所有运行测试由 `fileatlas-shell --mcp` 与窗口共享进程，使用 `target/` 隔离配置。详见 [summary.json](../../../.local/validation/artifacts/2026-10-16-sidebar-review/summary.json)。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
node scripts/check-ui-refresh.cjs
cargo +1.97.1 test --locked -p fileatlas-plugin-navigation -p fileatlas-plugin-settings -p fileatlas-plugin-session -p fileatlas-plugin-file-manager -p fileatlas-plugin-database -p fileatlas-plugin-watcher -p fileatlas-plugin-tagging -p fileatlas-plugin-file-operations -p fileatlas-plugin-thumbnails -p fileatlas-plugin-preview -p fileatlas-plugin-history -p fileatlas-plugin-mcp -p fileatlas-contracts -j 2
```

相关测试 65 通过、0 失败；完整日志 `target/sidebar-review-tests-final.log`。包含并发设置、旧修订拒绝、失败重试、会话无变化不写盘等回归。静态检查和刷新归属检查通过。

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-sidebar-review4 --allow-mutations --scenario scripts/validate-sidebar-review.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-sidebar-review4 --scenario scripts/validate-sidebar-review-restart.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-sidebar-review4 --allow-mutations --scenario scripts/validate-sidebar-review-pointer.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-selfwatch-review-final --allow-mutations --scenario scripts/validate-browser-selfwatch.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-browser-review-smoke --allow-mutations --scenario scripts/validate-browser-mcp.cjs
$env:FILEATLAS_BROWSER_LARGE_DIR='F:\AI\geren\pho_cordis\target\mcp-browser-extended2\100k'
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-browser-review3 --allow-mutations --scenario scripts/validate-browser-extended.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-browser-review2 --scenario scripts/validate-browser-restart.cjs
```

这些脚本适用全新隔离 profile；重复基本场景须改 profile 名，避免夹具状态干扰。restart/pointer 必须使用先前生成的相同 profile。

- 侧栏真实保存失败通过把隔离 settings.json 临时替换为目录复现；失败可见、同修订重试成功。脚本 finally 恢复文件。
- 基本浏览、子目录开关、原生新增/删除、标签缺失、候选重链和清理，以及扩展的覆盖刷新、收藏移除保留标签、列表末页、取消与快速切换均通过。
- 修复首批饥饿后，一次十万文件测量首个已发布快照 791ms，完整 14683ms，峰值工作集 251396096 bytes。测量来自 mcp-browser-review2；最终带显式部分结果断言的重跑首批 793ms（8224项）、完整 13332ms、峰值工作集 247332864 bytes，见 summary。计时不是首屏像素时钟；debug、热文件系统缓存且可能有编译争用，不当作通用性能保证。
- SQLite 离线检查：十万普通浏览之后，fileatlas-tags.db 的 media/media_tags/tag_previews 各 1 行，均属于明确标记的 version.png。
- Debug 静止 2.5 秒 latest_id 保持不变，数值见 summary；没有隐藏重复日志。

截图：[快速访问折叠](../../../.local/validation/artifacts/2026-10-16-sidebar-review/quick-collapsed.png)、[保存失败反馈](../../../.local/validation/artifacts/2026-10-16-sidebar-review/sidebar-save-failed.png)、[实际箭头点击后展开](../../../.local/validation/artifacts/2026-10-16-sidebar-review/sidebar-pointer-expanded.png)。逐张检查了现有主窗口结构、快捷目录树、内联错误位置、收藏分组和已移除的虚假 0 计数。

## 尚未验收的边界

原生文件选择器输入、内联编辑器完整键盘焦点/快捷键流程、离线网络盘阻塞与可移动盘插拔、全部权限拒绝、跨卷移动/回收站和全部媒体 codec 仍未逐项实机验收。驱动器后台读取不会阻塞渲染线程，但不能中断已进入的 OS 同步调用；不能把本地盘验证扩展为所有网络环境通过。设置写盘仍沿用 JSON 文件语义，不宣称断电下的原子事务保证。原先 F0 其他未验项目继续保留。

最终改动后 Navigation/Settings/Session/FileManager 再次运行 33 项测试全部通过（target/sidebar-review-final-small-tests.log）。缺失图片独立识别预览复测通过，取消包含实际生产线程停止的观察值 92ms；见 browser-saved-preview.png 与 summary。
