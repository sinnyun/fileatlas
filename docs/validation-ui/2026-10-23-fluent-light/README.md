# Fluent 轻质感浮起卡（亮色模式）— 2026-10-23

## 范围

用户指定：亮色模式下各框架布局区域（顶带/侧栏/内容区/检查器/状态栏）改为
微圆角浮起卡，彼此隔开一点距离并带柔和阴影；按钮/选框采用微微浮起的悬浮
样式；整体微软轻质感。同时移除「区域区分方式：边框/阴影」设置，统一为阴影。

## 实现

- `fileatlas-ui-bridge/src/theme.rs`
  - 删除 `RegionDepth` 枚举、`REGION_DEPTH` 原子量、`region_depth_name`/
    `set_region_depth_name`；`separate_region` 恒返回轻阴影（边缘方向偏移 2px、
    blur 6、spread -1）。
  - 新增共享阴影 token：`card_shadow`（key y=1/blur2 + ambient y=2/blur6/-1）、
    `control_shadow`（y=1/blur2）、`region_shadow`（key y=1/blur2 + ambient
    y=4/blur10/-2）、`floating_shadow`（key y=2/blur4 + ambient y=8/blur20/-3）。
    颜色一律解析原生 `overlay` 角色（亮色 5% 黑），无本地 palette。
  - 新增 `surface_canvas()`（映射原生 `muted` 角色：亮 `#f5f5f5`、暗 `#262626`）
    与 `REGION_GAP = 6`。
- `fileatlas-ui-bridge/src/components.rs`：`workbench_card` 白底
  （`surface_panel`）+ hairline + `card_shadow`；`workbench_section` 同步；
  `action`/`icon_button` 加 `control_shadow`。
- `fileatlas-ui-bridge/src/panels.rs`：通用浮窗外壳加 `floating_shadow`。
- `fileatlas-shell/src/shell.rs`：窗口根改画布底色（`surface_canvas`）+
  `REGION_GAP` 内边距/行间距；顶带、侧栏、内容列、检查器、状态栏各自成为
  圆角 8 白卡 + `region_shadow`；tools 弹层 top 偏移补 REGION_GAP。
- `fileatlas-shell/src/shell/panel_resize.rs`：splitter 定位补 REGION_GAP，
  对准卡片交界。
- `fileatlas-plugin-navigation/src/view/render/pages.rs`：rail 右缘加圆角。
- `fileatlas-plugin-settings`：删 `region_separation` 字段/默认值/外观设置行/
  启动应用调用；旧 JSON 含该键仍可加载（serde 忽略）。
- `fileatlas-plugin-mcp/src/tools_ui.rs`：删 `get_ui_state.region_separation`
  诊断与 `control_ui region_separation` 动作及 schema 描述。

## 验证

静态：`cargo +1.97.1 fmt --all -- --check`、locked check/build（ui-bridge、
settings、mcp、shell 及依赖）、四 crate 测试通过；`check-ui-theme.cjs
--self-test`/`check-ui-theme.cjs`、`check-ui-refresh.cjs`、
`check-architecture.cjs`、`check-feature-ledger.cjs`、`check-layering.cjs`
全部通过。

运行：`scripts/fileatlas-mcp-debug.cjs`，隔离 profile
`.local/validation/profiles/light-mode-review`（dev-test、显式隔离 DB、
`--access-level standard`）。场景脚本
`.local/validation/profiles/light-mode-review/fluent-scenario.cjs`：

1. 导航到 11 项隔离 fixture（图片/音频/文本/脚本/压缩包/损坏图）。
2. 亮色主界面截图（fluent-light-main）：画布浅灰、五张白卡（顶带/侧栏/
   内容/检查器/状态栏）圆角 8、间隙 6、双层阴影可见且柔和；无错位/裁切/
   双边框/阴影断裂。
3. 列表视图、扩展名弹出菜单截图：浮层层次正常。
4. 设置外观页截图：region_separation 行已消失（`get_ui_state` 断言该字段
   为 undefined）。
5. 真实点击左栏主题按钮切暗色截图（fluent-dark-main）：画布/卡片色差 +
   hairline 分隔，布局无回归；再切回亮色。

首版单层阴影经截图对照偏淡（"靠间隙不靠光影"），改为 Fluent 式
key+ambient 双层阴影后复测，视觉分析确认五张卡阴影均达可见级别且不发灰。

## 未覆盖 / 已知边界

- 文件卡片网格未加常驻阴影（数量大，hover 反馈保留原 overlay 方案）；
  截图分析指出内容区内子卡与白卡的内外层级对比可再强化，属后续可选优化。
- 格式徽标（TOML/PY 纯黑底）配色属内容语义，不在本轮框架样式范围。
- 小于最小窗口、quad 多栏布局下的浮起卡观感未逐一截图复核。
- 旧验证脚本 `validate-region-*.cjs` 引用已移除的 MCP 动作，作为历史证据
  保留，不再可运行。
