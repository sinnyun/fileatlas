# FileAtlas Rust 版

本仓库是 FileAtlas 0.2 项目的 Rust 实现，运行在 Windows 桌面，由 Cordis 管理功能插件、GPUI Kit 提供原生界面。**FileAtlas 是个人文件浏览器与工作台管理器**：主工作区支持单栏、双栏、四栏，每栏可独立浏览目录和文件、筛选/搜索、排列、选择并滚动；程序工作区保存各栏启动状态。媒体索引、缩略图、标签和信息检查器是浏览器内的文件能力，QuickLook 负责普通文件预览。用户可按需扩展用途独立的工作台插件。后端数据语义继续沿用 `vendor/photag-source/` 中仍适用的实现；项目基线版本不等同于已发布应用版本，Cargo workspace 版本仍独立管理。

## FileAtlas 0.2 开发基线与下一阶段

- [FileAtlas 0.2 开发基线](docs/development-baseline-0.2-2026-10-19.md)：当前产品定义、单/双/四栏主工作区、程序工作区、架构边界和验证状态；基线标识 `dev-baseline-0.2-2026-10-19`。
- [2026-10-14 开发基线（历史）](docs/development-baseline-2026-10-14.md)：早期主界面与侧栏布局基准，现由 FileAtlas 0.2 完整基线取代。
- [功能操作优化与功能扩展计划](docs/functional-development-plan.md)：从F0真实任务流审查开始，再修操作问题、实现用户确认的新功能。
- [FileAtlas 0.2 分栏与工作区实测证据](docs/validation-ui/2026-10-19-columns-foundation/README.md)。

## 当前设计依据

- [主窗口两侧分页/固定预览 v2](docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.md)：侧栏和右侧检查器的结构基准；中间文件区域的当前单/双/四栏行为见 FileAtlas 0.2 基线。
- [最终界面设计稿 v5](docs/reference-ui/final-v5/README.md)：其他页面的设计依据；主窗口两侧布局由v2更新。
- [概念稿与设计记录](docs/reference-ui/concepts/)：v2–v5 方案说明。
- [界面重构验证记录](docs/redesign-validation.md)：每轮界面改动的实测结论与尚未落地的设计项。
- [项目指引](AGENTS.md)：当前阶段的架构与实现规则。

## 迁移阶段历史资料

以下为迁移与等价验证阶段（截至 2026-10-13）的记录，用于查证既有后端行为，不再作为界面设计依据：

- [源码固定版本](SOURCE_PINS.md)
- [功能与源码映射、兼容性审计](docs/source-feature-audit.md)
- [迁移期验证报告](docs/validation-report.md)
- [源码界面布局规格](docs/reference-ui/source-layout-spec.md)
- [Rust 构建性能计划](docs/build-performance-plan.md)
- [云端开发环境指南](docs/cloud-development-environment.md)
- [MCP 接入计划](docs/mcp-integration-plan.md)

## 当前阶段工作方式

- 保留 FileAtlas 0.2 项目基线，优先审查和优化真实任务流；新增功能按用户确认的范围实现，必要的界面调整继续对照当前活动参考。
- 数据库结构、Cordis 服务与事件契约保持兼容，界面重构不改动后端行为语义。
- 每轮界面改动在[界面重构验证记录](docs/redesign-validation.md)登记实测结论和尚未落地的设计项。

## Workspace 组成

