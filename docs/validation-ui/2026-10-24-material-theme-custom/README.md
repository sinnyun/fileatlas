# Material 主题自定义设置验证

- 页面采用工作台声明式设置页，主要颜色在默认区，高级 ThemeConfig 角色由开关展开。
- 同进程 MCP 截图：`.local/validation/profiles/theme-custom-verify/screenshots/material-settings-final.png`（本地忽略产物）。组件几何审查颜色仍开启；截图只覆盖弹窗上半部，需滚动查看其余行。
- MCP 生成验证：隔离 profile `.local/validation/profiles/theme-custom-service/`，命令入口 `scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/theme-custom-service --access-level standard --plan .local/validation/theme-custom-service-plan.json`。生成命名文件并解析确认双模式的手动颜色角色值。
- 自动验证：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`。
- 未覆盖：picker 弹层操作、逐一展开高级色角色、保存后 GUI 即时应用截图及小窗口布局。
- 旧版同进程 MCP 截图 `theme-custom-scroll/screenshots/settings-scrolled.png` 展示了后续被用户要求移除的覆盖按钮和应用复选框；以本文件下方 2026-10-24 更正后的界面为准。
- MCP `apply=true` 生成 `Apply-Theme-Test.json` 返回 `last_error=null`；应用发生在隔离 profile。窗口主题角色投影未暴露本次 apply 的逐角色快照，因此只确认服务调用成功。
- 最新构建 `.local/validation/profiles/theme-custom-latest/screenshots/material-custom-latest.png` 确认“使用生成值”的背景/正文/主色角色显示各自的 Light ThemeSet 预览色，不再共用错误的深色回退值。
- 高级角色开关 MCP 点击验证：`.local/validation/profiles/theme-custom-advanced2/screenshots/advanced-expanded.png`，确认高级区域可展开且原生角色列表可滚动显示。
- 2026-10-24 用户反馈后修正：主题行现在区分“生成/手动”并显示当前色值；更改种子、风格或对比度会刷新未手动编辑角色。删除覆盖当前主题与双操作，设置页只保留“生成并应用主题”，需输入主题名称；同名文件拒绝写入，避免覆盖自定义主题。
- 重新通过同进程 MCP 操作单一按钮，在隔离 profile `.local/validation/profiles/theme-settings-live-apply3/` 创建并应用 `Generated-Apply-Check-24.json`；文件可解析为含亮/暗配置的 ThemeSet，`settings.json.theme_file` 指向该新文件。完整运行报告、profile 和场景脚本位于 `.local/validation/profiles/theme-settings-live-apply3/`、`.local/validation/material-theme-apply-test.cjs`。几何审查色仍开启，设置页底部可见但存在文字挤压；颜色更新行为见单元测试，不据此宣告所有设置页几何通过。

![应用前，设置页滚动到主题操作区](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/before-apply.png)

![应用后，显示命名主题路径和单一应用按钮](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/after-apply.png)

### 截图复核与布局修正

使用新的隔离 profile 关闭组件审查描边后再次截图。发现主题名称的 `Text` 控件沿用 `w_full`，在声明式设置行内挤掉标签宽度，中文标签因此逐字竖排。UI Bridge 现在将文本控件限制为 `CONTROL_WIDTH_EXTRA_WIDE`，标签保持可读；安全提示放到主题文件分区标题，避免长说明占据标签列。最新图：

![主题自定义页顶部与生成色值](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/current-top.png)

![主题名称和单一应用操作区](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/current-save.png)

截图 profile `.local/validation/profiles/material-theme-current-review-final/`，颜色模式为亮色，`show_ui_audit_frames=false`、`show_typography_audit_colors=false`。保存区标签现为横排；主题色行可读且显示实际生成色值。内容在设置面板内滚动，顶部与保存区需分别查看，这是当前 SettingsPage 的滚动行为。
- 覆盖保护单测：对已存在命名主题再次生成会报错且原始字节保持一致。完整命令：`cargo +1.97.1 fmt --all`、`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`；`node scripts/check-ui-theme.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs` 通过。

### 背景表面色与设置卡片复核（2026-10-24）

用户指出配置页的大块浅白表面没有跟随自定义背景。原因有两处：Material 的核心 `background` 只写入基础背景角色，容器/弹窗表面继续使用生成器默认值；SettingsView 根卡片没有声明主题表面，显示了 GPUI 默认底色。现已让 `background` 同步设置原生 group-box、accordion、列表/表格标题、tab-bar 和 popover 表面角色，并给设置卡片绑定原生 Modal 表面角色。核心编辑标签现在说明会同步面板和弹窗。

通过隔离同进程 MCP 生成并应用荧光绿 `#7CFC00` 背景、打开设置页并截图：设置内容大面板、主工作区、顶部栏、侧栏分组卡片及底栏均清晰呈现荧光绿；深蓝正文与洋红主色仍分别保留。MCP 读取的 `native_background`、ThemeSet 两种模式以及 `group_box.background`、`popover.background` 均为所设颜色。截图来自 `.local/validation/profiles/material-theme-color-role-check/`；验证报告确认当前亮色主题与新主题文件一致。

