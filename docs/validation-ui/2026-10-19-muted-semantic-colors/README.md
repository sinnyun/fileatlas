# 低饱和度语义配色与对比检查 — 2026-10-19

> 历史记录：下述手写 Palette 已由 GPUI Kit 原生主题取代。当前实现与验证见
> [原生主题及 Material 生成插件](../2026-10-19-native-material-theme/README.md)。

## 实现

- 参考：[低饱和度语义主题概念图](../../reference-ui/concepts/semantic-muted-theme-2026-10-19.png)及[检查清单](../../reference-ui/concepts/semantic-muted-theme-2026-10-19.md)。保留 PhoTag 0.2 既有主窗与插件布局。
- `photag-ui-bridge::theme` 统一管理暗/亮表面、正文、边框、状态、强调、按钮填充及前景色；所有功能 crate 的旧颜色常量读取改为动态语义访问器。
- Palette 改为低饱和度蓝灰和中性灰。危险、警告、成功、收藏、信息、分类和数据色均提供对应模式值及背景文字访问器。
- 新增 `contrasting_text(background)` 为实心按钮/徽标根据实际填充色选择前景；`readable_foreground(color)` 及 `accent_text()` / `danger_text()` 等为普通表面上的强调文字和状态文字保证至少 4.5:1 对比。
- 修正具体冲突：亮色弱化文字对比不足；警告徽标曾使用相同前景/背景；错误按钮使用普通正文色；用户标签色和强调色前景不能基于实际背景计算。状态文本和选中分段改为匹配其实际表面。

## 静态验证

- `cargo +1.97.1 fmt --all -- --check`：通过。
- `cargo +1.97.1 check --locked -p photag-shell -j 2`：通过。
- `cargo +1.97.1 build --locked -p photag-shell -j 2`：通过；Windows 链接器仍报告 LNK4098（LIBCMT 默认库冲突）警告。
- `node scripts/check-ui-theme.cjs --self-test`、`node scripts/check-ui-theme.cjs`、`node scripts/check-ui-refresh.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`：全部通过。架构检查报告两个已有的无 Cargo 清单目录。

## 同进程 MCP 检查

命令：

```powershell
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-muted-theme --plugins core --access-level standard --scenario scripts/validate-theme-contrast.cjs
```

MCP 进程在隔离 `target/mcp-muted-theme` profile 下扫描 `test-data/photag-library`，实际选择 README 文件；窗口 3286×1831 物理像素，150% 缩放。通过插件托槽的主题按钮切换暗→亮，打开 Settings → Appearance 检查浅色页，再切回暗色，并确认隔离 profile 最终保存为 dark。

场景读取应用实时主题 token，并计算正文/次要/弱化文字、强调文字、按钮前景与强调填充、危险/警告/成功按钮前景与底色、以及状态文字在凸起控件表面上的对比度；两种模式每一对都达到 WCAG AA 普通文字 4.5:1 下限。亮色弱化文字在首轮实测为 4.30:1，随即进一步加深后，最终场景完整通过。

截图：

- ![main-dark.png](../../../.local/validation/artifacts/2026-10-19-muted-semantic-colors/main-dark.png)
- ![main-light.png](../../../.local/validation/artifacts/2026-10-19-muted-semantic-colors/main-light.png)
- ![settings-light.png](../../../.local/validation/artifacts/2026-10-19-muted-semantic-colors/settings-light.png)
- ![main-dark-restored.png](../../../.local/validation/artifacts/2026-10-19-muted-semantic-colors/main-dark-restored.png)

## 验收范围

暗/亮主窗、实际选中文件状态、主题切换按钮、外观设置行及 token 前景/背景对比已验证。具体的警告/成功徽标与危险按钮、菜单悬停、Tab 焦点、禁用控件、小窗口状态以及运行时切换其它强调色预设尚未通过 MCP 单独演练；编译和对比度计算不能替代这些视觉状态检查。

## 后续更正

用户真实截图暴露格式角标、标签混色和图片遮罩前景仍有冲突：上文普通主题 token 对比通过不能代表这些实际组合通过。已在[缩略图对比纠错](../2026-10-19-thumbnail-contrast/README.md)修复并按实际角色/截图补验；相关视觉结论以新证据为准。
