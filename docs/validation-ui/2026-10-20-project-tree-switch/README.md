# 项目树层级与项目切换

## 验证结果

使用 FileAtlas MCP 插件启动的同进程 GPUI 窗口，在隔离 profile 中建立两个独立项目，各含两层目录。添加 `project-alpha`、`project-beta` 后分别检查侧栏截图：新加项目根行处于展开状态，根下面显示其子目录；每个根行保留折叠箭头。切换点击 `project-alpha` 后仍留在“项目”页签，浏览器路径切到 alpha，展开树也随之切换。点击箭头可单独收起子树。截图可见竖向点线和分支横线表达父子关系。

同时覆盖 canonical Windows 根路径带 `\\?\` 前缀时仍能匹配 Navigation 路径的情况；新增后能够立即选中并展开，修复了此前路径字符串不一致造成的折叠状态。

## 可复现命令

```powershell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/project-tree-switch-final2-20261020 --plugins core --access-level standard --scenario scripts/validate-project-tree-switch.cjs
```

结果：`passed`。复用 profile 已有运行结果；输出和隔离数据位于 `.local/validation/profiles/project-tree-switch-final2-20261020/`。

## 截图

- `screenshots/after-add-0.png`：添加 alpha 后自动展开。
- `screenshots/after-add-1.png`：添加 beta 后自动展开，alpha 留作折叠根行。
- `screenshots/project-beta-expanded.png`：beta 的两支两层子树及点状连接线。
- `screenshots/project-alpha-expanded.png`：切回 alpha 后展开 alpha 树。
- `screenshots/project-alpha-collapsed.png`：使用箭头收起 alpha 子树。

截图是本地验证产物，保存在 git 忽略的 `.local/validation/profiles/` 下；未加入仓库。

## 范围与未覆盖项

场景使用真实临时目录并通过 MCP 指针操作验证了新增、切换、展开/收起。窄窗、浅色主题、超大目录树性能及 Windows 原生文件拖放不属于本次项目树场景，仍未覆盖。静态构建不代表这些未覆盖路径已通过。
