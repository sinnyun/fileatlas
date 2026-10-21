# Navigation plugin ports / 导航插件数据对接

## MCP 贡献（2026-10-19）

Navigation 通过宿主发布的 `McpToolRegistry` 提供 `navigation.list_library`。
其处理函数读取插件自身的 `NavigationService` 快照，返回已注册文件夹的
标识/路径/扫描/计数数据以及层级化标签树。当 `fileatlas.navigation` 停止时，
由 Cordis 移除该工具。

Navigation 还提供 `navigation.add_folder`、`remove_folder`、
`rescan_folder`、`cancel_scan`、标签的创建/重命名/删除以及收藏
管理。add/rescan 保留并消费 Indexing 的进度接收器直至
完成；完成时发出 `MediaIndexChanged`，使 FileManager 在后台扫描结束后重新加载。
添加/移除文件夹和删除标签需要 `confirm=true`。
各 Schema 的参数为：add 使用 `path`；remove/rescan/cancel 使用 `folder_id`；
创建标签使用 `name` 和可选的 `parent_id`；重命名使用 `tag_id` 和 `name`；
删除使用 `tag_id`；收藏管理使用 `action`、`folder_id`，以及可选的
`name`/`delta`。`navigation.list_library` 不接受任何参数。旧的
无命名空间路由已弃用。

当前的浏览器运行时取代了下文关于索引库的较早描述；参见文末的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：Cordis 导航服务、文件夹树操作以及 GPUI 侧边栏均已存在。侧边栏使用 `rfd::AsyncFileDialog` 进行原生目录选择，并把选中的路径通过现有的文件夹注册、watcher 和索引流程发送出去。当该提供方可用时，`library_refresh()` 会转发 Watcher 的启动对账状态。文件夹与子目录菜单提供新建子文件夹和在文件管理器中打开等操作。标签界面提供创建、重命名、删除、颜色/图标、显示、单向关联标签分配、重新设定父级、合并，以及带可选祖先分配清理的确认删除。外观可以递归应用，同时跳过已有显式颜色或图标的后代节点。带类型的 GPUI 拖放会把被拖动的标签移动到显式行目标之前/之后/之内。FileAtlas 没有文件夹树拖放行为。

## 生命周期与依赖

- Cordis 插件名：`fileatlas.navigation`；发布的服务：`fileatlas/navigation`（`NavigationService`）。
- 依赖 `DatabaseService`、`IndexingService` 和 `SessionService`；在可用时挂载 `WatcherService`。
- GPUI 视图贡献：`workspace.sidebar` 中的 `navigation/sidebar-view`。
- 侧边栏挂载实时的 `navigation.extensions`、
  `navigation.launcher-shortcuts` 和 `navigation.sidebar-project-tree` UiBridge 槽位。它在渲染时依据
  catalog 的 `(id, generation)` 对每个已挂载视图进行核对：Cordis 注销
  会丢弃该视图，之后的重新注册会创建一个新视图。当插件拥有的视图注册发生变化时，
  Shell 会唤醒 navigation 轮询器。不要在超过其注册代次之后仍保留
  插件按钮实体。
- 只有在某个视图向 `navigation.sidebar-project-tree` 有贡献时才渲染 `Project tree`
  轨道标签页；如果该贡献者在 Project tree 处于活动状态时被移除，Navigation 会回退到
  Folders。project-tree 功能拥有其路径列表/树 UI，并通过 Cordis 注册此内容槽位。
- 侧边栏轨道扩展槽位：`navigation.launcher-shortcuts`；程序
  启动器在此提供 Apps 按钮，Navigation 把它挂载在
  Folders/Tags/Add-ons 旁边。
- UI 发出 `FolderSelectionChanged(Option<i64>)`、`TagFilterSelectionChanged(TagFilterSelection)` 和 `DirectoryPathSelected(Option<String>)`；`TagSelectionChanged(Option<i64>)` 作为遗留的单标签契约保留。在执行会改变索引成员归属的操作后，它发出 `MediaIndexChanged(())`。
- 侧边栏启动时，从 `SessionService` 恢复 `sidebar_tab`、`selected_folder_id`、`selected_path`、`expanded_subdirs` 和 `expanded_tag_ids`。文件夹 ID 必须仍然存在；选中的路径必须仍能解析到某个已注册文件夹之内；展开的路径会通过实时的子目录列表重建。当前标签树中缺失的标签展开 ID 会在启动时被清除。启动时按顺序发出文件夹和路径选择事件，使 FileManager 收到恢复后的作用域。选择某个文件夹或 All Media 会保存 `selected_folder_id` 并清空 `selected_path`；选择某个路径会清空 `selected_folder_id` 并保存 `selected_path`。标签页、目录展开和标签展开的更新会保留属于其他消费方的字段。
- 标签过滤会恢复并保存 `SessionState.tag_click_mode`（Select/Exclude）。模式按钮控制普通的标签点击，而源程序中配置的修饰键会反转该模式。All 操作把当前模式应用到完整标签树；发出的已包含/已排除 ID 通过 `TagFilterSelectionChanged` 流向 FileManager。

