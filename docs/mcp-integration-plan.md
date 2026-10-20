# FileAtlas MCP 接入方案与实施状态

## 目标

为 AI 客户端提供受支持的 MCP 接口，使其能够观察和操作 FileAtlas。接口包含当前 FileAtlas 窗口的真实截图，供 AI 查看实际渲染出的布局和界面状态。下文记录方案、已实现能力及尚待验证的事项；实现 MCP 接口不代表 FileAtlas 的原生 UI 迁移已经完成。

## 当前实施状态（2026-10-13）

- `fileatlas-plugin-mcp` 使用官方 `rmcp` 3.5.1，由 Cordis 管理生命周期。正常启动时在 GUI 主进程提供 localhost Streamable HTTP；`--mcp` 保留 stdio 调试模式。
- 已实现 `screenshot_window`、媒体库/搜索/详情读取、视图状态、预览/播放控制、扫描取消、评分/标签、文件夹管理、媒体文件操作和元数据导出工具。逐项服务映射和限制见 [`crates/fileatlas-plugin-mcp/PORTS.md`](../crates/fileatlas-plugin-mcp/PORTS.md)。
- Windows 截图通过 Shell 提供的本进程 GPUI HWND 调用 `PrintWindow(PW_RENDERFULLCONTENT)`，编码成 MCP PNG 图片内容。代码已实现；MCP 客户端实连、截图画面保真度和不同窗口状态仍需 Windows 实机验证。
- 以上为 2026-10-13 的历史记录。当前 HTTP 与 stdio 模式共用设置页里的三档权限，不再通过 `FILEATLAS_MCP_ALLOW_MUTATIONS` 环境变量授权修改。关闭档只读；常规控制档允许 UI 控制及普通增删改；完全控制档开放插件 MCP 元数据标记的高影响工具，但仍要求每次调用 `confirm: true`。新配置默认关闭，旧 `mcp_allow_mutations` 布尔字段兼容映射为 false→关闭、true→完全控制。详见 `docs/validation-ui/2026-10-19-mcp-access-stages/README.md`。
- 该状态快照最初记录 shell 检查被 FileManager `.track_scroll` API 编译错误阻塞；此问题已在后续整合中修复。2026-10-13 的最终整合记录中，格式检查及 `cargo check -p fileatlas-shell --locked -j 2` 均通过。MCP 实际客户端往返、窗口截图保真度及 Windows 行为仍待实机验证；以 [`docs/validation-report.md`](validation-report.md) 最新迁移记录为准。

## 架构

- 新增 `fileatlas-plugin-mcp` workspace crate，负责 MCP 协议适配、工具定义、传输与会话管理、请求校验。
- 生命周期和依赖仍由 Cordis 管理。`fileatlas-shell` 负责组合并按依赖顺序启动 MCP 插件；不在 Shell 中实现业务操作，也不另建一套插件管理器。
- MCP 工具通过现有公开 Cordis 服务和命令/事件契约调用功能。MCP 适配层不得直接执行 SQL、导入其他插件的私有模块，或把 GPUI 控件当作业务 API。
- 在 Shell/UI 边界增加小型界面捕获契约。Shell 持有原生窗口句柄，并能请求截取 FileAtlas 窗口；`fileatlas-plugin-mcp` 将结果作为 MCP 图片响应返回。平台相关的截图实现应封装在该契约之后。
- Cordis 插件停止时，清理 MCP 插件拥有的注册项、任务、IPC 端点和 UI 贡献。

### 传输方式

正常启动采用同进程 Streamable HTTP MCP：Shell 启动 MCP Cordis 插件，在 GUI 进程内监听 `127.0.0.1:38473/mcp`，可通过 `FILEATLAS_MCP_HTTP_PORT` 修改端口。MCP 客户端连接已运行实例，不负责启动 FileAtlas 子进程。`--mcp` 暂保留为 stdio 调试兼容入口。监听仅绑定回环地址。HTTP 与 stdio 共用设置页的 Off/Standard/Full 三档访问权限；权限变化立即生效，无需重启，也没有环境变量授权覆盖。高影响操作按插件 MCP 元数据分级，并保留逐次 `confirm=true`。若监听端口占用或启动失败，FileAtlas 会记录错误并继续启动 GUI。

Codex 客户端可在 `config.toml` 中配置：

```toml
[mcp_servers.fileatlas]
url = "http://127.0.0.1:38473/mcp"
```

产品功能工具按插件贡献，不在本文维护全局参数清单。宿主提前发布 `fileatlas_contracts::mcp::McpToolRegistry`；每个功能插件在自身源码中声明 namespace、工具名、输入 JSON Schema、handler 和修改/确认属性，并把注册 guard 交给 Cordis 管理。MCP 插件读取存活贡献并构造 rmcp 工具路由。停止插件会撤销其贡献。新增功能的参数与行为说明维护在对应插件的 `PORTS.md`。

## 截图工具

初始工具名：`screenshot_window`（最终名称可按选定 MCP SDK 的命名惯例调整）。

