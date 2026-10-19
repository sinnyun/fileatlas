# 用户截图中的缩略图颜色对比修复 — 2026-10-19

> 历史记录：下述混色/私有配色实现已被替换。当前徽标、标签及选中前景直接使用
> GPUI Kit 原生角色，见[原生主题及 Material 验证](../2026-10-19-native-material-theme/README.md)。

## 观察与实现

用户![reported-before.png](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/reported-before.png)中的文件格式角标为深底深字，图片底部紫/蓝标签和评分也有相同问题，左侧选中导航前景过浅。当前布局继续采用 PhoTag 0.2；配色遵循[语义主题清单](../../reference-ui/concepts/semantic-muted-theme-2026-10-19.md)，此次属于局部纠错。

根因：格式角标把深色 HSL 底色与无关的 `text_on_accent()` 配对；`accent_alpha()` 将 RGB 乘 alpha、等价混合到黑色，却被当成浅色胶囊背景；图片深色遮罩又使用亮色模式的深正文色。

修复：格式色集中到 Theme 并返回真实底色/前景对；标签使用不透明的卡片混色背景，文字与用户色图标分别依据实际底色校正；评分、播放与溢出计数采用图片徽标专用成对颜色；缺失文件提示使用危险色配对前景；选中导航使用按选中填充解析的文字色。格式字号由 9 提至主题细注字号 11。无需数据/持久化迁移。

## 实际运行与证据

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-thumbnail-contrast-final --plugins core --access-level standard --scenario scripts/validate-thumbnail-contrast.cjs
```

Windows，同进程 MCP 子进程；截图 3286×1831，150% DPI，client 2176×1183.33 逻辑像素。所有新文件、数据库、标签、评分、设置均在隔离 `target/mcp-thumbnail-contrast-final`；源照片未修改。原正常 EXE 被锁定时，核对本工作区路径后关闭，再使用原 target 增量构建。

场景在 profile 中生成黑/白/蓝色有效 PNG，复制已有渐变图片，创建其它扩展名文件以覆盖真实占位路径。通过生产插件 API 创建紫/蓝/白/黑标签、分配到四张图、设置四星，明确刷新以展示最终数据；不是把接口读取成功当成 UI 已更新。断言详情确有四个可见标签与四星。

| 检查 | 结果与图片 |
|---|---|
| 亮色格式角标、评分、三标签＋溢出，在黑/白/蓝/渐变图上 | ![thumbnail-light.png](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/thumbnail-light.png)，已打开检查；文字及图标可辨 |
| 暗色同一批文件、同一组覆盖信息 | ![thumbnail-dark.png](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/thumbnail-dark.png)，已打开检查；格式填充随模式切换，覆盖信息维持可读 |
| 缩略图 hover | ![thumbnail-light-hover.png](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/thumbnail-light-hover.png)；色彩配对保留 |
| 扩展名弹出筛选 | ![extensions-light.png](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/extensions-light.png)；行文、边框与底色可分辨 |
| 设置插件共享颜色 | ![tag-settings-light.png](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/tag-settings-light.png)；已打开检查，正文/选中导航/表单可读 |
| 实际角色对比度 | 来自与绘制相同的 Theme 函数；11 种格式、6 种标签色×普通/选中×文字/图标，以及评分/计数/导航共 114 次断言，均 ≥4.5:1，最低 4.51:1；详见 [contrast-summary.json](../../../.local/validation/artifacts/2026-10-19-thumbnail-contrast/contrast-summary.json) |
| quiet state | 静置一秒 Gallery snapshot revision 不变 |

格式、标签等元数据通过公共插件 API 写入隔离 profile 后，场景主动刷新 Gallery；本轮未验证每个外部 MCP 写入工具是否都自动刷新现有 UI，不能据此宣称该路径通过。

静态 fmt/check/build、theme ownership/self-test、refresh、architecture、layering、feature-ledger 检查通过。Layer map 补录前轮已新增的 shell-controls 插件（依赖仍只向公共桥接层）；没有放宽依赖规则。构建保留已有 LNK4098 默认库警告；architecture 报告已有两个无 Cargo 清单目录。

## 范围与剩余项

确认上述截图中的颜色缺陷已修复；没有重排界面。实际播放图标、缺失文件横幅未在此场景单独触发，配对已在代码修正；小窗口、键盘焦点、禁用状态、本轮未切换其它强调色。四种用户标签在主窗实际显示，黄/无色与选中胶囊通过实际角色函数的数值断言覆盖，不代替其独立交互截图。测试 profile 最终为亮色，普通用户设置未改写。
