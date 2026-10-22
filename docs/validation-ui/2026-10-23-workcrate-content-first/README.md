# WorkCrate 页面内容置顶（2026-10-23）

用户要求右侧删除页面信息，并将页面内容移到顶部。按活动参考及 editing-tools 本轮局部清单，
删除页面信息/只读标题卡片，将原内容检查器放在文字和图片编辑前；三栏、滚动区、底栏保持。
自动标题继续显示在列表与编辑顶栏。原卡片的错误反馈放入内容卡片，未删除错误处理。
无数据、文档、会话或主题模型变化。

Windows，150% DPI，Rust 1.97.1 MSVC，正常 target/debug/fileatlas-shell.exe；
同进程 FileAtlas MCP stdio，隔离 Full profile、官方 headless。TEMP/TMP 位于
.local/validation/profiles/wc-content-first-tests/temp。

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
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-content-first-2026-10-23 --access-level full --scenario scripts/validate-workcrate-content-first.cjs
```

格式/locked check/build/治理检查通过，12 项 UI 回归通过；现有 LNK4098 警告仍在。
同进程 34 个归档步骤通过：page-auto-title 控件不存在，物料类别行几何高于文字输入；
真实点击置顶物料名称并保存“置顶物料”，纸内同步更新。文字内容编辑、微软雅黑、角点缩放、
画布放大缩小和底栏对象信息保持。更换原生主题并确认焦点/选中及空选区态，静置3.5s reload不增长。

已实际查看截图并逐项比较当前清单：

- [正常编辑](../../../.local/validation/artifacts/2026-10-23-workcrate-content-first/content-first-scaled.png)：右侧页面内容第一，文字内容第二，画板/图片功能随后；无页面信息。
- [保存后及主题](../../../.local/validation/artifacts/2026-10-23-workcrate-content-first/content-first-accent.png)：置顶物料已保存，左侧状态和底栏右侧对象信息同时可见。
- [小窗](../../../.local/validation/artifacts/2026-10-23-workcrate-content-first/content-first-small-window.png)：1600×1100，顶部字段完整可操作，后续卡片在原滚动区访问。

生成报告、截图、原生样本和数据库仅保存在 .local/validation；对应 artifacts 目录作为本地归档。
本轮未扩展或重新验证图片链接和官方 GUI 专用路径。旧场景的自动标题卡片断言已更新为不存在，
标题生成本身仍由既有页面顺序服务负责。