## 服务输入/输出

- `snapshot()` -> 已注册文件夹及其媒体计数，以及递归的 FileAtlas 标签树。
- `request_ui(NavigationUiRequest)` -> 为已挂载的侧边栏排队一个带类型的操作。标签模态请求（`OpenCreate`、`OpenEdit(tag_id)`、`OpenDelete(tag_id)`、`OpenMerge(tag_id)`、`OpenMove(tag_id)` 以及 `CloseModal`）保留其现有的校验与行为。`ExpandFolder(folder_id)` 以幂等方式加载该已注册根目录的直接子项；`SelectPath(path)` 先校验规范化的目录位于某个已注册根之内，然后通过现有的文件夹/路径事件选中它并激活 Folders；`SelectPathPreservingPage(path)` 使用相同的路径校验/导航方式，同时为插件树导航保留当前活动的侧边栏页面。`AllFiles` 清空作用域。请求由侧边栏、面包屑和标签模态共用的同一个已缓存 `SidebarView` 在 GPUI 线程上消费。无效 ID/路径、队列状态损坏（poisoned）以及第二个待处理请求都会返回 `NavigationUiRequestError`。侧边栏使用现有的原生文件夹选择器请求一起轮询此队列。
- `subdirectories(path)` -> 惰性加载的直接子节点：名称、路径、是否存在子项以及索引计数。
- `add_folder(path)` -> 校验一个已存在的目录；在注册该目录并调度索引后返回 `(Folder, Receiver<Option<ScanProgress>>)`。watcher 注册随后在可用时执行。
- `rescan(folder_id)` -> 取消先前的扫描，清除该文件夹的媒体行并开始扫描；返回一个进度接收器。
- `cancel_scan(folder_id)` -> 请求取消该文件夹当前活动的索引工作；在没有活动任务时也会成功。
- 实时侧边栏显示活动文件夹扫描的阶段/计数/进度以及 Cancel 控件；Cancel 使用已注册的活动文件夹标识调用 `cancel_scan`，并丢弃该视图的接收器。即时的 `add_folder`/`rescan` 错误与终止性的 `ScanProgress.error` 事件都会渲染一张重试卡片；文件夹失败通过 `rescan` 重试，路径注册失败通过 `add_folder` 重试。
- `remove_folder(folder_id)` -> 取消扫描、移除索引行并注销 watcher；返回被移除的文件夹。
- `create_tag(name, parent_id)` -> 新的 `Tag`；父级为可选。创建模态提供可搜索的层级父级选择器和一个根选项；标签 ID 仍为内部值。每个标签行的 **Use as parent** 会在选择器中选中该行。
- `rename_tag(id, name)` -> `()`；**Edit fields** 载入名称，**Rename** 将其持久化。
- `delete_tag(id, clean_parents)` -> 受影响的媒体 ID；删除未分配的标签或根标签会立即执行。删除已分配的子标签时会询问是否也应从这些受影响的媒体上移除严格的祖先标签分配。两种确认选择都会删除目标；清理选项会先移除祖先关联。返回的 ID 会发送给 `TaggingService` 以刷新元数据。
- `set_tag_appearance(id, color, icon)` -> `()`；color 接受 `#RRGGBB` 或空字符串表示清除；icon 接受文本/emoji 或空字符串表示清除。**Edit fields** 载入现有值；**Save style** 把它们应用到选中标签。**Apply to children** 以深度优先方式传播两个样式字段；**Skip edited children**（默认启用）保留已有显式颜色或图标的后代节点，符合源程序的覆盖规则。
- `TaggingService::associated_with(id)` / `set_associations(id, ids)` -> 与源兼容的单向关联列表。**Edit fields** 载入选中标签的关联 ID；其复选框列表会排除标签自身，而 **Save style** 会替换其出站关联。分配该标签会通过 Tagging 展开其关联标签及这些标签所需的祖先节点。
- `set_tag_display(id, display)` -> `()`；将 file_manager 的 chip 可见性应用到选中标签及其全部后代，与原始编辑行为一致。
- `move_tag(id, TagDropPos)` -> 隐式标签集合发生变化的媒体 ID 列表。标签名是拖拽源；高亮显示在某标签上方、之上、下方的目标分别调用 `Before`、`Into` 和 `After`。移动模态提供可搜索的层级选择器并排除源子树；目标 ID 仍为内部值。数据库环检测与子树路径重算是权威依据；自我拖放和构成环的移动返回空列表，作为安全的无操作。只有在返回的受影响媒体列表非空时才发出 `MediaIndexChanged`。

