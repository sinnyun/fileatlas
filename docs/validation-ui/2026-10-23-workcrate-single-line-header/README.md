# WorkCrate 单行物料栏与差异保存（2026-10-23）

## 范围与设计清单

使用已实际查看的 workcrate-inspector-2026-10-21.png 和本轮用户明确要求；
清单位于 reference-ui/concepts/workcrate-editing-tools-2026-10-20.md。
保留三栏与检查器字段行，仅修改原生文档内的物料栏生成/保存方式。

- 左侧编号、独立物料类别各一个官方文本对象。编号仍按页面顺序生成。
- 右侧为一个官方右对齐点文本对象，以 ` | ` 连接物料名称、尺寸、数量与附加字段内容。
  只显示值，不显示类别、数量或自定义字段的名称；编辑面板保留名称。
- 空值不输出分隔符；显示将换行/连续空白折叠为一行。原始字段保存在官方节点 note，
  不通过分隔符反向拆分，因此内容本身含竖线、换行或空字段也能重开还原。
- 普通保存只对变化的整段文本调用 text.setText；背景、页码框、分组和其他文字的 ID 不变。
  不重复分组/创建/删除/移动/设样式，也不逐字段计算宽度。
- 固定左右区域，只检查变化后整个文本的官方边界。过长则撤销本次操作、保留输入并提示，
  不换行、截断或缩小字体。旧版物料栏首次成功保存迁移为 layout 2；浏览不迁移。

## 环境与复现

Windows，150% DPI，Rust 1.97.1 MSVC，官方 VectorCraft 安装在 C:/Program Files/VectorCraft。
正常 target/debug/fileatlas-shell.exe，通过同进程 MCP stdio 子进程进行全部交互；
隔离 profile 的 Full 权限仅用于测试。官方自动化使用无界面会话。
截图正常尺寸 3286×1831，小窗口 1600×1100。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-settings -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -- --test-threads=1
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-single-line-2026-10-23-values --access-level full --scenario scripts/validate-workcrate-single-line-header.cjs
```

测试 TEMP/TMP 使用 .local/validation/profiles/wc-single-line-tests-retry/temp。
构建、格式、检查与全部治理守卫通过；50 项测试通过，4 项既有官方门控测试仍 ignored。
第一次并发运行 Settings 的两个既有文件保存测试失败（PermissionDenied / 磁盘值断言），
独立目录串行重跑四个 crate 全部通过；并发文件测试的偶发问题未在此任务中修改。
链接器仍有现有 LNK4098 警告。

## 同进程结果与截图对照

最终场景 passed=true，migration=true。真实输入并保存类别、物料、数量；通过诊断
header_state 比较官方对象模型、ID、分组背景及最后写入命令，确认正常保存仅更新目标文字。
批量更新尺寸/数量/附加字段只写一次 details；仅空字段元数据变化不写 text.setText；
修改页面标题只改画板名，尺寸标记不修改物料栏。关闭并重开原生文档后值与对象 ID 保留。
长内容通过服务和真实编辑输入分别拒绝；撤销后所有文字模型一致，错误与草稿保留。
最终静置 3.5 秒目录 reload 计数不增长。

旧版测试复制已有 layout 0 文档，首次保存迁移一次，保留独立类别「活动展示」，
下次修改数量复用三个文本节点。脚本在旧样本不存在时跳过此分支并报告 migration=false；
本次样本来自 .local/validation/profiles/wc-defaults-2026-10-23-v3/documents/默认信息测试A.vectorcraft。

已实际查看以下同进程截图，和清单逐项比较：

- [完整单行](../../../.local/validation/artifacts/2026-10-23-workcrate-single-line-header/single-line-complete.png)：左侧编号/类别，右侧仅值、右对齐一整行；检查器保留字段名称。
- [超长错误](../../../.local/validation/artifacts/2026-10-23-workcrate-single-line-header/single-line-overflow.png)：纸内保留原值；编辑框保留草稿、保存/取消可见，底部错误未被被动刷新清除。
- [小窗口](../../../.local/validation/artifacts/2026-10-23-workcrate-single-line-header/single-line-small-window.png)：三栏保持，纸内仍单行；工具栏沿用现有自适应换行，检查器下方字段需滚动访问。

原始报告与摘要仅在 .local/validation/profiles/wc-single-line-2026-10-23-values，
对应归档位于 .local/validation/artifacts/2026-10-23-workcrate-single-line-header；不提交生成文件。
本轮未改变 UI 调色或控件样式，主题状态沿用前一轮 defaults 验证。

## 当前边界

物料栏左右区域固定；无限长内容不会自动扩展栏位，必须缩短后保存。
右侧各字段仍在检查器分别录入，聚合为一个原生文本对象交给官方 CLI。
手动解锁并只改 VectorCraft 中的派生右侧文字，不会反向覆盖结构化字段。
本次实际自动化验证为官方无界面会话；没有据此新增声称官方 GUI 操作验证通过。


## 黑色字体补测

同日用户要求全部黑字，编号、类别、右侧内容统一 #000000。布局及背景保持上方清单。
实际打开原参考图，并检查同进程最终截图：文字全黑，单行、间距及对齐无变化。

复现：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-black-header-2026-10-23 --access-level full --scenario scripts/validate-workcrate-black-header.cjs`。
环境、隔离及官方安装同上；脚本复用上次生成的 layout 2 非黑色旧样本。
新文档三个角色的官方 summary.fill 均 #000000；旧文档保存后全部黑色，节点 ID/背景分组不变，
只执行一次 text.setStyle，后续保存不再设样式。black-header-summary.json 全部通过。
fmt、locked shell check/build、theme self-test/theme/refresh/architecture/ledger 均通过。
截图和原始报告只保存于 .local/validation，归档对应路径如下。

[黑字效果](../../../.local/validation/artifacts/2026-10-23-workcrate-black-header/black-header-new.png)。
现有文档在下次保存页面内容时换色；本地浏览不自动改写用户文档。
