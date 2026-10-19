# 标签侧栏胶囊样式 — 2026-10-16

## 视觉参照与实现

依据用户反馈生成[带标签颜色的胶囊概念图](../../reference-ui/concepts/sidebar-tag-capsules-color-2026-10-16.png)，并调整实际 GPUI 标签树。每个标签是独立圆角胶囊，底色和边框从该标签的自定义颜色取色；未选中态用低透明度底色，选中态加强同一颜色，不再统一使用蓝色选中背景。每个胶囊含标签色点、名称、固定宽度的右对齐计数徽标和始终可见的“…”菜单按钮。计数徽标和菜单在胶囊右侧形成固定操作区，名称在余下空间中截断。子标签保持缩进及父子连线。

![带标签颜色的胶囊概念图](../../reference-ui/concepts/sidebar-tag-capsules-color-2026-10-16.png)

## 验证

- `rustup toolchain list`：已安装 Rust 1.97.1 MSVC。
- `cargo +1.97.1 fmt --all -- --check`：通过。
- `cargo +1.97.1 check --locked -p photag-plugin-navigation -j 2`：通过。
- `cargo +1.97.1 build --locked -p photag-shell -j 2`：通过。
- 使用 `scripts/photag-mcp-debug.cjs` 同进程 MCP 和隔离 profile `target/mcp-tag-style-final5`，建立 3 个父标签、7 个子标签，并为标签设置多种颜色。
- [实际窗口截图](../../../.local/validation/artifacts/2026-10-16-tag-sidebar/tags-expanded.png)显示胶囊按标签颜色着色，计数徽标在各行对齐，“…”操作入口始终可见；[菜单截图](../../../.local/validation/artifacts/2026-10-16-tag-sidebar/tags-menu-open.png)确认按钮可打开真实标签操作菜单。窗口 DPI 为 150%。

MCP 使用 `PHOTAG_MCP_ALLOW_MUTATIONS=1`，仅操作隔离 profile 中的临时标签；测试数据无媒体关联，因此计数为 0。非零计数显示和筛选行为未在本轮验证。
