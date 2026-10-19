# QuickLook 原始文件查看替换（2026-10-16）

按用户最终要求实现：空格只向本机正在运行的 QuickLook 发请求；QuickLook 未运行或未安装时
弹窗提示用户自己启动或安装，PhoTag 不启动、不结束、不安装它。原独立图片、音频、视频查看器
及其变换/胶片栏/播放工具、设置 Toolbar 页和 MCP 控制已删除。图库卡片缩略图、
视频缩略图生成、右侧180逻辑像素预览、文件信息/标签/评分/历史和文件操作保留。
AI标签建议迁入 Gallery，自动分析继续受已有设置控制；普通查看不额外显示AI面板。

这是功能替换，没有新增视觉设计。主界面继续依据
`docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png` 和
右侧 `right-inspector-unified-edit-2026-10-16` 概念；错误提示使用已有GPUI Alert。
旧 `final-v5/02` 图片查看器、`05` 播放器及其控件不再作为本功能的验收目标。

## 环境和检查

- Windows，150% DPI；Rust 1.97.1 MSVC，locked，构建/测试 `-j 2`。
- 本机已有自定义目录 QuickLook，定位自正在运行的进程。
- 构建：`target/debug/photag-shell.exe`。隔离档案：
  `target/mcp-quicklook-release-check`；正常 PhoTag DB/settings/session 未改。
- `cargo +1.97.1 fmt --all -- --check`：通过。
- `cargo +1.97.1 check --locked -p photag-shell -j 2`：通过。
- `cargo +1.97.1 build --locked -p photag-shell -j 2`：通过。
- `cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-quicklook -p photag-plugin-settings -p photag-plugin-i18n -p photag-plugin-session -p photag-plugin-thumbnails --lib -j 2`：36通过，0失败。
- 自动分析入口调整后重新运行 Gallery 23测试通过；QuickLook缓存调整后重新运行
  QuickLook测试通过。`node scripts/check-ui-refresh.cjs`：通过。
- 工作区成员删除导致必须更新Cargo.lock；最终从原锁文件离线增量解析，保留其他
  依赖版本并移除不再使用的rodio/音频依赖，没有附带升级第三方依赖。

## 同进程 MCP

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-quicklook-release-check --scenario scripts/validate-quicklook-mcp.cjs --allow-mutations
```

脚本创建隔离PNG/WAV/真实H264视频/文本文件，所有应用输入和截图由同进程PhoTag MCP
完成。视频夹具由FFmpeg生成；它不是单独启动的PhoTag或通用UI自动化。
最终9组检查通过，完整压缩报告见 `runtime-report.json.gz`，摘要见 `summary.json`：

1. tools/list不再包含control_preview、control_playback。
2. 图片通过Gallery派发给QuickLook。
3. 音频通过同一入口派发。
4. 视频通过同一入口派发。
5. 普通文本通过同一入口派发，不受旧媒体类型白名单限制。
6. 实际GPUI鼠标聚焦和Space事件派发当前选择；路径含中文、空格、`&`、`$`。
7. 静止1.5秒期间QuickLook快照不变，完整保留日志数228→228，无重复调用/读/通知日志。
8. 暂时移走隔离原文件后出现真实中文错误弹窗；恢复文件并用Esc关闭弹窗。
9. 设置默认Appearance，Toolbar旧页请求被拒绝，截图无独立查看器工具栏设置。

另一次只读重启通过：`get_view_state`为未调用状态，`open_media`被修改权限保护拒绝。
客户端没有触发外部程序，也没有自动恢复旧内置查看器。

首次实现检测约4.2–4.3秒；改为快速定位并在内存缓存桌面EXE后，最终首次约1668ms，
其后四次派发约20–21ms。这里测量的是PhoTag服务调用/进程派发，**不是QuickLook首帧**。
没有成功结果伪装成播放/解码完成，`LaunchSnapshot`只记录派发结果。

## 2026-10-16 追加：空格到 QuickLook 窗口出现的延迟

用户反馈"按下按钮到 QuickLook 界面出现大约需要 1-2 秒"，并要求以项目代码和官方文档
定位原因，不接受推测。

根因（依据上游 `PipeServerManager`/`App.xaml.cs` 源码）：原先 PhoTag 每次都调用
`QuickLook.exe <路径>`，那是 QuickLook 的第二实例启动器；它必须先冷启 .NET/WPF 运行时
（实测 1.06–1.12 s），再通过单实例管道把请求转给已运行的实例。QuickLook 的第一实例本来
就在 `\\.\pipe\QuickLook.App.Pipe.<用户SID>` 监听，直接写入一行
`QuickLook.App.PipeMessages.Toggle|<路径>|\r\n` 即可，绕开启动器。

只向正在运行的实例写管道后的同进程实测：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-ql-pipe-measure --scenario scripts/validate-quicklook-mcp.cjs --allow-mutations
```

