# PhoTag Rust 插件迁移

本仓库正在分阶段将 PhoTag 迁移到 Rust。当前已包含固定版本的 PhoTag 源码、功能审计、Cargo workspace、由 Cordis 管理的功能插件、GPUI Kit 原生界面以及迁移期的 MCP 接口。**迁移和等价验证尚未全部完成**。截至 2026-10-13，最新功能审计记录了图库、导航、预览、标签、设置、导出、文件操作、AI、播放和更新等已迁入的代码行为；仍有行为规格、回归覆盖、完整媒体样本和 Windows 实机验证缺口。请以[源码功能审计](docs/source-feature-audit.md)为当前状态依据；更早的验证记录是当时的历史快照，不代表当前功能清单。

- [源码固定版本](SOURCE_PINS.md)
- [功能与源码映射、兼容性审计](docs/source-feature-audit.md)
- [验证报告](docs/validation-report.md)
- [Rust 构建性能计划](docs/build-performance-plan.md)
- [云端开发环境指南](docs/cloud-development-environment.md)
- [MCP 接入计划](docs/mcp-integration-plan.md)
- [Codex 项目指引](AGENTS.md)

## Workspace 组成

- `crates/photag-shell`：组合 Cordis 运行时与 GPUI Kit 宿主，将插件创建的视图挂载到命名插槽，并分发 UI 命令事件。
- `crates/photag-ui-bridge`：提供类型化 UI 贡献、GPUI 视图工厂，以及由 Cordis 生命周期管理的元数据和可执行视图清理。
- `crates/photag-plugin-database`：将原有数据库结构和查询封装为 Cordis 服务插件，分别管理读写连接。
- `crates/photag-plugin-thumbnails`：通过 Cordis 服务提供原版图像缩略图解码与缓存流程。
- `crates/photag-plugin-indexing`：通过 Cordis 服务提供递归扫描、数据库索引、视频海报配对、缩略图回填和目录列表。
- `crates/photag-plugin-watcher`：监视已注册目录，防抖后协调 SQLite 数据，并发送 Cordis 媒体索引变更事件。
- `crates/photag-plugin-gallery`：提供紧凑查询结果、每页 200 条的窗口化媒体加载、可视行渲染、筛选与选择、月份分组、橡皮筋选择、命名标签筛选、统计、卡片操作、批量文件操作、撤销/重做、媒体详情和预览交接。Windows 交互和视觉一致性仍待实机验证。
- `crates/photag-plugin-navigation`：提供文件夹/标签树、惰性子目录加载、标签编辑菜单与关联/合并/删除操作、拖放排序、原生文件夹选择、文件夹添加/重扫/移除，以及选择和展开状态恢复。Windows 交互和视觉一致性仍待实机验证。
- `crates/photag-plugin-tagging`：通过 Cordis 服务提供媒体标签分配、关联标签、祖先关系、移除、清空、合并、重设父级，以及 JPEG IPTC/XMP 写回；Gallery 提供可搜索的标签选择和写回失败重试/忽略。临时样本、队列合并与错误反馈有自动化覆盖，真实相机和编辑器互通仍待验证。
- `crates/photag-plugin-settings` 与 `crates/photag-plugin-session`：提供兼容 PhoTag 的 JSON 设置/会话结构及 Cordis 持久化服务；设置面板和活动视图的设置/会话应用路径已接入。完整回归覆盖及 Windows 视觉验证见[功能审计](docs/source-feature-audit.md)。
- `crates/photag-plugin-metadata-export`：通过 Cordis 服务封装原版元数据查询和 JSON 写入；命令、原生 JSON 保存对话框、进度、完成结果和错误反馈已接入，Windows 交互仍待验证。
- `crates/photag-plugin-file-operations`：通过 Cordis 服务提供单项/批量回收、复制、移动、重命名、新建文件夹、打开和定位操作；Gallery/Preview 集成目标选择、确认、批量结果、选择修复和胶片条跟进。Windows Shell 行为仍待实机验证。
- `crates/photag-plugin-mcp`：通过 Cordis 服务向 MCP 客户端提供本机 stdio 接口，包括 PhoTag 窗口截图、图库/媒体读取、预览和视频播放控制、扫描操作、评分/标签修改及受控文件操作。修改工具默认关闭；Windows 截图画面尚待真实窗口验证。
- `crates/photag-plugin-dev-test`：仅在开发配置下启用的媒体测试面板，可递归扫描仓库样例媒体，并往隔离的 SQLite 开发库注入/清理 1 千、1 万、10 万条虚拟媒体记录，用于实测图库查询、分页、滚动和数据库负载。
- `crates/photag-plugin-preview`、`crates/photag-plugin-media-playback`、`crates/photag-plugin-history` 与 `crates/photag-plugin-drag-drop`：提供图像预览与渐进解码、胶片条、音频/视频播放、源码支持的撤销/重做，以及 Gallery 卡片拖出。媒体设备、Explorer/OLE 和 Windows 原生交互仍待验证。
- `crates/photag-contracts`：预留稳定的领域契约，仍在迁移中。
- `vendor/photag-source`：保留未修改的上游源码镜像。

