# WorkCrate 简化文字格式并保留缩放（2026-10-23）

用户明确删除文字格式编辑并默认微软雅黑，同时保留大小缩放。
沿用活动检查器概念及本轮 editing-tools 局部清单，删除右侧字体/字号/对齐卡片、
专用实体和事件处理；保留内容输入、更改/新建文字、画布对象句柄和工具栏缩放。
新增正文文字和物料栏统一使用 Microsoft YaHei 默认值，已有作品不批量重排或改写。

环境：Windows，150% DPI，Rust 1.97.1 MSVC，官方安装在 C:/Program Files/VectorCraft。
正常 target/debug/fileatlas-shell.exe，通过同进程 MCP stdio、隔离 profile Full 和官方 headless 验证。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -- --test-threads=1
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-plain-text-2026-10-23 --access-level full --scenario scripts/validate-workcrate-plain-text.cjs
```

上述检查通过，40 项回归通过、4 项原有官方门控测试仍 ignored。
TEMP/TMP 在 .local/validation/profiles/wc-plain-text-tests/temp；链接器仍有原有 LNK4098 警告。
同进程 24 个归档步骤通过：选中文字后确认格式输入与按钮不存在，内容编辑真实生效；
官方 text.getRange 返回 font_family=Microsoft YaHei；按实际画板映射拖动右下角句柄，
官方返回宽度增大且字体保持。画布放大、缩小按钮分别改变倍率并恢复原值。
切换原生主题并检查焦点、选区、禁用边界状态；静置 3.5s reload 不增加。

已实际查看并对照本轮清单：

- [缩放结果](../../../.local/validation/artifacts/2026-10-23-workcrate-plain-text/plain-text-scaled.png)：文字放大、选区句柄仍在，无文字格式卡片，其他卡片和三栏结构保持。
- [原生主题](../../../.local/validation/artifacts/2026-10-23-workcrate-plain-text/plain-text-accent.png)：原生紫色焦点/选中/缩放悬停提示保留。
- [小窗口](../../../.local/validation/artifacts/2026-10-23-workcrate-plain-text/plain-text-small-window.png)：1600×1100，文字内容编辑可见，其余内容沿用右侧滚动区。

生成报告、原生样本、数据库和截图只留在 .local/validation；对应归档为上述 artifacts 目录。
仅删除格式编辑界面；底层官方字体/变换服务未被替代。已有文档字体兼容保留，
本轮无 GUI 专用交互、其他字体覆盖率或实际打印验证声明。