## GPUI 控件与调试

v5 侧边栏有一个 48 逻辑像素的页面轨道，包含 Folders、Tags 和 Add-ons。宿主 shell 单独持有 Settings。选中的页面通过 UiBridge surface 值 `navigation.tab`（`folders`、`tags` 或 `extensions`）共享，并持久化在 `SessionState.sidebar_tab` 中；MCP 可以设置同一个 surface 值。Folders 提供一个真实的 `navigation.folder-search` surface 值，与其 Input 双向绑定。搜索过滤已注册的根节点以及已经展开/惰性加载的后代，因此它不会在每次按键时扫描文件系统。根文件夹展开和目录选择继续通过 NavigationService 和现有选择事件完成。文件夹树使用 Lucide 文件夹字形、右对齐计数、强调色选中行以及连接线。Add-ons 挂载注册到专用 `navigation.extensions` UiBridge 视图槽位的可执行视图贡献。若没有此类贡献，它显示一个明确的空状态；核心命令与菜单元数据不会作为产品内容呈现。

标签页面保留现有的 GPUI Kit `Input`、`Button`、下拉操作以及带类型的拖拽负载。它支持已包含和已排除标签集合、Select/Exclude 点击模式、用配置的修饰键对单次点击应用相反模式、在当前模式下全选或清空标签、按模式的清空操作，以及与源兼容的 Contains/Starts 搜索匹配，后者会展开显示匹配的后代及其祖先节点。`TagInteractionPreferencesChanged` 提供当前设置，而不引入 Settings → Navigation 的 crate 依赖；默认值为 Ctrl、Contains、持久化搜索，以及合并时重新设定子级父节点。标签模态 surface 按 Escape 关闭。把标签名拖到另一行上方/之上/下方的高亮目标上；数据库错误显示在快照面板中。自我拖放和成环拖放会被与源兼容的数据库操作静默拒绝。如果继承的标签分配变化后 FileManager 结果陈旧，请检查 `MediaIndexChanged` 的投递以及 FileManager 的订阅方。

## Original source mapping / 源行为映射

参考：锁定的 `vendor/photag-source/src/ui/sidebar.rs`（`show_sidebar`、`show_tag_tree`、`show_tag_node`）以及 `vendor/photag-source/src/app.rs` 中的标签操作。

- 已迁移：文件夹注册/重新扫描/移除、惰性目录列表、递归标签树、单标签过滤、根/子标签创建、重命名、删除、颜色/图标编辑、显示传播、重新设定父级到选中标签、Folder/Tags 标签页、标签搜索清空偏好、物理子文件夹创建、在文件管理器中打开操作，以及带显式子级保留规则的递归标签外观传播。
- 已迁移：通过 FileAtlas 的 `PendingAction::AddFolder` 使用的同一个 `rfd` API 实现的原生目录选择器。取消或后端返回 `None` 时侧边栏保持不变；文件夹校验与注册错误像以往一样显示在导航面板中。标签拖放重新设定父级与同级排序使用源程序的 `TagDropPos` 数据库命令。FileAtlas 未实现文件夹拖放。
- 源程序历史中没有标签树编辑的撤销条目。标签分配的最近使用记录由 FileManager 和 Preview 共享；树编辑不会改变最近使用记录。展开的标签节点会被持久化；Windows 视觉表现仍未验证。
- 源行为审计（2026-10-13）：`vendor/photag-source/src/ui/sidebar.rs` 的文件夹/标签切换、惰性文件夹展开、带祖先可见性的标签搜索匹配模式、包含/排除选择、修饰键覆盖、批量模式选择/清空、展开状态持久化以及行拖拽区域，在此都有对应实现。源程序的上下文菜单操作可创建子标签、编辑外观/显示、选择父级、合并到过滤后的目标列表，并对已分配的子标签进行确认删除，可选择清理或保留父级关联。合并不包含源子树，并让用户选择其子标签是移动到目标之下还是被删除。Windows 渲染与交互仍未验证。
- 本地集成：针对有代表性的 FileAtlas 数据库验证事件刷新和破坏性删除行为；执行 Windows GUI/文件系统流程；然后完成视觉与交互一致性。插件测试推迟到本地集成阶段。

## 共享视觉 token

侧边栏使用 fileatlas-ui-bridge::theme 提供源程序那种感知背景的基面（base surface）以及 secondary、muted、danger 文本颜色。布局间距与排版仍使用 GPUI Kit 预设；Windows 视觉一致性未验证。

## 代码地图

