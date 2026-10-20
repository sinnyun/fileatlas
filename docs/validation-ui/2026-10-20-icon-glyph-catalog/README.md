# 图标库使用审计与文字符号修复（2026-10-20）

## 审计结论

对全部插件扫描了内联 SVG、自定义 `.svg` 资源、emoji、Unicode 私用区字符与当作图标使用的文字符号：

- 插件目录没有任何内联 `<svg>`，也没有自带 `.svg` 资源；`.rs`/`.ftl`/`.json` 中不存在 emoji 或私用区字形。
- 违规只有一类：把 Unicode 文字符号直接写进按钮标签或菜单文本，绕开了 `gpui_kit::assets::IconName` 与
  `fileatlas_ui_bridge::theme::{icon, small_icon, large_icon}` 网关。共 15 处渲染点，分布在 launcher 11、debug 3、navigation 1。
- 治理守卫 `scripts/check-ui-theme.cjs` 此前只检查颜色所有权，没有图标来源检查，因此该类缺陷可以静默回归。

## 修复清单

统一改为图标库常量或组件库自带的下拉/勾选呈现，不再手写符号：

| 位置 | 原符号 | 现在 |
| --- | --- | --- |
| [launcher/src/view/layout.rs:157](../../../crates/fileatlas-plugin-launcher/src/view/layout.rs#L157) | `"{排序} ▾"` | `Button::dropdown_caret(true)` |
| [layout.rs:182](../../../crates/fileatlas-plugin-launcher/src/view/layout.rs#L182) | `"↓"` / `"↑"` | `theme::small_icon(IconName::ArrowDown / ArrowUp)` + `accessibility_label` |
| [layout.rs:297](../../../crates/fileatlas-plugin-launcher/src/view/layout.rs#L297) | `"◌ Scanning"` | `IconName::Loader` + 纯文案 |
| [layout.rs:299](../../../crates/fileatlas-plugin-launcher/src/view/layout.rs#L299) | `"● Scan complete"` | `IconName::CircleCheck` + 纯文案 |
| [inspector.rs:140](../../../crates/fileatlas-plugin-launcher/src/view/inspector.rs#L140) | `"{分类}  ▾"` | `dropdown_caret(true)` |
| [inspector.rs:162](../../../crates/fileatlas-plugin-launcher/src/view/inspector.rs#L162) | `"☑ "` / `"☐ "` | `IconName::SquareCheck` / `Square` + 纯文案 |
| [inspector.rs:187](../../../crates/fileatlas-plugin-launcher/src/view/inspector.rs#L187) | `"置顶: … ▾"` | `dropdown_caret(true)` |
| [inspector.rs:197](../../../crates/fileatlas-plugin-launcher/src/view/inspector.rs#L197) | 菜单项 `"✓ "` 前缀 | `PopupMenuItem::checked(scope 匹配)` |
| [menu.rs:39](../../../crates/fileatlas-plugin-launcher/src/view/menu.rs#L39) | 菜单项 `"✓ "` 前缀 | `PopupMenuItem::checked(...)` |
| [cards.rs:101](../../../crates/fileatlas-plugin-launcher/src/view/cards.rs#L101) | `"★"` / `"☆"` | `IconName::StarFill` / `Star`，保留 `theme::star_text()` 颜色 |
| [debug/src/view.rs:156](../../../crates/fileatlas-plugin-debug/src/view.rs#L156)、[:181](../../../crates/fileatlas-plugin-debug/src/view.rs#L181)、[:203](../../../crates/fileatlas-plugin-debug/src/view.rs#L203) | 分类/级别/每页数量标签尾随 `▾` | 三处 `dropdown_caret(true)` |
| [navigation/src/view/address_bar/render.rs:163](../../../crates/fileatlas-plugin-navigation/src/view/address_bar/render.rs#L163) | `"…"` | `IconName::Ellipsis` + `accessibility_label("Edit folder address")` |

同时把 [file_manager filter_row/helpers.rs](../../../crates/fileatlas-plugin-file-manager/src/view/filter_row/helpers.rs) 文档注释里的 `✕` 改成文字描述，避免守卫误报注释。

保留的合法例外（不是图标，不经 `IconName`）：`fileatlas-plugin-launcher/src/icons.rs` 的 Windows 原生外壳图标提取、
`fileatlas-plugin-drag-drop/src/dispatch.rs` 拖拽幽灵需要的 `RgbaImage` 回退位图、`fileatlas-plugin-i18n/src/lib.rs`
的历史字形前缀剥离器（现已无调用方产生前缀，可后续单独清理）。

## i18n 连带修改

- `keys.json`/`en.ftl`/`zh-CN.ftl`：`"◌ Scanning"→"Scanning"`、`"● Scan complete"→"Scan complete"`、
  `"☑ Hidden program"→"Hidden program"`、`"☐ Hide this program"→"Hide this program"`，中文值仍为“正在扫描/扫描完成/已隐藏程序/隐藏此程序”。
- 删除 4 条已无引用键：`"▦ Months"`、`"Folders ✓"`、`"Tags ✓"`、`"✓ Select"`（928→924 条）。
- 顺带修复既有缺陷：`en.ftl` 缺失 7 个已被引用的消息 id（`msg-3c6db1637ff3`、`msg-60acc53f13a5`、`msg-9f1e7d7d98b2`、
  `msg-bfa0cab91d75`、`msg-dbcd5e7bb7a0`、`msg-ec4b5fc6fd14`、`msg-fc43ea9804c7`，均为配色模式/工作台主题标签）。
  该缺失在本次改动前即导致 `cargo test -p fileatlas-plugin-i18n` 失败（`git show HEAD` 版本同样缺该 id）。

## 守卫

[scripts/check-ui-theme.cjs](../../../scripts/check-ui-theme.cjs) 在逐行检查中新增“文字符号即图标”规则：字符串字面量含
几何符号/箭头/装饰符号/emoji/私用区/变体选择符，或整串仅由符号与空白组成时，报错
`render icons with theme::icon(IconName::…) instead of the text glyph`，每行一条。
`--self-test` 新增 12 条探针：`★`、`↑/↓`、`"● Scan complete"`、`"✓ "`、`format!("{} ▾", …)`、🎨、私用区 `U+E021`
必须命中；`"…"`、正文 `"→"`、`"·"`、`IconName::Star`、行尾注释符号、`.label(" ")` 必须放行。

## 静态检查

Rust `1.97.1-x86_64-pc-windows-msvc`，工作目录 `F:\AI\geren\pho_cordis`：

```
cargo +1.97.1 fmt --all -- --check
node scripts/check-ui-theme.cjs --self-test     # UI theme guard probes: passed
node scripts/check-ui-theme.cjs                 # UI theme ownership: passed
node scripts/check-ui-refresh.cjs               # UI timer ownership: passed
node scripts/check-architecture.cjs             # passed (edges=108, debt=742, ui crates=17)
node scripts/check-feature-ledger.cjs           # passed (4 stage rows, links resolve)
cargo +1.97.1 test -p fileatlas-plugin-launcher -p fileatlas-plugin-navigation -p fileatlas-plugin-i18n --locked -j 2   # 4+13 通过
cargo +1.97.1 test -p fileatlas-plugin-debug --locked -j 2                                                  # 10 通过
```

`cargo +1.97.1 check --locked -j 2 --workspace` 在同一份源码上 Finished；编译产物
`target/debug/fileatlas-shell.exe` 即下列实测所用 EXE。

## 同进程 FileAtlas MCP 实测

隔离 profile：`.local/validation/profiles/icon-glyph-catalog`（`mcp_access_level: standard`，`FILEATLAS_DEV_PLUGINS=all`，
普通 `%LOCALAPPDATA%\FileAtlas` 数据库/设置/缩略图未触碰）。Windows 150% DPI，窗口 1500×940 物理像素。

```
FILEATLAS_DEV_PLUGINS=all node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
  --profile .local/validation/profiles/icon-glyph-catalog --access-level standard \
  --scenario docs/validation-ui/2026-10-20-icon-glyph-catalog/validate-icon-catalog.cjs
```

[validate-icon-catalog.cjs](validate-icon-catalog.cjs) 在一份全新隔离 profile 上完整跑通（无异常退出），
按名称顺序选中首个可见程序、置顶、打开右键子菜单与 inspector 菜单、切换 `all`/`none`、开合 debug 抽屉、
用 `control_navigation select_path` 让地址行进入面包屑态、最后把该程序置为隐藏并在“隐藏工具”分组检查开关状态，
下列截图即该次运行的产物（同时保留一份启动台面板覆盖抽屉时的对照）。

逐项打开检查的截图与结论：

- ![10-idle-footer.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/10-idle-footer.png)：
  空闲页脚为绿色 `CircleCheck` + “扫描完成”，说明改名后的 zh 键解析正常；排序按钮“名称”带原生箭头。
- ![30-inspector-top.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/30-inspector-top.png)：
  分类下拉原生箭头、“隐藏此程序”为 `Square` 图标、方向按钮为 `ArrowUp`。
- ![40-inspector-pin-row.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/40-inspector-pin-row.png)：
  inspector 滚动后“置顶: 未置顶/分类顶部”按钮使用原生箭头。
- ![50-inspector-pin-checked.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/50-inspector-pin-checked.png)：
  `launcher.set_pin(scope=category)` 后 inspector 置顶菜单中 ✓ 落在“置顶到分类顶部”，另两项无勾选。
- ![20-context-pin-checked.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/20-context-pin-checked.png)：
  右键“程序菜单 → 置顶”子菜单同样 ✓ 在当前作用域，父项与“移动分类”均为原生子菜单箭头；卡片角标为金色描边 `Star`。
- ![60-debug-drawer.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/60-debug-drawer.png)：
  `debug.control open` 后调试抽屉的“分类: 全部”“全部级别”“50 / 页”三处均为库自带箭头，标签内无 `▾`；
  主工具栏其余下拉同样使用原生箭头。
- ![61-debug-drawer-behind-panel.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/61-debug-drawer-behind-panel.png) 为启动台面板覆盖抽屉时的对照，
  说明抽屉需在主窗口表面可见时才进入截图。
- ![65-address-trail.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/65-address-trail.png)：
  `control_navigation select_path` 选中 `docs` 后，地址行面包屑为 `FolderOpen` + 目录段 + `ChevronRight`，
  其后紧跟 `IconName::Ellipsis` 图标按钮（原 `…` 文本），再后为打开文件夹与刷新图标；该行无任何文字符号按钮。
- ![71-inspector-squarecheck.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/71-inspector-squarecheck.png)：
  在“隐藏工具”分组选中已隐藏程序后，inspector 切换为 `SquareCheck` + “已隐藏程序”，与 `30` 的 `Square` + “隐藏此程序”构成两种状态。
- ![70-badge-starfill.png](../../../.local/validation/artifacts/2026-10-20-icon-glyph-catalog/70-badge-starfill.png)：
  `launcher.set_pin(scope=all)` 后卡片角标为实心金色 `StarFill`；`20` 中分类置顶为描边 `Star`。

## 范围与未验项

- 实测覆盖：启动台面板空闲/扫描页脚、排序与方向按钮、分类/置顶下拉、`checked()` 菜单在 inspector 与右键子菜单两处、
  卡片 `Star`/`StarFill` 两种角标、隐藏开关 `Square`/`SquareCheck` 两种状态、主窗口地址行 `Ellipsis` 图标按钮、debug 抽屉三处箭头。
- 未记为通过：亮色主题下的同一批控件、极窄窗口下的菜单翻转与溢出、键盘焦点遍历与 `accessibility_label` 的朗读文本、
  菜单项 ✓ 的悬停/禁用态、`Ellipsis` 按钮的悬停提示实际弹出。
- 逐图标形状按 12/16 px 在 1500×940 截图中确认存在且来自库渲染，未做像素级比对；概念图仅为主工作台布局参考，本轮无新增视觉设计。
- inspector 隐藏开关的鼠标点击未改变草稿（截图 `90` 与未点击状态一致，`保存修改` 始终禁用），因此该状态改由已隐藏程序的
  inspector 呈现验证。这属于既有的草稿刷新行为，不在本轮图标修复范围内，未继续追查。
- 编译期验证但无独立截图：`fileatlas-plugin-debug` 的分页 `ChevronLeft/ChevronRight`（原实现已用图标库）。
- 守卫只覆盖非 ASCII 符号区，`navigation` 地址错误按钮仍以文本 `!` 加危险色与提示呈现；它是 ASCII 标记而非字形图标，
  本轮按原样保留并在此记录。
- 顺带发现的文档陈旧项，本轮未修：`crates/fileatlas-plugin-launcher/PORTS.md` 仍写 `get_launcher_state`、
  `add_launcher_program`，实际工具为 `launcher.snapshot`、`launcher.add_program`；
  `scripts/validate-program-launcher.cjs` 使用同一批旧名，且启动台面板的 `panel_instances[].bounds` 现为 `null`，
  点击坐标需由 `window_status.geometry.screenshot_to_pointer` 换算。
