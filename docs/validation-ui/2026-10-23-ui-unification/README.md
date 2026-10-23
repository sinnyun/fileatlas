# 全局组件样式收敛 — 2026-10-23

## 范围

统一插件按钮入口、字号梯度、常用控件尺寸与状态呈现。当前所有界面只有三个字体角色：标题 15 px 半粗、正文 14 px 常规、辅助说明 12 px 常规；标题按用户反馈从 16 px 再下调。共享图见 `docs/reference-ui/concepts/typography-three-tiers-2026-10-23.png`，尺寸间距规范见相邻 Markdown 清单。

## 实施与检查

- 279 个插件按钮创建点迁移到 `fileatlas_ui_bridge::components::button`。
- UI Bridge 只公开 `workbench_title/body/assistant` 三种文字角色；插件直接字号/GPUI 字号快捷类/本地字重已迁移到共享角色，设置文字层级间距使用统一 token。主窗口筛选按钮、地址栏、检查器页签也已接入正文审查色；暂存面板的标题、计数、空状态及按钮文字显式使用共享角色。
- 21 个插件文件中重复控件高度映射到 `CONTROL_HEIGHT_*`；其余常用行高补入 compact/small/dense/nav token。
- 主题守卫拒绝插件直接调用 `Button::new` 或直接使用 `.font_bold()`/`.font_semibold()`。
- 已通过：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`、`cargo +1.97.1 check --locked -j 2 -p fileatlas-shell`、主题守卫 self-test、refresh、architecture、feature-ledger、layering。此前正式颜色代码下的全量主题守卫通过记录不代表当前审查色阶段；当前全量守卫会按预期指出临时 RGB 标记。

## 同进程 MCP 亮色检查

命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/ui-unification-20261023 --access-level standard --scenario .local/validation/profiles/ui-unification-20261023/capture.cjs`；之后用 `settings-capture.cjs` 打开外观设置。运行环境为 Windows MSVC、Rust 1.97.1、dev-test 隔离 profile；截图和报告仅存在于 `.local/validation/profiles/ui-unification-20261023/`。

浅色外观设置及真实 fixture 检查器截图分别为 `screenshots/unified-light-appearance.png` 和 `screenshots/unified-light-inspector.png`；插件面板和旧窗口压力截图为 `screenshots/unified-light-plugin-panel.png`、`screenshots/unified-light-small-window.png`。该旧图尺寸低于用户确认的最低支持尺寸 1920×1080，仅作为非验收历史记录。本轮另在最低支持尺寸采集主窗口、剪贴板、启动台和设置页；证据见 `.local/validation/profiles/ui-minimum-window-audit-20261023/`。本轮重新采集 `screenshots/audit-light-launcher-forms.png`、`screenshots/audit-light-workcrate-tiers.png`、`screenshots/audit-light-shortcuts.png`。临时审查色按三种实际字体角色呈现为红色标题、绿色正文、蓝色辅助说明。主窗口顶部工具项、筛选、地址栏、左侧导航及检查器页签现已显式归入正文/辅助说明角色；本轮亮色截图再次确认顶部筛选文字是绿色正文、搜索占位及次级提示为蓝色辅助说明。原生搜索框占位文字也已通过 UI Bridge 主题入口归入蓝色辅助层。所有标题统一为 15 px。统计面板分区标题归为标题层，卡片指标值归为正文层。暂存空状态可容纳现有文案。

另使用 `PANEL_ATLAS_PHASE=all node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/ui-typography-atlas-20261023 --plugins dev-test --access-level standard --scenario scripts/capture-panel-atlas.cjs` 对主界面、设置、模态框与注册浮窗进行同进程抽查：56 张截图可用。截图与清单仅存在于 `.local/validation/profiles/ui-typography-atlas-20261023/`。4 项脚本步骤未通过：启动瞬间窗口尚未就绪（后续截图成功）、脚本请求不存在的 i18n 设置页两次、可选 WorkCrate 创建因本机未安装 `vectorcraft-cli.exe` 未执行；MCP 子进程未崩溃。

## 尚未覆盖

