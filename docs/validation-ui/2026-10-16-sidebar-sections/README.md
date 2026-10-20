# 侧栏 Explorer 风格分区验证（2026-10-16）

设计对照：`1792074142274.png`（Windows 资源管理器风格：驱动器 + 快速访问 + 自定义分组收藏夹）。
沿用已接受的 `dev-baseline-2026-10-14` 整体布局：左侧图标栏、Folders 页、中部“文件夹收藏”库树保持不变，
新增的“驱动器/快速访问”在上、“分组收藏夹”在下。

## 改了什么

- `fileatlas-plugin-settings`：JSON 模型新增 `sidebar_folders` 区块（`quick_access`、`favorite_groups`、
  `show_network_drives`）。SQLite 与 `%LOCALAPPDATA%\FileAtlas` 既有字段未改。
- `fileatlas-plugin-navigation`：Windows 驱动器枚举与用量（`GetLogicalDrives`/`GetDriveType`/
  `GetDiskFreeSpaceExW`）通过 `read_model::ReadModel` 被动读取；侧栏渲染驱动器区（盘符+卷标+已用/总量 GB+
  进度条，网络盘只留盘符+用量+地球图标）、快速访问区（系统 6 项 + 用户项）、分组收藏夹区（分组/收藏行 +
  “新建分组”）。
- 语义通道：`NavigationUiRequest::Sidebar(SidebarUiRequest)` 共 14 种请求，在
  `NavigationService::request_ui` 边界校验目标后，由 `apply_sidebar_request` 调用与渲染控件完全相同的
  handler；只读快照 `NavigationService::sidebar_state` 由渲染期签名门控发布，未新增计时器。
- MCP：`control_ui {action:"sidebar_request", request:{...}}`、`get_ui_state.sidebar_folders`、
  `window_status.geometry`（`scale_factor`、`client_size`、`screenshot_to_pointer`）。

## 命令与环境

```
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 test -p fileatlas-plugin-navigation -j 2
cargo +1.97.1 build -p fileatlas-shell -j 2
node scripts/check-ui-refresh.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe \
  --profile target/mcp-sidebar-sections --plugins dev-test --allow-mutations \
  --scenario target/mcp-sidebar-sections/<scenario>.cjs
```

同进程 FileAtlas MCP（`fileatlas-shell --mcp` 子进程），隔离 dev-test profile
`target/mcp-sidebar-sections`（独立 DB/settings/session/缩略图缓存）。未使用 OS 鼠标键盘注入，
未附加到另行启动的进程。主机：2560×1440，`GetDpiForWindow` = 144 → 缩放 150%。

## 结果

### 语义动作（`scenario-semantic-requests.cjs`，32 步全通过）

网络盘开关往返；清空后重新添加快速访问并经内联编辑器改显示名；分组创建 → 加收藏文件夹 → 改文件夹名 →
改分组名 → 移除文件夹 → 删除分组；编辑器取消不落盘；区块折叠/展开；`toggle_drive` 展开盘内子目录；
`toggle_expanded`；`activate_path` 选中；`add_to_library` 入库。4 项非法目标被服务边界拒绝：
`unknown favorite group`、`unknown section key`、`path is not an existing directory`、
`drive root is not present`。

最终状态既出现在 `get_ui_state.sidebar_folders`，也写入
`profile-settings-after.json`（本次隔离 profile 的 `settings.json`）：
`quick_access` 为改名后的 crates 项、`favorite_groups` 为 `seedgroup/摄影分组` + 1 个收藏文件夹、
`show_network_drives: true`。正常 `%LOCALAPPDATA%\FileAtlas` 未被写入。

### 指针弹出菜单（`scenario-pointer-*.cjs`）

点击坐标由 `window_status.geometry` 换算：`pointer = (screenshot_px - (11,45)) / 1.5`。

| 证据 | 目标 | 结果 |
| --- | --- | --- |
| `01-drives-and-quick-access.png` | 9 个盘（C/D/E/F 本地 + V/W/X/Y/Z 网络） | 盘符+卷标+用量+进度条、网络盘地球图标、无 UNC |
| `02-drives-header-menu.png` | 驱动器标题“…” | 弹出“✓ 显示网络驱动器 / 隐藏网络驱动器” |
| `03-system-row-menu.png` | 系统快速访问行（桌面） | “添加到库 / 在文件管理器中打开” |
| `04-user-row-menu.png` | 用户快速访问行 | “添加到库 / 在文件管理器中打开 / 重命名 / 从快速访问移除” |
| `05-favorites-expanded.png` | 分组收藏夹展开 | 分组行、收藏文件夹行、“新建分组”按钮 |
| `06-group-row-menu.png` | 分组行“…” | “添加文件夹 / 重命名 / 删除分组” |
| `07-favorite-folder-menu.png` | 收藏文件夹行“…” | “添加到库 / 在文件管理器中打开 / 重命名 / 从收藏夹移除” |
| `08-drive-row-menu.png` | 驱动器行“…” | “添加到库 / 在文件管理器中打开” |

`scenario-pointer-drives-menu.cjs` 还做了一次端到端闭环：指针点击“隐藏网络驱动器”后
`sidebar_folders.show_network_drives` 变为 `false`，再点击“显示网络驱动器”恢复为 `true`，
即弹出菜单项确实驱动了生产设置写入路径。

### 静态门

`cargo +1.97.1 fmt --all -- --check` 无差异；`test -p fileatlas-plugin-navigation` 7 项通过（含新增的
`sidebar_requests_parse_from_json_and_validate_known_targets`）；`check -p fileatlas-shell` locked 构建通过；
`node scripts/check-ui-refresh.cjs` → “UI timer ownership: passed”。

## 仍未验证

- 快速访问/分组的“+”按钮走原生 Windows 文件夹选择器后的添加（本轮只用 `add_quick_access` /
  `add_favorite_folder` 语义路径覆盖同一 handler，未点原生对话框）。
- 拖拽放置到分组/驱动器行。
- 内联编辑器里的键盘输入（本轮用 `open_editor` 预填文本 + `confirm_editor` 覆盖提交/取消两条路径）。
- 文件监视器（watcher）对新增快速访问/收藏路径的联动，以及非 150% 缩放、多显示器下的几何。
- 重启后新区块行的选中恢复与驱动器用量刷新节奏。


## 后续审查修正（2026-10-16）

上面的初次验证不能覆盖重复箭头操作、保存乱序/失败、任意目录稳定重启等情况。后续发现并修复了实际问题，完整列表及已检入 scripts/ 的复现脚本见 [侧栏与浏览审查](../2026-10-16-sidebar-review/README.md)。当前点击不隐式展开；驱动器校验不再枚举容量；事件已改为带修订号；新增刷新/重试/分组请求，因此上文 14 请求与旧接口描述属于初次实现历史。
