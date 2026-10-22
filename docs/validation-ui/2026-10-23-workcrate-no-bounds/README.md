# 移除 WorkCrate 位置与尺寸面板（2026-10-23）

用户要求删除页面编辑右侧的位置与尺寸区块。已查看活动检查器概念图
workcrate-inspector-2026-10-21.png，本轮局部清单在 WorkCrate editing-tools 概念文档。
保留其余三栏及卡片，选中对象后由“画布对象”直接接“文字内容/文字”。
删除 X/Y/宽高输入、应用按钮及专用输入实体、数字解析和同步；
官方对象 bounds、画布拖动/缩放和公共 Boards/MCP 服务不变，无存储格式变化。

Windows，150% DPI，Rust 1.97.1 MSVC，正常 target/debug/fileatlas-shell.exe。
全部交互通过同进程 FileAtlas MCP，隔离 profile、Full、官方 headless 自动化；
所有 TEMP/TMP 位于 .local/validation/profiles/wc-no-bounds-tests/temp。

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
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-no-bounds-2026-10-23 --access-level full --scenario scripts/validate-workcrate-no-bounds-panel.cjs
```

静态/构建/治理检查全部通过，12 项 UI 回归通过。链接器仍有原有 LNK4098 警告。
同进程 15 个归档步骤通过：新建并选中文字，确认四个数字输入与应用按钮不存在；
文字输入和字体控件、自动页面标题仍存在；实际输入并应用“保留文字编辑”，
官方对象返回新文字。更换原生主题后检查焦点和选中态，静置 3.5s 目录 reload 不变。

已实际查看并和删除清单对照：

- [选中对象](../../../.local/validation/artifacts/2026-10-23-workcrate-no-bounds/no-bounds-selected.png)：无位置与尺寸区块，文字/格式/图片/只读标题卡片保持。
- [主题与焦点](../../../.local/validation/artifacts/2026-10-23-workcrate-no-bounds/no-bounds-accent.png)：文字输入焦点和选区来自原生主题，新文字已应用。
- [小窗](../../../.local/validation/artifacts/2026-10-23-workcrate-no-bounds/no-bounds-small-window.png)：1600×1100，无删除区块，文字/格式可见，其余卡片继续在原滚动区访问。

原始报告和样本只保存在上述隔离 profile；对应证据归档在
.local/validation/artifacts/2026-10-23-workcrate-no-bounds，不提交生成截图或数据库。
本轮没有重新验证图片链接或官方 GUI 专用行为；未将其扩大报告为本轮覆盖。