- 设置面板截图：`../../../.local/validation/artifacts/2026-10-24-material-theme-custom/custom-colors-settings-fixed.png`
- 主界面截图：`../../../.local/validation/artifacts/2026-10-24-material-theme-custom/custom-colors-main-after.png`
- 命令：`cargo +1.97.1 fmt --all`；`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`（6/6）；`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`；同进程 MCP `scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/material-theme-color-role-check --access-level standard --scenario .local/validation/material-theme-color-role-check.cjs`。

### 2026-10-24 完整颜色角色面板复核

根据用户要求再次生成高辨识度颜色并使用完整主窗/设置面板截图逐屏核对。隔离同进程 MCP profile 为 `.local/validation/profiles/material-theme-color-role-check/`，当前输出 `themes/Color-Role-Role-Audit-06.json`。主背景 `#7CFC00`、正文 `#172554`、主色 `#C2185B`、强调色 `#00897B`、边框 `#7E57C2`、侧栏 `#FF00FF`、输入/浅表面 `#00D9FF`、卡片 `#9C27B0`、弹窗 `#FF9800`、普通按钮 `#FF1744`、成功 `#2E7D32`、警告 `#EF6C00`、错误 `#C62828` 均在完整主窗或设置颜色行中对应可见。亮/暗配置中的手动角色逐项与请求色值一致，当前亮色运行快照也与选中文件一致。

完整截图和角色快照均为本机忽略产物，位于 `.local/validation/profiles/material-theme-color-role-check/`：`screenshots/custom-colors-main-after.png`、`screenshots/custom-colors-settings-top-03.png`、`screenshots/custom-colors-settings-middle-03.png`、`screenshots/custom-colors-settings-advanced-04.png`、`screenshots/custom-colors-settings-bottom-04.png`、`color-role-summary.json`。高级区截图暴露底层 `base.*` 原先回退显示成同一主色；现已为 Material 主题生成对应的基础色/浅色值，并在 `every_native_color_role_has_a_generated_light_and_dark_preview_value` 中确保原生 ThemeConfigColors 的全部角色在亮/暗配置均有实际颜色。MCP 角色快照补齐各类状态按钮的背景、前景、悬停和按下色。