## MCP 本机连接

MCP 客户端需要将 PhoTag 作为 stdio 子进程启动，Shell 会同时打开 GPUI 窗口。开发环境可配置如下命令：

```json
{
  "command": "cargo",
  "args": ["run", "--locked", "-p", "photag-shell", "--", "--mcp"],
  "env": {
    "PHOTAG_DEV_PLUGINS": "dev-test"
  }
}
```

也可以将 `command` 指向已构建的 `photag-shell.exe`，并设置 `args` 为 `["--mcp"]`。设置 `PHOTAG_DEV_PLUGINS=dev-test` 时，客户端启动的这个 PhoTag 子进程会启用开发测试面板及 MCP 开发测试工具；AI 可通过 MCP 请求扫描样例目录、注入/清理虚拟数据、轮询任务状态、查询图库结果并截取同一窗口。开发测试数据自动放入隔离目录。MCP 服务默认只开放读取、截图、预览、播放控制和隔离的开发测试操作。若需要修改日常图库的评分、标签、文件或索引，请在 MCP 子进程环境显式设置 `PHOTAG_MCP_ALLOW_MUTATIONS=1`，并配置 MCP 客户端对修改工具逐次询问用户；破坏性工具还要求 `confirm: true`。不要将此 stdio 服务交给不可信客户端。

可用工具和当前限制见 [MCP 对接说明](crates/photag-plugin-mcp/PORTS.md)。

## 开发媒体库规模测试

运行 `start-dev.bat dev-test`，或设置 `PHOTAG_DEV_PLUGINS=dev-test` 后启动 Shell。开发测试面板可扫描[样例媒体目录](test-data/README.md)，以及注入/清理 1,000、10,000、100,000 条虚拟媒体记录。开发数据库、设置、会话和缩略图缓存均放在 `target/dev-test/`，不会使用日常 PhoTag 数据库；可用 `PHOTAG_DEV_TEST_DIR` 指定开发数据目录、`PHOTAG_DEV_TEST_DB` 单独覆盖数据库路径、`PHOTAG_TEST_LIBRARY` 指定扫描目录。虚拟行只写数据库，不会生成同等数量的实体文件。插件细节和验证入口见[开发测试插件对接说明](crates/photag-plugin-dev-test/PORTS.md)。

## 构建状态

当前验证结论及准确命令见[验证报告](docs/validation-report.md)。格式检查和锁定模式 shell 编译检查已在最新迁移整合记录中通过；迁移期间的 UI 启动观察和较早失败记录保留为历史。Windows 交互、真实媒体与文件系统行为仍需集中实机验证。
