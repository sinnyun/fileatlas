# 主窗图标工具栏与筛选栏右侧视图操作

依据本轮用户截图及[局部调整清单](../../reference-ui/concepts/main-toolbar-icons-2026-10-18.md)，
顶栏搜索位置/范围/Everything/工具改为图标；网格、列表、子文件夹与缩放移入筛选栏右端；
底部标签/更多移除。地址面包屑、输入框占位、筛选文字和状态反馈仍显示文字。
这是用户明确指定的定向布局调整，未另生成替代设计图。主题继续使用共享深蓝组件。

环境 Windows、150% DPI、Rust 1.97.1 MSVC。隔离 dev-test profile 为
`target/mcp-toolbar-icons-final`，进程32936，启动于2026-10-18T07:23:15.382Z。
EXE SHA256：`97299DEDEB512ECF80773F80B9C345371BB2DC256D3C0CB722D95B2F01E78992`。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-gallery --lib -j 2
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
git diff --check
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-toolbar-icons-final --allow-mutations --scenario scripts/validate-toolbar-icons.cjs
```

构建、fmt、24项 Gallery 测试与治理守卫通过。首次构建结束时覆盖EXE遇到 Windows
os error5（访问拒绝）；确认没有运行的 photag-shell 后重试构建成功。保留已有
LNK4098 链接警告与历史孤立目录提示，未降低工具链或修改架构基线。

运行全部通过同进程 PhoTag MCP 发送 GPUI 指针，无OS注入。测试生成的两个文本文件
仅在隔离 profile 内。真实点击 relocated grid/list 确认布局状态；子文件夹开关令
文件数1→2→1；真实滑块点击将Session保存尺寸150→269.2；当前目录/全机图标往返、
范围弹窗、工具图标均实点验证。静置900ms Gallery snapshot revision保持9。
[精简结果](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/toolbar-summary.json)，复现入口为 scripts/validate-toolbar-icons.cjs。

| 检查 | 实际结果 / 截图 |
|---|---|
| 顶栏图标化、筛选栏右对齐、底部清理 | [main](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/main.png)，已打开检查；原文字操作已移除，搜索/地址保留 |
| 选中与真实视图切换 | [list-selected](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/list-selected.png)、[subfolders-selected](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/subfolders-selected.png)，状态与内容一致 |
| 缩放、悬停提示 | [zoom-larger](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/zoom-larger.png)、[grid-tooltip](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/grid-tooltip.png)，已查看提示及尺寸变化 |
| 搜索与工具 | [scope-popover](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/scope-popover.png)、[tools](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/tools.png)，已查看弹层；[全机状态](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/search-entire-computer.png) |
| 1510×1000物理像素小窗 | [compact](../../../.local/validation/artifacts/2026-10-18-toolbar-icons/compact.png)，已查看；顶部全部图标和右侧视图/滑块可见，左侧筛选横向滚动布局 |

小窗地址面包屑按可用空间截断；左侧筛选不保证全部同时可见。未实测触摸板横向手势、
原生文件夹选取或 Everything 安装下载；相关操作回调沿用原入口。工具弹层沿用既有
透明背景；不把本轮图标替换描述为所有弹层视觉问题已解决。无新增数据库、设置模型
或后端行为变更，移除底部入口不删除右键/信息区的标签和文件操作能力。
