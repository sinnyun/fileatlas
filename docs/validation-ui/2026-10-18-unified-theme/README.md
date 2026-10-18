# 剪贴板基准的全局主题统一

2026-10-18。依据用户明确要求，统一宿主和各功能插件样式；这是实现者的视觉核对记录，
不代替用户最终签收。规范为 [主题开发约束](../../ui-theme.md)，参考为
[统一概念与逐项清单](../../reference-ui/concepts/unified-workbench-theme-2026-10-18.md)。
概念通过内置 ImageGen 生成并打开检查；生成器误画的额外功能不进入实现。

## 改动范围

- UI Bridge 提供唯一深蓝 palette、14px 正文、间距与圆角 token，映射所有 Kit
  颜色角色（包括表格、列表、accordion、状态按钮和图表），强调色变化同步全部状态。
- 公共组件提供 section/card/nav/action/icon/heading，以及单行、搜索、多行输入。
  剪贴板不再保留私有颜色；启动台不再固定青色选中和字符类别图标。
- Gallery 主窗/检查器/历史/统计/批量操作、Navigation、Settings、Launcher、Clipboard、
  Debug、Shell 的自绘颜色使用公共语义 token，导出/更新等原生 Kit 控件继承同一主题。
  全部插件的 Input/Textarea 经公共构造器创建，保留原有 Entity、事件和行为。
- 修正 Kit `input` 实为边框的映射，输入背景由公共构造器负责；保留原生焦点环。
  修正批量操作评分/天数/大小框在 14px 字号下的提示截断，扩大到 80/120px。
- 保留每页的业务布局；通用浮窗继续由已有 `panels` 拖动/调整大小。旧显式强调色、
  设置/会话路径、数据库和用户媒体不迁移。用户标签颜色、媒体像素及系统图标保留。

## 环境和准确命令

Microsoft Windows 11 专业版 10.0.26200，150% DPI，zh-CN。
Rust `1.97.1 (8bab26f4f 2026-07-14)` Windows MSVC，Cargo 默认 dev incremental，`-j 2`。
主窗物理截图 3286×1831；小窗由 MCP 请求 1510×1000，客户区为 992×629.33 逻辑像素。
测试启动的是该次构建的 `target/debug/photag-shell.exe --mcp`，MCP 与 GPUI 窗口同进程，
没有附加到其他 PhoTag、没有通用 UI 自动化或 OS 输入注入。

```powershell
rustup toolchain list
rustc +1.97.1 --version
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-ui-bridge -p photag-plugin-settings -p photag-plugin-i18n --lib -j 2
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-unified-theme-verified --allow-mutations --scenario scripts/validate-ui-theme.cjs
```

仅此隔离验证启用 mutation，客户端强制 `dev-test` 并指定隔离 DB/settings/session/cache；
添加的剪贴板文本为场景内三条样例，启动台仅添加本次 EXE 用作图标 fixture，没有启动它。
媒体测试只浏览已有 `test-data/photag-library/02-mixed-formats`，没有评分、标签或文件写操作。
测试中点击设置色板蓝→灰青→蓝并检查真实 token，最后恢复工作台蓝。

## 结果与逐图对照

最终截图及精简运行元数据保存在本目录；场景入口为 `scripts/validate-ui-theme.cjs`。
初轮曾把统计和批量操作同时打开，已用 `gallery_dialogs_close` 修正场景隔离；
后续图还揭示输入边框映射和窄数字框问题，修正后重建再截，本目录不沿用早期截图。

