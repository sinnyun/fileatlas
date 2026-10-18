# 插件独立浮窗修复（2026-10-18）

后续更正：本记录的 visible/最小化检查未证明前台激活。用户随后指出首次窗口在其他
程序后面，已补统一激活路径和真实 GetForegroundWindow 断言，见
[前台激活验证](../2026-10-18-native-foreground/README.md)。默认 EXE 已正常增量重建；
本记录早期另命名构建属于历史操作，不再沿用。

用户确认：选择“仅浮窗”时只打开插件独立窗口，不拉起主界面。
活动清单：[设置中心更正](../../reference-ui/concepts/plugin-settings-hub-2026-10-18.md)。
设置参考仍为 settings-clipboard-2026-10-18.png，插件正文布局保留。

## 实现

沿用 UI Bridge PanelCatalog/Cordis 生命周期，以 `open_presented(id,true)` 创建原生窗口请求。
Shell 只组合 GPUI Kit Root 和插件 factory；系统标题栏负责关闭/移动/缩放。
去掉隐藏主工作区的渲染分支，去掉系统快捷键派发前无条件恢复主窗。
两种方式切换仍为 Single 实例，独立工具可以并存。设置与所有用户数据格式/路径不变。
独立窗口位置和尺寸当前为会话内状态，不新增持久化。

## 环境与命令

Windows，150% DPI；同进程 `photag-shell --mcp`，隔离 `target/mcp-native-panels`。
原有 photag-shell.exe 正运行，无法覆盖，未停止用户进程。使用独立命名的构建：

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p photag-shell
cargo +1.97.1 rustc --locked -j 2 -p photag-shell --bin photag-shell -- -o target/debug/photag-shell-native-panels.exe
cargo +1.97.1 test --locked -j 2 -p photag-ui-bridge -p photag-plugin-shortcuts
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-native-panels.exe --profile target/mcp-native-panels --plugins launcher,clipboard,screenshot,shortcuts,dev-test --scenario scripts/validate-native-panels.cjs --allow-mutations
```

第一轮39步通过：主窗最小化保持、两个独立窗口、重复唤起复用、Escape、Cordis停用关闭、
重新启用、切回工作区浮窗恢复主窗、再切独立时主窗内容保留。截图已实际打开检查。
UI Bridge 19测试通过，包括新增两项独立/嵌入切换、并存及清理测试；shortcuts没有单元测试。
第二轮补验最小化目标窗重唤起、实际指针切换捕获、较小窗口、截图管理器、WM_CLOSE，结果补记下文。

主题守卫（含self-test）、refresh、architecture通过。layering检查仍报告现存
launcher/clipboard(L3)依赖settings(L4)两项；本次没有新增依赖边，保留该既有问题。
静态检查不代表运行交互通过；Windows真实系统快捷键派发仍未验证，MCP调用同一命令处理链。

## 视觉对照与边界

- 独立剪贴板窗口保持三栏与完整编辑底部操作，无空白主窗外框；启动台保留分类/网格/信息三栏。
- 主窗可见时保留顶栏、侧栏、图库、信息栏；工作区+浮窗继续原有遮罩和公共浮窗。
- 独立窗口改用系统深色标题栏，属于用户要求独立原生窗口的针对性差异；插件正文沿用公共主题。
- 没有在本轮全面复验所有插件控件、菜单、颜色预设和全部窗口尺寸。系统级按键抢占/从其他软件触发仍待验。


## 最终结果

最终构建的64步场景完整通过，退出码0；同进程摘要见[summary.json](summary.json)，
可复现场景见 `scripts/validate-native-panels.cjs`，完整本机逐步报告在 `target/mcp-native-panels/report.json`。
最终源代码的fmt、locked Shell check、独立命名EXE构建与19测试均通过。
构建保留既有 LNK4098 LIBCMT 链接警告；指定-o的两项Cargo输出命名警告不影响生成EXE。

| 检查 | 结果与证据 |
|---|---|
| 主窗最小化打开剪贴板 | 原主窗保持minimized，独立HWND属于本进程且visible；[剪贴板](clipboard-native.png) |
| 目标窗最小化后重唤起 | 恢复同一实例，主窗仍最小化 |
| 实际控件操作 | 同进程指针点击暂停/恢复，真实capture_paused两次反转并恢复 |
| 多工具并存 | 剪贴板与启动台两个独立HWND；[启动台](launcher-native.png) |
| idle | 两次1.5秒等待之间，独立根视图render_count不变；不扩展为全部子视图/IO计数证明 |
| 较小窗口 | 请求1450×1050，系统/插件最小宽度使实际1522×1050；编辑底部完整；[截图](clipboard-compact.png) |
| 截图管理器 | [独立窗口](screenshot-native.png)；实际指针点击正文“关闭”成功；重开后WM_CLOSE成功；主窗保持最小化 |
| Escape / 停用 | Escape关闭目标；停用Clipboard经Cordis移除原生窗口；重新启用后可以重新打开 |
| 切回工作区 | 从native切为workspace，恢复主窗并沿用旧浮窗；[截图](workspace-restored.png) |
| 主窗内容保持 | 主窗可见时再开native，顶栏/侧栏/信息栏均保留，无空白替代；[截图](main-content-retained.png) |
| 设置说明 | [设置](settings-native-mode.png)保持两项策略、横向快捷键、固定底栏，说明明确不唤起主界面 |

最终截图均已实际打开核对。较小剪贴板窗口沿用插件原有自适应逻辑，隐藏“我的分组”创建区；
本次未调整该既有响应行为，不能据此宣称较小窗口下全部分组管理控件均可见。
默认尺寸下该区可见。改色、长内容、菜单、OS真实全局按键本轮不作新的通过声明。
