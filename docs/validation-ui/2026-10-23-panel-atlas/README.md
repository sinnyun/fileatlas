# FileAtlas 面板截图全集（2026-10-23）

同进程 FileAtlas MCP 驱动开发界面，逐面板截图。窗口尺寸 3200×1800（用户指定），
共 60 张 PNG，每张都是隔离测试实例（独立 profile / 数据库 / 缩略图缓存），
未触碰用户日常使用的 FileAtlas 数据。

## 复现命令

场景脚本：`scripts/capture-panel-atlas.cjs`（按 `PANEL_ATLAS_PHASE` 分段，
每段一个独立 MCP 子进程，因此单个界面崩溃不会带走其余截图）。

```bash
P=.local/validation/profiles/panel-atlas-final
run() { PANEL_ATLAS_PHASE=$1 node scripts/fileatlas-mcp-debug.cjs \
  --exe target/debug/fileatlas-shell.exe --profile $P \
  --plugins dev-test --access-level full --scenario scripts/capture-panel-atlas.cjs; }

run browser                 # 播种 + 主窗口/侧栏/检查器/工具/筛选/分组/多列
run settings                # Settings 各标签页 + 插件设置页
run modals                  # 工作区管理、元数据导出、更新检查、导航标签对话框
PANEL_ATLAS_WORKCRATE_EDIT=1 \
FILEATLAS_VECTORCRAFT_EXE="C:\Program Files\VectorCraft\vectorcraft.exe" run panels
```

内容来源：`dev-test.scan_fixture` 扫描 `test-data/fileatlas-fixtures`（12 行），
并开启"显示子文件夹内容"，否则根目录只有 `README.txt` 一行；标签
`设计资产 / 投放渠道 / 主题背景`、4 条剪贴板记录、6 个暂存文件、
WorkCrate 工程"截图巡检工程"（5 个真实画板 + 已放置图片预览）均由脚本播种。
WorkCrate 的建页/刷新/放图/预览走官方自动化会话；本次运行时官方 GUI 未启动，
因此全部由 headless `vectorcraft-cli` 完成，没有打开用户桌面上的官方程序窗口。

## 截图清单

| 前缀 | 覆盖内容 |
| --- | --- |
| `000`–`001` | 启动窗口、3200×1800 主窗口 |
| `100`–`102` | 主窗口单列 + 侧栏文件夹 / 标签树 / 扩展（项目树） |
| `110`–`114` | 选中态、检查器信息 / 历史 / 扩展、文件夹上下文 |
| `120`–`122` | 工具区、统计对话框、批量操作面板 |
| `130`–`135` | 类型 / 标签 / 扩展名 / 日期 / 评分 / 排序浮层 |
| `140`–`141` | 搜索结果、搜索范围浮层（见下方缺陷） |
| `150`–`152` | 文件右键菜单、按文件类型分组、按月份分组 |
| `160`–`162` | 双列、四列、活动列指示 |
| `600`–`607` | Settings：外观 / 快捷键 / 标签 / AI / 更新 / 软件 / 插件 / 插件页 |
| `610`–`612` | 插件设置页：WorkCrate、Material 主题生成、MCP |
| `620`–`622` | 程序工作区管理、元数据导出、手动更新检查 |
| `630`–`633` | 导航标签对话框：新建 / 删除 / 合并 / 移动 |
| `640` | 侧栏文件夹搜索 |
| `700`–`705` | 浮动面板：剪贴板工作台、文件暂存篮、程序启动器、截图工具、开发任务状态（单开 + 多开） |
| `800`–`805` | WorkCrate 三层：工程管理、页面矩阵 + 物料栏、缓存预览、页面编辑器、已放置图片、项目默认信息 |
| `900`–`910` | 调试抽屉、两个浮动面板叠加主界面 |

`INDEX.md` 是按拍摄顺序生成的缩略图索引，`manifest.json` 记录每一步的调用结果；两者与 PNG
一起留在本地 profile，见文末"证据位置"。

## 已发现缺陷（截图为证）

1. **打开"编辑标签"对话框会 panic**：`gpui-pre-0.3.8/src/window/a11y.rs:415`
   `Duplicate a11y node id: #756260934691697492`，进程随即退出，MCP 全部超时。
   本次按用户指示跳过该面板（`navDialogs` 已移除 `edit`），其余标签对话框正常。
2. **搜索范围浮层不渲染**：`control_file_manager {action:"search_scopes_open",enabled:true}`
   返回 `accepted`，`get_ui_state.file_manager.search_scopes_open` 也为 `true`，
   但 `141-search-scope-panel.png` 与 `140-search-results.png` 逐字节相同——
   受管 Popover 只有状态、没有画面。
3. **`workcrate-ui.update_page` 的 `info.extra` 需要对象**：传字符串报
   `invalid type: string …, expected a map`；`material_category/material/size/quantity`
   四个字符串字段正常。
4. **`panel_close` 只认实例 id**（`<定义 id>@<序号>`），传定义 id 报
   `Unknown panel instance …`；MCP 描述未写明，早期一轮因此让面板叠加到后续截图里，
   现已按实例 id 关闭并在每段前统一清理。

