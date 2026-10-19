# 开发基线验证证据（2026-10-14）

对应Git标记：`dev-baseline-2026-10-14`。用户确认的完整边界见[开发基线](../../development-baseline-2026-10-14.md)。

![当前实际界面](../../../.local/validation/artifacts/2026-10-14-baseline/main.png)

| 文件 | 内容 |
|---|---|
| [main.png](../../../.local/validation/artifacts/2026-10-14-baseline/main.png) | 最新源码构建经PhoTag MCP获取的真实窗口；层级线、选中蓝边、固定预览和恢复后的双侧宽度 |
| [layout-report.json](../../../.local/validation/artifacts/2026-10-14-baseline/layout-report.json) | 32步布局/目录/搜索/分页/宽度/小窗口/折叠结果 |
| [pointer-report.json](../../../.local/validation/artifacts/2026-10-14-baseline/pointer-report.json) | 16步同进程GPUI down/drag/up和真实页点击；左264→304、右300→340 |
| [restoration-report.json](../../../.local/validation/artifacts/2026-10-14-baseline/restoration-report.json) | 5步重启恢复304/340、默认只读拒绝pointer修改 |
| [layout-plan.json](layout-plan.json) | 对应布局操作计划 |
| [pointer-plan.json](pointer-plan.json) | 对应鼠标事件操作计划 |
| [restoration-plan.json](restoration-plan.json) | 对应只读恢复操作计划 |

报告保留原始exe/profile/截图绝对路径及本机运行时间用于溯源；这些target路径不随Git分发。完整本机截图/日志仍在target/mcp-sidebar-v2和target/sidebar-v2-*.log。Git中的main.png为最终实际画面；报告中的其他截图引用仅作本机历史证据定位。

重新检查时，在仓库根目录使用已安装的合适MSVC工具链（此主机+1.97.1），先locked -j2编译photag-shell，再用scripts/photag-mcp-debug.cjs --exe ... --profile target/<isolated-profile> --plan <plan>启动同进程MCP。layout/pointer需要--allow-mutations；restoration保持只读。原计划使用当时隔离DB的folder1/media7和12行数据；新档案须先通过生产MCP扫描样例、读取list_library/search_media并替换对应ID/数量。不得为复用计划修改正常用户库。测试库和exe不提交。

格式、Shell check/build和相关5crate tests25通过；精确命令和剩余边界见[界面验证记录](../../redesign-validation.md)最后v2节。GPUI内拖动/点击通过不代表OS窗口外捕获、所有媒体拖放或原生对话框已验。

本机已验debug构建的SHA-256/工具链指纹见[build-manifest.json](../../../.local/validation/artifacts/2026-10-14-baseline/build-manifest.json)，不要求其他机器的重建二进制字节一致。
