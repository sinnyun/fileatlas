# 剪贴板工作台界面优化 — 2026-10-18

用户未接受本版样式与分类行为；本文件保留为历史。其 `core` profile 不能保证隔离。
当前参考还原、完整分类和显式隔离的实测结果见[v2记录](../2026-10-18-clipboard-workbench-v2/README.md)。

## 设计方向

概念稿：[clipboard-workbench-layout-2026-10-18.png](../../reference-ui/concepts/clipboard-workbench-layout-2026-10-18.png)。
采用 FileAtlas 深色蓝灰色表面和蓝色选中态，把面板分成分类导航、历史卡片和内容编辑三栏；
左侧放搜索、分类/置顶筛选、分类新建、清理与捕获状态，中间突出记录预览和选中态，
右侧集中多行内容编辑、分类分配、置顶、保存、复制和删除。

概念稿中来源程序、嵌套分类、自动保存和详细时间元数据不是当前剪贴板数据模型提供的行为，
实现没有伪造这些信息。当前分类保持扁平，编辑仍需显式保存。

## 实现与验证

- 更新 `crates/fileatlas-plugin-clipboard/src/view.rs`：三栏排版、分类/置顶筛选选中状态、
  卡片选中边框和底色、统一间距/面板色阶、捕获状态/清理操作定位，以及 GPUI Kit 多行文本编辑器。
- 保留现有主窗口布局和通用悬浮面板能力。
- 工具链：`1.97.1-x86_64-pc-windows-msvc`。
- `cargo +1.97.1 fmt --all -- --check` — 通过。
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2` — 通过。
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2` — 通过；存在既有 LNK4098 默认库提示。
- `node scripts/check-layering.cjs`、`node scripts/check-architecture.cjs`、
  `node scripts/check-ui-refresh.cjs`、`node scripts/check-feature-ledger.cjs` — 通过。
- 同进程 FileAtlas MCP（隔离 profile，仅数据库 fixture 开启 mutation）：
  `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-clipboard-ui-redesign-final-2 --plugins core --scenario scripts/validate-clipboard-workbench.cjs --allow-mutations`
- MCP 实际点击选中历史记录并检查捕获暂停/恢复（状态 false → true → false）；截图：
  [clipboard-workbench-selected.png](../../../target/mcp-clipboard-ui-redesign-final-2/screenshots/clipboard-workbench-selected.png)。

## 未覆盖

- 当前 MCP 文本注入场景不能稳定验证分类名称键入，所以分类创建按钮的实际保存仍未由此场景覆盖；搜索、分类筛选、编辑/保存、复制和删除的各自点击流程也未在本次视觉验收中逐一覆盖。
- 同进程截图不证明系统剪贴板访问或全局快捷键投递；仍按插件既有运行边界记录。