`window_ms` 只统计标题包含本次文件名的 QuickLook 窗口，避免沿用上一文件的残留窗口把
数字压低。`dispatch_ms` 含 MCP 调用往返，纯派发成本见 PORTS.md。

| 类型 | outcome | dispatch_ms | 按键/调用→该文件窗口出现 window_ms |
| --- | --- | --- | --- |
| image | pipe_toggle_sent | 16 | 173 |
| audio | pipe_toggle_sent | 33 | 148 |
| video | pipe_toggle_sent | 30 | 95 |
| text | pipe_toggle_sent | 17 | 610 |
| 真实 GPUI Space | pipe_toggle_sent | 32 | 192 |

删除自动启动后又复跑两次，确认同一实现覆盖"未运行提示"和"运行中派发"。QuickLook 刚启动
时（profile `target/mcp-ql-pipe-final`）：image 755、audio 158、video 98、text 625、Space 176 ms。
脚本补上"用同一入口再 Toggle 关闭自己的窗口"后再跑（profile `target/mcp-ql-pipe-close`，
10 组用例全通过）：image 37、audio 96、video 98、text 81、Space 254 ms。

由此修正上一版的判断：610/625 ms 不是文本插件天生慢，而是 QuickLook **首次加载该类型插件**
的开销；同一类型热身后为 37-254 ms。上表 image 的 755 ms 同理（QuickLook 刚启动）。
两次运行结束后的只读观测都是 `instance_listening: true`、`windows: []`，即 PhoTag 的写入
不影响 QuickLook 的监听，也不会把窗口留在桌面上。

9组检查全部通过；报告 `target/mcp-ql-pipe-measure/quicklook-report.json`、
`target/mcp-ql-pipe-final/quicklook-report.json`、`target/mcp-ql-pipe-close/quicklook-report.json`。
`cargo +1.97.1 fmt --all -- --check`、`clippy -p photag-plugin-quicklook --all-targets`、
`test -p photag-plugin-quicklook -p photag-plugin-gallery --lib`（27通过）、
`node scripts/check-ui-refresh.cjs` 均通过。

本轮造成的一次外部影响及纠正：调试脚本以"打开管道但不写入"的方式探测，触发上游读取
循环 `ReadLine()` 返回 null 后抛异常、`using` 释放服务流，导致用户的 QuickLook 进程存活
却永久失去监听（`\\.\pipe\` 枚举中无任何 QuickLook 条目）。用户完整重启 QuickLook 后恢复，
`pipe_probe.instance_listening` 实测为 true，5次派发后仍为 true。现在请求行在打开管道之前
组装完成，诊断探测只 `WaitNamedPipeW`、绝不建立连接；QuickLook 进程生命周期归用户管理，
PhoTag 只发请求，不结束/重启该程序。

仍未覆盖：QuickLook 外部窗口首帧解码/音质/焦点/DPI、Store 版运行时的管道派发、
QuickLook 忙碌（`WaitNamedPipeW` 250 ms）时重试的实际表现、真实未安装机器的提示。

## 2026-10-16 追加：删除自动启动，未运行时弹窗提示

用户实测反馈：PhoTag 运行中关掉 QuickLook 后再按空格，程序会把 QuickLook 重新拉起来，
窗口闪一次后彻底不再响应预览。按用户要求删除 PhoTag 端的全部启动逻辑，改为提示用户自己启动。

删除内容：`launch()`、`src/launch.ps1`（桌面 EXE 发现与启动、Store AppsFolder 激活）、
`QuickLookError::Launch`、缓存的可执行文件字段与 `desktop_request_sent`/`store_request_sent`
两种结果。现在 `crates/photag-plugin-quicklook/src` 里没有任何 `Command::new`/`spawn`，
唯一的外部动作是对 `\\.\pipe\QuickLook.App.Pipe.<SID>` 的 `WaitNamedPipeW` + 一次写入完整
请求行；Store 版 QuickLook 运行时监听同一管道名，因此照常可被派发。

新增 `QuickLookError::NotRunning`（管道无监听实例、被拒绝且非提权、写入失败时返回），
中文提示"QuickLook 没有响应。请先启动 QuickLook，然后再按空格查看文件；如果尚未安装，
请先安装 QuickLook。"，走 Gallery 既有 GPUI Alert；`PrivilegeMismatch` 仅在管道返回
`ERROR_ACCESS_DENIED` 时才做进程完整性比较。`WaitNamedPipeW`/打开各重试 3 次，避免快速
连按或单实例被占时误报；名字不存在时立即失败，不产生等待。

QuickLook 关闭状态下的同进程实测（profile `target/mcp-ql-nostart2`，截图
[quicklook-not-running-alert.png](../../../.local/validation/artifacts/2026-10-16-quicklook/quicklook-not-running-alert.png)）：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-ql-nostart2 --scenario scripts/validate-quicklook-mcp.cjs --allow-mutations
```

