# 右侧信息面板：完整内容与概念图对照

2026-10-16，Windows / 150% DPI / Rust 1.97.1 MSVC。

设计依据是[完整状态概念 v2](../../reference-ui/concepts/right-inspector-complete-2026-10-16-v2.md)，
该文件包含内置生图生成的图片、完整提示和界面约束。原程序深色稿、历史树稿继续作为局部依据。

## 本轮结果

四种上下文及文件信息/历史/扩展分页已用同进程 PhoTag MCP 实测。
程序、标签、文件夹和文件信息使用实际模型；历史的多版本数据使用明确标识的隔离演示模型。
所有截图均为 MCP `screenshot_window` 的原始窗口截图，没有重绘、拼接或用概念图代替。

| 状态 | 已填充及核对内容 | 运行截图 |
|---|---|---|
| 程序 | 身份卡、2×2概览、实际位置、绿色就绪状态、提示；当前18文件/5子目录/8标签；无具体标签列表、媒体库容量或扫描任务 | [程序](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/program-complete.png) |
| 标签 | 名称、色值、父级、18关联、5个子标签；输入与保存、7色色板、可见性开关、子项数量/菜单 | [标签](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/tag-complete.png)、[子项菜单](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/tag-child-menu.png) |
| 文件夹 | 实际名称、18当前结果、5子目录、完整路径、浏览状态、复制/打开/刷新入口 | [文件夹](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/folder-complete.png) |
| 文件 | 真实缩略图、4星、5个彩色标签、名称/大小/尺寸/时间、相机/镜头/光圈/快门/ISO/拍摄时间与路径 | [文件](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/file-complete.png)、[EXIF与路径](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/file-exif-complete.png) |
| 模拟历史 | 主线v1/v0、弯曲侧分支v0.1、图片缩略图、选中描边、分支选择、创建/切换、完整版本详情 | [历史](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/history-complete.png)、[分支筛选](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/history-branch-filtered.png) |
| 普通历史 | 真实当前副本和元数据、无旧版本状态，演示开关关闭且没有残留模拟节点 | [普通启动](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/history-normal.png) |
| 文件扩展 | 当前无文件扩展贡献，显示实际空态 | [空态](../../../.local/validation/artifacts/2026-10-16-inspector-complete/screenshots/file-extensions-empty.png) |

## 数据及边界

- `target/mcp-inspector-complete-20261016/fixtures/2026-秋日旅行` 的根目录有18个真实JPG副本、
  5个子目录，每个子目录另有3个副本。标签为3个根、5个子标签。文件内容是项目的合成渐变样例，
  非相机实拍。仓库保存的3张小JPG见本目录 `fixtures/`。
- `prepare-fixture.py` 给隔离的 `DSC08812.jpg` 加入有效 EXIF APP1 元数据，保留JPEG栅格字节。
  相机/镜头/曝光字段是测试信息；这验证字段解析及显示，不代表真实相机/editor互操作验收。
- 文件夹和标签通过真实服务创建，关联数经过 `assign_tags` 写入并由 `list_library` 核对。
  代表文件实际存在、rating=4、thumb_path有效、关联5个标签；没有使用missing虚拟文件替代预览。
- 多版本图只在 `PHOTAG_DEMO_INSPECTOR=1`、`dev-test`、隔离profile同时启用时出现。
  创建/切换只是内存演示，最多16节点，不保存版本或改写文件内容。普通启动没有这些示例。
  真实版本存储与恢复策略继续按用户此前决定暂缓。
- 程序、标签、文件夹上下文不显示文件独有的历史/扩展页签。
  不存在的数据（如标签创建时间）未虚构为产品字段；更多既有标签选项仍由完整编辑器承载。

## 发现并修正的问题

