# 独立插件窗口前台激活修复（2026-10-18）

用户反馈：快捷键打开的插件窗口留在其他窗口后面，需手动点击才置前。
新建窗口分支只创建 Root，漏掉前台激活；原重复打开分支才有激活。

现在新建/复用统一在窗口和 Root 就绪后 defer 执行目标窗口显示/恢复及 GPUI 平台激活。
仅对显式打开请求执行一次，主窗口不恢复；不设置永久 topmost，不在 quiet 轮询中抢焦点。
Windows 辅助函数只在最小化时用 SW_RESTORE，普通/最大化窗口使用 SW_SHOW。

## 命令与环境

Windows / 150% DPI，Rust 1.97.1 MSVC；复用已有 target 增量构建默认 EXE。
本轮构建前无运行中的工作区 PhoTag 进程，无需结束其他程序。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 check --locked -p photag-shell -j 2
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-native-foreground --plugins launcher,clipboard,screenshot,shortcuts,dev-test --scenario scripts/validate-native-foreground.cjs --allow-mutations
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-native-panels-foreground-regression --plugins launcher,clipboard,screenshot,shortcuts,dev-test --scenario scripts/validate-native-panels.cjs --allow-mutations
```

## 证据与边界

修复前同场景失败：Clipboard 首次创建后 visible=true 但 active=false。
仅补 ShowWindow/SetForegroundWindow 仍失败；接入 GPUI 平台激活后通过。
前台状态来自同进程 MCP 读取 Windows GetForegroundWindow，不根据截图标题颜色推断。
测试端只通过项目 MCP 调用生产命令处理链，不调用通用 computer-use 或测试端 OS 按键注入。
GPUI Windows 依赖的标准激活实现包含上游原有焦点兼容处理，未修改或复制该上游实现。

最终36步通过：剪贴板、启动台、截图管理器首次创建和最小化后重唤起均 active=true，
复用同一实例；主窗始终最小化。激活后等待1.8秒稳定，再等待1.2秒，根视图 render_count 不变。
初版测试仅等0.6秒仍有激活后续重绘，未记为 quiet 通过。
截图均已打开检查，正文布局不变；[摘要](summary.json)。

同一默认 EXE 另用隔离 profile 执行原独立窗口64步场景，全部通过（退出码0）：
实际指针暂停/恢复、较小窗、并存、Escape/正文关闭/WM_CLOSE、Cordis停用/重启、模式切换保持正常。

- [剪贴板](clipboard-foreground.png)
- [启动台](launcher-foreground.png)
- [截图管理器](screenshot-foreground.png)

fmt、locked Shell check/build、theme/refresh/architecture/feature-ledger 通过。
保留既有 LNK4098 链接警告与两个无 Cargo.toml 目录的架构警告。
没有在本轮执行真实 OS 全局按键，也未测试系统安全桌面或其他应用永久置顶窗口；
验证的是快捷键和 MCP 共用的生产打开处理链与真实前台结果，不能声明 OS 按键派发已验。
前台约束参考 [Microsoft SetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow)。
