# GPUI Kit 原生主题与 Material 生成插件 — 2026-10-19

> 后续用户要求种子使用开源色板，已由 GPUI Kit ColorSelect 替换文本框。
> 本文输入截图保留历史；当前交互与场景见[种子色板验证](../2026-10-19-material-color-picker/README.md)。

## 当前方案

用户要求：GPUI Kit 负责主题解析、应用和分发，Material 只负责生成配置。
此记录取代前几轮手写深浅色表、accent 插值和主题字段逐项覆盖方案。
结构继续遵循 PhoTag 0.2；配色层次按[低饱和度概念及检查清单](../../reference-ui/concepts/semantic-muted-theme-2026-10-19.md)。
没有重排主窗、导航或检查器。旧概念中的具体 RGB 不再是实现要求。

- 删除旧 `theme/components.rs` 配色覆盖表；唯一应用入口为 UI Bridge 的原生
  `ThemeRegistry → ThemeConfig → Theme::apply_config`。默认直接使用 Kit 内嵌主题。
- 全部插件及自绘视图的旧角色访问器读取原生 `ThemeColor` 的只读投影；
  新视图优先 `cx.theme()`。投影订阅原生 Theme 更新，没有第二套可配置 palette。
- 背景与前景按原生角色成对使用。修复选中导航、标签/格式角标、图片覆盖信息、
  剪贴板选中内容等的前景配对。拖放兜底图标也由调用者传入当前主题角色。
- 原生 Button 保留 Kit 的 selected/hover/focus 状态处理。验证曾发现叠加 `.hover`
  导致 GPUI `hover style already set`，已移除叠加，改用原生 primary/ghost/selected。
- 新 Cordis 插件 `photag.material-theme` 注册设置页与 MCP 工具，使用固定
  `material-colors 0.4.2`，默认 Neutral 低色度。生成原生亮/暗 ThemeSet JSON；
  无独立主题管理器，也不从插件修改 Theme。
- 正常数据位于 `%LOCALAPPDATA%/PhoTag/themes/`：`material.json` 和版本化
  `material-settings.json`。单文件原子替换、保留备份；两文件不是一个事务。
  Settings 保存 `theme_file` 与既有 `color_mode`，旧 accent 字段只保留兼容。

接口、错误与恢复、上游版本和许可证详见
[Material PORTS](../../../crates/photag-plugin-material-theme/PORTS.md)；
后续开发必须遵循 [ui-theme.md](../../ui-theme.md) 和 AGENTS.md 的原生主题规则。
主题检查脚本包含 theme 目录、新 UI 模块及所有 Rust 文件中的 Theme 修改检测。

## 环境与精确命令

Windows MSVC，Rust 1.97.1，主窗截图 3286×1831，缩放 150%；小窗 1510×1000。
使用原有 target/incremental。仅同进程 PhoTag MCP，隔离 profile
`target/mcp-native-material-ok`，Standard 权限；未改用户正常数据。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-material-theme -j 2
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-native-material-ok --plugins core --access-level standard --scenario scripts/validate-native-material-theme.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-native-material-ok --plugins core --access-level standard --scenario scripts/validate-native-theme-input.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-native-material-ok --plugins core --access-level standard --scenario scripts/validate-native-theme-restart.cjs
```

以上均通过。构建保留既有 LNK4098 LIBCMT 警告。架构检查通过（106 edges，debt 742）；
报告已有两个无 Cargo.toml 的孤立目录，不属于本次主题功能。

## 验证结果与截图

| 检查 | 结果 / 证据 |
|---|---|
| 原生默认主题与侧栏模式按钮 | Default Dark/Light 与框架名称正确；![native-default-light.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/native-default-light.png) |
| 设置页真实“生成并应用”按钮 | 指针点击后生成文件并即时切换；![material-ui-generated.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-ui-generated.png) |
| Material 原生角色一致 | 实际背景/强调色等于输出 JSON；![material-blue-light.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-blue-light.png)、![material-blue-dark.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-blue-dark.png) |
| 状态、主按钮、选中、标签与格式徽标 | 实际前景/背景对比断言通过，数据见 [native-theme-summary.json](../../../.local/validation/artifacts/2026-10-19-native-material-theme/native-theme-summary.json) |
| 不同种子/对比参数 | 蓝色 standard 与青色 0.5 均生效；![material-teal-light.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-teal-light.png) |
| 其他插件随主题变化 | ![clipboard-material-dark.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/clipboard-material-dark.png)、![launcher-material-light.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/launcher-material-light.png) |
| 实际输入无效种子 | 输入“你好”后显示错误，原 JSON 字节未变；![material-input-error.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-input-error.png) |
| 实际输入有效种子 | 输入 #8a7098 并点击生成，原生主题名称与保存参数更新；![material-input-applied.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-input-applied.png) |
| 原生下拉菜单 | 指针打开扩展名菜单，底色、正文、选中按钮可辨；![material-native-menu.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-native-menu.png) |
| 小窗 | 正文可滚动到两项主题动作，页脚保留；![material-small-scrolled.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-small-scrolled.png) |
| 重启 | 同 profile 新 MCP 子进程恢复青色生成主题、亮色模式及参数；![material-restart.png](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-restart.png) |
| 静置 | Gallery revision 在 1.2 秒内不变，无循环重载 |

两个自动回归测试通过：3 个极端种子 × 3 风格 × 2 对比值 × 2 模式，
通过原生 Theme::apply_config 验证 JSON 字段被识别及代表性文字对 ≥4.5:1；
另外验证 Unicode 无效输入、文件保留、备份和损坏参数恢复。

完整 MCP 步骤见 [integration-report.json](../../../.local/validation/artifacts/2026-10-19-native-material-theme/integration-report.json)、
[input-report.json](../../../.local/validation/artifacts/2026-10-19-native-material-theme/input-report.json)、[restart-report.json](../../../.local/validation/artifacts/2026-10-19-native-material-theme/restart-report.json)。
[material-example.json](../../../.local/validation/artifacts/2026-10-19-native-material-theme/material-example.json) 是此次实际输出的原生 ThemeSet。

## 覆盖边界与剩余差异

全程序颜色来源的迁移由代码与检查脚本覆盖；运行截图只证明上述具体路径。
未逐一打开每个功能的所有 hover/focus/disabled 状态，未验证原生系统文件操作或真实拖放。
原生默认主题的所有状态也没有逐项作 WCAG 声明；对比断言针对生成主题的代表性角色对。
用户媒体、标签保存色与 OS 图标属于内容数据，不把这些像素重绘成主题色。
小窗设置内容通过滚动访问，不承诺一屏展示全部。启动台部分系统图标截图时仍在加载。
脚本中早先的 legacy popover 动作并未打开分栏菜单，只有 input 场景的实际指针菜单截图
作为菜单证据。设置编辑草稿不会被外部 MCP 生成覆盖；重新启动后读取持久化参数。