复核中发现顶部“按月分组”按钮虽然已选中仍为浅灰：GPUI Ghost Button 使用 `secondary.active.background`，原强调色映射漏掉了默认/次要/ghost 按钮 active 角色。现在这三类 active 背景也映射到用户配置的强调色；同一完整主窗截图中该按钮已显示强调色 `#00897B`，MCP 快照的 `native_secondary_active`、`native_button_active`、`native_button_secondary_active` 均为 `#00897B`。剩余浅灰是未激活控件的 hover 状态（如 `#EAE7E9`）和 Windows 标题栏，不是选中状态或大块内容面板。验证命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`（9/9）、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、`node scripts/check-ui-theme.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`。Shell 构建有既有 LNK4098 链接器警告但成功；主题、架构和功能账本守卫通过。MCP 场景：`.local/validation/material-theme-color-role-check.cjs`。

### 12 个夹具文件的全界面颜色审计（2026-10-24）

按用户建议，验证不再只依赖空文件夹截图：使用 `scripts/validate-material-theme-colors.cjs` 在隔离同进程 MCP profile 中生成荧光/高对比主题，扫描生产夹具目录的 12 项，并分别截取完整设置页、网格选中态、列表选中态和右侧文件信息面板。场景现在会断言亮/暗 ThemeSet 中每个手动角色都等于输入值，并断言默认、次要、Ghost 按钮的选中背景都等于强调色；任一角色不符即失败。报告与完整原始截图保存在 `.local/validation/profiles/material-theme-color-role-check/`。

![含 12 个夹具文件、选中项、侧栏和右侧信息面板的全窗口截图](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/populated-grid-selected.png)

![含选中行的完整列表与面板截图](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/populated-list-selected.png)

![主题设置页的颜色行、高级区和主题操作区域](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/color-settings-middle.png)

角色快照中所有实际表面均已跟随配置：基础/面板/凸起/音频表面 `#7CFC00`，卡片/内容区 `#9C27B0`，模态框 `#FF9800`，浅表面 `#00D9FF`，侧栏/轨道 `#FF00FF`。唯一保留的浅灰表面是未激活控件的 hover 状态 `surface_group_header_hover=#EAE7E9`；它不是默认面板底色或选中态。快照中的白色角色均为按钮/状态色上的前景文字或图标（`*_foreground`、`text_on_*`），用于与饱和色背景配对；窗口最上方白条是 Windows 原生标题栏，不属于 GPUI ThemeSet。

运行场景：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/material-theme-color-role-check --access-level standard --scenario scripts/validate-material-theme-colors.cjs`。结果：12 项可见，选中项为媒体 ID 1；亮/暗两套手动颜色校验通过，所有三个按钮 active 角色为 `#00897B`。当前构建使用 Rust 1.97.1；本轮代码修改只加强了可复跑 MCP 审计断言，没有修改运行逻辑。

### 2026-10-24 修正：允许覆盖同名主题

用户要求允许覆盖已有主题。主题名称仍用于目标路径；再次生成同名主题前，旧 JSON 会复制到 `<名称>.json.bak`，新主题先写入 `.json.tmp`，再替换目标文件。Windows 使用 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`，其他平台使用原子 rename。生成校验在任何文件写入前完成；无效种子/颜色不会改动主题文件。每次生成使用新的 ThemeConfig ID，确保当前进程的 GPUI ThemeRegistry 应用新配置而不命中旧缓存。设置页继续只显示一个“生成并应用主题”按钮，目标路径行说明覆盖与备份行为。

![Material 设置页中的同名覆盖与备份说明](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/overwrite-theme-settings.png)

同进程 MCP 场景 `scripts/validate-material-theme-overwrite.cjs` 在隔离 profile `.local/validation/profiles/material-overwrite-check/` 先对同一 `Overwrite-Theme-Check.json` 连续生成洋红和青绿色主题，再通过设置页输入同名并点击唯一的“生成并应用主题”按钮覆盖一次。断言确认每次都替换同一路径、`.json.bak` 精确保留上一个版本、`settings.json.theme_file` 指向目标文件，运行中的 Theme ID/accent 与最终主题配置一致。截图使用 `show_ui_audit_frames=false`、`show_typography_audit_colors=false`。报告：`.local/validation/profiles/material-overwrite-check/material-overwrite-audit.json`。

### 2026-10-24 统一按钮与文件/树行悬停色

用户指出顶部按钮悬停与左侧文件夹树行、中间文件夹卡片悬停分别使用强调色和原生 `list.hover.background`。Material 核心“强调 / 悬停色”现同步写入 `list.hover.background` 与 `table.hover.background`；共享 UI Bridge 的 `surface_hover()` 使用原生 list hover 角色，因此文件夹树行、文件夹卡片和表格 hover 与 Ghost 按钮强调色一致。高级角色仍可由用户明确覆盖。

同进程隔离 MCP 运行 `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/material-hover-unified-check --plugins dev-test --access-level standard --scenario scripts/validate-material-theme-hover-role.cjs`。场景以 `#FF00AA` 生成为主题并应用，断言亮/暗两种 ThemeSet 下 `accent.background`、`list.hover.background`、`table.hover.background` 三者值相同；报告 `.local/validation/profiles/material-hover-unified-check/material-hover-accent-audit.json` 通过。主窗截图在同 profile 的 `screenshots/material-hover-accent-hovered.png`，但该截图启用了组件审查描边，且初始中心文件夹视图为空，所以只用于检查 MCP 窗口仍正常，不作为两个悬停区域的视觉验收证据。Material 单测 9/9、Rust fmt、Shell build、UI theme/architecture/feature-ledger 守卫均通过；构建有既有 LNK4098 警告。