- `src/view.rs` 持有侧边栏视图状态、文件夹扫描/选择、选中状态以及视图生命周期。
- `src/view/tag_actions.rs` 实现标签创建/编辑、外观/关联更新、删除/合并/移动父级操作，以及标签显示传播。
- `src/view/render.rs` 构建文件夹/标签侧边栏；`src/view/render/tag_render.rs` 渲染标签行与子树；`src/view/tag_modal.rs` 渲染创建/编辑/重命名/删除/合并/移动 surface；`src/view/support.rs` 存放树遍历与会话恢复辅助函数。


## 本地化集成（2026-10-13）

可见控件标签使用由 fileatlas.i18n 提供的公共 UI Bridge 翻译器。
稳定的命令、源设置/schema 值、媒体路径以及
用户创建的标签数据保持不变。缺失的翻译会回退到英文；
OS/后端错误以及部分生成的诊断信息未完全本地化。
真实仅 MCP 的验证覆盖范围以及剩余的源布局差异
记录在 docs/validation-report.md 中；截图不能证明指针
交互或动画一致性。

## 空库文件夹命令（2026-10-13）

Cordis 拥有的 `UiCommandInvoked` 监听器接受
`"navigation/add-folder"`。一个合并后的原子 pending 标志由
已挂载的 Sidebar 实体在 GPUI 线程上每 100 ms 消费一次，并调用其现有的
原生 `pick_folder` 流程。取消是无操作。有效目录使用
`NavigationService::add_folder`、生产 Indexing、watcher 注册
以及现有的进度/错误处理。监听器在插件停止时移除；
当其视图实体消失时轮询器结束。未引入
任何新的公共服务负载或数据库 schema。

## v5 导航投影更新（2026-10-13）

侧边栏现在同时渲染 All files、已注册文件夹树和标签树；
旧的 Folders/Tags 切换已移除。All files 是第一行，其后是 Folders 分组标题和树。文件夹/子目录操作
仍保留在悬停 `⋯` 菜单中；子目录名称用于选中其路径，而折叠箭头
用于展开后代。Tags 标题保留 Search 和 Create；其可搜索的
过滤/模式控件只在 Search 被激活时展开。标签名称
切换其选中状态，选中行使用强调色条和背景；
创建子标签、合并、编辑、重命名、显示、外观、父级、移动和删除
仍保留在每行的悬停 `⋯` 菜单中。All files 同时清空文件夹/路径选择
和已包含/已排除标签过滤。

`workspace.sidebar` 中的 `navigation/sidebar-view` 与
`command.breadcrumb` 中的 `navigation/breadcrumb-view` 是同一个
已缓存 `SidebarView` 实体的投影。面包屑观察该实体，并从其现有的会话与导航
快照派生出当前位置；其
All files 操作清空完整的导航作用域。这两个视图仍由
Cordis 持有，并在 navigation 插件停止时移除。最初的
FileAtlas MCP 截图已与 `00-main-library.png` 对照检查；观察到的
侧边栏密度与顺序不一致已在最新代码中修正。
修订后的状态仍需一次新的 MCP 截图与交互检查。

同一视图集合中的 `workspace.modal` 投影负责标签创建、
重命名/编辑、考虑分配情况的删除确认、合并目标选择
以及重新设定父级。`NavigationService::request_ui` 暴露带类型的 create/edit/delete/
merge/move/close-modal 请求，以及供已挂载视图使用的
`ExpandFolder(folder_id)` 和经过校验的 `SelectPath(path)`；请求调用现有操作，并拒绝无效 ID/路径或被占用的单请求槽位。父级和
移动模态使用可搜索的层级路径，Escape 可关闭当前活动的标签
模态。合并仍在选中目标时执行；与源兼容的
子级重设父级/删除选项予以保留。编辑模态包含名称、颜色
色板、图标预设、关联标签、子级外观传播以及
缩略图显示状态。在通过 FileAtlas MCP 检查之前，实时外观与交互
仍未验证。

## 2026-10-14 当前实现与 MCP 证据

Reference00/04 侧边栏和五个标签模态布局已通过 FileAtlas MCP 渲染。新的 NavigationUiRequest::AllFiles 派发现有的文件夹/标签取消选择。MediaIndexChanged 监听器/轮询器刷新侧边栏计数；模态渲染同样消费队列中的请求，因此在模态打开时关闭/切换仍可达。终止性扫描错误保留最后一次进度错误以便重试。真实标签树/生产数据语义保持不变。原生标签提交/拖拽/扫描失败的交互仍未验证。

Finalv2 修正：ExpandPath(String) 校验与 SelectPath 相同的已注册根目录边界。请求路径在选中前会被规范化，从而使 SQLite 的 path_prefix 匹配真实索引的 Windows 路径。树渲染直接接收 selected_directory；它无法读取自己当前正在更新的实体。标题/菜单现在采用紧凑覆盖层，rgba white0x38 的树连接线可见；蓝色选中填充是显式 RGBA。MCP 展开了 root 与03-nested，selected02-mixed-formats 匹配到3 个真实媒体，搜索了 mixed/noresults，并通过 GPUI 事件点击了 Folders/Tags。

