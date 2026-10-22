# WorkCrate 插件设置与项目默认信息（2026-10-23）

## 范围与参考

用户要求集中 WorkCrate 基础配置、独立物料类别、项目级默认信息以及按列提升。
本轮遵守原 SettingsPage/PanelCatalog/Cordis 分工，未引入新插件管理器、数据库或绘图器。
已实际打开 settings-minimal-2026-10-18.png 与 workcrate-inspector-2026-10-21.png；
编码清单追加于 reference-ui/concepts/workcrate-editing-tools-2026-10-20.md。
旧继承名称要求由本轮用户指令覆盖，其余三层及检查器行结构保持。

## 功能与存储

- 设置中心 → 插件设置 → WorkCrate：宿主继续提供运行状态/启动方式；正文增加默认排序、默认 PNG/JPEG、完整 GUI/CLI 路径和控制端口，并管理四个基本字段及自定义字段的多值默认信息。
- 项目管理/页面管理/装配页顶部默认信息入口使用公共模态面板；范围菜单可切换程序或具体项目。项目列表、空列、录入、失败、保存和禁用状态均来自真实服务。
- 页面内容首行独立「物料类别」；改项目标题不改类别，清空类别不回退标题。旧明确局部名在读旧缓存时保留为类别；旧继承名不复制，原生文档中旧继承文字在下次成功字段保存时清理。浏览本身不改写原生文档。
- Projects 现有 project.json v1 添加可选 field_presets，缺省为空列；复制项目保留它，删除仍走现有回收目录。复用原子 JSON + .json.bak 恢复。程序默认信息继续 field-presets.json；GUI/CLI/端口继续 vectorcraft.json；浏览/导出默认值进入现有 SettingsService 的 serde-default workcrate 字段。
- 下拉在打开时读取项目优先、程序随后、同值去重的并集；默认信息的变化不覆盖页面值。每列「合并到程序默认信息」只追加这一列并去重，保留其他列及原有程序选项。

## 环境与复现