- `crates/fileatlas-shell`：组合 Cordis 运行时与 GPUI Kit 宿主，将插件创建的视图挂载到命名插槽，并分发 UI 命令事件。
- `crates/fileatlas-ui-bridge`：提供类型化 UI 贡献、GPUI 视图工厂，以及由 Cordis 生命周期管理的元数据和可执行视图清理。
- `crates/fileatlas-plugin-database`：将原有数据库结构和查询封装为 Cordis 服务插件，分别管理读写连接。
- `crates/fileatlas-plugin-thumbnails`：通过 Cordis 服务提供原版图像缩略图解码与缓存流程。
- `crates/fileatlas-plugin-indexing`：通过 Cordis 服务提供递归扫描、数据库索引、视频海报配对、缩略图回填和目录列表。
- `crates/fileatlas-plugin-watcher`：监视已注册目录，防抖后协调 SQLite 数据，并发送 Cordis 媒体索引变更事件。
- `crates/fileatlas-plugin-file-manager`：实现主文件管理窗口，包括目录/文件内容、筛选、搜索、排列、分组、选择、媒体标签与详情等集成能力。支持最多四个独立浏览栏；每栏拥有独立地址、查询、选择和滚动状态。普通文件预览交给用户已运行的 QuickLook。
- `crates/fileatlas-plugin-columns`：提供主工作区单栏、双栏、四栏布局及激活栏状态；各文件浏览栏拥有独立地址、内容和浏览状态，公共视图操作跟随激活栏。
- `crates/fileatlas-plugin-workspaces`：以版本化 JSON 管理程序启动工作区及每栏地址/视图状态；它不保存照片或媒体记录。
- `crates/fileatlas-plugin-navigation`：提供文件夹/标签树、惰性子目录加载、标签编辑菜单与关联/合并/删除操作、拖放排序、原生文件夹选择、文件夹添加/重扫/移除，以及选择和展开状态恢复。左侧已实现文件夹/标签/扩展独立页，树数据与目录协调语义保持兼容。
- `crates/fileatlas-plugin-tagging`：通过 Cordis 服务提供媒体标签分配、关联标签、祖先关系、移除、清空、合并、重设父级，以及 JPEG IPTC/XMP 写回；FileManager 提供可搜索的标签选择和写回失败重试/忽略。临时样本、队列合并与错误反馈有自动化覆盖，真实相机和编辑器互通仍待验证。
- `crates/fileatlas-plugin-settings` 与 `crates/fileatlas-plugin-session`：提供兼容 FileAtlas 的 JSON 设置/会话结构及 Cordis 持久化服务；设置面板和活动视图的设置/会话应用路径已接入；面板已按设计实现，JSON 模型与存储路径保持兼容。
- `crates/fileatlas-plugin-metadata-export`：通过 Cordis 服务封装原版元数据查询和 JSON 写入；命令、原生 JSON 保存对话框、进度、完成结果和错误反馈已接入，Windows 交互仍待验证。
- `crates/fileatlas-plugin-file-operations`：通过 Cordis 服务提供文件回收、复制、移动、重命名、新建文件夹、打开和定位等单项/批量操作；FileManager 集成目标选择、确认、结果反馈与选择修复。Windows Shell 行为按验证记录确认。
- `crates/fileatlas-plugin-quicklook`：把普通文件交给用户已经运行的 QuickLook。FileAtlas 不负责启动、安装、更新或关闭 QuickLook。
- `crates/fileatlas-plugin-versions` 与 `crates/fileatlas-plugin-version-info`：为文件提供 Lore 版本历史、分支操作和版本元数据；版本信息经共享数据库服务访问，Lore 文件内容存放在独立版本目录。
- `crates/fileatlas-plugin-mcp`：正常启动时在 GUI 进程提供 loopback Streamable HTTP MCP；`--mcp` 保留 stdio，适合隔离调试。工具涵盖窗口/文件浏览状态、界面控制及插件提供的读写操作；访问由 Off、Standard、Full 分级控制，高影响操作仍须逐次确认。具体工具和限制见 [MCP 对接说明](crates/fileatlas-plugin-mcp/PORTS.md)。
- `crates/fileatlas-plugin-dev-test`：仅在开发配置下启用的媒体测试面板，可递归扫描仓库样例媒体，并往隔离的 SQLite 开发库注入/清理 1 千、1 万、10 万条虚拟媒体记录，用于实测文件管理窗口查询、分页、滚动和数据库负载。
- `crates/fileatlas-plugin-history` 与 `crates/fileatlas-plugin-drag-drop`：提供全局撤销/重做及 FileManager 卡片拖出。拖放和 Explorer/OLE 行为按对应 Windows 验证记录确认。
- 其他工作区插件还包括 Everything 搜索、分组、启动台、快捷键、剪贴板、截图、AI 标签建议、元数据导出、设置、标签、导航、索引、缩略图和调试工具；完整成员列表以根目录 `Cargo.toml` 为准。
- `crates/fileatlas-contracts`：存放迁移中的共享领域契约；按功能现状使用，不假设所有数据路径已迁移至该 crate。
- `vendor/photag-source`：保留未修改的上游源码镜像。

