# 区域区分、分组间隙与收藏区域验证（2026-10-16）

设计对照：用户在 2026-10-16 提交的标注截图（浏览模式，深色石墨主题），四条标注即本次范围：

1. 各个不同的区域都做出明显的区分；主题配置中增加可切换“显示边框或阴影”的选项。
2. 分组标题和内容卡片之间增加间隙（圈出“当前文件夹 16 项”标题行）。
3. 收藏夹相关功能整合成一个由多个自定义分组组成的收藏区域，分组之间增加明显隔断并用颜色区分。
4. 最左侧图标栏与可折叠侧栏做明显区分并使用不同颜色。

整体布局沿用已接受的 `dev-baseline-2026-10-14`，只改区域边界、分组间隙与侧栏收藏区的分组容器。

## 改了什么

- `photag-ui-bridge/src/theme.rs`：新增表面阶梯 `SURFACE_RAIL` 0x171a1d → `SURFACE_SIDEBAR`
  0x1f2328 → `SURFACE_REGION` 0x1a1d21 → 内容列 → `SURFACE_CARD` 检视器；新增
  `REGION_EDGE`（0xffffff2e 亮色细线，`SEPARATOR` 比所有表面都暗，在深色主题上看不见）、
  几何令牌 `GROUP_HEADER_HEIGHT`/`GROUP_HEADER_GAP`(10)/`GROUP_TOP_GAP`(18)/`REGION_HEADER_HEIGHT`、
  `RegionDepth{Border,Shadow}` + `region_depth()/region_depth_name()/set_region_depth_name()`、
  唯一的区域边界原语 `separate_region(div, RegionEdge)`（边框模式画 1px 亮细线，阴影模式改用
  `region_shadows`：偏移 ±10/6、模糊 18、alpha 0.72）与分组配色 `GROUP_TINTS`/`group_tint(index)`。
- `photag-shell/src/shell.rs`：标题带（下边）、logo 区（右边）、侧栏列（右边）、检视器（左边）、
  状态栏（上边）、左下角设置入口（上边）全部改走 `separate_region`，侧栏/导轨改用新表面令牌。
- `photag-plugin-navigation`：导轨 `SURFACE_RAIL` + 右边界；侧栏的驱动器、快速访问、
  “文件夹收藏 + 收藏分组”各自包进 `sidebar_region()`（`SURFACE_REGION` 内凹 + 四边边界 +
  `region_caption` 小标题），`render_favorite_groups()` 把自定义分组追加进同一个收藏区域，
  分组之间 `mt/pt 8px + border_t_1`，分组标题行按 `group_tint(index)` 上色（星形图标 + 15%/24% 透明底纹）。
- `photag-plugin-gallery`：分组标题条改用 `SURFACE_GROUP_HEADER`/`_HOVER` + `separate_region(All)`；
  标题与卡片之间 `mt(GROUP_HEADER_GAP)`，非首组再加 `GROUP_TOP_GAP`；`gallery_row_height` 与
  橡皮筋命中测试（`view/behavior.rs`）同步这两个偏移，避免选中框与卡片错位。
- `photag-plugin-settings`：JSON 模型新增 `region_separation`（默认 `border`，兼容旧文件），
  Appearance 页新增“区域区分方式”小节（边框/阴影下拉）；`SettingsService::persist` 成为唯一写入口，
  成功后重发 `set_accent_name` + `set_region_depth_name`，视图侧只保留需要 `&mut App` 的
  `apply_accent_components(cx)` 并新增 `cx.refresh_windows()`（与语言切换同一全窗口重绘路径）。
- `photag-plugin-i18n`：新增 6 个键（区域区分方式 / 边框 / 阴影 / 说明句 / 库文件夹 / 收藏分组），
  `keys.json` 与 `en.ftl`、`zh-CN.ftl` 同步，界面全中文。
- `photag-plugin-mcp`：只读 `get_ui_state.region_separation`（读实际生效的绘制令牌）与
  受 mutation 开关保护的 `control_ui {action:"region_separation", value:"border"|"shadow"}`，
  后者通过 Cordis 取 `SettingsService` 并走与视图完全相同的 `update` 路径。
- 顺带修复：`photag-ui-bridge/Cargo.toml` 缺 `tokio` 的 `time` dev-dependency，导致
  `refresh.rs` 测试自 e0932a3 起无法编译（本次改动前既存）。

## 命令与环境

