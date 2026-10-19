# 画廊未保存版本提示点

用户要求：已创建版本的文件被修改后，用显眼颜色提示用户手动创建版本，避免混淆
旧版本、最新已存版本和当前未保存内容。

| 内容状态 | 提示点 | 历史页说明 |
| --- | --- | --- |
| 没有已存版本 | 无 | 原有创建版本入口 |
| 与当前所选版本一致（可为旧版） | 无 | 显示基准版本名称 |
| 匹配另一份已存版本 | 无 | 同时显示匹配版本与所选基准，不自动修改当前标记 |
| 不匹配任何已存版本 | 左上琥珀色实心点 | 未保存提示、基准名称、手动创建入口 |
| 检查失败/文件检查中变化 | 不误报未保存 | 暴露错误，等待实际变更/显式刷新重试 |

标记与蓝色选择边框、右上格式、底部评分/视频标记分开。悬停显示说明，点击选择
文件并打开历史页，不自动创建/覆盖/切换版本。网格与列表共用卡片；列表的点位于文件名前，避免与小缩略图格式角标重叠。属于原有画廊
小范围功能提示，沿用已确认的 sidebar-pages-preview v2 / Inspector complete v2。

## 实现

`VersionsService::file_state` 读取VersionInfo的稳定ID与当前标记，通过固定Lore API
`file.hash` 与指定revision的 `file.info` 比较内容hash/size。不会把最新分支头、
mtime、缩略图或可能含失败暂存的私有工作副本当作基准。读取前后源文件变化返回
未确认错误；未创建版本的文件在打开Lore/hash前返回。

Gallery `version_badges` 使用共享ReadModel对可见窗口最多200文件串行后台检查。
键包含快照代次、历史代次、数据revision与路径；合并在途、丢弃过期、缓存成功和
失败。渲染只读结果，poll只发布完成变化，不定时重新hash。

MCP新增只读 `get_file_version_state {media_id}`；`get_ui_state.gallery` 新增
`version_states`（当前/匹配ID、错误、badge_visible）和 `version_states_loading`。
测试入口 `scripts/validate-version-badges.cjs` 可通过 `PHOTAG_LORE_TEST_FILE`
使用原路径测试文件，最终恢复开始时内容和时间戳。

服务回归覆盖无版本、同大小不同内容、旧版未改、仅时间变化、匹配另一已存版本、
私有工作副本含未保存内容与源文件丢失。历史树的持久化当前标记改称“版本基准”，
避免文件已被外部修改时仍误称其为“当前文件”。

## 静态与服务检查

Windows 150% DPI；已检查 rustup toolchain list，使用已装1.97.1 MSVC。

- `cargo +1.97.1 test --locked -j 2 -p photag-plugin-versions -p photag-plugin-gallery -p photag-plugin-i18n`：31/31通过（Versions4、Gallery23、i18n4）。
- 新增“版本基准”文案后i18n4/4再次通过。
- `cargo +1.97.1 fmt --all -- --check`、`node --check scripts/validate-version-badges.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs` 通过。架构仍为edges83/debt711；两个已存在的无Cargo清理目录警告保留。
- Shell编译/链接完成，运行中的用户窗口占用 `target/debug/photag-shell.exe`，Cargo最后复制失败（os error5）。通过 `target/debug/deps/photag_shell.exe` 本次链接产物进行MCP实测；未停止用户窗口。LNK4098为现存链接警告。

## 同进程真实文件验证

使用 `D:\桌面文件夹\测试\临时文件\00.png`，每轮全新隔离profile，调用项目MCP。
未使用操作系统输入注入或通用computer-use。开始和finally恢复后的SHA256相等。

```powershell
$env:PHOTAG_LORE_TEST_FILE='D:\桌面文件夹\测试\临时文件\00.png'
$env:PHOTAG_BADGE_POINTER='297,184'
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-version-status.exe --profile target/mcp-version-badges-20261017-list-final --plugins dev-test --allow-mutations --scenario scripts/validate-version-badges.cjs
```

运行结果：

1. 无版本无标记；创建01后无标记。外部写入蓝色PNG后，Watcher自动更新为modified并显示琥珀点，不显式刷新。
2. MCP向真实GPUI窗口发送pointer move/down/up（297,184），显示中文悬停说明，点击选中00.png并打开历史页。
3. 网格/列表均显示标记；创建02后自动消失。版本数量不因标记/点击增加。
4. 从02切回01后无标记；重写01相同字节但改变时间，无标记。
5. 所选基准01，当前内容重写为02：matches_saved_version，无标记；历史说明同时列出01/02，不修改所选基准。
6. 同为363字节的蓝色/绿色PNG仍能识别不同内容；从01修改创建03成功，标记消失，版本数恰好3。
7. 空闲前后snapshot_revision均9，version_states_loading=false；当前窗口没有重复重载。

初次脚本把目录导航与搜索同时入队（单待处理请求被覆盖），第二次未归一化Windows规范路径前缀，均在修改文件前超时。脚本现等待导航消费并兼容 `\\?\` 前缀；随后两次完整流程通过。

界面仍沿用已确认的两侧分页v2/Inspector complete v2；仅新增左上标记和历史提示，未发现被本功能遮挡的设计控件。覆盖本地文件、真实Watcher、单个搜索结果、网格/列表、版本切换与点击。未把该实测扩展为网络盘、高频连续写入或大型媒体库吞吐结论。



最终列表避让调整后，再次用上述完整流程通过（profile `mcp-version-badges-20261017-list-final`）。
Shell locked check与治理/fmt复查均通过；本次成功链接产物复制为
`target/debug/photag-shell-version-status.exe` 可直接运行。

- [最终运行报告](../../../.local/validation/artifacts/2026-10-17-version-status/version-badges-report.json)
- [网格提示点](../../../.local/validation/artifacts/2026-10-17-version-status/unsaved-grid.png)
- [中文悬停说明](../../../.local/validation/artifacts/2026-10-17-version-status/unsaved-dot-tooltip.png)
- [点击打开历史页](../../../.local/validation/artifacts/2026-10-17-version-status/unsaved-history-opened-by-dot.png)
- [列表提示点（文件名前，避开格式角标）](../../../.local/validation/artifacts/2026-10-17-version-status/unsaved-list.png)
- [匹配其他已存版本与版本基准说明](../../../.local/validation/artifacts/2026-10-17-version-status/matches-other-saved-version.png)
- [保存后无提示点](../../../.local/validation/artifacts/2026-10-17-version-status/saved-no-dot.png)