### 2026-10-24 全局八类语义色收敛

依据[八类语义色工作台概念](../../reference-ui/concepts/unified-eight-family-theme-2026-10-24.md)，主题设置现在只开放主色种子、中性表面、正文、次要、成功、警告、错误和信息八类输入；逐原生角色任意选择色已移除。Light/Dark 与按钮、表面、前景、图表和状态角色只生成这些语义类的色阶。`material-settings.json` schema 从 v2 升到 v3；v1/v2 的种子、风格、对比度被保留，旧主要/强调角色折入主色种子，可归类的高级颜色折入最近语义类，边框改用生成中性色。原有 ThemeSet 文件不自动重写，生成器下一次保存时输出新角色映射并继续同名备份。

同进程 MCP 使用 `scripts/validate-material-theme-colors.cjs` 和隔离 profile `.local/validation/profiles/material-palette-eight-audit/`，以高辨识度八类色生成并应用主题，读取亮/暗 ThemeSet 的全部八类角色，加载 12 个生产夹具文件，并在真实窗口截图中心文件夹卡片 hover、左侧文件夹树 hover、网格/列表、选中项、检查器和主题设置色板。证据、截图与生成报告均保存在该忽略目录。`scripts/validate-material-theme-hover-role.cjs` 另外断言两种模式的 `button.hover.background`、`button.secondary.hover.background`、`secondary.hover.background`、`accent.background`、`list.hover.background`、`table.hover.background` 完全一致；Material 测试覆盖八类上限、旧配置迁移、所有原生角色生成、共享 hover 原生解析与自定义填充前景对比。旧的高级逐角色截图作为历史记录保留，不再代表当前设置界面。

单测覆盖命名主题覆盖与备份、默认 `material.json` 覆盖、错误输入保留当前 JSON。检查：`cargo +1.97.1 fmt --all -- --check`；`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`（9/9）；`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`（成功，仍有已有 LNK4098 警告）；`node scripts/check-ui-theme.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs` 通过。首次构建因仍运行的 workspace EXE 锁定失败；按本地授权正常关闭经路径核验的进程后复建成功。

### 2026-10-24 全局 8 类语义色归并

Material 生成器保留为唯一的主题编辑/生成入口。设置页仅有八类可编辑输入：主色种子，以及中性表面、正文、强调、成功、警告、错误、信息色；不再呈现任意 ThemeConfig 角色选择器。每类由 GPUI 原生 ThemeSet 在亮/暗模式中派生必要的明暗/前景色阶，因此“最多 8 色”指八类语义色，不是只允许八个十六进制色值。文件树、文件夹卡片、表格与按钮 hover 现共用 accent 交互族，由 GPUI Kit/Bridge 读取，不由插件自绘。

验证：`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`（12 passed）；`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、`node scripts/check-ui-theme.cjs`、`node scripts/check-feature-ledger.cjs` 通过。Material 服务由同进程 MCP 在隔离 profile `.local/validation/profiles/material-palette-service/` 生成并应用 `Eight-Family-Audit.json`；Node 对 ThemeSet JSON 检查亮/暗两套七项手动角色，并确认按钮/列表/表格 hover 与强调色一致。该 profile 和脚本报告均在 `.local/validation/profiles/`。

首次 MCP 查询与 GPUI 主窗口创建发生竞态；场景现先轮询窗口归属和指针坐标，再截图。最终同进程 MCP 场景在 `.local/validation/profiles/material-palette-eight-final5/` 成功完成，生成并应用 `ColorRoleAudit…json`，扫描夹具 12 项，并截取主题设置、填充文件网格/选中态、文件夹卡片 hover 和未选中“桌面”树行 hover。手工色在亮/暗 ThemeSet 中均与输入一致；按钮、列表、表格 hover 角色一致。视觉核查确认整张文件夹卡片和树行都呈现同一强调 hover 色。设置页截图显示八类可编辑颜色及单一“生成并应用主题”操作。

此次实际截图发现 Bridge 的 `workbench_hover_surface()` 与 `workbench_tree_row()` 仍用 `rgba` 绘制不透明 hover 表面，文件夹图标区域也被静态卡片表面遮住。现在它们都使用共享原生 `surface_hover`；文件夹图标面板改用 Bridge 标准 `workbench_card`。重新构建后 MCP 截图确认文件夹卡片整块与未选中的树行显示一致的 `#FADBE0`，不再残留亮蓝或不同 hover 色。MCP 指针移动会排队到下一帧派发，验证脚本会等指针位置匹配目标后再截图。