## 原生文件夹选择器崩溃修复（2026-10-14）

Sidebar +、Browse 以及命令请求现在都在 GPUI 任务中启动 rfd::AsyncFileDialog::pick_folder。原生模态循环运行在 rfd 独立的 Windows STA 线程上，位于 Sidebar/App 借用之外。返回的 FileHandle 路径仍会调用 add_folder_path→NavigationService::add_folder→watcher/indexing；None 仍是无操作。FolderPickerState 与 PickerLease 强制只有一个进行中的对话框，在 pending 期间禁用选择器按钮，并在完成/drop 时重置 pending。Sidebar 持有该 Task，因此视图销毁会取消它；lease 释放时会请求关闭其同进程中已创建的原生窗口。

公共 NavigationService::folder_picker_pending()->bool、folder_picker_window()->Option<isize>、cancel_folder_picker()->Result<(),String> 支持直接的插件 MCP 验证。原生查找仅限于本进程、#32770 类以及当前选择器的确切标题。取消只向该已识别的对话框投递 WM_CLOSE，从不生成鼠标/键盘输入，也从不选择/注册目录。任务销毁期间的窗口延迟创建未被单独验证。真实的 Windows 按钮→原生对话框→取消→重新打开已验证两次；本次运行中原生文件夹选择的确认仍未验证。参见 docs/validation-ui/2026-10-14-folder-picker。

## 调试请求中索引路径的拼写形式（2026-10-14）

SelectPath/ExpandPath 仍由 NavigationService::request_ui 进行规范化，用于已注册根包含校验。它们的 GPUI 消费方在选中/展开时会保留所提供的索引路径拼写形式；不再第二次添加 Windows 扩展前缀。这与原生树点击行为一致，并避免对常规拼写的索引路径执行空的 FileManager 查询。真实的隔离 MCP root/nested 选择现在返回4/2 条匹配。文件夹/标签快照回归测试现在在既有全文件策略之下，同时对 .hidden 以及 nested 作出预期。

## FileManager 文件夹卡片导航（2026-10-14）

FileManager 消费公共 NavigationService::request_ui 的 SelectPath/AllFiles；没有私有 UI 导入。SelectPath 现在会在应用选中之前，于已注册根之下展示并刷新祖先节点，因此文件夹卡片导航会更新现有的树/面包屑/会话路径。按路径选中的根节点与按 ID 选中同一根节点时的高亮表现一致。现有的规范化包含校验与索引输入路径拼写保持不变。网格/列表的实际双击和多级导航已通过 FileAtlas MCP 执行。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动以及现有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公共操作会上报挂钟（wall-clock）跨度时长与错误。领域服务契约、事件负载与源操作顺序保持不变。捕获范围与限制参见 fileatlas-plugin-debug/PORTS.md。

## 标签选项失效刷新（2026-10-15）

`NavigationService::tag_revision() -> u64` 是一个仅存在于内存中的版本号。
成功的标签创建/重命名/删除/移动/外观/显示操作会推进它；
Cordis 拥有的 MediaIndexChanged 观察者也会在 Tagging 合并
和元数据对账时推进它。它不会查询 SQLite，也不会发出额外事件。
Settings 使用这一标识来刷新后台选项，同时在无变化时保持空闲。
SQL 排序/schema 以及标签命令结果保持不变。

## 共享刷新归属（2026-10-15）

侧边栏与扫描监视器使用 UI Bridge refresh::poll。扫描采样在抽取事件前后比较 scanned/total/phase/error；仍在活动但没有进展时不会发出通知。完成时保留现有的刷新与 MediaIndexChanged 发出行为，然后终止这个有限次数的轮询器。


## 浏览器运行时更新 — 2026-10-15

运行时文件夹记录现在是**收藏（favorites）**。`add_folder` 保存一个书签并选中它而不做索引；`rescan` 请求对该目录重新浏览，并保留批注；remove 只移除书签。遗留的扫描接收器签名仍为与源兼容的调用方保留，并在浏览器模式下立即完成。`rename_favorite(i64,&str)` 修改别名；`move_favorite(i64,isize)` 修改顺序。两者都不会改变本地名称。

对已存在目录的任意 `SelectPath(String)` 都会被接受。`browse_subdirectories(&str)->io::Result<Vec<SubdirectoryNode>>` 只读取一层，不做子项探测/计数扫描；Sidebar 通过一个 ReadModel 排队展开的路径。行会在实际展开之前乐观地显示展开箭头。磁盘上缺失的收藏仍保留在导航中，并带有浏览错误。UI 贡献与选择器归属仍保留 Cordis 清理。未安装任何收藏 watcher 或启动扫描。

