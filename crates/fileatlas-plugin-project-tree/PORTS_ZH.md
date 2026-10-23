# 项目树（Project tree）插件

状态：初始功能实现。该功能在界面中命名为 **Project tree**。它提供一个生命周期归属清晰的 Navigation 侧边栏页面，用于展示项目根及其递归文件夹树。它是目录导航辅助工具，而不是那个独立的项目管理面板功能。

## Cordis 与 UI

- 插件身份：`fileatlas.project-tree`；服务：`fileatlas/project-tree`（`ProjectTreeService`）。
- 依赖 `fileatlas/navigation` 与 `fileatlas/ui-bridge`。
- 在 `navigation.sidebar-project-tree` UI 槽位注册它的可执行视图。Navigation 只在这个生命周期归属清晰的贡献项存在时才显示 Project tree 的导轨标签页；插件停止时 Cordis 会移除该视图。
- Project tree 页面的加号按钮会添加 Navigation 地址栏当前显示的目录；它不会要求用户手动输入路径。点击某个项目目录会派发 `NavigationUiRequest::SelectPathPreservingPage`，因此活动浏览器列会随之导航，而 Project tree 页面保持打开。

## 数据与行为

- 在 `%LOCALAPPDATA%\FileAtlas\project-tree\project-tree.json` 存储一个版本 1 的 JSON 文档；隔离的 dev-test 配置档使用 `<profile>\project-tree\project-tree.json`。
- 该文档的 `roots` 字段只包含项目根路径与显示名称，因此对这个小巧、整篇文档式的数据形态来说 JSON 是合适的。它会读取旧的 `%LOCALAPPDATA%\FileAtlas\projects\projects.json` 路径与遗留的 `projects` 字段，然后把转换后的文档写入新路径。旧文件会被保留。写入使用同目录临时文件与 `.bak` 恢复副本。未知版本会以显式错误终止启动。
- 添加路径时会校验并规范化一个已存在的目录。重复路径会依据 Windows 路径的不区分大小写标识而被忽略。移除项目只改变这份列表；不会删除或移动任何文件。
- 打开该侧边栏页面时，会在共享的 `ReadModel` 工作线程上递归枚举目录子项。包含当前浏览器目录的根（若无则第一个根）以展开状态开始；一次只显示一个项目的子树。选择另一个根会导航过去并切换展开的树；它自己的箭头（chevron）可以独立折叠或展开它。添加项目会立即选中并展开该根。子级行使用虚线祖先导轨与分支标记来表达父子关系。目录符号链接不会被遍历，以避免形成环路。
- 每个根目录独立扫描（`scan_project_tree` 返回 `ProjectScan`，不再有整批错误）。无法读取或已被删除的根目录只记录属于自己的 `UnavailableRoot`：行内显示简短的“不可用”标记，展开时再补一条带普通显示路径的本地化说明；其余根目录仍然完整渲染自己的子树。`ErrorKind::NotFound` 使用“已不存在”文案，其他操作系统错误使用“无法读取”，原始错误信息只写入 `tracing::warn!`。某个子目录读取失败时跳过该子树并记录日志，不再让整棵根目录树失败。扫描不会改写 JSON 清单。
- Windows 路径在统一斜杠并移除扩展前缀 `\\?\` 之后按不区分大小写比较，因此规范化的项目根能与普通的 Navigation 路径匹配。清单里仍保存带 `\\?\` 的规范路径以支持长路径 IO；界面统一通过 `fileatlas_ui_bridge::display_path` 显示。

## 限制与诊断

- Shell 以文件系统浏览模式打开共享数据库服务。Navigation 在该模式下接受任何已存在的目录，而 `DirectoryPathSelected` 会把它路由到当前活动的工作区列。项目根无需先注册。
- 非常大的树会被完整枚举并全部保留在内存中；没有分页，且一次只渲染一棵展开的树。
- 2026-10-19 的同进程 MCP 验证覆盖了页面激活、导航到一个配置档本地的 fixture 并用页面加号按钮添加它、通过 `docs/notes` 与 `src/deep` 展开，以及导航到 `src/deep` 同时保持 Project tree 处于活动状态。确切命令、截图路径与剩余的 UI 覆盖范围见 `docs/validation-ui/2026-10-19-project-folders/README.md`。
- 2026-10-20 的同进程 MCP 验证覆盖了添加两个根并自动展开、在根之间切换、折叠，以及虚线层级导轨。见 `docs/validation-ui/2026-10-20-project-tree-switch/README.md`。
- 2026-10-23 的同进程 MCP 验证使用一个存活根加一个已删除根的清单：存活子树仍然渲染，失效根只显示“不可用”标记与普通路径说明，JSON 清单未被改写。见 `docs/validation-ui/2026-10-23-project-tree-missing-root/README.md`。
- 当文件夹选择失败时，请检查 `fileatlas/project-tree` 的启动错误以及 Navigation 的请求拒绝日志。每个根目录的扫描失败由视图工作线程记录为 `project root scan failed` / `project subfolder skipped`。
