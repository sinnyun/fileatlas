# MCP 分阶段访问控制验证（2026-10-19）

FileAtlas MCP 设置页现在提供关闭、常规控制、完全控制三个互斥阶段。常规阶段开放界面操作和普通信息增删改；完全控制阶段还开放插件声明为高影响的工具，但每次仍要求 `confirm=true`。

## 实际运行验证

- 命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-access-stages-2026-10-19 --plugins all --access-level full --scenario scripts/validate-mcp-access-stages.cjs`
- 环境：Windows，150% 显示缩放，工作区 `fileatlas-shell.exe --mcp` 同进程窗口，独立 `target/mcp-access-stages-2026-10-19` profile。
- 结果：完全控制阶段的文件重命名请求缺少 `confirm=true` 时拒绝；带确认但目标 ID 不存在时到达真实 handler 并返回 `media item … is not indexed`，未产生文件变更。通过 MCP 指针输入切换为常规控制后，剪贴板新增成功，高影响重命名在 handler 前被拒绝。再切为关闭后，剪贴板新增被拒绝。设置页切换立即影响同一个 MCP 子进程，无需重启。
- 缺失字段默认验证：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-settings-default-2026-10-19 --plugins all --scenario scripts/validate-mcp-settings-default.cjs` 通过，新 profile 未写访问级别字段时读回 Off，普通写入被阻止。
- 旧字段迁移验证：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-settings-legacy-2026-10-19 --plugins all --legacy-mutations true --scenario scripts/validate-mcp-settings-legacy.cjs` 通过；旧值 `true` 读回 Full，且高影响工具仍要求单次确认。

## 画面

- 完全控制：[mcp-access-full.png](../../../target/mcp-access-stages-2026-10-19/screenshots/mcp-access-full.png)
- 常规控制：[mcp-access-standard.png](../../../target/mcp-access-stages-2026-10-19/screenshots/mcp-access-standard.png)
- 关闭：[mcp-access-off.png](../../../target/mcp-access-stages-2026-10-19/screenshots/mcp-access-off.png)

截图中设置页显示三个分段选项，当前等级采用主题选中态。三次画面分别对应相同设置面板内即时切换后的状态。

## 静态检查

`cargo +1.97.1 fmt --all`、`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -j 2 -p fileatlas-plugin-settings -p fileatlas-plugin-mcp -p fileatlas-contracts`、`cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`、`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs` 通过。架构守卫提示既存 orphan 目录 `fileatlas-plugin-media-playback`、`fileatlas-plugin-preview`，并报告 `edges=99, debt=712`；守卫通过。链接器输出既有 MSVC `LNK4098` 默认库冲突警告，构建成功。

历史双态验证 `2026-10-19-mcp-settings-toggle` 记录的是上一版布尔开关；当前状态与行为以本记录为准。旧设置键仍能读取：`false` 映射为关闭、`true` 映射为完全控制。
