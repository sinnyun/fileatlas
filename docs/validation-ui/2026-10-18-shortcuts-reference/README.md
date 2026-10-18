# 快捷键面板：按参考图修正与复核

用户指出上一轮实现没有按图修改，本轮以
[2026-10-18 选用图](../../reference-ui/concepts/shortcuts-settings-2026-10-18.png) 和
[逐项布局清单](../../reference-ui/concepts/shortcuts-settings-2026-10-18.md) 为目标。
新实现位于 `photag-plugin-settings/src/view/shortcuts.rs`，与旧页的区别有真实截图佐证。
本记录是实现者对照结果，不代表用户已进行最终视觉签收。

## 前后核对

| 检查项 | 参考要求 | 本轮实际结果与证据 |
|---|---|---|
| 插件行结构 | 命令、范围、按键、更改/恢复/禁用在同一横行，列对齐 | [之前](before.png) 把按钮固定放到第二行；[修正后](final.png) 横向对齐，插件 ID 移入信息提示 |
| 正文层级 | 大标题、范围说明、插件组、其他快捷键组，细线分区 | [final.png](final.png) 使用 22/16/14/12 字号层级和 20 逻辑像素正文留白，移除重复的小节标题 |
| 默认窗口内容 | 两个插件绑定及 F1–F9 可同时看见 | 面板 1120×840 逻辑像素，正常与冲突图均可看见 F9；正文可滚动，页脚固定 |
| 录入状态 | 同行蓝框、组合键提示、取消入口 | 实际点击更改，[recording.png](recording.png)；可点击取消，Esc 也取消且不关设置 |
| 冲突状态 | 录入框下红字说明占用命令，原值保留 | 向剪贴板录入 Ctrl+F，显示启动台占用，[conflict.png](conflict.png)；断言磁盘不变，仍可继续录入 |
| 禁用/恢复/成功录入 | 禁用可读，恢复插件默认值，合法值即时保存 | [disabled.png](disabled.png)；真实点击保存 Ctrl+Alt+Q 后检查 JSON，再点击恢复删除覆盖项 |
| 标签选择器 | F1–F9 单列九行，选择器填满剩余宽度 | [final.png](final.png)；修正 GPUI DropdownMenu 外层未继承 flex 的问题，宽度按当前内容区计算 |
| 小窗口 | 列布局和页脚完整，纵向滚动可到 F9 | [compact.png](compact.png) → MCP 实际滚轮 → [compact-scrolled.png](compact-scrolled.png)；[F9 菜单](compact-f9-menu.png) 在右侧箭头处实际点击打开 |
| 临时状态清理 | 取消、切页、关闭释放键盘捕获 | 场景断言 `settings_shortcuts.capturing_key` 返回 false；防止系统键继续暂停 |
| 静置 | 不重复请求标签模型/改变图库 revision | 等待 700ms 后标签 ReadModel 快照及图库 revision 不变；不把它描述为 OS 系统级键验证 |

## 环境与复现

Windows，150% DPI，zh-CN，Rust 1.97.1 Windows MSVC。大窗口物理截图 3286×1831，
小窗口通过 MCP `resize_window` 设置为 1510×1000；该命令使用像素，面板尺寸使用逻辑像素。
验证专用 `core,dev-test` profile 在 `target/mcp-shortcut-layout`。测试数据、设置和 DB
由隔离 profile 承载，未用用户 PhoTag JSON/数据库作为 fixture。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
Copy-Item -LiteralPath target/debug/deps/photag_shell.exe -Destination target/debug/photag-shell-shortcut-layout.exe -Force
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-shortcut-layout.exe --profile target/mcp-shortcut-layout --plugins core,dev-test --scenario scripts/validate-shortcuts-layout.cjs --allow-mutations
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
git diff --check
```

fmt、locked shell check 和四项治理脚本通过。shell **编译与链接成功，但 Cargo 最终复制
`target/debug/photag-shell.exe` 失败（os error 5，旧用户进程占用）**，因此不记为完整的
`cargo build` 通过。保留原窗口，使用本次链接的 `target/debug/deps/photag_shell.exe`
另存为验证专用 EXE；两个新文件哈希一致，见 [summary.json](summary.json)。现有主 EXE
仍是旧版本，旧窗口不会热更新；运行新界面需关闭旧窗口后启动验证专用 EXE。
链接保留项目既有 LNK4098 警告。治理守卫仅报告既存 preview/media-playback 空目录警告。

MCP 场景全部通过。截图与状态来自同一新 EXE 的同一 MCP 子进程，pid、时间、隔离参数
记录在 summary。完整步骤保留在 `target/mcp-shortcut-layout/report.json`，
可复现场景为 [validate-shortcuts-layout.cjs](../../../scripts/validate-shortcuts-layout.cjs)。
旧场景 `scripts/validate-shortcuts.cjs` 同步修正了横向行的点击坐标；使用同一新 EXE、
profile `target/mcp-shortcut-layout-compat` 连跑两次，首轮真实录入 Ctrl+Alt+Q 和次轮
重启读取均通过（完整记录保留在该隔离 profile）。
MCP 新增 `pointer_scroll` 同进程滚轮事件，测试了格式错误、负坐标、超范围及 NaN 的拒绝，
随后用合法事件滚动到底部；没有用 OS 输入注入。未添加或重跑无关单元测试。

## 适配差异及未覆盖项

共享浮窗标题栏、GPUI Kit 图标、当前主题强调色沿用项目基础设施，未复制参考图里的装饰
渐变、徽标和光晕；录入用静态键盘图标提示，不伪造忙碌进度。插件顺序按生命周期命令目录，
图中的 Nature/Landscape 是样例，这个空标签 profile 显示真实“无”。用户已删除的工具栏
分类不恢复。这些是有记录的适配，不能用于推导其他布局可任意更改。

当前截图覆盖两个真实系统级插件贡献。大量第三方贡献、超长名称、程序内绑定实际触发、
标签有色图标状态未在本轮 UI 场景实测；名称截断与完整 tooltip 仅作代码检查。
OS 前台切换、真实系统全局热键派发和与其他程序抢占仍未覆盖。