1. 历史占位缩略图的 `size_full` 抢占整行，版本文本被挤出；改为固定尺寸图片槽。
2. 上轮“缩略图解码未通过”的实际原因是引用了不存在的文件路径；改用已存在JPG后正常绘制。
3. 历史大预览、重复标题和多余留白挤掉版本详情；改为紧凑身份栏，详情卡使用图文并列布局。
4. 文件夹沿用索引/扫描字段；改为当前浏览投影及真实未知计数状态。
5. 空白取消选择后仍显示目录详情；现在显示程序概览并保留当前目录统计，再选同目录不重扫。
6. 保存标签后旧快照回填输入框；现在根据ID/模型名称同步，同时保留未提交编辑。
7. 分支筛选后仍展示主线标识/不在分支内的选中详情；现在同步颜色、线条及有效选中项。
8. EXIF同时包含友好名和原始字段，造成重复和英文长标签；展示层合并完全相同别名，保留不同值及未知字段。

## 验证结果

静态与回归：

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-i18n -j 2
node scripts/check-ui-refresh.cjs
git diff --check
```

最终 Gallery 21项、i18n 4项通过；包括一项防止EXIF合并丢失不同值/未知字段的回归。
MCP crate 编译/test目标通过（0测试，不宣称自动覆盖工具行为）。
默认构建为 `target/debug/photag-shell.exe`；完整检查的独立输出为
`target/debug/photag-shell-inspector-complete-final.exe`。

`verify.cjs` 完成95步同进程调用并断言通过：

- 实际指针保存改名/恢复、色板蓝→红、可见性关→开；父级现有规则同步影响子标签；子项菜单可见。
- 选中/取消、同目录重选不重新枚举、实际空白处点击返回程序。
- 隔离目录真实新增/删除文件后结果18→19→18，程序上下文保持。
- 版本v0选中、切换为当前、创建v2后节点3→4、分支菜单及筛选选中v0.1；宽度360/300和较短窗口截图。
- 静止两次日志latest_id为969→969。

随后 `verify-normal.cjs` 在**无演示环境变量**的最终构建上完成20步，核对去重后的EXIF/路径、
普通工作副本历史和1920×1080小窗口；演示enabled=false，节点count=null，静止日志426→426。
两个脚本均只由项目MCP控制窗口。简要结构化证据见 [results.json](../../../.local/validation/artifacts/2026-10-16-inspector-complete/results.json)，
完整报告、DB、EXE和补充截图位于隔离profile的target输出中。

小窗口内容通过既有滚动区/展开区访问；截图检查了布局，不宣称本轮已穷尽滚轮、非150%DPI、
所有格式/相机EXIF、原生复制路径/Explorer交互或真实非空文件扩展贡献。

## 复现

关闭该隔离测试实例后准备样例；以下脚本只接受workspace/target下的profile。

```powershell
python docs/validation-ui/2026-10-16-inspector-complete/prepare-fixture.py
$fixtureAssets = (Resolve-Path docs/validation-ui/2026-10-16-inspector-complete/fixtures).Path
$env:PHOTAG_DEMO_INSPECTOR = '1'
$env:PHOTAG_DEMO_HISTORY_THUMBS = (@('history-v1.jpg','history-v0.jpg','history-branch.jpg') | ForEach-Object { Join-Path $fixtureAssets $_ }) -join ';'
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-inspector-complete-final.exe --profile target/mcp-inspector-complete-20261016 --plugins dev-test --allow-mutations --scenario docs/validation-ui/2026-10-16-inspector-complete/seed.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-inspector-complete-final.exe --profile target/mcp-inspector-complete-20261016 --plugins dev-test --allow-mutations --scenario docs/validation-ui/2026-10-16-inspector-complete/verify.cjs
Remove-Item Env:PHOTAG_DEMO_INSPECTOR
Remove-Item Env:PHOTAG_DEMO_HISTORY_THUMBS
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-inspector-complete-final.exe --profile target/mcp-inspector-complete-20261016 --plugins dev-test --allow-mutations --scenario docs/validation-ui/2026-10-16-inspector-complete/verify-normal.cjs
```

指针场景按本机1920×1800、150%DPI及360宽度面板记录；新profile由seed以固定顺序建立标签和文件。
MCP `window_status.geometry` 给出截图至逻辑指针的换算。本机为 `(screenshot - (11,45)) / 1.5`。
复用任意已有profile或改变DPI时，应先核对ID与窗口几何。