颜色审查仍在进行中，临时层级色必须保留，直到用户明确批准结束审查。当前调试映射：标题红、正文绿、辅助说明蓝；原生 `text_primary` 暂时投影为正文绿，`text_secondary/text_muted` 投影为辅助蓝。UI Bridge 还将 GPUI Kit 的默认、链接、强调、按钮变体及悬停前景统一投影为正文绿，以免原生 Button hover/variant 绕开三层审查色；muted 前景暂投影为辅助蓝。临时颜色覆盖位于 `crates/fileatlas-ui-bridge/src/theme/roles.rs`、`components.rs` 和 `theme/runtime.rs`。总览截图已覆盖深色模式下文件浏览器、设置页、对话框与大部分注册浮窗；仍需继续检查深色长文本及 hover/focus/disabled 状态，并修正总览步骤的 i18n 页签映射。不能在用户批准前恢复正式色或移除这些标记。

本轮继续全仓扫描：插件 Rust 源中未发现独立 `.text_size`、GPUI 字号快捷类或本地粗体/半粗体调用，字号定义集中在 UI Bridge 三个角色；启动器“分类”分组标题已由辅助说明提升为标题。删除了插件 177 处重复正文/辅助文字色和 44 处角色之后覆盖红绿蓝的状态文字色，避免字体角色标记被局部颜色覆盖；角色色集中到 UI Bridge 主题投影。浅色设置截图 `.local/validation/profiles/ui-unification-20261023/screenshots/unified-light-appearance.png`、暂存面板截图 `.local/validation/profiles/ui-unification-20261023/screenshots/unified-light-plugin-panel.png` 确认窗口标题/分组标题红、正文绿、辅助说明蓝；主窗口顶部筛选、地址栏与检查器标签保持正文绿，原生搜索占位和检查器标签说明为辅助蓝。深色启动器同进程截图 `.local/validation/profiles/ui-typography-atlas-20261023/screenshots/unified-dark-launcher.png` 同样确认标题红、程序名称与顶部工具栏文字绿、辅助说明/计数蓝；长名称在统一卡片区域换行显示。`node scripts/check-ui-theme.cjs --self-test`、refresh、architecture、feature-ledger、layering、fmt 和 shell locked check/build 均通过；全量主题守卫仅报告 5 处集中于 UI Bridge 主题角色/运行时的临时审查 RGB（标题/正文/辅助色及原生 muted 前景），待用户明确批准后再恢复。Build 仍有已有 LNK4098 链接器警告。


本次三层字体参考概念图为 `docs/reference-ui/concepts/typography-three-tiers-2026-10-23.png`。截图和 MCP 报告仅保存在 `.local/validation/profiles/ui-unification-20261023/`。

本轮补充扫描修正扩展页空状态、启动器空列表、调试表头/反馈、工作区错误/状态文本、文件夹空状态，以及 AI 标签组、媒体记录、统计和设置页错误消息层级。错误详情目前按辅助说明层标色；错误图标继续使用语义图标色。重新构建后通过同进程 MCP 复采启动器、WorkCrate 与设置截图；顶部筛选标签仍显示正文绿，辅助提示显示蓝。统计详情/文件浏览空态的完整可见内容仍需在有相应真实数据/状态的 profile 中复核。

交互态核对曾在设置快捷键 F5 下拉项悬停时观察到文字从正文绿变为深色；随后把 GPUI Kit 通用 foreground 和各按钮 variant foreground 接入 UI Bridge 临时正文色。重新构建及同进程截图显示下拉值和箭头均为绿色。需继续针对按钮 hover/focus/disabled 与弹出菜单做定点交互测试；截图仅展示静态结果，未证明所有交互态覆盖。

三层字号与公共文字角色已静态收敛，但整体视觉验收仍未完成：仍需检查真实文件卡片、按钮 hover/focus/disabled、弹出菜单、其他插件浮动面板、1920×1080 最低支持尺寸下未覆盖的状态及长文本；不足空间时继续改布局或文案。主题守卫 self-test 通过；全量主题守卫当前会拒绝审查期 RGB 标记，这是临时审查色按用户指令保留造成的预期结果，结束颜色审查后再恢复原生 ThemeColor 并复验。

## 2026-10-23 交互状态统一批次（待最终验证）

- 新配置默认使用亮色，`color_mode` 明确保存的用户选择继续生效；隔离 UI 验证 profile 显式设为 `light`。
- 主导航竖栏的文件夹/标签/项目树/扩展和插件快捷入口统一由 `components::rail_shortcut` 构造，统一 42 px 宽、密度缩放 54 px 高、辅助说明文字层级、选中底色及 hover 背景；插件只传文案、图标、选中状态及行为。
- 插件内直接 `.hover()` 被 `check-ui-theme.cjs` 拦截；可选行、一般表面、评分文字和分组标题状态收敛到 UI Bridge `WorkbenchStyle`。按钮依旧由 GPUI Kit 语义 variant 管理 hover/pressed/focus/disabled。
- 修改后的全量交互验收将使用同进程 MCP、亮色和 3000×2000 窗口；`scripts/capture-panel-atlas.cjs` 已更新为该尺寸，并检查主窗口截图确为 3000×2000。测试尚未执行完，最终覆盖情况与剩余面板/状态需在下节补记。

