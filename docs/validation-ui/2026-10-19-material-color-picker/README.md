# Material 种子颜色：原生开源色板 — 2026-10-19

## 改动与参考

按用户要求，将 Material 页的种子文本输入替换为已有 GPUI Kit 的原生
`ColorSelect` / `ColorPickerState`，直接复用开源组件的多色阶色板和 HSLA 模式。
不增加依赖，不在插件维护 RGB 色表。已有依赖为 `gpui-kit 0.7.1`，实际组件
`gpui-component 0.7.2` / `gpui-base 0.7.2`，Apache-2.0；
[官方组件文档](https://gpui-kit.com/component/color-picker/)。

沿用[设置母版](../../reference-ui/concepts/settings-minimal-2026-10-18.png)及
[本次局部检查清单](../../reference-ui/concepts/settings-complete-2026-10-18.md)：
左侧导航、横向设置行、240px 右侧控件、分组、滚动与固定页脚不重排。
只有种子控制方式按用户要求改变。色值仍作为只读触发器说明；无需手输。
原生弹层内部保留上游的可选精确输入/滑杆，不另造颜色编辑器。

色板选择只更新草稿；悬停不提交。点击“生成并应用”时将选中颜色转换为 RGB 种子，
沿现有 Material → ThemeSet JSON → GPUI Kit ThemeRegistry 管线保存和应用。
订阅随 view 销毁。Material 不使用透明种子，选色器 alpha 不作为主题透明度。
数据格式和路径不变，详见[插件 PORTS](../../../crates/fileatlas-plugin-material-theme/PORTS.md)。

## 验证

Windows，150% DPI。主窗 3286×1831，小窗 1920×1200。隔离
`target/mcp-material-picker`，同进程 FileAtlas MCP，Standard 权限。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-material-picker --plugins core --access-level standard --scenario scripts/validate-material-color-picker.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-material-picker --plugins core --access-level standard --scenario scripts/validate-material-color-picker-restart.cjs
```

以上均通过。构建时正常关闭了经路径确认的工作区旧进程，再使用原 target 重建；
保留原有 LNK4098 警告。架构守卫通过并报告原有两个孤立目录。
本次未新增或运行单元测试。

| 项目 | 实际行为 / 证据 |
|---|---|
| 主控制器 | 色块、色值和下拉箭头替代文本框；![picker-page.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-page.png) |
| 暗色色板 | 实际指针打开，九组多色阶和精选色完整可见；![picker-palette.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-palette.png) |
| 选色草稿 | 点击内置 blue 500 后，服务参数及既有 JSON 未变；![picker-selected.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-selected.png) |
| 生成并应用 | 点击实际按钮后保存 seed #3b82f6，生成两模式并切换原生主题；![picker-generated-dark.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-generated-dark.png) |
| 亮色 | 实际侧栏按钮切换后再次打开原生色板；![picker-palette-light.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-palette-light.png) |
| 悬停取消 | 移至另一色块后 Escape，再生成，已保存 seed 仍为 #3b82f6 |
| 小窗 | 色板位于窗口内，所有色阶可访问；![picker-small-palette.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-small-palette.png)；关闭弹层后页脚和动作可见 |
| 重启 | 新 MCP 子进程恢复原 seed、生成主题和亮色模式；![picker-restart.png](../../../.local/validation/artifacts/2026-10-19-material-color-picker/picker-restart.png) |

脚本断言见 [summary.json](../../../.local/validation/artifacts/2026-10-19-material-color-picker/summary.json)，完整步骤为
[interaction-report.json](../../../.local/validation/artifacts/2026-10-19-material-color-picker/interaction-report.json) 与 [restart-report.json](../../../.local/validation/artifacts/2026-10-19-material-color-picker/restart-report.json)。
`validate-native-theme-input.cjs` 已转向本次色板场景；之前的文本录入截图属于历史证据。

## 范围

已检查上述实际点击及截图；没有逐项操作 HSLA 四个滑杆、全部色阶或辅助技术。
上游原生十六进制文本显示采用通道截断，个别通道与保存时四舍五入结果相差 1；
保存/重启参数断言保持 #3b82f6，此处未分叉上游组件以更改其格式化算法。
主题系统仍由 GPUI Kit 所有，新增色板不会重新引入插件私有主题。
