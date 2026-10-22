# WorkCrate Boards 端口

Cordis 插件 `fileatlas.workcrate-boards` 发布 `fileatlas/workcrate-boards`，并依赖 Projects。`VectorCraftConfig { executable, cli_executable, port }` 仍保存在 `vectorcraft.json` 中；两个可执行文件字段都是可选字符串，`port` 默认为 17979。留空的 CLI 路径会解析为所配置 GUI 可执行文件同目录下的 `vectorcraft-cli.exe`。遗留的回环（loopback）传输只被 `connect()` 与 `open_editor()` 使用；只有显式的编辑器动作才会启动 GUI 窗口。

`board_catalog`、`load_board`、`board`、`boards_for_project` 与 `render_preview` 只读取 FileAtlas 自有的 `projects/<uuid>/boards-cache.json` 与 `native-previews/`。它们从不启动或联系 VectorCraft。目录清单缺失时返回空；缓存预览缺失时返回 `PreviewUnavailable`。缓存条目绑定到已持久化的原生文档关联 UUID，因此 `attach_document` 会清除旧缓存，而任何绑定关系的轮换都会使旧的 board ID 不可用。

`create_project_document(project, directory)` 通过官方的 `vectorcraft-cli.exe mcp --headless` JSON-RPC stdio 服务器创建首个 A3 横向文档，把 `.vectorcraft` 文件写入所选文件夹且不覆盖已有文件，追加原生素材头部（material header），并把目录清单/预览缓存存放在 FileAtlas 内部。`refresh_project(project)` 是显式的“磁盘到缓存”同步动作；它走同一条被路由的会话，因此当 GUI 已持有该文档时读取的是 GUI 的实时状态而不是直接拒绝。创建文档则始终使用静默无头会话：新建原生文件绝不能占用用户已打开的 GUI 窗口。两个操作都使用有界的子进程生命周期、隐藏的 Windows 进程创建、位于 FileAtlas 项目根下的隔离 CLI 偏好设置，以及一个显式的原生文件夹读/写根。预览 PNG 字节由官方渲染器内联返回，并由管理器写入 FileAtlas。

`create_page`、`update_info`、`place_reference`、`place_file`、`export_native` 与 `refresh_project` 都通过 `official_session` 执行，由它选择传输通道，并在写入成功后刷新持久快照：用户 GUI 的控制端口可达时用它，否则用静默无头会话。GUI 正在运行不再是拒绝后台写入的理由，因为写入落到该 GUI 自己的引擎上，不会与过期的内存副本抢写。放置操作仍要求原生文档与源文件共享同一个折叠后的读取文件夹；跨卷的源文件会被拒绝。导出从原生文件夹读取并写入所选输出文件夹。只有 `open_editor` 会连接或启动 GUI 窗口、导航到活动画板（artboard）并聚焦编辑器。遗留的 TCP JSON-line 客户端（`protocol.rs`）现在只服务 `connect()` 与 `open_editor()`；其余原生调用一律使用 `mcp::McpSession`，它讲换行分隔的 JSON-RPC，把回复限制在 32 MiB 内，超时 65 秒，绝不重试任何变更操作，且只在结果不确定（超时、响应 ID 不匹配、超长或无法解析的行）之后才让子进程失效（poison）。命令级的 `isError:true` 回复是一个仍然正常服务的服务器给出的合法应答，因此会话能在它之后继续被调用；`command_error_keeps_the_headless_session_usable` 守护该行为。

## 画布编辑会话

`canvas_open(board_id)` 启动页面编辑区的官方编辑会话，或在同一文档内把已有会话移到另一页；`canvas_close()` 结束它；`canvas_is_open()` 报告是否持有会话，`canvas_open_quiet(board_id)` 执行同样的建会话流程但不渲染预热帧，`canvas_holds(board_id)` 报告当前会话是否已属于该页（字段保存用这两者保证只渲染一次）。同一个 `mcp::McpSession` 承担两条通道：`port_is_listening` 有应答时用 `McpSession::live(cli, port)` 跑 `mcp --connect 127.0.0.1:<port>` 驱动用户 GUI，否则用 `McpSession::headless` 静默跑 `mcp --headless`，并把 `VECTORCRAFT_CONFIG_DIR` 指向该工程自己的 `cli-config` 目录，长生命周期会话因此不会在共享偏好目录上再起第二个引擎。编辑语义全部留在官方：`pointer_gesture`、`select_tool`、`object.move`、`object.setBounds`、`file.place`、`links.update`、`links.relink`、`text.create`、`text.setText`、`undo`、`redo`、`doc_inspect`、`render_preview` 与 `save_file`。`CanvasSnapshot` 只携带官方报告的内容（`revision`、`dirty`、`tool`、`selection`、`selectionBounds`、画板几何、渲染预览路径与尺寸、最后一条历史步骤），`canvas_object` 返回官方节点的 kind、名称、bounds、文本与链接状态（`kind` 统一小写，链接状态报为 `ok`/`modified`）。`object.setBounds` 一律带 `reference:0`，这样 x/y 表示界框左上角；官方默认是中心点。官方会话报告没有打开文档时，这是一个空状态而不是失败：`open_document` 会打开工程文档，实时 GUI 关闭了已绑定文档时 `inspect_bound` 会重新打开它。如果可用的 GUI 仍无法让该文档进入可编辑状态，`canvas_session` 会丢弃该通道，改用静默无头会话打开已保存文档，因此后台编辑器不依赖可用的 GUI 窗口。若文档最终仍未成为活动文档，界面显示中文提示而不是官方协议原文。