验证命令：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 test --locked -p fileatlas-plugin-material-theme -j 2`（12 passed）、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、`node scripts/check-ui-theme.cjs`、`node scripts/check-architecture.cjs`、`node scripts/check-feature-ledger.cjs`。MCP 场景：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/material-palette-eight-final5 --plugins dev-test --access-level standard --scenario scripts/validate-material-theme-colors.cjs`。Shell 构建成功，保留既有 LNK4098 链接器警告。

截图产物（均在 git 忽略的 `.local/validation/artifacts/2026-10-24-material-theme-custom/`）：

![主题生成器设置与八类调色](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/palette-eight-settings-middle.png)

![文件夹卡片整块 hover 使用共享强调色](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/palette-eight-folder-card-hover.png)

![侧栏未选中树行 hover 与卡片 hover 一致](../../../.local/validation/artifacts/2026-10-24-material-theme-custom/palette-eight-folder-tree-hover.png)

其他设置顶部、保存区、主窗和选中文件网格截图分别为 `palette-eight-settings-top.png`、`palette-eight-settings-save.png`、`palette-eight-main.png`、`palette-eight-populated-grid-selected.png`。颜色来源盘点以 `node scripts/check-ui-theme.cjs` 为准：插件组件消费原生 Theme/Bridge 角色；用户标签颜色、真实媒体像素和 WorkCrate 的 VectorCraft 文档画布色属于内容数据，保持原值，不作为 FileAtlas 界面调色项。

### 2026-10-24 单色主题残留色排查

用户要求将颜色全部改为一个颜色并通过截图查找仍绕过主题的显示。为避免修改普通主题或用户设置，在独立 profile `.local/validation/profiles/single-color-audit/` 中复制一份有效 ThemeSet，将亮/暗两种模式的全部 140 个 GPUI 原生颜色角色覆盖成精确的 `#FF00FF`，并通过 `settings.json.theme_file` 启动同进程 MCP；Material 设置草稿中的主种子色和七个语义色也全部设成相同颜色。`single-color-state.json` 的 GPUI Kit ThemeRegistry 快照中，accent、背景、面板、卡片、边框、正文、状态色及所有 `roles` 都是 `#FF00FF`；主题色报告中带透明度的派生值也只保留洋红 RGB 通道。

截图结果：空工作区及设置页的应用表面统一为洋红色；夹具文件网格中保留的蓝/绿/黄等像素来自夹具实际缩略图内容；标题栏仍由 Windows 窗口框架绘制白色。设置页的原生颜色选择器仍显示彩色色谱，这是编辑颜色的选择面板自身的颜色表示，不是 FileAtlas 的表面、正文、边框或状态色角色。此次未发现需要新增私有主题色覆盖的 FileAtlas 界面区域。

同进程命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/single-color-audit --plugins dev-test --access-level standard --scenario scripts/validate-single-color-theme.cjs`。完整运行 profile、角色快照和运行报告在 `.local/validation/profiles/single-color-audit/`。截图：

![全部原生 ThemeConfig 角色设为洋红后的主窗口](../../../.local/validation/artifacts/2026-10-24-single-color-audit/single-color-main.png)

![文件夹网格中的剩余色来自实际媒体缩略图](../../../.local/validation/artifacts/2026-10-24-single-color-audit/single-color-populated-grid.png)

![主题设置页的界面表面应用单色主题](../../../.local/validation/artifacts/2026-10-24-single-color-audit/single-color-settings.png)
