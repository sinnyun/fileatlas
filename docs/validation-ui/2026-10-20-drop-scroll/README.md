# 拖放完成后保留文件管理窗口浏览位置

2026-10-20 用户反馈：拖放成功后整个文件视图重新从开头显示。

根因：共同的 `start_file_operation` 完成回调在 `refresh=true` 时调用 `reload()`。
该方法是导航/筛选重载，会清空 `row_layout`、选择和 `ListState`，将 rich window 请求置于 0。
改为已有的 `refresh_index()`：保留旧内容显示、当前滚动位置及有效选择，在后台替换新数据。
文件移动/碰撞/数据库更新顺序未改变。该回调也供其他需要刷新结果的文件操作复用。

第二处原因在 Watcher 的背景重扫：扫描先发布少量/空的临时目录结果，原 monitor 直接替换
完整列表，导致 ListState 的内容范围变短，约半秒后又把滚动位置夹回顶部。
现保留同一目录已完成的画面，等新扫描完成再发布替换；首次进入新目录仍可流式显示批次。
仅修改完成回调不足以解决该延迟回跳；相关中间失败保留在 after/diagnostic/final/final2/final3
隔离 profile。临时调用追踪、额外滚动恢复和高度提示试验均已撤掉，最终只保留两处针对性修复。

## 同进程 MCP 回归

脚本 `scripts/validate-drop-scroll.cjs` 使用生产索引器扫描隔离的 760 张小图：
`00-before`/`30-after` 各 360 张，`10-source`/`20-target` 各 20 张。
启用子文件夹分组、按名称排序；滚动到媒体下标 360，点击选中未被移动的相邻文件，再使用实际 GPUI
pointer_down → pointer_drag → pointer_drag → pointer_up 放到目标组内卡片区域。
仅隔离 profile 的内部拖拽调试开关跳过平台 OLE 交接；本轮不调用直接移动控制入口。

修复前：`.local/validation/profiles/drop-scroll-before/`，真实文件移动成功，
但脚本断言失败 **viewport jumped: 8842 → 0**，复现用户反馈。

修复后运行同一脚本，检查：磁盘文件确实移动、总数仍为 760、目标组计数变为 21；
记录连续 35 次视口状态，滚动不归零；选中 ID 保留，rich window 仍处于 200 行之后，
静置一秒快照修订保持不变。截图用于比较移动前后可见的分组位置。

命令（运行前使用全新 profile 生成上述小图；复制自 `test-data/fileatlas-fixtures/01-small/tiny.jpg`）：

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked --bin fileatlas-shell -j 2
$dropProfile = Join-Path (Resolve-Path .local/validation/profiles).Path 'drop-scroll-final4'
$dropFixture = Join-Path $dropProfile 'library'
New-Item -ItemType Directory -Force -Path (Join-Path $dropFixture '00-before'), (Join-Path $dropFixture '10-source'), (Join-Path $dropFixture '20-target'), (Join-Path $dropFixture '30-after') | Out-Null
foreach ($dropIndex in 0..359) {
  foreach ($dropFolder in @('00-before','30-after')) {
    Copy-Item test-data/fileatlas-fixtures/01-small/tiny.jpg (Join-Path $dropFixture ('{0}/padding-{1:D3}.jpg' -f $dropFolder,$dropIndex))
  }
}
foreach ($dropIndex in 0..19) {
  Copy-Item test-data/fileatlas-fixtures/01-small/tiny.jpg (Join-Path $dropFixture ('10-source/source-{0:D2}.jpg' -f $dropIndex))
  Copy-Item test-data/fileatlas-fixtures/01-small/tiny.jpg (Join-Path $dropFixture ('20-target/target-{0:D2}.jpg' -f $dropIndex))
}
$env:FILEATLAS_TEST_LIBRARY = $dropFixture
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile $dropProfile --plugins all --access-level standard --scenario scripts/validate-drop-scroll.cjs
```

Windows，Rust 1.97.1，正常 target 的增量产物，MCP stdio child 与 GPUI 窗口同进程。
原始报告/截图/profile 全部在 `.local/validation/profiles/drop-scroll-before` 和
`.local/validation/profiles/drop-scroll-final4`。本次不改变布局或主题；沿用 FileAtlas 0.2。
物理 OLE、跨卷、删除/重命名的本轮实时回归不在该场景范围内。

## 最终运行结果

`drop-scroll-final4` **通过**：滚动位置 **8842→8842**；35 次连续采样最小/最大值均为 8842；
rich window offset **279→278**（列表重新排列后的当前中部页），选中 ID **382→382**。
源文件移除、目标文件存在，最终仍 760 项，源/目标分组变为 19/21。
移动过程中曾短暂显示 759 项（源项移除、目标项尚未交付），没有空白/批次列表替换或滚动重置。
静置一秒快照修订号 **11→11**。

最终 EXE SHA-256：`B29CA03A1C5B541BA315CFD8D659B3E59B66160272AB781A7CDF280EB651A68F`。
原图 `screenshots/before-drop.png`、`screenshots/after-drop.png` 与 `drop-scroll-report.json`
均位于上述本机 profile。正常 EXE 曾被路径已核实的本工作区进程占用，正常关闭成功后沿用原 target
增量构建，没有独立命名 EXE、新 target 或 cargo clean。

实际打开两张同进程截图比对：`10-source` 与 `20-target` 的标题和可见区域位置一致，
相邻的 `source-01.jpg` 仍选中，`source-00.jpg` 出现在目标组中。画面未跳到最前面的目录卡片。
原图 3286×1831，缩放 150%。FileManager 25 项回归测试、locked Shell check/build、fmt、
UI refresh、architecture、feature-ledger、theme 及 diff whitespace 检查均通过。
