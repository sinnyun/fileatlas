# 剪贴板工作台 v2：参考还原与分类修复

2026-10-18，Windows / 150% DPI / Rust 1.97.1 MSVC。
按用户反馈重新实现；覆盖此前两份剪贴板视觉验证记录中的当前状态。
设计来源、开源软件调研与 ImageGen 提示见[概念说明](../../reference-ui/concepts/clipboard-workbench-layout-2026-10-18-v2.md)。

## 修复内容与根因

- 视觉：深蓝区域/卡片/编辑区、明确区域分隔、14px正文、Lucide图标、日期与字数、实际分组徽标、蓝色选中态。
  默认面板1160×760，左右220/324，中间自适应。对照概念后调整了按钮字号、分组选择器宽度，并把分组编辑表单移出导航滚动区，修正保存按钮被裁切。
- 分类：文本/链接/代码自动类型与用户分组独立；新增未分类筛选，分组选择即时保存，正文编辑单独保存。
  补齐新建、重命名、删除分组；删除分组不删除历史记录，侧栏数量和卡片徽标同步。
- 持久化根因：默认数据库使用 `open_browser_at`，原 `browser_store::persist` 表清单没有剪贴板历史与分组。
  原先界面内操作能显示变化，但新进程无法恢复。已加入现有共享持久化清单及删除同步。
  新回归先复现“重开后分组数0”的失败，再验证修复后重开、重命名、删除分组/历史和重复捕获行为。
- MCP中文输入：以实际字符及对应按键转发，修正旧合成 `text` 键不能稳定送入聚焦输入框的问题。

## 构建与回归

以下通过：

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-clipboard --lib -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-database --lib -j 2
node scripts/check-ui-refresh.cjs
node scripts/check-layering.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
```

自动类型1项通过；数据库11项通过。Shell构建保留既有LNK4098提示。
架构脚本保留既有两个无Cargo.toml目录提示，不属于本次新增问题。

## 同进程 FileAtlas MCP

调试客户端现在强制把 `dev-test` 加入插件组合，并显式设置 profile 内数据库路径；
报告记录 `plugins`、`processId`、`isolated`。不再依赖Windows已知文件夹可能忽略的LOCALAPPDATA覆盖。

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-clipboard-v2-isolated-final --plugins core --scenario scripts/validate-clipboard-workbench.cjs --allow-mutations
```

实际启用 `core,dev-test`；91步通过：中文分组键入、创建、即时分配（未点击正文保存）、
分组筛选、链接/代码筛选、未分类排除已分组项、搜索/清空及检查器跟随结果、面板关闭重开、重命名、
删除分组保留4条历史、暂停/恢复捕获和静止时revision不变。截图只显示隔离样例：

- [工作台](../../../.local/validation/artifacts/2026-10-18-clipboard-workbench-v2/clipboard-workbench.png)：卡片“工作”徽标、侧栏数量1和右侧分组一致。
- [分组筛选](../../../.local/validation/artifacts/2026-10-18-clipboard-workbench-v2/group-filter.png)：可见ID只包含分配到该组的记录。

真实进程重启使用同一隔离profile，依次运行下列命令两次，再去掉 `--allow-mutations` 运行第三次：

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-clipboard-v2-isolated-restart --plugins core --scenario scripts/validate-clipboard-restart.cjs --allow-mutations
```

| 子进程 | 结果 |
|---|---|
| 34136 | UI创建1条历史、1个分组并即时分配 |
| 13584 | 新进程恢复相同历史/分组ID与归属；再通过UI删除分组和历史 |
| 38344 | 只读确认历史0、分组0，已删除数据未复活 |

[重启恢复截图](../../../.local/validation/artifacts/2026-10-18-clipboard-workbench-v2/restart-restored.png)。元数据证据在
`target/mcp-clipboard-v2-isolated-restart/restart-{created,restored,deleted,deleted-verified}.json`。

## 验证隔离更正

前期脚本使用 `core` 而没有启用 `dev-test`，环境变量未隔离Windows已知文件夹路径；
在补齐持久化之后，5条本轮fixture和“重启验证”分组进入了默认库。
已按本轮记录的ID、长度和创建时间逐条核对后，经同进程GPUI控件清理，并确认历史/分组均为0。
没有删除数据库文件，也未修改任何不匹配的条目。清理日志在
`target/mcp-clipboard-fixture-cleanup/report.json`。
上面的最终91步和三个进程结果全部来自修正隔离后的显式profile路径。

## 边界

当前捕获范围仍是文本，自动代码类型为启发式识别。图片/文件、来源程序、嵌套分组、
持久隐私规则未实现。实际OS剪贴板读写、真实系统快捷键和跨程序粘贴没有由这些fixture场景证明。
