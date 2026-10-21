# 剪贴板工作台 / Clipboard workbench

## Cordis 身份

- 插件 `fileatlas.clipboard`；服务 `fileatlas/clipboard`（`ClipboardService`）。
- 需要共享的 `fileatlas/database` 与 `fileatlas/ui-bridge` 服务。
- 还读取 `fileatlas/settings` 以获取 `Settings.plugin_launch_modes["fileatlas.clipboard"]`。Settings 可以释放并重新生成这个独立工具 Fiber；Cordis 清理会通知剪贴板监视器并移除其命令、面板、快捷键与侧边栏贡献。
- Shell 在 `core`、`all`、MCP 以及显式的 `clipboard` profile 中启用此插件。Navigation 提供其左侧导航栏插槽；现有的 shortcuts 插件注册 Ctrl+Shift+V。
- 剪贴板变更监视使用开源的 `clipboard-master` crate（MIT）；文本与图像的读写使用 `arboard`（MIT OR Apache-2.0，启用 `image-data`）。类型检测的思路参考审阅了 [ZClipboardManager](https://github.com/TheHolyOneZ/ZClipboardManager)（GPL-3.0）；它的 Tauri 应用不是可复用的库，且未复制任何上游代码。

## 数据与操作

- 用户数据存储在 `%LOCALAPPDATA%\\FileAtlas\\clipboard\\clipboard.json`；图像资产位于
  `clipboard/images/<sha256>.png`。在隔离的 MCP profile 中，两者都在所选 profile 目录下。
  JSON 带版本号，先写入同目录的临时文件再替换。首次启动会从共享 SQLite 导入现有的
  剪贴板行/分类，并复制可用的图像资产。JSON 一旦存在即成为权威来源；遗留的 SQLite 行
  保持不变以备恢复，且不再被读取或更新。

- 监视器在每次剪贴板变更时独立尝试捕获非空文本（最大 1 MB）和图像；文本与图像是分开的历史条目。图像以全分辨率 PNG 文件保存，文件名由 SHA-256 命名。临时条目会去重并限制为 500 行；收藏是持久的且永不清理。当历史快照重新加载时，孤立的已生成 PNG 会被移除。
- 捕获只读取剪贴板格式并写入 FileAtlas 属有的 JSON/图像文件。它不调用 `set_text`、`set_image`、`EmptyClipboard` 或任何剪贴板清除 API。选择“复制到系统剪贴板”是唯一会替换当前 OS 剪贴板的历史操作。
- `ClipboardService::use_item` 将某个条目恢复到系统剪贴板，以便用户可以在目标应用中粘贴。UI 还支持搜索、分类创建/过滤/指派、文本编辑、本地收藏、删除临时条目以及清空临时历史。
- `assign_category(id, Option<i64>)` 立即保存成员关系，与内容编辑器相互独立。`rename_category(id, name)` 和 `delete_category(id)` 维护分组；删除分组会将其记录移至未分类而不删除文本。空名称/重复名称/超过 40 字符的名称会被拒绝。
- 公开的 `content::classify(&str) -> ContentKind` 在不修改内容或用户分组的前提下推导出 `Text`、`Link`、`Code` 或可检测的 `Html`。链接要求是带有主机名的单个 HTTP(S)/FTP URL；代码识别有效的 JSON 容器、围栏式代码片段以及选定的多行声明。这是启发式判断，不是语言解析器。图像条目使用显式的图像类型，并渲染缓存的预览/详情图像。
- 现有的 SQLite 剪贴板表作为迁移/恢复数据保留，且不再位于剪贴板的运行时写入路径中。不会引入任何照片 schema 变更。

## UI 与限制

- `clipboard-workbench/panel` 是单个通用浮动面板；左侧导航按钮和 Ctrl+Shift+V 打开它。面板打开期间 `PanelOnly` 会隐藏主工作区。Ctrl+F 仍指派给程序启动面板。
- 默认尺寸为 1160×760，最小 1000×660。视觉参考：`docs/reference-ui/concepts/clipboard-workbench-layout-2026-10-18-v2.png`。深蓝色表面、Lucide 图标、类型/分组导航、卡片徽标、多行编辑器与即时分组控件均已实现。分组编辑位于导航滚动区域之外，因此保存控件保持可见。清空历史与删除分组操作需要 UI 确认。
- 支持的捕获表示形式为 arboard 文本与解码后的位图/图像数据。Windows 文件拖放列表与任意应用特定的剪贴板格式尚未存储。面板可以在当前插件会话内以内存方式暂停捕获；针对私有应用的排除项与持久化的隐私模式仍需设计。
- 当其他进程拥有/锁定剪贴板时，OS 剪贴板访问可能失败；请检查 `fileatlas-clipboard-listener` 日志。全局热键注册可能因冲突而失败；请检查 `fileatlas.shortcuts` 日志。
- 监听器使用专用的 OS 线程，并由 Cordis 清理关闭。实际的 OS 捕获/写入以及跨应用粘贴仍未验证；UI 证据不能确立这些路径。
- MCP 通过 `McpToolRegistry` 贡献 `clipboard.snapshot` 和 `clipboard.add_text`；schema 与 handler 位于本 crate 中，且其唯一命名的 Cordis 生命周期包装器保留路由守卫。snapshot 不接受参数、省略内容并返回类型、修订号、分类与条目元数据；`add_text` 接受 `content`、可选的 `category_id` 和可选的 `favorite`，并直接通过 `ClipboardService` 写入而不触及 OS 剪贴板。旧的无命名空间工具名称已退役。`clipboard.ui` 的选中/过滤详情仍可通过通用的 `get_ui_state` 获取。元数据在 `ChangeGate` 之后使用 `UiSurfaceState`；`refresh::listen` 投递服务变更而无需轮询 IO。

## 本地收藏（2026-10-18）

- `add_favorite(id)` 将现有历史条目保存为持久的本地收藏。`remove_favorite(id)` 永久删除的仅是收藏内容；UI 明确命名此操作并需要确认。不存在“取消星标转为临时”的操作。
- `delete(id)` 会拒绝收藏条目。`clear_unpinned()` 只清除临时行；自动保留策略同样排除收藏。保存编辑或添加重复文本不会使收藏降级。
- 现有的 `ClipboardHistoryRecord::pinned` 与遗留 SQLite 的 `pinned` 值含义为收藏。此前已置顶的记录导入为收藏。JSON 同时保留临时历史与收藏；只有临时历史受保留策略/清除影响。
- 侧边栏使用 收藏 / 本地收藏；加星标的卡片与检查器提供显式的移除操作而不是置顶切换。通用检查器的回收站按钮对收藏条目隐藏。`clipboard.ui.confirm_favorite_delete` 包含待确认的 ID；改变选中项/过滤条件会取消其待确认状态。
# 共享主题（2026-10-18）

视图不再拥有私有调色板。颜色、运行时强调色与选中色来自 UI Bridge 的
`theme`；区块、卡片、导航和操作/图标按钮复用
`components`。内容类型图标与日期格式化仍由功能自己负责。本次
样式变更不改变捕获、持久化、分类与收藏行为。未来的 UI 贡献参见
`docs/ui-theme.md`。

PanelOnly 展示方式使用 `PanelCatalog::open_presented(PANEL,true)` 打开独立的原生工具窗口；WorkspaceAndPanel 保持内嵌的公开面板。捕获/历史存储不变。
