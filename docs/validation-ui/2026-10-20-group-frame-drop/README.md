# 子文件夹整体分组框架与拖放

本轮按用户 2026-10-20 的要求，将分组标题和文件区放入共同框架。标题只处理折叠；
框架拥有目录身份和文件放置。长组按行虚拟化，连续行保留同一 group_label、边缘及共享拖拽反馈。
本轮覆盖前次 [仅标题目标记录](../2026-10-20-subfolder-drop/README.md) 的命中区域限制。

## 对照清单

参考为 FileAtlas 0.2 主窗与用户本轮整体分组要求，具体清单追加于
`docs/reference-ui/concepts/multi-column-workspaces-2026-10-19.md`；属于局部分组组件调整。

| 检查项 | 要求与范围 |
|---|---|
| 容器 | 框架包含标题、卡片、文件区空白；后续行同属一组 |
| 分区 | 组间间距在框架之外；保持现有标题高度、缩略图尺寸、栏目位置 |
| 边缘/反馈 | 首末行收口；边框与目标反馈消费原生 Theme；同一组可见段共享拖拽反馈 |
| 状态 | 展开、折叠、跨行；非子文件夹分组不移动文件 |
| 性能 | 保留按行虚拟化及 200 条 rich window；不整体加载长组 |

## 同进程 MCP 场景

Windows，Rust 1.97.1 MSVC；使用同一 target 的正常 `target/debug/fileatlas-shell.exe`。
MCP stdio child 和 GPUI 窗口同进程，Standard 权限。生成数据及输出全部留在 `.local/validation/`。

场景：`scripts/validate-group-frame-drop.cjs`。样例库 alpha 有五个待移动文件，beta 有 25 个文件，
确保目标有多行内容。通过 `dev-test.scan_fixture` 调用生产索引器。所有放置均由
`control_ui` 的 pointer_down → pointer_drag → pointer_drag → pointer_up 进入真实 GPUI
卡片和框架回调，不调用 `move_files_to_subfolder` 直接移动入口。

| 放置位置 | 检查 |
|---|---|
| 分组标题 | 源文件消失、目标文件存在、组计数更新 |
| 第一行内容下方空白 | 同上，命中框架而非标题/卡片 |
| 无标题后续行的空白 | 同上，验证保留的组目录身份 |
| 文件卡片上 | 同上，验证子元素不阻断框架接收文件 |
| 折叠组 | 同上，折叠后框架仍可放置 |

原生 OLE 拖出会等待物理鼠标，不能继续处理 MCP 合成鼠标。初次尝试记录在
`.local/validation/profiles/group-frame-20261020-a/`，表现为 pointer queue pending。
新增仅显式隔离 profile 可用的 `debug_internal_file_drag` 临时开关，跳过外部 OLE 交接，
保留同一 typed GPUI drag/drop；测试结束关闭。它不写设置、不改变正常启动默认行为。
该证据验证应用内部真实事件路由，尚不覆盖物理 Windows OLE 或外部应用拖入。

复现命令（每次使用新的 profile 名称）：

```powershell
$frameProfile = Join-Path (Resolve-Path .local/validation/profiles).Path 'group-frame-20261020-final'
$frameFixture = Join-Path $frameProfile 'library'
New-Item -ItemType Directory -Force -Path (Join-Path $frameFixture 'alpha'), (Join-Path $frameFixture 'beta') | Out-Null
foreach ($frameName in @('01-header','02-content-blank','03-continuation','04-card','05-collapsed')) {
  Copy-Item test-data/fileatlas-fixtures/01-small/tiny.jpg (Join-Path $frameFixture "alpha/$frameName.jpg")
}
foreach ($frameIndex in 1..25) {
  Copy-Item test-data/fileatlas-fixtures/01-small/tiny.jpg (Join-Path $frameFixture ('beta/target-{0:D2}.jpg' -f $frameIndex))
}
$env:FILEATLAS_TEST_LIBRARY = $frameFixture
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile $frameProfile --plugins all --access-level standard --scenario scripts/validate-group-frame-drop.cjs
```

## 静态/回归命令

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked --bin fileatlas-shell -j 2
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-file-manager -p fileatlas-plugin-mcp
node scripts/check-ui-theme.cjs
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
```

新增行布局测试检查无标题后续行与折叠行保留组身份和组末边界。
既有 LNK4098 链接警告保留，未修改工具链、依赖锁或构建目录。

## 最终结果

最终运行 profile：`.local/validation/profiles/group-frame-20261020-final`。
窗口缩放 150%，逻辑 client 为 2176×1183.33；最终 EXE SHA-256：
`FF16924B3E3ED0CC624D6F54B0E1D1ABC5EA801B561C7DD2CE0ADE2ED45F59C8`。
五类 GPUI pointer 放置 **5/5 通过**；源目录由 5 项变为 0 项，目标目录由 25 项变为 30 项。
测试关闭内部拖拽开关后，静置一秒 `snapshot_revision` 为 **28→28**，没有重复重载。
FileManager **25 项测试通过**，MCP 测试目标与 doc-tests 通过（该 crate 无独立单元测试）；
locked Shell check/build、fmt、主题 ownership/self-test、refresh、architecture、feature-ledger
及 diff whitespace 检查均通过。

本机证据：`group-frame-report.json` 和 `screenshots/` 均在上述 profile 下；
截图 `group-frames-before.png`、`group-frame-content-hover.png`、`group-frames-after.png`
由该轮 MCP child 捕获，原图尺寸 3286×1831。实际打开并检查悬停图：目标 beta 的所有可见
连续行共享背景和边框反馈，首个分组 alpha 不被高亮。之前逐行高亮造成的视觉断裂已修正。
折叠后边界由标题框架收口；正常画面保持卡片/标题/组间距结构。所有生成证据均为本地忽略文件。

## 边界

普通截图与内部拖动悬停截图单独查看。小窗口、其他主题/强调色、物理 OLE 和十万条的本轮
回归不在本次运行范围。没有将接口移动测试冒充物理鼠标输入覆盖。