## 未覆盖 / 说明

- 浮动面板注册表已截全：`control_ui panel_open` 对 `dev-test/panel-view`、
  `workspace/manager-panel`、`metadata-export/panel`、`settings/panel`、`debug/panel`
  均返回 `Unknown panel`（这些只是测试里的槽位名），因此 PanelCatalog 里真实存在的
  8 个面板定义（剪贴板、暂存篮、启动器、截图工具、WorkCrate 总览、WorkCrate 默认信息、
  dev-test 单开/多开）已全部有图。
- `613-settings-page-i18n`：i18n 插件没有注册 Settings 页，脚本按"取不到就跳过"记录为 warn。
- 播放/预览依赖外部 QuickLook、真实相机 EXIF、Windows 拖放与资源管理器集成等，
  本套截图只证明界面渲染与状态，不构成这些平台行为的验证结论。

## 证据位置

生成物一律不入库（`.local/` 已在 `.gitignore`），本目录只保存这份说明。同进程 MCP 场景脚本把
截图直接写进 `--profile` 的 `screenshots/` 子目录，因此原始运行目录就是证据目录：

- 60 张 PNG、`INDEX.md` 缩略图索引、`screenshots.zip` 打包副本：
  `.local/validation/profiles/panel-atlas-final/screenshots/`
- 每一步调用结果与报错原文：`.local/validation/profiles/panel-atlas-final/manifest.json`
- 数据库、settings、日志等原始隔离 profile：`.local/validation/profiles/panel-atlas-final/`

重跑某一阶段会原地更新这些文件；仓库内不再保留截图副本。

## 2026-10-24 单色全界面复核与后续复用

使用同一 `scripts/capture-panel-atlas.cjs` 对 `#FF00FF` 单色主题重新批量截取，用于查找其余面板是否仍有非主题 UI 颜色。隔离 MCP profile 为
`.local/validation/profiles/single-color-panel-atlas/`；各阶段共享同一份测试数据库，
每个阶段启动独立 MCP 子进程。当前 profile 的有效原生 ThemeSet 两种模式各 140 个
角色均为 `#FF00FF`，Material 设置草稿的种子色和七个语义色也相同。

在 PowerShell 中重跑全部批次：

```powershell
$profile = '.local/validation/profiles/panel-atlas-rerun'
foreach ($phase in @('browser', 'settings', 'modals', 'panels')) {
  $env:PANEL_ATLAS_PHASE = $phase
  node scripts/fileatlas-mcp-debug.cjs `
    --exe target/debug/fileatlas-shell.exe `
    --profile $profile `
    --plugins dev-test `
    --access-level full `
    --scenario scripts/capture-panel-atlas.cjs
}
Remove-Item Env:PANEL_ATLAS_PHASE
```

如果要复核现成单色主题，先把该 profile 的 `settings.json`、`themes/single-color-magenta.json`、
`themes/material-settings.json` 复制进新 profile，并让 `settings.json.theme_file` 指向新 profile 下的
ThemeSet；普通界面巡检无需复制主题文件。`browser/settings/modals/panels` 分开启动可隔离单面板崩溃，
`manifest.json` 和 `screenshots/INDEX.md` 会累计更新；需要更大截图时调整场景中的 `resize_window`。
`PANEL_ATLAS_WORKCRATE_EDIT=1` 才会继续创建 WorkCrate 画板并放图，要求官方 VectorCraft 自动化
会话可用；未设置时仍会尝试创建工程，但不能把工程创建失败后的空 WorkCrate 画面当成已覆盖编辑器。

本次四阶段累计截图调用数依次为 browser 27、settings 40、modals 50、panels 61；各阶段都会重拍
两张启动图，因此 61 次调用对应 55 个唯一 PNG 文件。MCP 子进程均正常退出。除首次启动图外，
其余截图为 3000×2000。清单的 4 项未完成为两次
不存在的 i18n 设置页请求及其跳过提示（没有注册该页），以及本机缺少 `vectorcraft-cli.exe`，
导致 WorkCrate 工程/编辑器截图未能验证。其余窗口、主视图状态、设置页、对话框、浮动面板和组合态
均已截取并查看。单色复核没有发现绕过 FileAtlas 主题的面板/正文/边框/状态色；剩余彩色像素来自
Windows 标题栏、测试夹具缩略图/程序图标和颜色选择器的色谱控件。该审查只比较颜色来源，单色文字与
背景相同导致内容不可读，不作为布局或可读性验收。

本轮截图集和缩略索引：
[完整截图索引](../../../.local/validation/artifacts/2026-10-24-single-color-panel-atlas/INDEX.md)。
原始 profile 中还保留四阶段 MCP 报告、179 步 manifest、最终 UI 快照及所有原始 PNG；这些都位于
`.local/validation/profiles/single-color-panel-atlas/`。可复用实现继续维护在
`scripts/capture-panel-atlas.cjs`，本次完整命令及覆盖结果也记录在此处，未来可直接复制上面的 PowerShell 批次。
