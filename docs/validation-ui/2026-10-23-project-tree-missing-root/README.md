# 项目树失效根目录隔离

## 缺陷

用户在真实 profile 的项目树里看到：

```
\\?\D:\桌面文件夹\测试23: 系统找不到指定的路径。(os error 3)
```

该红字挂在另一个仍然存在的“测试”根目录下面，而“测试”自己的子树完全不显示。清单
`%LOCALAPPDATA%\FileAtlas\project-tree\project-tree.json` 里有两个根，其中
`\\?\D:\桌面文件夹\测试23` 已被删除。

根因：`scan_project_tree` 把所有根目录合并成一个 `Result`，任一目录 `read_dir` 失败就
让整批扫描失败；页面又把这一个全局错误画在每个展开的根下面。因此一个失效根既抹掉了
其它项目的目录树，又把自己的原始操作系统错误（含 `\\?\` 规范前缀和 `os error 3`）显示给
用户。子目录读取失败同样会中断整棵根树。

## 修改

- `crates/fileatlas-plugin-project-tree/src/view.rs`：扫描改为逐根独立，返回
  `ProjectScan { nodes, unavailable }`；失效根记录 `UnavailableRoot`，
  `ErrorKind::NotFound` 与其它 IO 错误区分文案；子目录失败只跳过该子树并
  `tracing::warn!`。页面只在失效根自己的行上显示“不可用”，展开时补一条
  “项目文件夹已不存在：<普通路径>”的弱化说明；根行名称可截断，移除按钮保持不收缩。
- `crates/fileatlas-ui-bridge/src/lib.rs`：新增公共 `display_path`，Navigation、
  FileManager 检视面板与 WorkCrate 面板的三份同名私有实现改为复用同一函数。清单里仍保留
  `\\?\` 规范路径用于长路径 IO，只有界面显示时去掉前缀。
- `crates/fileatlas-plugin-mcp/src/tools_ui.rs`：`navigation_tab` 增加当前页签标识
  `project-tree`，项目树页面可按语义打开，不再依赖 rail 坐标点击。
- i18n：`keys.json` 与 `en.ftl`/`zh-CN.ftl` 新增
  `msg-project-tree-folder-gone`、`msg-project-tree-folder-unreadable`；“不可用”复用既有
  `msg-ca1844969742`。

## 静态检查

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 clippy -j 2 --locked --offline --all-targets -p fileatlas-ui-bridge -p fileatlas-plugin-project-tree -p fileatlas-plugin-navigation -p fileatlas-plugin-file-manager -p fileatlas-plugin-workcrate-ui
cargo +1.97.1 test -j 2 --locked --offline -p fileatlas-plugin-project-tree -p fileatlas-ui-bridge -p fileatlas-plugin-i18n -p fileatlas-plugin-navigation -p fileatlas-plugin-file-manager -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-mcp
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
node scripts/check-ui-theme.cjs; node scripts/check-ui-refresh.cjs; node scripts/check-architecture.cjs; node scripts/check-feature-ledger.cjs
```

结果：全部通过。新增单元测试
`a_missing_root_only_marks_itself_and_keeps_the_other_tree`（同批一个存活根与一个不存在根，
断言存活子树仍为 `docs/notes/src`，失效根单独记录 `missing: true`）与
`display_path_strips_only_the_windows_namespace_prefix`。这些只证明逻辑，不证明界面。

## 可复现命令

同进程 FileAtlas MCP，隔离 profile，Standard 访问级。种子必须先单独跑一轮：调试客户端会
在场景执行前启动应用，项目树服务只在启动时读取清单。

```powershell
$profile = ".local/validation/profiles/project-tree-missing-root-20261023"
foreach ($order in "live-first","gone-first") {
  $env:FILEATLAS_SEED_PROJECT_TREE = "1"; $env:FILEATLAS_PROJECT_TREE_ORDER = $order
  node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile $profile --plugins dev-test --access-level standard --scenario scripts/validate-project-tree-missing-root.cjs
  $env:FILEATLAS_SEED_PROJECT_TREE = $null
  node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile $profile --plugins dev-test --access-level standard --scenario scripts/validate-project-tree-missing-root.cjs
}
```

场景脚本：`scripts/validate-project-tree-missing-root.cjs`。两轮均以 `passed` 结束，并断言
扫描结束后 `project-tree.json` 与种子内容逐字节相同（失效根不会被自动删除）。

## 截图（本地，未入库）

- `.local/validation/profiles/project-tree-missing-root-20261023/screenshots/project-tree-live-first.png`：
  `tree-live` 展开并显示 `docs → notes`、`src`；`tree-gone` 行只显示“不可用”，没有错误文字。
- 同目录 `project-tree-gone-first.png`：失效根展开后只有一条
  “项目文件夹已不存在：<普通路径>”说明，画面内不再出现 `\\?\` 前缀或 `os error 3`；
  根名过窄时以省略号截断，移除按钮保持完整可点。

## 范围与未覆盖项

- 覆盖：一个存活根 + 一个已删除根，两种展开顺序；页面通过 `navigation_tab=project-tree`
  打开；清单不被扫描改写。
- 未覆盖：权限受限（非 `NotFound`）目录的真实 Windows ACL 场景，只由单元测试外的日志分支
  处理；目录被重建后当前缓存的扫描不会自动刷新，需要增删项目或重开页面才会重扫；窄窗口、
  浅色主题、超大树性能与原生拖放仍未验收。静态构建不代表这些路径已通过。