## 2026-10-23 插件组件所有权源码迁移（进行中）

- 按用户最新决定改为关键词/源码结构审查，不把截图作为判断组件是否共享的依据。插件 UI 应传递内容、语义状态、事件和尺寸约束；按钮、基础表面、分割线、模态外框及遮罩样式归 UI Bridge。
- UI Bridge 新增/收敛 `SurfaceRole`、`CornerRole`、`ForegroundRole`、状态徽标/圆点、`separator_line`、`subtle_divider`、缩略图框架/悬停遮罩、不可用缩略图提示、媒体徽标、文件类型徽标、滑块轨道与手柄、树状虚线引导点、时间线轨道、菜单面板、状态提示条、`modal_frame` / `modal_overlay` 等组件。插件调用点已迁移设置弹层外框/分隔线、文件管理器通用弹层、导航标签颜色/图标选项、剪贴板分类编辑框、启动台侧栏 section、文件管理器批量操作/详情/筛选列表、缩略图徽标/遮罩、标签树引导线、导出状态卡、快捷键键帽、检查器页签、WorkCrate 菜单面板、画板/选区/手柄和标签可见性开关。按钮创建仍只走 `components::button/action/icon_button`。
- 源码审查脚本覆盖全部 302 个插件 Rust 文件（含 MCP 插件）。当前插件源中直接调用 `.border_*()`、`.bg()`、`.rounded*()`、`.text_color()`、`.shadow*()`、`.opacity()` 和 `.group_hover()` 均为 0；对应样式通过 UI Bridge 语义 API 解析。共享语义入口调用数：SurfaceRole 64、CornerRole 54、ForegroundRole 66、Bridge button 265、modal overlay 6。剩余 29 处 `.absolute()` 集中在内容、徽标、拖放命中区和测量层的定位；仍按源码逐项确认具体元素由哪个共享组件提供，定位本身属于插件允许提供的信息。插件直接 `theme::scrim()` / `modal_rgba()` 以及 Button/Input/Textarea/Checkbox/Radio/DatePicker/Popover 构造均为 0。
- `check-ui-theme.cjs` 已禁止所有插件路径（包括非 `view` 源文件）直接调用上述底层样式方法、直接构造标准控件或调用插件私有 modal scrim；self-test、全量主题守卫和 `check-ui-geometry-coverage.cjs` 通过。`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -j 2 -p fileatlas-shell`、design-token self-test/full、refresh、architecture、feature-ledger 和 layering 检查均通过。未做截图/视觉验收；本阶段按用户要求只审查底层组件所有权，审查色仍保留。
- 后续检查聚焦剩余绝对定位的组件子元素归属，以及按钮 variant 是否仅表达语义、未新增插件专属按钮/面板组件；之后运行完整治理脚本集合。此项仍在进行中，审查色保留规则不变。

## 2026-10-23 GPUI Kit 原生组件复用核对

