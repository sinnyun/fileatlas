# WorkCrate 拖选、原生文本设置和默认信息同步 — 2026-10-21

## 实现范围和参考

用户要求：双击文本后可拖选多个字；核对参考源码并增加文本设置；修复设置中默认信息未出现在编辑器。
保留 [三层/A3结构](../../reference-ui/concepts/workcrate-three-level-2026-10-20.md)，按
[文本扩展图和最新清单](../../reference-ui/concepts/workcrate-editing-tools-2026-10-20.md)
在原检查器内增加紧凑操作和展开区。已打开概念及本次同进程截图比较，不新设计主页面。

源码读查/映射：已固定的 DesignCraft `crates/doc/src/attrs.rs`、`item.rs`、`story.rs`、
`engine/src/cmd/text.rs`、`object.rs`、`ui-egui/src/panels/properties.rs` 和 `typography.rs`；
另核对 FigCraft `style_picker.rs`/`font_preview.rs`，PdfCraft `bulk_fields.rs`/`comment_props.rs`。
核心排版继续由 DesignCraft 处理，未引入其 egui 界面。PdfCraft 的表单/批注编辑不属于 A3 文本设置。

## 环境与命令

Windows，已安装 Rust 1.97.1；正常工作区 EXE `target/debug/fileatlas-shell.exe`。
MCP 直接启动同进程 GPUI stdio 子进程，无独立进程附着、OS键鼠注入或外部UI自动化。
隔离 profile：`.local/validation/profiles/workcrate-typography-final-2026-10-21`。
截图原始3286×1831，默认原生暗色主题；源图片/纸张/文字内容色不被改成主题色。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-export
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-typography-final-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-typography.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-typography-final-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-field-presets-auto.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-typography-inline-regression-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-inline-text.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-typography-final-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-preset-reopen.cjs
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
```

## 实际交互与视觉核对

| 检查 | 实际结果 | 本机证据 |
|---|---|---|
| 拖选与替换 | 原生指针拖选得到UTF-8范围[0,7]，框不移动；键入中文只替换该范围 | native-drag-selected-characters.png、typography-result.json |
| 原生字符设置 | 鼠标选区下划线实际生效；属性搜索tracking并填写90成功；后续输入/重载保留run格式 | native-character-and-frame-options.png、Rust测试 |
| 段落/框 | 真实右对齐、分栏2，信息条和A3外框保持原样 | native-paragraph-options.png |
| 完整属性目录 | 读取实际依赖默认模型：110字符+117段落+27文本框；serde省略的7项也可设置 | native catalogue断言；全部默认值按组送入原生命令成功 |
| OpenType | 原生字体特性与样式集来自type.openTypeFeatures，默认选择实际存在的项 | native-open-type-options.png |
| 默认信息自动保存 | 添加制作工艺/安装方式，每项两种默认信息，未点击保存；返回编辑器可展开和选择 | custom-fields-saved.png、field-preset-dropdown.png、presets-auto-result.json |
| 重启持久化 | 独立MCP进程重启后两字段四默认信息、下划线/字距和分栏仍在 | presets-and-native-styles-after-restart.png、preset-reopen-result.json |
| 连续添加 | 自动写入期间添加/删除保持可用；新值继续由有限队列写入 | 同一默认信息场景 |
| 布局/状态 | 原三栏/纸张结构保留；常用格式在字号/宽高下方，更多设置在右侧原滚动区展开 | 本次图与活动清单对照 |
| 空闲 | 设置值稳定后两次1秒UI状态一致，有限poll不因忙态/空接收触发通知 | 默认信息场景quiet断言 |

生成截图、完整report和JSON仅在本机profile下，未纳入Git：
`.local/validation/profiles/workcrate-typography-final-2026-10-21/screenshots/`、
`typography-result.json`、`presets-auto-result.json`、`preset-reopen-result.json`。
回归场景输出位于另一个上述profile的 `inline-result.json` 与 `screenshots/`。

## 修正经过与覆盖边界

- 取消输入态底层对象命中，原生Textarea接收按下和拖动；上沿/手柄仍操作框。
- 初次设置自动保存时保存忙态曾禁用下一次添加，真实场景未生成第二行；已移除结构操作的忙态禁用并重跑。
- 测试脚本一度随滚动读取移动的测量原点，误报按钮不可见；改为固定本次可见检查器范围。
- 一个单测曾把run边界的“前一个字符格式”误认为后一个字符；按源码char_format_at的caret语义修正采样位置。
- 全部属性默认值已通过原生命令，不代表每一属性/枚举/字体组合均逐个做UI操作；上述列的是实际操作代表项。
- 快捷字号/颜色保持整框语义；更多字符设置可针对选区，段落设置作用于相应段落，框选项始终作用于整框。
- 复合原生选项提供JSON值编辑。普通输入态能体现基础字体/装饰/行距/左中右对齐，复杂多栏、CJK、路径文字、两端对齐等以应用/结束输入后的DesignCraft排版和导出为准。
- 没有声称复刻所有上游发布应用模块：样式库管理、独立故事编辑器、PDF表单/批注管理不属于本次属性目录。
- 物理中文IME候选窗/组合输入、多显示器DPI切换、极窄窗口以及每一种高级排版组合未重复实测。
- 单次构建曾与自己的MCP子进程重叠导致EXE锁定，待该场景结束后沿用原target重新构建成功；未换EXE名、target或清理缓存。
- 最终四个crate18项测试通过；Shell构建成功，只有既存LNK4098 CRT提示。主题/刷新/架构/功能台账/分层守卫通过。
