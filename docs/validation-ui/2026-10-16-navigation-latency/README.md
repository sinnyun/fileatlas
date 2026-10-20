# 真实 D 盘目录进入延迟（2026-10-16）

本轮响应用户“使用系统其他硬盘的实际文件夹测试”。Windows / 150% DPI，
Rust 1.97.1 MSVC debug，同进程 FileAtlas MCP，所有配置、DB、缩略图和报告均在
`target/mcp-*` 隔离 profile。仅浏览真实 D 盘目录，没有对源文件执行写入、标签、
移动或删除。前后核对直接子项的名字、大小和修改时间，均保持一致。

## 目录与计时边界

| 实际目录 | 直属文件 | 子目录 |
| --- | ---: | ---: |
| `D:\D盘收集资料库` | 0 | 11 |
| `D:\桌面文件夹` | 55 | 9 |
| `D:\桌面文件夹\套图0306` | 12 | 4 |

`measure-real-navigation.cjs` 交替切换前两个目录10次，通过产品 navigate 请求
至 MCP `get_ui_state` 发布当前目录及非空内容计时，同时断言最终直属数量正确。
优化前中位722.0ms（575.1–732.8ms），最终常用构建中位67.4ms（48.5–82.0ms），
地址状态16.4–36.3ms。这是应用已发布的呈现模型时间，
不是相机测量的像素首帧，也不包含全部缩略图解码完成。目录预检查会预热文件系统
元数据；两边采用相同脚本，不能当作冷启动或所有硬盘的通用性能保证。

`measure-real-folder-click.cjs` 在真实桌面目录对“套图0306”文件夹卡片发送
GPUI 双击，5次从松开请求到目标内容发布：优化前563.5–783.6ms，
最终常用构建129.5–192.5ms。该时间还包含 MCP 输入投递的定时器开销。
原始报告：`target/mcp-real-click-before/real-click-timing.json` 与
`target/mcp-real-click-default-final/real-click-timing.json`。
开发中间构建为97.2–160.6ms，保留在 mcp-real-click-after，不替代最终样本。

## 原因与实现

此前 FileManager200ms动作轮询、Navigation100ms请求轮询、FileManager200ms范围轮询，
加上快照与可见窗口完成等待，产生了可叠加的固定延迟。

- UI Bridge 新增合并就绪信号 Wake 与视图持有的 listen Task。动作、范围变化、
  目录块、ReadModel结果就绪时调用原有变化采样器；无变化不 notify，无空闲新计时器。
- 浏览目录不再为旧资料库成员限制加载所有收藏与标签。真实目标仍在动作时校验。
- 生产者首批32项（包含子目录），后续2048项或有数据时32ms交付，保留4消息有界通道、
  单worker、取消与 generation 过期拒绝。
- 中间昂贵快照合并，至少50ms间隔；新范围第一批与最终完成直接交付。
- 回归发现并修复：标签历史不能用旧渲染树校验新标签；两批结果在布局完成前交付时，
  ListState 未测量行的滚动条估值不能用于恢复位置，改用逻辑锚点及已知行高。

### Yazi 精确源码对应

固定版本 `ffb2e78cc70341946c04befa52e14a4494015ce8`，MIT。

- [Actor::act/cd](https://github.com/sxyazi/yazi/blob/ffb2e78cc70341946c04befa52e14a4494015ce8/yazi-actor/src/mgr/cd.rs)：
  先替换当前目录/历史，再触发 refresh/render。保存原样参考至 vendor/yazi-reference。
  FileAtlas 沿用 Navigation 范围事件和 GPUI，通过就绪通知消除串联轮询等待。
- [VfsEntries::from_dir](https://github.com/sxyazi/yazi/blob/ffb2e78cc70341946c04befa52e14a4494015ce8/yazi-vfs/src/entries.rs)：
  后台枚举流、接收端关闭退出；继续复用已适配的 live.rs。
- [Refresher](https://github.com/sxyazi/yazi/blob/ffb2e78cc70341946c04befa52e14a4494015ce8/yazi-watcher/src/refresher.rs)：
  Part 事件分批、ticket与最新任务合并。FileAtlas 保留有界背压，未引入无界消息队列。

## 复现

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-before-fast-nav.exe --profile target/mcp-real-nav-baseline-final --scenario scripts/measure-real-navigation.cjs --allow-mutations
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-fast-nav.exe --profile target/mcp-real-nav-releasecheck --scenario scripts/measure-real-navigation.cjs --allow-mutations
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-fast-nav.exe --profile target/mcp-real-click-after --scenario scripts/measure-real-folder-click.cjs --allow-mutations
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-real-nav-default-final --scenario scripts/measure-real-navigation.cjs --allow-mutations
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-real-click-default-final --scenario scripts/measure-real-folder-click.cjs --allow-mutations
```

这里 allow-mutations 仅允许 MCP 导航/GPUI 控制，不执行源文件修改。
运行新比较时请换用空 profile；原报告为本次样本，不覆盖。

十万真实文件扩展场景全部11项通过：首批159ms，枚举完成17775ms，另验证末页、取消、
快速切换、真实监听、缩略图关闭及标签定位。完整枚举仍有大量元数据、排序和快照成本；
本轮不能据首批改善宣称十万项全量处理也变快。网络盘、离线盘、冷磁盘、杀毒软件干扰及
release性能尚未验证。

## 最终回归与构建

- `cargo +1.97.1 fmt --all -- --check`、`check --locked -p fileatlas-shell -j 2`、
  `build --locked -p fileatlas-shell -j 2`、`node scripts/check-ui-refresh.cjs` 通过。
- `cargo +1.97.1 test --locked -p fileatlas-ui-bridge -p fileatlas-plugin-navigation
  -p fileatlas-plugin-file-manager -p fileatlas-plugin-mcp -j 2`：42通过、0失败。
  包括就绪许可合并、无工作时等待、成功和panic完成通知及既有过期结果测试。
- 最终 fast-nav 构建：address-fixed profile 11项通过；refresh-fixed profile
  4项通过（含三次实际最小化/激活）；selfwatch-fixed 2项通过。
  对应脚本 validate-address-bar / validate-browser-refresh / validate-browser-selfwatch。
- 扩展场景空闲2.5秒日志 latest_id 536→536，地址栏稳定后查询修订和目录generation不变；
  自身profile浏览无空闲session写盘，外部新增后正常刷新并再次静止。
- 只读检查真实D盘最终profile的持久DB：media/media_tags/tag_previews各0行。
- 常用 `target/debug/fileatlas-shell.exe` 已成功更新，并通过上述D盘导航与真实双击复测。
  旧比较版本保存在 target/debug/fileatlas-shell-before-fast-nav.exe。

本轮没有新增界面布局，保持用户顶部地址栏与已接受面板结构；检查 address-fixed 的
address-breadcrumb.png，地址栏、三导航按钮、文件和预览结构正常。没有新增缺失控件。
先前失败的 address-diagnose 与 refresh-final 报告/日志保留在target，已以修复后的全新
profile重跑，未通过放宽断言来隐藏问题。聚合结果随 [summary.json](../../../.local/validation/artifacts/2026-10-16-navigation-latency/summary.json) 保存。
