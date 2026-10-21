# WorkCrate Boards 端口

Cordis 插件 `fileatlas.workcrate-boards` 发布 `fileatlas/workcrate-boards`，并依赖 Projects。`VectorCraftConfig { executable, cli_executable, port }` 仍保存在 `vectorcraft.json` 中；两个可执行文件字段都是可选字符串，`port` 默认为 17979。留空的 CLI 路径会解析为所配置 GUI 可执行文件同目录下的 `vectorcraft-cli.exe`。遗留的回环（loopback）传输只被 `connect()` 与 `open_editor()` 使用；只有显式的编辑器动作才会启动 GUI 窗口。

`board_catalog`、`load_board`、`board`、`boards_for_project` 与 `render_preview` 只读取 FileAtlas 自有的 `projects/<uuid>/boards-cache.json` 与 `native-previews/`。它们从不启动或联系 VectorCraft。目录清单缺失时返回空；缓存预览缺失时返回 `PreviewUnavailable`。缓存条目绑定到已持久化的原生文档关联 UUID，因此 `attach_document` 会清除旧缓存，而任何绑定关系的轮换都会使旧的 board ID 不可用。

`create_project_document(project, directory)` 通过官方的 `vectorcraft-cli.exe mcp --headless` JSON-RPC stdio 服务器创建首个 A3 横向文档，把 `.vectorcraft` 文件写入所选文件夹且不覆盖已有文件，追加原生素材头部（material header），并把目录清单/预览缓存存放在 FileAtlas 内部。`refresh_project(project)` 是显式的“磁盘到缓存”同步动作；它走同一条被路由的会话，因此当 GUI 已持有该文档时读取的是 GUI 的实时状态而不是直接拒绝。创建文档则始终使用静默无头会话：新建原生文件绝不能占用用户已打开的 GUI 窗口。两个操作都使用有界的子进程生命周期、隐藏的 Windows 进程创建、位于 FileAtlas 项目根下的隔离 CLI 偏好设置，以及一个显式的原生文件夹读/写根。预览 PNG 字节由官方渲染器内联返回，并由管理器写入 FileAtlas。

`create_page`、`update_info`、`place_reference`、`place_file`、`export_native` 与 `refresh_project` 都通过 `official_session` 执行，由它选择传输通道，并在写入成功后刷新持久快照：用户 GUI 的控制端口可达时用它，否则用静默无头会话。GUI 正在运行不再是拒绝后台写入的理由，因为写入落到该 GUI 自己的引擎上，不会与过期的内存副本抢写。放置操作仍要求原生文档与源文件共享同一个折叠后的读取文件夹；跨卷的源文件会被拒绝。导出从原生文件夹读取并写入所选输出文件夹。只有 `open_editor` 会连接或启动 GUI 窗口、导航到活动画板（artboard）并聚焦编辑器。遗留的 TCP JSON-line 客户端（`protocol.rs`）现在只服务 `connect()` 与 `open_editor()`；其余原生调用一律使用 `mcp::McpSession`，它讲换行分隔的 JSON-RPC，把回复限制在 32 MiB 内，超时 65 秒，绝不重试任何变更操作，且只在结果不确定（超时、响应 ID 不匹配、超长或无法解析的行）之后才让子进程失效（poison）。命令级的 `isError:true` 回复是一个仍然正常服务的服务器给出的合法应答，因此会话能在它之后继续被调用；`command_error_keeps_the_headless_session_usable` 守护该行为。

## 画布编辑会话

