# WorkCrate 预览同步与编辑优化（2026-10-22）

## 范围与实现

用户反馈预览页没有跟随画布修改，并要求优化项目/页面、图片文字编辑与右侧面板。
保留三层布局及官方 VectorCraft 编辑语义，不新增渲染器、历史系统或存储格式。

- 修复编辑帧只更新中央画布的问题：同一解码图片回填当前页/左侧缩略图，
  官方 PNG 通过现有原子缓存写入器更新版本 1 预览目录。重新进入页面无需手动全工程同步。
- 预览/编辑开关保留显式打开的官方会话和撤销历史，预览态继续观察官方 revision。
  初次浏览仍无官方进程/端口访问；离开页面/项目提交并关闭。显式 GUI 打开后
  建立预览观察会话，但本轮没有实际启动 GUI 验证该分支。
- 800 ms 官方变化探测只在版本变化时取图；空闲无可见状态变化。修复探测完成被
  timer 提前消费以及过期探测覆盖新状态的风险。大范围手势保留松手位置，
  提交中禁用重复编辑，画板外松手也结束手势。
- 同项目重载保留旧图、优先读当前页；页面内操作不重复解码所有项目封面。
  项目未指定主图时显示首个本地页面（Contain，整页不裁切），搜索随输入更新。
- 右侧使用多行文字输入；更改文字、更新链接按选区类型禁用；切换对象刷新字段。
  字体/字号在同一行，应用字体字号及左/中/右对齐调用官方 text.setStyle。
  格式框为更改请求，留空保留；不是混合格式属性的完整回读。
- 坐标/尺寸拒绝非有限数字，文件选择结果到达后复核页面/选区，旧目标不再误写。

## 参考与视觉核对

已打开并检查活动 October 21 projects/pages/studio 用户稿、inspector 图，以及
October 22 canvas-edit 概念。清单见 concepts/workcrate-three-level-2026-10-20.md
的本轮补充。默认 Windows 截图 3286×1831，缩窗请求 1280×960；使用 GPUI
原生主题，截图包含实际窗口缩放，未使用 OS 输入模拟。

| 检查 | 结果与边界 |
|---|---|
| 三层导航、左栏/画布/右栏 | 保留；编辑与预览的选区显示不同，内容同步 |
| 多行输入及字号 | 真实 GPUI ctrl-a、输入、Enter、应用按钮通过，官方对象回读两行一致 |
| 字号/段落对齐 | 真实按钮通过，官方 revision 增加，截图显示变更；完整字体家族回退未穷举 |
| 缩略图与预览 | 已验 BoardCard revision 同步，官方集成测试核对缓存 PNG 字节与画布 PNG 相同 |
| 小窗口 | 右栏独立滚动可到格式/图片按钮，中央保持整页适配；工具带允许换行 |
| 项目卡片 | 本地页面作为主图后备，最终改为 Contain 避免裁切；自选照片保留 Cover |
| 静止状态 | 场景前后 ui 状态完全相同 |

保留差异：没有扩展概念中所有字体样式/颜色/裁切工具；本轮只实现上述真实官方
能力。不是整个 WorkCrate 全功能或性能基准验收。没有量化宣称 FPS/毫秒改善。
普通默认截图下右侧为密集表单，小窗口通过滚动访问底部；大型真实工程、所有字体、
亮暗/强调色跨插件矩阵、原生文件对话框、GUI 开关/崩溃恢复及外部 GUI 反向编辑
没有在本轮覆盖。画板外松手新增接线已编译，独立 OS 窗口外释放未验。

## 命令及证据

Windows，Rust 1.97.1 MSVC，官方 vectorcraft-cli 0.14.0。知识图谱以 FileAtlas
查询 canvas/workcrate 返回 0，按仓库规则回退到源码读取。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-boards -j 2
$env:FILEATLAS_VECTORCRAFT_EXE='C:\Program Files\VectorCraft\vectorcraft.exe'
$env:WORKCRATE_NATIVE_TEST_ROOT='F:\AI\geren\pho_cordis\.local\validation\profiles\wc-refresh-native-20261022'
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-boards canvas_edits_land_in_the_native_document -j 2 -- --ignored --nocapture
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-refresh-20261022b --access-level full --scenario scripts/workcrate-canvas-scenario.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-refresh-20261022b --access-level full --scenario scripts/validate-workcrate-refresh-editor.cjs
```

上述静态检查通过，普通测试 16 passed / 3 ignored；另外显式运行的官方画布集成
测试通过（并非把 ignored 当作 passed）。同进程两个场景通过。theme 自检及 theme /
refresh / architecture / feature-ledger / layering 守卫通过，git diff --check 通过。
链接器仍报告已有 LNK4098 库冲突警告，build 成功。

首轮 wc-refresh-20261022 场景在后台操作未完成时点击禁用缩放按钮失败，场景修正
为等待 busy=false；重跑 wc-refresh-20261022b 通过。

全部产物仅在 git-ignored `.local/validation/profiles/`：
`wc-refresh-20261022b/screenshots/` 包含 preview-live-update、inspector-multiline-style、
preview-multiline-result、editor-small-window、editor-small-inspector-scroll、
pages-updated-cached-previews、projects-local-page-cover PNG；
同 profile 的 canvas-scenario.json、editor-refresh-result.json 和 MCP 报告为本机证据。
没有提交截图/原生文档/数据库/日志。
