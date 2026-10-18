# PhoTag Rust 版

本仓库是 PhoTag 的 Rust 版本，运行在 Windows 桌面，由 Cordis 管理功能插件、GPUI Kit 提供原生界面。**当前界面已确认为开发基线，下一阶段转向功能操作优化与功能扩展**：主窗口采用两侧分页/固定预览 v2，其他页面沿用定稿 v5，不再追求与原版 PhoTag（egui）界面逐控件对齐；数据模型、扫描、索引、标签、文件操作等后端行为继续沿用 `vendor/photag-source/` 的语义。

## 开发基线与下一阶段

- [2026-10-14 开发基线](docs/development-baseline-2026-10-14.md)：Git标记 `dev-baseline-2026-10-14`，固定布局、架构、验证证据和已知边界。
- [功能操作优化与功能扩展计划](docs/functional-development-plan.md)：从F0真实任务流审查开始，再修操作问题、实现用户确认的新功能。
- [当前实际截图与验收摘要](docs/validation-ui/2026-10-14-baseline/README.md)。

## 当前设计依据

- [主窗口两侧分页/固定预览 v2](docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.md)：当前主窗口面板结构。
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

- 保留已确认的界面基线，优先审查和优化真实任务流；新增功能按用户确认的范围实现，必要的界面调整继续对照当前设计。
- 数据库结构、Cordis 服务与事件契约保持兼容，界面重构不改动后端行为语义。
- 每轮界面改动在[界面重构验证记录](docs/redesign-validation.md)登记实测结论和尚未落地的设计项。

## Workspace 组成

- `crates/photag-shell`：组合 Cordis 运行时与 GPUI Kit 宿主，将插件创建的视图挂载到命名插槽，并分发 UI 命令事件。
- `crates/photag-ui-bridge`：提供类型化 UI 贡献、GPUI 视图工厂，以及由 Cordis 生命周期管理的元数据和可执行视图清理。
- `crates/photag-plugin-database`：将原有数据库结构和查询封装为 Cordis 服务插件，分别管理读写连接。
- `crates/photag-plugin-thumbnails`：通过 Cordis 服务提供原版图像缩略图解码与缓存流程。
- `crates/photag-plugin-indexing`：通过 Cordis 服务提供递归扫描、数据库索引、视频海报配对、缩略图回填和目录列表。
- `crates/photag-plugin-watcher`：监视已注册目录，防抖后协调 SQLite 数据，并发送 Cordis 媒体索引变更事件。
- `crates/photag-plugin-gallery`：提供紧凑查询结果、每页 200 条的窗口化媒体加载、可视行渲染、筛选与选择、月份分组、橡皮筋选择、命名标签筛选、统计、卡片操作、批量文件操作、撤销/重做、媒体详情和预览交接。界面以当前开发基线为准，数据与操作行为保持兼容。
- `crates/photag-plugin-navigation`：提供文件夹/标签树、惰性子目录加载、标签编辑菜单与关联/合并/删除操作、拖放排序、原生文件夹选择、文件夹添加/重扫/移除，以及选择和展开状态恢复。左侧已实现文件夹/标签/扩展独立页，树数据与目录协调语义保持兼容。
- `crates/photag-plugin-tagging`：通过 Cordis 服务提供媒体标签分配、关联标签、祖先关系、移除、清空、合并、重设父级，以及 JPEG IPTC/XMP 写回；Gallery 提供可搜索的标签选择和写回失败重试/忽略。临时样本、队列合并与错误反馈有自动化覆盖，真实相机和编辑器互通仍待验证。
- `crates/photag-plugin-settings` 与 `crates/photag-plugin-session`：提供兼容 PhoTag 的 JSON 设置/会话结构及 Cordis 持久化服务；设置面板和活动视图的设置/会话应用路径已接入；面板已按设计实现，JSON 模型与存储路径保持兼容。
- `crates/photag-plugin-metadata-export`：通过 Cordis 服务封装原版元数据查询和 JSON 写入；命令、原生 JSON 保存对话框、进度、完成结果和错误反馈已接入，Windows 交互仍待验证。
- `crates/photag-plugin-file-operations`：通过 Cordis 服务提供单项/批量回收、复制、移动、重命名、新建文件夹、打开和定位操作；Gallery/Preview 集成目标选择、确认、批量结果、选择修复和胶片条跟进。Windows Shell 行为仍待实机验证。
- `crates/photag-plugin-mcp`：通过 Cordis 服务向 MCP 客户端提供本机 stdio 接口，包括 PhoTag 窗口截图、图库/媒体读取、预览和视频播放控制、扫描操作、评分/标签修改及受控文件操作。修改工具默认关闭；真实Windows截图、GPUI输入与布局已通过隔离profile验证，系统集成未覆盖项见基线文档。
- `crates/photag-plugin-dev-test`：仅在开发配置下启用的媒体测试面板，可递归扫描仓库样例媒体，并往隔离的 SQLite 开发库注入/清理 1 千、1 万、10 万条虚拟媒体记录，用于实测图库查询、分页、滚动和数据库负载。
- `crates/photag-plugin-preview`、`crates/photag-plugin-media-playback`、`crates/photag-plugin-history` 与 `crates/photag-plugin-drag-drop`：提供图像预览与渐进解码、胶片条、音频/视频播放、源码支持的撤销/重做，以及 Gallery 卡片拖出。媒体设备、Explorer/OLE 和 Windows 原生交互仍待验证。
- `crates/photag-contracts`：预留稳定的领域契约，仍在迁移中。
- `vendor/photag-source`：保留未修改的上游源码镜像。

