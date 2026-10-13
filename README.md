# PhoTag Rust 插件迁移

本仓库正在分阶段将 PhoTag 迁移到 Rust。当前已包含固定版本的 PhoTag 源码、功能审计、Cargo workspace、由 Cordis 管理的 SQLite 插件、具备生命周期清理能力的 UI 注册桥接，以及 GPUI Kit 外壳。**迁移尚未完成**：图库、预览、文件操作和许多产品功能仍未达到原版水平。文件夹索引和缩略图回填已完成初步迁移；设置与会话持久化已通过 Cordis 服务保留 PhoTag JSON 数据结构。这些部分尚未针对真实素材库和 Windows 媒体完成验证。

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
- `crates/photag-plugin-gallery`：提供紧凑查询结果、每页 200 条的窗口化媒体加载、筛选和选择模型，以及绘制缓存缩略图的 GPUI 媒体卡片；支持普通/Ctrl/Shift 选择、批量和键盘 0–5 评分、方向键/Home/End/Ctrl+A/Escape、媒体类型控制、原版八种排序、日期/评分/扩展名/搜索筛选、标签快捷键、拖出、图像/视频预览交接、部分撤销/重做、批量文件操作和所选项目详情。自动视口分页、键盘目标滚动、日期分组、橡皮筋选择、原生日期选择器、标签筛选项及完整交互仍待完成。
- `crates/photag-plugin-navigation`：提供文件夹/标签快照、惰性子目录计数与展开、数据库标签命令、协调扫描与监视的文件夹操作，以及 GPUI 树形侧栏；支持原生文件夹选择、按路径添加、重新扫描/移除、扫描进度和 Cordis 文件夹/标签/路径筛选。标签上下文菜单、拖放编辑和完整视觉一致性仍待迁移。
- `crates/photag-plugin-tagging`：通过 Cordis 服务提供兼容原版的媒体标签分配、关联标签、自动祖先关系、移除、清空、合并、重设父级，以及 JPEG IPTC/XMP sidecar 写回；标签选择器 UI 和用户可见的元数据写入错误提示仍待完成。
- `crates/photag-plugin-settings` 与 `crates/photag-plugin-session`：提供兼容 PhoTag 的 JSON 设置/会话结构及 Cordis 持久化服务；设置面板支持外观、快捷键、标签目标、AI/更新选项，Gallery 筛选和导航选择/展开状态可恢复。会话中的滚动/选择、侧栏尺寸、预览状态和更多设置集成仍未完成。
- `crates/photag-plugin-metadata-export`：通过 Cordis 服务封装原版完整元数据查询和 JSON 写入；命令面板已调用原生 JSON 保存对话框并展示操作状态，Toast 样式和 Windows 运行验证仍待完成。
- `crates/photag-plugin-file-operations`：通过 Cordis 服务提供兼容原版的单项/批量回收、复制、移动、重命名、新建文件夹、打开和定位操作；Gallery 已接入筛选式批量操作和删除确认，文件目标选择对话框及更多操作反馈仍待完成。
- `crates/photag-plugin-mcp`：通过 Cordis 服务向 MCP 客户端提供本机 stdio 接口，包括 PhoTag 窗口截图、图库/媒体读取、预览和视频播放控制、扫描操作、评分/标签修改及受控文件操作。修改工具默认关闭；Windows 截图画面尚待真实窗口验证。
- `crates/photag-plugin-preview`、`crates/photag-plugin-media-playback`、`crates/photag-plugin-history` 与 `crates/photag-plugin-drag-drop`：提供图像预览缩放/平移与 EXIF 信息、FFmpeg 视频帧和 Rodio 音频播放、部分撤销/重做及 Gallery 卡片拖出；完整胶片条/渐进解码、剩余历史操作和 Windows 原生交互验证仍未完成。
- `crates/photag-contracts`：预留稳定的领域契约，仍在迁移中。
- `vendor/photag-source`：保留未修改的上游源码镜像。

## MCP 本机连接

MCP 客户端需要将 PhoTag 作为 stdio 子进程启动，Shell 会同时打开 GPUI 窗口。开发环境可配置如下命令：

```json
{
  "command": "cargo",
  "args": ["run", "--locked", "-p", "photag-shell", "--", "--mcp"]
}
```

也可以将 `command` 指向已构建的 `photag-shell.exe`，并设置 `args` 为 `["--mcp"]`。MCP 服务默认只开放读取、截图、预览和播放控制。若需要修改评分、标签、文件或索引，请在 MCP 子进程环境显式设置 `PHOTAG_MCP_ALLOW_MUTATIONS=1`，并配置 MCP 客户端对修改工具逐次询问用户；破坏性工具还要求 `confirm: true`。不要将此 stdio 服务交给不可信客户端。

可用工具和当前限制见 [MCP 对接说明](crates/photag-plugin-mcp/PORTS.md)。

## 构建状态

当前 Windows MSVC 环境使用 Rust 1.97.1；格式检查、锁定模式 shell 编译检查和本轮八个受影响 crate 的定向测试均已通过。Gallery 开发配置能启动原生窗口，但有两次启动复现 GPUI `Duplicate a11y node id` 崩溃，另一次保持响应；Explorer 拖出、原生对话框和 watcher 仍未完成交互验收。具体命令和结果见 [验证报告](docs/validation-report.md)。