- 捕获 FileAtlas 当前实际渲染的顶层窗口，包括当前窗口尺寸、主题、控件、滚动位置和可见浮层。不得根据视图模型重绘图片，也不得返回另行生成的示意图。
- 通过 MCP 图片内容返回图像字节，并使用正确的 MIME 类型。客户端和协议允许时，同时返回像素尺寸、截图时间等简要信息。
- 第一版只捕获 FileAtlas 窗口，不截取整个桌面、无关窗口或窗口当前不可见的内容。
- 明确处理窗口最小化、被遮挡、正在关闭或尚未就绪等情况。无法准确截图时返回类型明确的错误，不得静默返回过期或合成的截图。
- 默认保留足够的原始分辨率供界面分析。若受消息大小限制必须缩放，应明确报告缩放后的图像尺寸。
- 截图实现使用 Shell 发布的 GPUI HWND，通过 Windows `PrintWindow(PW_RENDERFULLCONTENT)` 捕获并编码为 PNG。需验证普通、调整尺寸、最小化和被遮挡等窗口状态。代码已接入，但真实 Windows 画面保真度与可靠性仍未验证。

## 工具范围与安全边界

提供输入输出明确、范围狭窄的类型化操作，不提供通用命令执行或任意 UI 脚本工具。

1. **观察（第一阶段）：** 截图、应用/窗口状态、已注册文件夹和标签、扫描进度，以及现有服务契约支持的媒体搜索和选中项详情。
2. **低风险修改：** 通过现有服务设置评分、分配标签、修改搜索/筛选状态，以及开始或取消扫描。
3. **高影响操作：** 文件移动、复制、删除、重命名、元数据写回和导出。只有在服务契约、确认流程、路径校验和结果报告明确后才开放。批量调用需逐项报告结果，并限制请求规模。

只读观察不应依赖开启破坏性操作。破坏性操作必须经过明确的确认流程，不能仅凭 AI 发出工具调用就视为用户确认。文件系统访问遵守 FileAtlas 现有的注册文件夹和文件操作语义。长时间扫描或批量任务应返回结构化错误、任务标识和进度；由现有服务支持时提供取消能力。

## 交付阶段

### 阶段 0：契约与传输验证

- 使用 MCP 官方 Rust SDK `rmcp` 3.5.1；依赖版本固定在 Cargo.lock 中。
- 使用最小纵向切片验证客户端启动/连接流程、类型化工具注册和图片内容响应格式。
- 在 Windows 上截取真实 FileAtlas HWND，并在至少一种目标 MCP 客户端中检查返回的图片。
- 扩展 API 前记录传输方案和截图能力的限制。

### 阶段 1：只读接入

- 将 `fileatlas-plugin-mcp` 加入 workspace，并纳入 Shell 的启动/停止顺序。
- 注册截图和少量基于现有服务的只读工具。
- 为新增 crate 编写 `PORTS.md`，记录插件/服务标识、依赖、数据结构、生产者/消费者、限制和排错方式。
- 在同一实现变更中更新 `docs/source-feature-audit.md`，如实标记 MCP 能力，并保留原生 FileAtlas UI 与 MCP 适配能力的状态区别。

### 阶段 2：可逆操作

- 通过公开服务加入评分、标签分配、搜索/筛选和扫描控制。
- 确保这些操作触发与应用内等效操作相同的刷新或事件。
- 为长时间任务提供清晰的结构化结果、服务错误详情，以及可用的进度和取消能力。

### 阶段 3：受控文件操作与导出

- 在确认流程、路径范围、批量限制、逐项结果和失败处理完成后，再加入文件修改和元数据导出。
- 对照 `docs/source-feature-audit.md` 保留原生 UI 的实际迁移状态；增加 MCP 入口不代表原生 UI 迁移完成，也不应把已迁入 UI 的状态回退成未完成。

## 验收标准

- MCP 随 Cordis 生命周期启动和停止；工具注册前依赖服务已就绪，插件停止时清理其拥有的全部资源。
- 支持的客户端可以调用 `screenshot_window`，并收到可解码、反映 FileAtlas 当前可见状态的真实窗口图片。
- 只读和修改类工具都映射到已记录的公开服务，校验输入并返回类型明确的结果或错误。
- 破坏性操作在既定确认流程完成前无法执行；批量规模和路径范围约束确实生效。
- `PORTS.md` 与实际公开签名一致；Windows 专属行为在实际 Windows 验证前明确标记为未验证。

## 验证计划

遵循仓库对此实现阶段的验证要求：仅运行 `cargo fmt --all -- --check` 和 `cargo check -p fileatlas-shell --locked`，启用增量编译并限制并行任务数；在 `docs/validation-report.md` 记录准确命令和结果。该阶段不运行 workspace 或单个插件测试。另行安排集中式 Windows 验证，覆盖原生窗口截图、MCP 客户端启动、不同窗口状态的截图、原生文件操作和系统集成。在该验证完成前，不得将相关行为报告为已验证。

## 待确定事项

- 首批正式支持哪些 MCP 客户端？
- 如何避免不同 MCP 客户端同时启动多个 FileAtlas 实例？
- 默认允许 AI 读取哪些媒体元数据和应用状态，尤其是截图与 EXIF 详情？
- 哪些修改工具默认启用？MCP 客户端应通过什么确认交互处理破坏性操作？
- 截图大小限制和可选缩放策略如何设定，才能兼顾目标客户端限制和界面细节？
