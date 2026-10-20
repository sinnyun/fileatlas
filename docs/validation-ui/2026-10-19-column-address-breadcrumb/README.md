# 栏目地址面包屑：当前目录并入路径尾段 — 2026-10-19

## 用户反馈

界面中间“前进/后退”所在的面包屑没有显示当前文件夹，末尾只到上一级目录名。

## 根因

`fileatlas-plugin-file-manager` 的每栏地址行把上级路径段放在一个 `overflow_x_scrollbar()`
容器里。gpui-component 的 `Scrollable` 包装会给容器加 `size_full()`，因此上级路径
区始终撑满剩余宽度，当前目录段被推到地址行最右端，与上一级之间出现大段空白，
读起来像“面包屑结束于父目录”。当前目录段其实存在，只是视觉上脱离路径。

## 实现改动

`crates/fileatlas-plugin-file-manager/src/view/directory_browser.rs` 的 `render_column_address`：

- 上级路径容器由 `flex_1 + overflow_x_scrollbar` 改为 `flex_shrink_1 + min_w_0 +
  overflow_x_scroll`（带 `file_manager-column-{n}-address-trail` 元素 id），按内容自然宽度
  排布，只在空间不足时收缩并保留横向滚动。
- 当前目录段保持钉在路径尾部（`flex_shrink_0`），因此路径可读为
  `F:\ › … › 上一级 › 当前目录`；空间不足时上级段在自身宽度内被裁切/滚动，当前目录仍可见。
- 当前目录段之后新增 `flex_1` 空白段，编辑（铅笔）、打开文件夹与刷新按钮继续靠右，
  地址行其余控件位置不变。

侧栏 Navigation 面包屑（`command.breadcrumb`）原本已包含当前目录，未改动。

## 验证

- `cargo +1.97.1 fmt --all -- --check`：通过（改动后重新格式化）。
- `cargo +1.97.1 build -p fileatlas-shell --locked -j 2`：通过（关闭本工作区正在运行的
  `target/debug/fileatlas-shell.exe` 后重建常规 EXE）。
- `cargo +1.97.1 test -p fileatlas-plugin-file-manager --locked -j 2`：24 passed / 0 failed。
  该测试目标此前无法编译，本轮顺带修复 `src/tests.rs` 的插件依赖装配：
  `DragDropService::default()`、补 `fileatlas/indexing`、`fileatlas/workspaces` 服务与
  `fileatlas.columns` 插件启动。
- `node scripts/check-ui-theme.cjs`、`check-ui-refresh.cjs`、`check-architecture.cjs`、
  `check-feature-ledger.cjs`：全部通过（未新增颜色、计时器或跨插件私有依赖）。

同进程 FileAtlas MCP（`scripts/fileatlas-mcp-debug.cjs`，隔离 profile
`.local/validation/profiles/breadcrumb-check`，`--access-level standard`，
`FILEATLAS_DEV_PLUGINS=dev-test`，Windows 150% DPI）导航到
`.local/validation/profiles/breadcrumb-check/甲/乙/丙-当前目录` 后截图：

- 单栏：面包屑连续显示到 `丙-当前目录`，编辑/打开/刷新按钮仍靠右。
  [单栏截图](../../../.local/validation/artifacts/2026-10-19-column-address-breadcrumb/single-deep.png)
- 双栏：仅第 0 栏有路径，面包屑尾段仍是当前目录，上级段在栏宽内裁切。
  [双栏截图](../../../.local/validation/artifacts/2026-10-19-column-address-breadcrumb/dual-deep.png)
- 四栏：同上，当前目录保持可见。
  [四栏截图](../../../.local/validation/artifacts/2026-10-19-column-address-breadcrumb/quad-deep.png)

场景计划：`.local/validation/profiles/breadcrumb-check/plan2.json`、`plan3.json`（本地生成物，
不入 Git）。

## 仍待验证 / 已知限制

- 上级路径被裁切时不再有可见滚动条（原 `overflow_x_scrollbar` 覆盖层已移除），
  被隐藏的中间层级需横向滚轮查看；未实现资源管理器式“…”折叠层级入口。
- 面包屑点击跳转、编辑态与真实键盘/鼠标输入未在本轮重复回归（沿用
  [2026-10-16 地址栏验证](../2026-10-16-address/README.md) 与
  [2026-10-19 栏目地址验证](../2026-10-19-columns-foundation/README.md) 的结论）。
- 未覆盖极小窗口下单栏地址行的换行/裁切表现。