`canvas_open(board_id)` 启动页面编辑区的官方编辑会话，`canvas_close()` 结束它；`canvas_is_open()` 报告是否持有会话。同一个 `mcp::McpSession` 承担两条通道：`port_is_listening` 有应答时用 `McpSession::live(cli, port)` 跑 `mcp --connect 127.0.0.1:<port>` 驱动用户 GUI，否则用 `McpSession::headless` 静默跑 `mcp --headless`，并把 `VECTORCRAFT_CONFIG_DIR` 指向该工程自己的 `cli-config` 目录，长生命周期会话因此不会在共享偏好目录上再起第二个引擎。编辑语义全部留在官方：`pointer_gesture`、`select_tool`、`object.move`、`object.setBounds`、`file.place`、`links.update`、`links.relink`、`text.create`、`text.setText`、`undo`、`redo`、`doc_inspect`、`render_preview` 与 `save_file`。`CanvasSnapshot` 只携带官方报告的内容（`revision`、`dirty`、`tool`、`selection`、`selectionBounds`、画板几何、渲染预览路径与尺寸、最后一条历史步骤），`canvas_object` 返回官方节点的 kind、名称、bounds、文本与链接状态（`kind` 统一小写，链接状态报为 `ok`/`modified`）。`object.setBounds` 一律带 `reference:0`，这样 x/y 表示界框左上角；官方默认是中心点。

每次编辑一旦官方报告 `dirty` 就立刻通过 `save_file` 写回原生文档，因此 `.vectorcraft` 文件、GUI 与 `refresh_project` 始终描述同一页；离开画布时由 `canvas_commit` 兜底。`canvas_probe(known)` 是观察者侧的变化探针：调用方传入自己上次显示的 revision，因为由其它客户端（自动化调用、第二个面板）做出的改动会推进会话，而该观察者从没见过被返回的快照。`type_text` 只存在于实时通道；无头会话用 `text.create`/`text.setText` 新建或替换文本。

官方启动器只接受一个 `--automation-read-root` 与一个 `--automation-write-root`，因此会话的全部输入会折叠成包含它们的最深单个文件夹（`automation_folder`）。画布会话初始只被授予文档本身加上已经位于文档自己文件夹内的工程引用；当用户指向别处的文件时，`canvas_grant` 先提交文档、关闭正在运行的会话，再带着扩权后的折叠根重启会话，因为引擎在启动时就固定了自动化根。实时 GUI 保留其启动者声明的根，所以那里被拒绝的路径保持为可见错误，不会静默重启用户的程序。实时会话与用户共享 GUI，因此每次变更前都会重新校验活动文档仍是绑定的那一个；`open_document` 对已经打开的窗口执行激活（通过 `inspect_ui` 找到、用 `document.activate` 切换），而不是从磁盘重读。

`attach_document(project, path)` 使用只读的无头会话（`open_file` 加上 `doc_inspect` 路径检查）校验已存在的 `.vectorcraft`，且从不联系 GUI；`native::revalidate_open_headless` 与 GUI 打开器共享同一套打开报告策略。VectorCraft 0.14.0 的无头 `export` 工具声明 `additionalProperties:false`，只接受 `artboard`、`format`、`options`、`outlineText`、`path`、`range`、`scale`、`selection`；PDF 使用 `range`（绝不使用 `useArtboards`，该参数只存在于 GUI 命令目录中），位图则使用单个 `artboard` 加 `scale`。`operations.rs::OFFICIAL_EXPORT_ARGS` 锁定了该参数列表，而 `export_params_only_use_official_tool_arguments` 这个不会被跳过的测试对其加以守护。只有当被链接文件在原生文档文件夹（读取根无法覆盖它）之外的磁盘上仍然存在时，无头 `open_file` 的 `missingLinks` 条目才被当作沙箱产物对待；警告、被修改的链接以及确实缺失的链接都是阻塞性的，并会被记录到诊断映射中，直到重新校验成功为止。

页面身份仍然是项目 UUID + 已持久化的关联 UUID + 原生数字画板 ID。头部内容是真实的原生文本，带有版本化的 `fileatlas.workcrate` 备注；编辑操作按撤销分组（undo group）组织。位图/PDF 导出都是官方 CLI 导出，并以原子方式放到请求的输出路径。失败排查请通过返回的 BoardsError 值以及 WorkCrate UI/MCP 操作消息。没有嵌入任何上游源代码或渲染器依赖。
