# 进入目录与恢复窗口重复刷新修复

用户复现：进入目录后内容先出现、随后刷新重显；切到桌面再回到程序时再次刷新两遍。

## 原因与实现

- Gallery 的窗口 activation 回调无条件 invalidate + refresh，并重试监听，直接启动整目录枚举。
- 监听注册晚于第一次枚举；为补订阅空窗，注册后伪造 Any(root)，800ms 后又触发完整扫描。
- 背景 refresh 使用完整 reload 的列表重置流程；再次读取还默认退回前 200 行，导致重显、滚动或 rich 数据闪动。

改为先订阅、后由可取消的后台生产线程等待注册完成再枚举；取消伪造根事件。窗口焦点变化不触发 Gallery 重扫，现有监听始终工作。初次快照在当前枚举首批发布前不展示旧缓存清单。背景刷新保留列表、选择、滚动和当前 rich window，选择在新快照交付时验证。侧栏原有盘容量后台刷新保留；它不触发图库枚举。

布局对照仍为已接受的 sidebar-pages-preview-v2 与 Explorer 侧栏，本轮不改屏幕结构。[恢复后的实际窗口](after-activation.png)，[状态摘要](summary.json)。

## 验证

Windows / 150% DPI，Rust 1.97.1 MSVC，debug。当前用户窗口占用 target/debug/photag-shell.exe，正常 build 的最终替换报 os error 5。因此用相同增量依赖输出独立验证程序，没有终止用户窗口：

```powershell
cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-refresh.exe
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-watcher -p photag-plugin-mcp -j 2
node scripts/check-ui-refresh.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-refresh.exe --profile target/mcp-refresh-fix2 --allow-mutations --scenario scripts/validate-browser-refresh.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-refresh.exe --profile target/mcp-refresh-smoke --allow-mutations --scenario scripts/validate-browser-mcp.cjs
$env:PHOTAG_BROWSER_LARGE_DIR='F:\AI\geren\pho_cordis\target\mcp-browser-extended2\100k'
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-refresh.exe --profile target/mcp-refresh-extended --allow-mutations --scenario scripts/validate-browser-extended.cjs
```

全部通过。23 项自动测试包含新增“注册不产生伪事件、真实文件创建仍有通知”，并扩展当前 rich window 保留测试。测试日志 target/refresh-fix-tests-final.log。最初新增窗口测试误用未指定目录的 None 收藏首页，已改成 Month 的全结果窗口查询；没有放宽断言。

同进程 PhoTag MCP 通过其拥有的 HWND 执行实际最小化/恢复/前台激活；不是模拟回调或 OS 输入注入。450 个真实文件进入后持续采样约 4.8 秒，非空后未再归零且枚举 generation 始终为 1。三次最小化/恢复后 generation 均 1→1、query revision 均 2→2、rich offset 均 250→250，选择保留；window_status.active 验证实际前台状态。最小化期间新增文件恢复后为 451，再删除回 450，仍能更新，并保留当前 rich 窗口。

基本浏览/原生监听/缺失重链 8 项通过；十万真实文件扩展 11 项通过，首批约 788ms、完整约 10809ms（本机 debug 热缓存样本，非像素首帧或通用性能保证）。完整原始报告在相应 target 隔离 profile；正常 PhoTag 配置未参与测试。

仍未覆盖：强制 native/PollWatcher 初始化失败、断网/可移动盘阻塞的全部路径。后台等待可取消，但不能中断已进入的 OS 同步注册调用。当前已经打开的旧程序不会热更新，需退出后启动新构建。