- 真实 GPUI 空格 → 弹窗提示：6 ms；命令入口 → 114 ms（含 MCP 往返）。
- `pipe_probe.instance_listening=false`、`windows=[]`、进程表中无 `QuickLook.exe`：
  PhoTag 没有拉起任何进程，也没有回退到别的查看器。
- 提示文案为中文，说明 `keys.json` → `msg-quicklook-not-running` 的翻译链在运行时生效。

顺带修掉一处同类缺陷：`keys.json` 里 `"{count} selected"` 被映射了两次（调试面板新增的
`msg-debug-selected-count` 与图库既有 `msg-529aacfdfd2b`），后插入者覆盖前者，使
`photag-plugin-i18n` 的目录测试失败并让图库计数变成"条"。已删除重复项，两处共用一条消息，
并全表扫描确认 778 条映射无重复。

检查：`cargo +1.97.1 fmt --all -- --check`、`clippy --locked -p photag-plugin-quicklook
--all-targets`、`build --locked -p photag-shell`、
`test --locked -p photag-plugin-quicklook -p photag-plugin-i18n -p photag-plugin-gallery --lib`
（23+4+5 全通过）、`node scripts/check-ui-refresh.cjs`、`git diff --check`。
用户启动 QuickLook 后复跑运行中派发：脚本在无监听实例时只跑提示用例，在监听正常时跑完整
10 组用例（含收尾关闭），两次运行的分段延迟见上一节。

## 截图与边界

- [图库缩略图](../../../.local/validation/artifacts/2026-10-16-quicklook/gallery-preserved.png)：PNG和视频卡片缩略图仍存在。
- [右侧预览](../../../.local/validation/artifacts/2026-10-16-quicklook/inspector-preserved.png)：同一预览、文件名、评分、信息/历史/扩展页仍存在。
- [真实错误提示](../../../.local/validation/artifacts/2026-10-16-quicklook/missing-original-alert.png)：沿用已有Alert，未创建新的查看窗口。
- [QuickLook 未运行提示](../../../.local/validation/artifacts/2026-10-16-quicklook/quicklook-not-running-alert.png)：中文 Alert；未启动任何外部进程，也没有回退到其他查看器。
- [设置](../../../.local/validation/artifacts/2026-10-16-quicklook/settings-without-viewer-toolbar.png)：原独立查看器Toolbar页已移除。

本机已安装QuickLook，未卸载用户软件来测试“确实未安装”的系统状态；NotInstalled
返回分支及中英文本已实现，真实未安装机器仍待验。Store启动/桥接、未运行的其他安装
布局、完整codec、QuickLook外部窗口首帧/音质/焦点/DPI及多实例兼容性未在此轮宣称通过。
位于任意自定义目录的便携版需要先运行，或放在PATH中；不会扫描磁盘寻找程序。