## 资源管理器风格侧边栏分区（2026-10-15）

Folders 页面在现有收藏树的上方/下方堆叠了三个新分区：
**Drives**（顶部）、**Quick access**（顶部），以及分组的 **Favorites**
（底部，位于其他内容之上；现有文件夹树保留在中间）。

- `src/drives.rs` 的 `enumerate_drives(include_network) -> Vec<DriveInfo>` 使用
  windows-sys 的 `GetLogicalDrives`/`GetDriveTypeW`/`GetVolumeInformationW`/
  `GetDiskFreeSpaceExW`（仅 Windows；其他目标返回空列表）。
  网络驱动器只显示盘符+用量（不含 UNC）。读取通过
  `ReadModel<(show_network, roots_revision), Vec<DriveInfo>>` 执行；渲染只
  发起请求，由 refresh::poll 定时器交付结果（`drives_read.poll()`）。
- `src/sidebar_folders.rs` 的 `SidebarFoldersService` 在 RwLock 之后
  持有 `fileatlas_contracts::SidebarFoldersConfig`（quick_access 条目、带文件夹的
  命名 favorite_groups、`show_network_drives`，默认为 true），
  外加一个 AtomicU64 版本号。`update()` 递增版本号并返回
  新文档；`restore()` 在启动时以“静默但递增版本”的方式填充初值。它不是
  Cordis 服务；Settings 通过
  `NavigationService::sidebar_folders()` 访问它。
- 持久化：每次变更后该视图发出
  `SidebarFoldersChanged(SidebarFoldersUpdate { revision, config })`；Settings 插件把它
  存入自己的 JSON（`settings.sidebar_folders`），Navigation 在
  Settings 启动时恢复。没有 SQLite 改动。侧边栏轮询把
  `folders.revision()` 与视图快照比较，以便在外部
  恢复/持久化循环之后重新渲染。
- 交互遵循已确认的决定：浏览模式下点击
  选中路径而不展开它（`activate_sidebar_path`）；库模式下点击/菜单把
  文件夹加入库（`add_folder_path`）；逐行菜单提供添加/在文件
  管理器中打开/重命名/移除；内联编辑行（`SidebarEditor`）编辑路径、
  分组名称与显示名称，原生选择器通过
  `picker_editor` 路由。文件夹行共享惰性的 `expanded_directories` 树和
  会话展开持久化。

## 侧边栏语义请求与调试快照（2026-10-16）

资源管理器风格的分区可以在没有指针命中测试的情况下驱动，因此 MCP
验证不再依赖截图像素标定。

- `src/ui_requests.rs` 新增 `NavigationUiRequest::Sidebar(SidebarUiRequest)` 以及
  `SidebarEditorRequest`。`SidebarUiRequest` 派生 `serde::Deserialize`，使用内部的
  `"kind"` 标签（snake_case），因此 MCP 客户端发送与其日志完全一致的 JSON
  形式，例如 `{"kind":"remove_quick_access","path":"F:\library"}` 或
  `{"kind":"open_editor","text":"新分组","editor":{"target":"create_group"}}`。
- `ui_requests::validate_sidebar_request(folders, request)` 在排队之前运行于
  `NavigationService::request_ui` 内：分区 key 限定为
  `drives`/`quick_access`/`favorites`/`folders`，驱动器根必须存在于廉价的
  `GetLogicalDrives` 位掩码结果中（不做容量探测），路径必须是已存在的目录，而收藏
  分组/文件夹 id 必须存在于当前 `SidebarFoldersConfig` 中。拒绝会
  返回 `NavigationUiRequestError::InvalidSidebarRequest(reason)`。
- 已挂载的视图在 `view/sections.rs::apply_sidebar_request`
  中应用排队的请求，该函数只调用现有的行和菜单处理函数
  （`toggle_section`、`set_show_network_drives`、`toggle_drive`、
  `activate_sidebar_path`、`toggle_directory`、`add_folder_path`、
  `add_quick_access_path`、`remove_quick_access`、`add_favorite_folder_path`、
  `remove_favorite_folder`、`remove_favorite_group`、`open_sidebar_editor`、
  `confirm_sidebar_editor`）。它是一个触发器，而不是并行实现：
  指针路径与请求路径共享同一份代码，包括
  `SidebarFoldersChanged` 持久化发出。
