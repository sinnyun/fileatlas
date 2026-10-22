# WorkCrate 对象信息移至底栏（2026-10-23）

用户局部布局调整：右侧画布对象信息卡片删除，选中数量、单对象名称/类型移到固定底栏右侧。
沿用已查看的活动检查器/画布概念以及 editing-tools 本轮清单；其余文字编辑、图片和物料卡片保持。
无选中显示“未选中对象”，多选显示数量。名称折叠空白，单行右对齐，宽度上限60%/600逻辑像素；
长内容按原生 truncate 省略。底栏左侧保留操作状态/错误，底栏高度仍24，不新增 IO、轮询或文档写入。

环境：Windows，150% DPI，Rust 1.97.1 MSVC，正常 target/debug/fileatlas-shell.exe；
同进程 FileAtlas MCP，隔离 Full profile，官方 headless 会话。TEMP/TMP 在
.local/validation/profiles/wc-object-footer-tests/temp。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-ui -- --test-threads=1
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-object-footer-2026-10-23 --access-level full --scenario scripts/validate-workcrate-object-footer.cjs
```

格式、locked check/build、治理检查通过，12 项 UI 回归通过；现有链接器 LNK4098 警告仍在。
同进程 26 个归档步骤通过：选中文字后状态包含数量/名称/类型，几何断言在画板下方右侧；
实际修改内容后名称更新；空白处取消选中后变为未选中对象。还验证默认微软雅黑、文字编辑、
真实角点缩放与画布缩放按钮保留，主题变更、焦点及静置3.5秒目录安静。

已实际查看并对照清单：

- [正常编辑](../../../.local/validation/artifacts/2026-10-23-workcrate-object-footer/object-footer-scaled.png)：右侧从文字内容开始，底部右侧单行显示对象信息。
- [小窗选中](../../../.local/validation/artifacts/2026-10-23-workcrate-object-footer/object-footer-small-window.png)：1600×1100，底部右对齐，卡片留在原滚动区。
- [取消选中](../../../.local/validation/artifacts/2026-10-23-workcrate-object-footer/object-footer-empty.png)：底栏为未选中对象，文字更改按钮沿用禁用态。

生成报告、样本、数据库和图片仅保存在 .local/validation；证据归档在对应 artifacts 目录。
本轮没有单独进行多选或超长名称的人工交互测试；其显示采用同一现有选区数量和单行截断逻辑。
未扩大声称其他未改动的图片或 GUI 专用路径已通过。