- 锁定依赖为 `gpui-kit 0.7.1`，其重导出 `gpui-component 0.7.2`。原生组件库含 Button、Input、Checkbox、Radio、Popover、Dialog、Notification、Alert、Badge、Slider、Separator 等。
- Bridge 已有的标准交互控件包装仍保留为主题/尺寸/审查入口，但绘制与交互使用 GPUI Kit 控件。Bridge 自绘的四个通用分隔线入口（`divider`、`subtle_divider`、`vertical_divider`、`separator_line`）已改为在审查框架中承载原生 `Separator`，颜色与尺寸仍由 FileAtlas 主题提供。
- 未将 `status_badge` 替换为 GPUI Kit `Badge`：Kit 的 Badge 是数字/圆点/图标角标，不等同于承载任意状态文案的胶囊。`status_dot` 是独立圆点，Kit Badge.dot 是附着式角标，也不等价。`status_toast` 是已有状态文本的持续展示容器，Kit Notification 带消息调度/自动隐藏/OS 通知语义，当前调用行为不同。`modal_overlay` 负责已有窗口内容上的简单遮罩与居中；Kit Dialog 管理焦点、关闭与对话框生命周期，不能安全地仅替换绘制函数，需单独迁移调用状态。范围滑块同时呈现上下界，原生 Slider 当前为单值控件。
- 验证：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -j 2 -p fileatlas-ui-bridge`、主题守卫 self-test/full、几何覆盖（31 组）和 design-token 检查通过。该变更为组件实现核对，尚未重新进行应用截图验收。

## 2026-10-23 审查框设置开关

- 外观设置新增“界面审查 → 显示组件审查框”，写入现有 Settings JSON。字段为 `show_ui_audit_frames`，新配置默认 `true` 以延续当前审查；旧配置缺少字段时也默认为 `true`。设置 Service 在启动加载和成功持久化后同步 UI Bridge 运行时开关；设置界面编辑会调用 `cx.refresh_windows()`，立即刷新所有窗口。
- 这一开关控制审查框和组件几何采集入口；之后增加独立的字体层级审查色设置，当前有各自的持久化与运行时状态。
- GPUI Kit 的 Inspector 是逐个点选的开发期元素查看器（调试构建下 Ctrl+Shift+I），没有提供全树同时显示布局边界的模式。Android 的 Show Layout Bounds 是系统开发者选项式诊断覆盖，不能作为 Windows/GPUI 组件库直接接入。当前锁定 GPUI 暴露 Canvas paint callback 和 `Window::paint_quad`；共享组件、浮动面板及 UI Bridge 文本/多行输入审查边界均为 paint-only 覆盖，不需新增审查库。输入外层框架保留 GPUI Kit 原生输入控件的输入、焦点与禁用状态。
- 初始设置、旧配置兼容及关闭值 round-trip 测试通过。开关和布局检查截图使用同进程 MCP、3000×2000、隔离 profile `.local/validation/profiles/audit-overlay-20261023/`：`screenshots/overlay-final-enabled-confirmed.png` 显示浮动面板和组件覆盖边界；`screenshots/overlay-final-off.png` 显示关闭后审查色立即消失、正式浅色主题仍在；`screenshots/overlay-clipboard-inputs.png` 显示搜索框与多行输入框的审查覆盖。几何快照移除了审查边框内缩补偿，返回 Canvas 实测边界。`cargo +1.97.1 fmt --all -- --check`、Shell locked check/build、主题、几何覆盖（31 组）、design-token、refresh、architecture、feature-ledger、layering 守卫通过。截图仅覆盖设置、剪贴板面板和主窗口当前可见组件，未覆盖全部插件的 hover/focus/disabled/长文本状态。

## 2026-10-23 字体层级审查色开关

- 外观设置新增独立的“字体层级审查 → 显示字体层级审查色”。字段 `show_typography_audit_colors` 新配置与旧配置缺省均为 `true`，保留当前用户要求的临时审查色；边框审查开关不控制这组文字颜色。
- 关闭该项时，UI Bridge 重新应用当前选定 GPUI Kit 主题，并移除审查期的原生按钮、链接、弹出菜单和辅助前景投影；标题/正文/辅助说明恢复原生 `foreground` / `muted_foreground` 角色。再次开启恢复红/绿/蓝层级色。组件审查框开关与边框采集保持独立。
- 关闭时 UI Bridge 重新应用当前 GPUI Kit 原生主题：标题、正文、辅助说明及直接读取 Kit 前景色的按钮/链接/菜单恢复主题文字色；组件审查边界保持各自开关状态。Settings 序列化测试覆盖新旧默认和关闭后 round-trip。
- 同进程 MCP 使用亮色、3000×2000 窗口和隔离 profile `.local/validation/profiles/typography-audit-setting-20261023/`。`screenshots/typography-off-geometry-on.png` 显示字体开关关闭后文字回到原生主题色，同时几何审查框仍开启；`screenshots/typography-restored.png` 显示两个审查开关重新开启及红/绿/蓝审查色恢复。隔离 profile 最终 `settings.json` 保存 `show_ui_audit_frames=true`、`show_typography_audit_colors=true`、`color_mode=light`。
- 验证命令：`cargo +1.97.1 fmt --all -- --check`、设置字段 serde round-trip 单测、Shell locked build、主题守卫（含 self-test）、几何覆盖（31 组）、design-token、refresh、architecture、feature-ledger 和 layering 均通过。该开关按亮色运行态实测；暗色下关闭/重开后的视觉对照尚未单独采集。