每次编辑一旦官方报告 `dirty` 就立刻通过 `save_file` 写回原生文档，因此 `.vectorcraft` 文件、GUI 与 `refresh_project` 始终描述同一页；离开画布时由 `canvas_commit` 兜底。`canvas_probe(known)` 是观察者侧的变化探针：调用方传入自己上次显示的 revision，因为由其它客户端（自动化调用、第二个面板）做出的改动会推进会话，而该观察者从没见过被返回的快照。`type_text` 只存在于实时通道；无头会话用 `text.create`/`text.setText` 新建或替换文本。

官方启动器只接受一个 `--automation-read-root` 与一个 `--automation-write-root`，因此会话的全部输入会折叠成包含它们的最深单个文件夹（`automation_folder`）。画布会话初始只被授予文档本身加上已经位于文档自己文件夹内的工程引用；当用户指向别处的文件时，`canvas_grant` 先提交文档、关闭正在运行的会话，再带着扩权后的折叠根重启会话，因为引擎在启动时就固定了自动化根。实时 GUI 保留其启动者声明的根，所以那里被拒绝的路径保持为可见错误，不会静默重启用户的程序。实时会话与用户共享 GUI，因此每次变更前都会重新校验活动文档仍是绑定的那一个；`open_document` 对已经打开的窗口执行激活（通过 `inspect_ui` 找到、用 `document.activate` 切换），而不是从磁盘重读。

`attach_document(project, path)` 使用只读的无头会话（`open_file` 加上 `doc_inspect` 路径检查）校验已存在的 `.vectorcraft`，且从不联系 GUI；`native::revalidate_open_headless` 与 GUI 打开器共享同一套打开报告策略。VectorCraft 0.14.0 的无头 `export` 工具声明 `additionalProperties:false`，只接受 `artboard`、`format`、`options`、`outlineText`、`path`、`range`、`scale`、`selection`；PDF 使用 `range`（绝不使用 `useArtboards`，该参数只存在于 GUI 命令目录中），位图则使用单个 `artboard` 加 `scale`。`operations.rs::OFFICIAL_EXPORT_ARGS` 锁定了该参数列表，而 `export_params_only_use_official_tool_arguments` 这个不会被跳过的测试对其加以守护。只有当被链接文件在原生文档文件夹（读取根无法覆盖它）之外的磁盘上仍然存在时，无头 `open_file` 的 `missingLinks` 条目才被当作沙箱产物对待；警告、被修改的链接以及确实缺失的链接都是阻塞性的，并会被记录到诊断映射中，直到重新校验成功为止。

页面身份仍然是项目 UUID + 已持久化的关联 UUID + 原生数字画板 ID。头部内容是真实的原生文本，带有版本化的 `fileatlas.workcrate` 备注；编辑操作按撤销分组（undo group）组织。位图/PDF 导出都是官方 CLI 导出，并以原子方式放到请求的输出路径。失败排查请通过返回的 BoardsError 值以及 WorkCrate UI/MCP 操作消息。没有嵌入任何上游源代码或渲染器依赖。

## 2026-10-22 预览与编辑优化

编辑结果发布到现有版本 1 预览缓存，无存储迁移。显式打开编辑后，预览/编辑
切换复用官方会话，左侧缩略图同步当前帧；离开页面/项目提交关闭。初次浏览
仍只读本地。项目无自选主图时显示首个缓存页面；同项目后台加载不清空旧图。
右侧支持多行文字、字体/字号、左/中/右对齐，全部转发官方 text.setStyle。
新增 Full + confirm 的 canvas_text_style（id、可选 font/size/justify）；格式输入
为空表示保留属性，不代表混合字体的当前值。几何拒绝非有限数，异步选择文件
后复核页面/选区。公开签名、控件与限制以同目录 PORTS.md 最新节为准。

## 2026-10-22 紧凑物料信息条与单页字段保存

`update_info(id, title, info) -> Result<InfoSave, BoardsError>` 返回
`InfoSave { board, frame: Option<CanvasSnapshot>, changed: bool }`，并只走三条路
中的一条。草稿与已存页面相同即 `changed:false, frame:None`：什么都不写，官方会
话不产生步骤，页面也不会重渲染。该页已有会话时（UI 显式打开的编辑会话，或上一次
保存预热出来的会话），写入复用同一条会话（`canvas_write_info`）并带回它渲染出的
`frame`，覆盖层直接显示保存后的页面，变化探针不会再渲染一次。其余情况走被路由
的会话写入，并只替换该页的目录记录与预览（`store_page`），其它页面保留各自的预
览文件。`canvas_open_quiet(board_id)` 与 `canvas_open` 共用建会话流程但不渲染预热
帧，`canvas_holds(board_id)` 报告当前会话是否已属于该页；字段保存调用方用它们静默
预热并只渲染一次，草稿最终未变时只关闭自己刚开的会话。在此之前，ID 的关联段会与
`projects.document_binding(project)` 比对，绑定轮换后宁可拒绝也不会写进过期页面。