Windows，150% DPI，安装的 Rust 1.97.1 MSVC，官方 VectorCraft 程序/CLI 位于 C:/Program Files/VectorCraft。
正常主窗截图 3286×1831 物理像素；小窗口 1600×1100；通过 get_ui_state 返回的逻辑坐标与控件几何发送输入。
所有交互均由 FileAtlas --mcp 同进程 stdio 子进程完成，Full 仅用于隔离验证；没有通用电脑控制或 OS 输入注入。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-settings -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-defaults-2026-10-23-final --access-level full --scenario scripts/validate-workcrate-defaults.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-defaults-2026-10-23-final --access-level full --scenario scripts/validate-workcrate-settings.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-defaults-2026-10-23-final --access-level full --scenario scripts/validate-workcrate-defaults-theme.cjs
```

测试 TEMP/TMP 显式设置为 .local/validation/profiles/wc-defaults-tests/temp。
本轮场景代码位于 scripts；完整报告、截图、数据库及原生样本只在 .local/validation。
最终 EXE 为正常 target/debug/fileatlas-shell.exe；链接器现有 LNK4098 警告仍存在，非编译失败。

## 已复现并修正的问题

1. 独立类别为空时，旧信息条先创建空文本、按无布局字段删除，然后分组仍引用已删 id：官方 select.set 报 no such object。现在空字段不创建/测量/加入分组顺序，旧节点交给 stale cleanup；新建和清空重读已纳入同进程场景。
2. 两个非模态工具面板同时打开时默认信息被主 WorkCrate 遮挡：该设置编辑器使用公共模态面板，实际字段输入和按钮可以操作，未改宿主堆叠实现。
3. GUI/CLI 输入的百分比宽度在 Custom 设置行内退化为极窄控件：将路径编辑区域约束为 320 逻辑像素，在真实设置页确认可编辑、保存及水平查看完整路径。

## 验证状态

最终构建的 fmt、locked Shell check/build、六项治理守卫（含 theme self-test）全部通过；
四个受影响 crate 共 59 项测试通过、0 失败，另 4 项既有官方门控测试保持 ignored。
旧缓存明确局部名迁为类别、清空不回退标题、旧 Settings 缺省兼容、项目复制保留预设、
项目隔离/重启恢复及单列提升不改其他列均有自动回归覆盖。

同进程最终运行：核心场景 49 步、设置补测 57 步、主题补测 16 步，共 122 个记录步骤通过。
场景内部还执行读取/等待断言；步数只统计客户端归档步骤，不当作覆盖率百分比。

| 检查项 | 参考要求 | 实际表现 | 本机证据/剩余差异 |
|---|---|---|---|
| 设置外框与分区 | 左导航、标题、横向配置行、独立滚动、全局固定页脚 | 沿用 SettingsPage/设置宿主；新增配置均可通过滚动访问 | [程序配置](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/settings-complete-program-config.png)，GUI/CLI 长路径使用原生输入水平滚动，完整值在状态/文件中核对 |
| 项目默认信息 | 清晰范围、字段值/删除同行、逐列提升、真实空态 | 范围显示项目名称；增改自动保存；仅此项目可见；合并仅作用指定列并去重 | [项目录入](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/project-defaults.png)，模态防止被父面板遮挡 |
| 页面字段与下拉 | 32px 横向行，物料类别独立，不出现继承按钮 | 已填/空值/预设选择/项目改名/原生重读均正确，其他页面结构保留 | [下拉](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/category-menu.png)、[清空类别](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/empty-category.png) |
| 错误、焦点、禁用 | 原生状态、有效配置保存、失败不替换旧配置 | 端口 0 明确报错，改回合法端口后 GUI/CLI/端口正确持久化；输入聚焦与空列提升禁用可见 | [无效端口](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/settings-invalid-port.png)；默认信息删除/全部输入冲突组合未穷举 |
| 小窗口与长内容 | 正文滚动，最下方操作可达 | 1600×1100 物理窗口可新增长名称自定义字段，并滚动到完整保存按钮 | [小窗底部](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/settings-small-bottom.png)；顶部部分内容随滚动离开视口是预期，不是完整显示全部字段 |
| 原生主题 | 默认主题及 changed accent，亮暗态、选中/禁用/公共设置一致 | 检查并打开紫色 Material 原生主题下的默认信息与 Settings 外观截图；未画私有颜色 | [暗态](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/defaults-purple-dark.png)、[亮态](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/defaults-purple-light.png)、[公共设置亮态](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/screenshots/settings-native-light.png) |
| quiet-state | 真实改变才发布；普通设置不重载画板目录 | 核心场景静置 3.5s，reloads 不增加；项目预设变化走现有 Wake/内存差异比较 | 原生会话探针可因官方会话状态变更更新，但未引入新无限轮询/无变化通知 |

最终报告分别为 [核心](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/core-report.json)、
[设置](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/settings-report.json)、
[主题](../../../.local/validation/artifacts/2026-10-23-workcrate-defaults/theme-report.json)。
这些链接是 git 忽略的本机产物，非随仓库提交的公开证据；可提交的脚本及本页结论用于复现。
隔离 profile 为 .local/validation/profiles/wc-defaults-2026-10-23-final；三个场景的摘要和截图
复制到 .local/validation/artifacts/2026-10-23-workcrate-defaults/。
主题场景先在同进程 UI 中恢复暗态；子进程退出后按 theme-restore.json 恢复隔离 Settings 的
原 color_mode/theme_file 两个键（保留本轮 WorkCrate 测试配置），生成的测试主题仍留在隔离目录供复查。
复跑主题脚本时也应在子进程退出后做相同测试档案清理。

知识图谱查询已先行尝试；当前 FileAtlas 索引未返回 WorkCrate 符号，随后使用 rg/源码读取补足发现。
本轮没有剩余已发现的目标布局偏差；未穷举的交互边界列在上表和下节，不能据此称整软件所有 Windows 操作都通过。

## 边界

原生 EXE 浏览对话框本轮未点击（路径由真实文本输入填写）；跨机器安装位置、全部系统 DPI 和大量长选项未穷举。四个既有官方门控 Rust 用例默认 ignored，不能把普通 cargo test 描述为这四项通过；本轮原生字段写入、创建及重新读取的实际证据来自上述同进程 MCP 场景。没有改动实际用户 WorkCrate 文件或默认配置。