## MCP 本机连接

正常 FileAtlas 启动会在 GUI 进程提供 loopback Streamable HTTP MCP；同进程调试也可通过 `--mcp` 将 FileAtlas 作为 stdio 子进程启动，Shell 会同时打开 GPUI 窗口。开发环境可配置如下命令：

```json
{
  "command": "cargo",
  "args": ["+1.97.1", "run", "--locked", "-p", "fileatlas-shell", "-j", "2", "--", "--mcp"],
  "env": {
    "FILEATLAS_DEV_PLUGINS": "dev-test"
  }
}
```

也可以将 `command` 指向已构建的 `fileatlas-shell.exe`，并设置 `args` 为 `["--mcp"]`。设置 `FILEATLAS_DEV_PLUGINS=dev-test` 时，客户端启动的这个 FileAtlas 子进程会启用开发测试面板及 MCP 开发测试工具；AI 可通过 MCP 请求扫描样例目录、注入/清理虚拟数据、轮询任务状态、查询文件管理窗口结果并截取同一窗口。开发测试数据自动放入隔离目录。MCP 权限由 FileAtlas 设置 → 插件设置 → MCP → AI 控制统一管理：关闭档只读，常规控制档允许界面控制和普通增删改，完全控制档额外开放插件标记的高影响操作；高影响操作每次仍需 `confirm: true`。新配置默认关闭，旧 `mcp_allow_mutations` 设置会兼容映射。stdio 客户端连接的是其启动的 FileAtlas 子进程；正常 HTTP MCP 则连接当前已运行的主进程。不要将此服务交给不可信客户端。详细说明见 [MCP 接入与实现状态](docs/mcp-integration-plan.md) 和 [分阶段权限验证](docs/validation-ui/2026-10-19-mcp-access-stages/README.md)。

可用工具和当前限制见 [MCP 对接说明](crates/fileatlas-plugin-mcp/PORTS.md)。

## 开发媒体库规模测试

运行 `start-dev.bat dev-test`，或设置 `FILEATLAS_DEV_PLUGINS=dev-test` 后启动 Shell。开发测试面板可扫描[样例媒体目录](test-data/README.md)，以及注入/清理 1,000、10,000、100,000 条虚拟媒体记录。开发数据库、设置、会话和缩略图缓存均放在 `target/dev-test/`，不会使用日常 FileAtlas 数据库；可用 `FILEATLAS_DEV_TEST_DIR` 指定开发数据目录、`FILEATLAS_DEV_TEST_DB` 单独覆盖数据库路径、`FILEATLAS_TEST_LIBRARY` 指定扫描目录。虚拟行只写数据库，不会生成同等数量的实体文件。插件细节和验证入口见[开发测试插件对接说明](crates/fileatlas-plugin-dev-test/PORTS.md)。

## 构建状态

本机默认 `stable` 是 Rust 1.94.1，但已安装 1.96.0 和 1.97.1；项目要求 Rust ≥1.95。执行 Cargo 时请显式使用已安装的 `cargo +1.97.1 ...`，避免旧默认版本提前拦截。无需重复安装 Rust 或修改全局默认版本。具体命令见[本地工具链说明](docs/rust-toolchain.md)。

迁移阶段的格式检查、锁定模式编译与自动化测试结论见[迁移期验证报告](docs/validation-report.md)，界面改动结论见[界面重构验证记录](docs/redesign-validation.md)。界面重构阶段的每轮改动仍需实际启动 Windows 应用验证；Windows 原生对话框、拖放、Shell 集成、编解码与系统级行为在完成实测前记为未验证。
