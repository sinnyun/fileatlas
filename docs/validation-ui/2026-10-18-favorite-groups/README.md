# 2026-10-18 独立收藏分组验收

用户本轮第二张图决定分组结构，沿用当前深蓝主题和驱动器/快速访问的紧凑几何。
参考与编码前清单：[设计记录](../../reference-ui/concepts/sidebar-favorite-groups-2026-10-18.md)。
已检查用户两图、ImageGen 概念及本目录实际同进程截图。

## 实现与对照

- 驱动器、快速访问、常用、AI、工作均为平级独立边框，调用同一个
  UI Bridge `sidebar_section/sidebar_header`；移除旧复合收藏标题与搜索区。
- 每组右侧＋直接收藏已提交的主窗目录。当前为 Home/非目录时禁用；目录失效时
  显示错误。重复添加不覆盖别名、不增加配置 revision，不触发登记/扫描或路径编辑器。
- 默认完整路径截断，真正自定义别名保留；保留懒展开、选中、菜单、组折叠与行内编辑。
  旧登记文件夹一次性导入常用组，删除收藏不会移除数据库登记，也不会重启后复活。
- 小窗口整列滚动，分组不压缩，底部新建分组可达。首轮截图发现内容越过状态栏，
  修正滚动容器后重新构建并以 v2 profile 完整重跑；首轮结果不作为最终视觉证据。

## 环境与命令

Windows，150% DPI，Rust 1.97.1 MSVC；独立 profile `target/mcp-favorite-groups-v2`，
由调试客户端创建同进程 GPUI+MCP 子进程，默认启用 dev-test。测试目录仅在 profile 内。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-navigation -p fileatlas-plugin-settings -p fileatlas-ui-bridge -p fileatlas-plugin-i18n --lib -j 2
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
git diff --check
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-favorite-groups-v2 --allow-mutations --scenario scripts/validate-favorite-groups.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile target/mcp-favorite-groups-v2 --scenario scripts/validate-favorite-groups-restart.cjs
```

构建、fmt、治理检查通过；33项单元测试通过（Navigation13、Settings3、UI Bridge13、i18n4）。
保留已有 LNK4098 链接警告及历史 feature-ledger 孤立目录提示。迁移回归包含旧 JSON、
路径规范化去重、别名/已有组保留、删除后重建服务，以及 SQLite 重用 ID 的新路径导入。

首个运行场景通过测量的组标题 bounds 发送真实 GPUI 指针移动/按下/抬起，验证＋、菜单
和折叠；其余场景复用公开语义处理器。验证当前根目录/子目录添加到指定组、折叠组＋
展开、重复不变、长路径、失效目录、空组、Home 禁用、删除后库记录不变、小窗滚动。
静置900ms完整 sidebar 状态不变（revision=10）。[场景摘要](../../../.local/validation/artifacts/2026-10-18-favorite-groups/favorite-groups-summary.json)。

第二个新子进程禁用 mutations，确认组顺序、别名、收藏及折叠状态恢复，旧收藏不复活，
写入工具仍受禁用保护。[重启摘要](../../../.local/validation/artifacts/2026-10-18-favorite-groups/favorite-groups-restart.json)。客户端 profile/report.json
会被第二次运行覆盖，因此不把该文件描述为两次完整运行日志。

## 已查看截图与边界

- [全部独立分组](../../../.local/validation/artifacts/2026-10-18-favorite-groups/independent-groups.png)、[组菜单](../../../.local/validation/artifacts/2026-10-18-favorite-groups/group-menu.png)、[折叠](../../../.local/validation/artifacts/2026-10-18-favorite-groups/collapsed-group.png)。
- [长路径](../../../.local/validation/artifacts/2026-10-18-favorite-groups/long-path.png)、[Home 禁用＋](../../../.local/validation/artifacts/2026-10-18-favorite-groups/home-plus-disabled.png)、[失效目录反馈](../../../.local/validation/artifacts/2026-10-18-favorite-groups/missing-current-folder.png)。
- [空组](../../../.local/validation/artifacts/2026-10-18-favorite-groups/empty-group.png)、[小窗滚动到底](../../../.local/validation/artifacts/2026-10-18-favorite-groups/compact-scrolled.png)、[新进程恢复](../../../.local/validation/artifacts/2026-10-18-favorite-groups/restarted.png)。

采用用户图的结构和现有深蓝组件，未复制第二张的灰色底与黄色系统文件夹图标；这符合
同时要求与上方两区统一的方向。长路径在行内省略，保留地址栏/信息区的完整路径入口。
小窗截图中主窗横向工具栏仍有既有裁切，本轮只修复左侧收藏整列溢出；不宣称全窗口响应式
布局完成。未实测原生文件选择器、登记移除确认、网络/离线路径、跨卷权限或大规模分组。
旧库排序不再作为单独栏目展示，统一使用分组/别名组织；数据库登记和后端能力保持。