- `view/sidebar_state.rs::publish_sidebar_state` 在现有的
  `refresh::poll` 定时器中运行，并把一份 JSON 快照写入
  `NavigationService::sidebar_state()`（`publish_sidebar_state`/`sidebar_state`
  访问器位于 `src/lib.rs`）。发布由一个签名进行变更门控，签名涵盖
  page、浏览模式、折叠的分区、展开的驱动器、config 版本、驱动
  读取状态、展开目录的路径/版本、分组折叠状态、编辑器的 target+text+error 以及会话
  选中路径，因此空闲的侧边栏不产生任何开销。
- 快照字段：`page`、`browse_mode`、`*_collapsed`、`show_network_drives`、
  `config_revision`、`drives_pending`、`drives[]`（root/letter/label/kind/usage
  字节/expanded/selected）、`system_quick_access[]`、`quick_access[]`、
  `favorite_groups[]`（含嵌套文件夹）、`library_folders[]`，以及
  `editor`（`target` 的调试名、当前 `text`、内联 `error`）。其他字段：`roots_revision`、`system_folders_pending`、`save_error`、驱动器的 `usage_error` 以及行的 `child_count`。
- 检查失败：位于 profile `stderr.log` 中的
  `navigation request` tracing 行会记录每个排队的请求；校验错误则作为
  MCP 工具错误负载返回。

## 侧边栏评审修正 — 2026-10-16

`SidebarFoldersService::update` 对无操作返回 None，并在重复添加时保留别名。Quick access 与驱动器折叠箭头共享 `expanded_directories`；分区 key 为 `quick_access`。`SessionState.sidebar_collapsed_sections/sidebar_collapsed_groups` 持久化分区/分组状态。书签之外的现有目录现在会在浏览器模式下恢复；遗留模式保留其根成员判定规则。

新增 `SidebarUiRequest::{RefreshRoots, RetrySave, ToggleGroup { group_id }}`，路由到相同的 UI 处理函数。窗口激活/手动 RefreshRoots 使驱动器和已知文件夹的 ReadModel 失效，且与收藏编辑彼此独立。系统已知文件夹查找/探测在渲染之外执行。已展开活动目录的子项会在本地索引事件时刷新。浏览器中未知的媒体计数会被隐藏。扩展 Windows 路径按字面比较以实现选中，避免渲染期的文件系统 IO。

保存事件携带单调递增的版本号。视图会显示失败的最近一次保存并提供 Retry；陈旧的成功或失败不会覆盖更新的状态。内联路径/名称校验保持在编辑器内，而不是替换库快照。容量读取失败显示 Unavailable，而不是虚构的 0/0。缓慢的 OS/网络调用无法被强制中断；离线网络和可移动驱动器对话框尚未通过本地驱动器检查得到验证。

可复现的检查与缺陷记录：docs/validation-ui/2026-10-16-sidebar-review/README.md。渲染已拆分到 sections/render.rs 及其 quick_access/favorites/editor 模块。


## 资源管理器地址栏 — 2026-10-16

延迟更新：request_ui 在释放队列锁之后触发一次合并的共享 Wake；Sidebar 持有该监听任务。SelectPath/ExpandPath/AddressBar 会跳过无关的收藏/标签快照重载。浏览器路径校验不再为了遗留的成员判定而加载收藏根。目标存在性仍在操作时检查。标签历史针对全新的服务快照校验，而不是可能陈旧的已渲染树。100ms 被动采样器保持不变。

`command.breadcrumb` 中现有的 navigation/breadcrumb-view 贡献现在就是完整的地址栏。view/address_bar.rs 持有其带类型的操作/输入/历史；address_bar/render.rs 渲染 Back/Forward/Up、从根到叶可点击的面包屑以及可编辑路径。location.rs 持有纯路径展示与有界的窗口内 NavigationHistory（200 个位置）。没有新的数据库或持久化历史表。当前作用域仍使用现有的会话路径/文件夹/标签字段；恢复时恰好发出一个匹配的作用域事件，因此空目录事件不会清除已恢复的标签/收藏。

公共 NavigationUiRequest::AddressBar(AddressBarRequest) 接受 serde kind back/forward/up/edit/cancel/submit/set_text {text}/crumb {path}。它由已挂载的 Sidebar 消费，使用与指针控件相同的处理函数。Crumb 会针对当前祖先链校验；输入的路径与历史中的路径在操作时检查。支持带引号的绝对路径和相对路径；相对路径基于当前文件夹。无效/缺失的位置会保留作用域与历史游标，并显示可见的错误状态。当前面包屑与未变化的提交不会重载。驱动器根与 UNC 共享根没有 Up 目标。

来自 FileManager 标签过滤的会话变化也会同步历史而不产生文件系统 IO；选择文件夹会清除标签作用域。Home 和标签视图有上下文标签，而不是虚构的文件系统路径。Back/Forward 恢复目录/home/标签作用域，在新分支上截断前向历史，并对等价的 Windows 路径去重。对于新窗口，历史在恢复位置处重置。