```
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --workspace --locked --offline -j 2
cargo +1.97.1 test --locked -p photag-ui-bridge -p photag-plugin-settings \
  -p photag-plugin-navigation -p photag-plugin-gallery -p photag-plugin-i18n -p photag-plugin-mcp -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
node scripts/check-ui-refresh.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe \
  --profile target/mcp-region-sep --plugins dev-test --allow-mutations \
  --scenario scripts/validate-region-separation.cjs
PHOTAG_POINTER_TAB=... PHOTAG_POINTER_CLICK=... PHOTAG_POINTER_ITEM=... \
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe \
  --profile target/mcp-region-sep --plugins dev-test --allow-mutations \
  --scenario scripts/validate-region-pointer.cjs
```

Windows 11 / 150% DPI（`scale_factor` 1.5），Rust 1.97.1 MSVC，locked `-j 2`。同进程 PhoTag MCP
（`photag-shell --mcp` 子进程），隔离 dev-test profile `target/mcp-region-sep`（独立 DB、settings、
session、缩略图缓存，未触碰 `%LOCALAPPDATA%\PhoTag`）。未使用 OS 鼠标键盘注入，未附加到另行启动的进程。
坐标换算按 `window_status.geometry.screenshot_to_pointer`：`pointer = (screenshot_px - (11,45)) / 1.5`，
先用“工具栏”页签点击校准（`settings_tab` 语义值确实变为 `toolbar`）后再点目标控件。

静态与自动回归结果：fmt 通过、workspace check 通过、`UI timer ownership: passed`、
49 项测试通过（navigation 20、ui-bridge 12、gallery 10、i18n 4、settings 3）。

## 结果

`region-report.json` 的 6 条用例全部通过：

1. 启动时 `settings.json` 的 `region_separation="shadow"` 被应用到绘制令牌
   （`get_ui_state.region_separation == "shadow"`）。
2. 按修改时间分组得到 3 个月份带（`2026-03 / 2025-01 / 2024-06`，各 3 项），标题条与卡片之间有间隙。
3. 侧栏只有一个收藏区域，内含 3 个自定义分组（`Work shots`、`Archive set`、`Phone imports`），
   各自带 tint 与隔断线；分组的折叠/展开、菜单与“新建分组”仍走原 handler。
4. 真实指针点击 Settings → 外观 → “区域区分方式”下拉 → “阴影”，`region_separation`
   由 `border` 变为 `shadow` 并写入磁盘（`region-pointer-report.json`）。
5. 非法值被拒绝：`{"error":"region_separation must be border or shadow"}`，不写盘。
6. 静止态：切换生效后再等 3 秒，调试日志 `latest_id` 282 → 282（无重复 IO/重绘记录）。

截图（1942×1256 逻辑像素窗口）：

| 文件 | 内容 |
| --- | --- |
| `01-regions-border.png` | 边框模式：导轨/侧栏/三个侧栏区域/分组标题条/检视器均有亮细线边界 |
| `02-regions-shadow.png` | 阴影模式：同一画面改为柔和投影，无细线，两种模式肉眼可分 |
| `03-settings-appearance-border.png` | 外观页“区域区分方式”小节（下拉显示“边框”，说明句不重复标题） |
| `04-settings-dropdown-open.png` | 真实点击展开的下拉：`✓ 边框` / `阴影` |
| `05-pointer-switch-to-shadow.png` | 点击“阴影”后主窗口即时渲染为阴影模式 |
| `06-favorites-group-collapsed.png` | 收藏区域内单个分组折叠后的样子 |

## 仍未验证 / 边界

- 区域边界与阴影是绘制期令牌，切档后靠 `cx.refresh_windows()` 全窗口重绘；本次只实测了
  Settings 视图写盘路径。MCP 的 `region_separation` 动作直接写服务，不经过 GPUI 视图，
  因此场景里靠开合 Settings 面板触发重绘后再截图，未单独证明“非视图写入也能即时重绘”。
- 阴影模式在纯黑背景上的可见度依赖对比度，已把 alpha 提到 0.72、模糊 18；浅色主题（若有）未验证。
- 未验证：检视器/预览浮动面板在两种模式下的全部边界、列表布局与胶片条分组标题、
  非 150% DPI、窗口最小化恢复后的重绘、拖拽与原生选择器交互。
- 分组 tint 只按索引取色，6 色循环；用户自定义分组颜色尚未提供（本次只满足“颜色区分”）。
- 本条不是完整功能验收：只覆盖上述四条标注的渲染与切换路径。