## MCP 本机连接

MCP 客户端需要将 PhoTag 作为 stdio 子进程启动，Shell 会同时打开 GPUI 窗口。开发环境可配置如下命令：

```json
{
  "command": "cargo",
  "args": ["+1.97.1", "run", "--locked", "-p", "photag-shell", "-j", "2", "--", "--mcp"],
  "env": {
    "PHOTAG_DEV_PLUGINS": "dev-test"
  }
}
```

也可以将 `command` 指向已构建的 `photag-shell.exe`，并设置 `args` 为 `["--mcp"]`。设置 `PHOTAG_DEV_PLUGINS=dev-test` 时，客户端启动的这个 PhoTag 子进程会启用开发测试面板及 MCP 开发测试工具；AI 可通过 MCP 请求扫描样例目录、注入/清理虚拟数据、轮询任务状态、查询图库结果并截取同一窗口。开发测试数据自动放入隔离目录。MCP 权限由 PhoTag 设置 → 插件设置 → MCP → AI 控制统一管理：关闭档只读，常规控制档允许界面控制和普通增删改，完全控制档额外开放插件标记的高影响操作；高影响操作每次仍需 `confirm: true`。新配置默认关闭，旧 `mcp_allow_mutations` 设置会兼容映射。stdio 客户端连接的是其启动的 PhoTag 子进程；正常 HTTP MCP 则连接当前已运行的主进程。不要将此服务交给不可信客户端。详细说明见 [MCP 接入与实现状态](docs/mcp-integration-plan.md) 和 [分阶段权限验证](docs/validation-ui/2026-10-19-mcp-access-stages/README.md)。

可用工具和当前限制见 [MCP 对接说明](crates/photag-plugin-mcp/PORTS.md)。

## 开发媒体库规模测试

运行 `start-dev.bat dev-test`，或设置 `PHOTAG_DEV_PLUGINS=dev-test` 后启动 Shell。开发测试面板可扫描[样例媒体目录](test-data/README.md)，以及注入/清理 1,000、10,000、100,000 条虚拟媒体记录。开发数据库、设置、会话和缩略图缓存均放在 `target/dev-test/`，不会使用日常 PhoTag 数据库；可用 `PHOTAG_DEV_TEST_DIR` 指定开发数据目录、`PHOTAG_DEV_TEST_DB` 单独覆盖数据库路径、`PHOTAG_TEST_LIBRARY` 指定扫描目录。虚拟行只写数据库，不会生成同等数量的实体文件。插件细节和验证入口见[开发测试插件对接说明](crates/photag-plugin-dev-test/PORTS.md)。

## 构建状态

本机默认 `stable` 是 Rust 1.94.1，但已安装 1.96.0 和 1.97.1；项目要求 Rust ≥1.95。执行 Cargo 时请显式使用已安装的 `cargo +1.97.1 ...`，避免旧默认版本提前拦截。无需重复安装 Rust 或修改全局默认版本。具体命令见[本地工具链说明](docs/rust-toolchain.md)。

迁移阶段的格式检查、锁定模式编译与自动化测试结论见[迁移期验证报告](docs/validation-report.md)，界面改动结论见[界面重构验证记录](docs/redesign-validation.md)。界面重构阶段的每轮改动仍需实际启动 Windows 应用验证；Windows 原生对话框、拖放、Shell 集成、编解码与系统级行为在完成实测前记为未验证。