信息条本身按 10 月 21 日手绘稿实现为紧凑浅色横条：左侧页码与醒目的项目名称，右
侧同一基线上以分隔符排布物料/尺寸/数量，附加字段落到主行下方的“值+标签”次要行。
页码坐在白色圆角底片中；底片与信息条都是官方圆角矩形，统一置于所有标题文字之后，
因此 FileAtlas 自有图形不会盖住标签。
`header_layout` 是 crate 内部纯几何模块：每个文字框尺寸都由官方 `document.node`
报告给出，该模块只把这些测量值换算成稍后用 `object.transform` 增量落位的目标框。
驱动与几何之间的角色契约是精确的——`page_number`、`page_number_chip`、
`project_name`、`material`/`size`/`quantity`、`size_label`、`extra:<key>`、
`label:<role>`、`band`，以及每个分隔符各自的索引角色 `separator:<n>`（裸
`separator` 从不落位，`each_separator_is_measured_under_its_own_index` 守护这一
点）。第一遍排主行与其分隔符并判断是否放得下；放不下时
`Layout::group_in_primary` 为 false，物料组降级。第二遍为每个次要行同时落位值与
标签（都用 `SUB_SIZE`），因为未创建的文字没有可用的度量。

两个官方行为决定了测试写法而非缺陷：官方程序对指针拖拽施加智能参考线（实测约
2.4 pt 吸附到别的节点边缘，所以画布拖拽测试选择页面中部为落点），且 `text.create`
会把过长文字自动折行到画板宽度，因此 `HeaderLayoutError::TooWide` 只是兜底分支，实
际的超长内容由请求边界检查拒绝。用户能触达的每条拒绝都是可执行的中文提示；
`describe` 把几何模块的内部原因（角色名、度量名）挡在面板消息之外。

原生文档的资源诊断不再把官方 JSON 原样抛给面板：`describe_open_report` 最多列出三
个缺失文件名（超出时给出总数），并统计被改写的链接与警告数量，然后提示先在
VectorCraft 中恢复或重新链接这些资源并保存，再回 WorkCrate 点击“关联工程”重新
选择该文档，让静默会话重新校验。静默会话仅因读取根之外而报告的链接，若文件在磁
盘上确实存在，会在展示前被过滤，避免把正常文件说成缺失。命中缓存时显示同一句提
示；`ACTIVE_DOCUMENT_CHANGED`、等待对话框和 `DirtyDocument` 也已中文化。链接
id、`\\?\` 路径与内部属性名不会进入界面。

## 2026-10-22 预览自动获取

`BoardCache.source: Option<DocumentStamp { modified_ns, len }>` 记录该目录清单
基于哪个已保存原生文件构建（`serde(default)`，因此旧版本 1 缓存没有 source，
会补同步一次）。所有缓存写入者都会盖戳：`refresh_project`（在
`official_session` 之后，因为该调用可能先提交热画布）、
`refresh_cache_from_client`、`create_project_document`、`store_page` 以及画布
帧发布。若在写入前采样文件，缓存会一直显示为过期并触发多余同步，因此每个写入
者都在自己的原生写入之后采戳。

`preview_state(project) -> PreviewState { project_id, stamp, cache_stamp }` 只是
一次本地 stat 加读取缓存戳，不启动进程、不打开端口。
`auto_refresh_project(project) -> AutoSync { synced, boards }` 在缓存已匹配时返回
`synced:false`，否则只通过静默 `mcp --headless` 会话读取已保存文件（绝不使用实时
GUI 通道，也不走 `official_session`），用 `rebuild_cache` 重建目录与预览，再经
原子写入发布；随后 `write_cache` 删除新映射不再引用的预览文件。文档缺失不算
“过期”：没有已保存文件可读，缓存视图保持不变。这里只读取已保存文件；运行中 GUI
里尚未保存的内容，要等用户在 VectorCraft 中保存后才会出现。

## 2026-10-22 切页不重连

一个画布会话属于整份原生文档，而不属于单页。`canvas_open_inner` 在
`holds_document` 确认仍是同一份 `.vectorcraft` 时保留已有会话：先清掉旧页的官方
选择（`select.none`），再移动 `board_id`/`native_id`/`artboard`/`preview`，并在同
一个进程里渲染新画板。`CanvasSnapshot.session_id` 标识官方进程：任意次切页都保持
不变，只有服务真正重启（关闭再开、`canvas_grant`、文档丢失或原生目录事务）才会分
配新值。切页不写文件；离开编辑区时才提交文档。被忽略的官方测试
`page_switches_reuse_the_same_official_session` 覆盖了复用、每页独立预览路径、清空
选择，以及真正关闭后重新打开会得到新 id。