| 检查项 | 参考要求 | 实际表现及证据 |
|---|---|---|
| 主窗 | 保留横向工具/筛选带、导航/媒体/检查器、状态带，统一深蓝层级 | [main](main.png)，实际选择 medium.webp；标签数据色和测试媒体条纹不重染 |
| 剪贴板 | 三栏、分区薄边框、深凹输入、蓝选中、主/次按钮 | [clipboard](clipboard.png)；卡片/导航/按钮来自公共组件，右侧保存与复制均完整 |
| 输入焦点/空态 | 原生焦点环，清除搜索后恢复条目 | [clipboard-empty-focus](clipboard-empty-focus.png)，真实输入无结果文本并断言可见项为零；Ctrl+A/Backspace 恢复 |
| hover/选中 | 普通卡提亮，选中悬停保持蓝底蓝边 | [普通悬停](clipboard-hover.png)、[选中悬停](clipboard-selected-hover.png) |
| 菜单 | 同样面板底、边框与文字色 | 实际点击历史菜单：[clipboard-menu](clipboard-menu.png)，没有执行清理 |
| 启动台 | 分类/图标网格/详情保持原布局，类别与 fallback 用 Lucide | [launcher](launcher.png)、[单击选中与表单](launcher-selected-focus.png)；保存无改动时禁用，未触发程序启动 |
| 强调色同步 | 设置、剪贴板、启动台均同步，选中/主按钮不能残留蓝色 | 真实色板点击后 [settings-teal](settings-teal.png)、[clipboard-teal](clipboard-teal.png)、[launcher-teal](launcher-teal.png)；断言灰青后恢复蓝 |
| 设置 | 左导航、右横向设置行、分区线、固定页脚 | [外观](settings-appearance.png)、[快捷键](settings-shortcuts.png)、[标签](settings-tags.png)、[AI](settings-ai.png)、[更新](settings-updates.png)、[软件](settings-software.png)；默认 F1–F9 完整 |
| 历史/辅助弹层 | 相同输入/状态/颜色角色，保留原分区 | [空历史](history.png)、[统计](statistics.png)、[批量操作](bulk-actions.png)、[JSON导出待机](export.png)；统计/批量读模型待 ready 后截图；没有执行删除或导出 |
| 小窗口 | 正文可滚动，关键操作和页脚不被裁掉 | [小窗设置](compact-settings.png)；[小窗剪贴板](compact-clipboard.png)→[滚动到底](compact-clipboard-scrolled.png)，“我的分组”可达、右侧保存/复制和捕获栏完整 |
| quiet/真实变化 | 静置不重复绘制/查询，真实输入和设置变化更新 | Launcher 稳定后 `ui.render_count` 不变；Gallery `snapshot_revision`、Clipboard revision 静置不变；搜索和强调色实际操作有相应变化，场景有断言 |

fmt、locked check/build、20 项测试（UI Bridge 13 / Settings 3 / i18n 4）通过；
主题守卫正反探针及全部治理守卫通过，`git diff --check` 通过。
构建有既存 Windows `LNK4098 LIBCMT` 警告，无编译错误；架构守卫仍提示两个无 Cargo.toml
的历史目录（preview/media-playback），本轮未清理。未扩大架构基线或新增跨插件依赖。

## 差异与未验证边界

- 主窗保持已接受的紧凑树/媒体布局，剪贴板仍为三栏；概念拼图中的四栏、图片/PDF
  采集、媒体勾点和新设置项未实现，因为它们是生成图偏差，不是用户指定功能。
- 小窗口的导航分组及设置 F1–F9 采用已有正文滚动退化；页脚固定。不是所有内容在
  992×629 逻辑像素时都同时展开可见。默认尺寸完整，小窗滚动可达。
- 本次统计为隔离库的零索引状态，历史为未纳入 Lore 的空态；不能据此宣称统计数据、
  历史分支/冲突/错误、标签编辑/拖放、Debug 每种状态均已重验。它们已迁移公共 token/
  输入，但本轮没有逐状态运行覆盖。既有英文文案未作全量翻译。
- 导出仅核对待机，更新仅查看已有状态，未下载/安装；OS 文件选择、删除、覆盖、
  卸载对话框和 Windows 原生标题条由系统管理，本轮没有强制重绘或验收其主题。
- 词法守卫能阻断 UI 六/八位硬编码色、局部 Theme 改写和直接 Input/Textarea 构造，
  无法证明所有布局像素相同，也不识别任意十进制/计算色。继续执行主题规范和实图审查。