get_ui_state.sidebar_folders.address_bar 采用变更门控：path、editing、text、error、can_back/can_forward/can_up、breadcrumbs[{label,path}]。Input 使用 InputState 与真实的 PressEnter；Escape 取消，Ctrl+L 进入编辑，Alt+Left/Right 在该栏聚焦时导航。FileManager 会从其自身焦点转发 Ctrl+L 和 Alt+Left/Right；现有 Alt+Up 予以保留。较长的面包屑可横向滚动；编辑状态暴露完整路径。未添加任何渲染期的元数据/canonicalize 探测。参见 docs/validation-ui/2026-10-16-address/README.md。

## 统一主题（2026-10-18）

视图绘制使用 UI Bridge 的语义主题 token，而不是本地颜色字面量。
FileManager 的检查器/统计卡片与标题使用共享组件；紧凑
导航保留其布局并使用通用选中/悬停绘制。状态、连接线与
媒体覆盖层颜色在中心主题中仍保持语义区分。
产品的服务/数据/事件契约无改动。参见 `docs/ui-theme.md`。
# 统一收藏分组侧边栏（2026-10-18）

每个收藏分组都是一个同级区域，使用与 Drives/Quick Access 相同的 UI Bridge
`components::sidebar_section/sidebar_header`。不再残留外层
Folder favorites / Library folders / Favorites collection 容器。
整个侧边栏列在宿主视口内滚动。分组的加号调用
`add_current_favorite`：解析已提交的 Session 目录（含嵌套
路径），检查该分组与目录仍然存在，通过现有的带版本号的 Settings 事件
添加一个快捷方式，展开目标分组，并且从不打开
选择器或启动索引。Home/标签集合会禁用加号。逐分组的路径匹配
使用规范化的 Windows 标识，在重复时保留别名与版本号。

`SidebarFoldersService::import_library_folders(&[(i64, String, String)], &str)`
在实际导入时返回 `Option<SidebarFoldersConfig>`。现有库
根在默认 Favorites 分组中只表示一次（或表示在
一个同名的既有分组中）；既有分组/别名/顺序得以保留。附加式的
serde 默认字段 `SidebarFoldersConfig.imported_library_paths: Vec<String>` 存储规范化后的
路径，而不是可能被复用的 SQLite ID。移除快捷方式/分组
不会移除数据库注册，也不会使其在重启后重新出现。导入
运行在现有的快照变更/启动轮询上，而不是每次渲染都运行；持久化
与重试仍使用 `SidebarFoldersChanged` / SettingsService。没有 SQL
schema 迁移，也没有并行的持久化设施。

公共 `SidebarUiRequest::AddCurrentFavorite { group_id }` 映射到真实的加号
处理函数。现有 `AddFavoriteFolder` 仍是显式路径变体。遗留的
`ToggleSection favorites/folders` 现在切换全部同级分组；单个分组
折叠使用 `ToggleGroup`。已注册书签的上下文菜单保留重新扫描、
创建子文件夹以及显式的 Remove from library；普通的 Remove from favorites
只移除快捷方式。旧的库根显示重排/重命名 UI 已被分组
归置/别名编辑取代；公共后端兼容方法予以保留。

`sidebar_state()`/MCP `get_ui_state.sidebar_folders` 新增 `favorite_current_path`、
`favorite_action_error`、`favorite_imported_paths`，以及逐分组的 `header_bounds`
（以逻辑 GPUI 客户端坐标表示的 [x,y,width,height]）。Bounds 来自同一
视图的绘制画布，为诊断目的采用变更门控，且不触发重绘/IO。
持久化失败请检查 `save_error`。参考：`docs/ui-theme.md` 和
`docs/reference-ui/concepts/sidebar-favorite-groups-2026-10-18.md`。

## MCP 功能贡献

Navigation 提供 `navigation.list_library`、`navigation.add_folder`、
`navigation.remove_folder`、`navigation.rescan_folder`、
`navigation.cancel_scan`，以及标签树 CRUD 工具 `navigation.create_tag`、
`navigation.rename_tag` 和 `navigation.delete_tag`。Schema 与处理函数
归属此处并调用 `NavigationService`；文件夹的添加/移除/重新扫描和标签删除
需要显式确认。`navigation.manage_favorite` 重命名或重排
文件夹收藏别名而不改变本地路径。文件夹移除只移除 FileAtlas 库
注册与索引行。迁移期间保留遗留 MCP 名称。

## 原生主题消费（2026-10-19）

Navigation 轨道、选中行、层级连接线、popovers、对话框与颜色建议
现在通过 UI Bridge 消费原生 GPUI Theme 角色。建议的标签色板来自
当前活动主题；用户保存的颜色会被保留。没有按模式的手动调色板，
也没有私有 Theme 更新。
