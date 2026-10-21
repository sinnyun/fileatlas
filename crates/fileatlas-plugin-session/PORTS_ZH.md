# Session 插件对接 / 会话数据对接

当前的浏览器运行时取代了下文较旧的已索引库描述；参见末尾的 2026-10-15 更新以及 `docs/validation-ui/2026-10-15-browser/README.md`。

状态：被锁定的 PhoTag `session.json` schema 与 Cordis 服务已完成迁移。FileManager 的过滤、所选媒体 ID、滚动偏移、缩略图尺寸与月份分组设置在视图创建时恢复，并在选择/过滤变化、滚动变化以及视图拆卸时保存。Navigation 恢复文件夹/路径选择、已展开目录与已展开标签 ID。shell 恢复并保存侧边栏宽度。Preview 重新打开最后一个图片路径，并在该路径仍存在时恢复其变换、缩放、平移、缩放锚点与信息可见性。

## 生命周期与存储

- Cordis 插件：`fileatlas.session`；服务名：`fileatlas/session`（`SessionService`）。
- 无服务依赖。默认文件为 `%LOCALAPPDATA%\FileAtlas\session.json`；该路径可被覆盖。FileManager 依赖本服务。
- `SessionState` 保留 FileAtlas 字段名、serde 默认值、排序顺序、媒体类型，以及 tag/extension 点击模式枚举字符串。该模型覆盖所选文件夹/路径/媒体 ID、预览路径与可选的预览变换字段、侧边栏 tab/宽度、展开节点、file_manager 滚动、过滤、缩略图尺寸、月份分组、折叠月份以及下拉展开。新字段带 serde 默认值，以兼容已有的 session 文件。

## 数据接口

- `snapshot()` 返回当前状态的一个克隆。`replace(session)` 更新内存中的状态，并将美化 JSON 写入原路径。缺失或无效的文件加载为默认会话，与源码行为一致。
- 复用数据库 `SortOrder`，从而让序列化后的顺序名与 file_manager 查询契约保持一致。

## 调试

对照 `vendor/photag-source/src/session.rs` 检查 `session.json` 的键与值。FileManager 在滚动变化与视图拆卸之后持久化其当前选择与像素滚动偏移。Shell 在侧边栏拖拽结束时持久化宽度。Preview 为匹配的源路径持久化/恢复其视图状态；Navigation 从自由形式的 sidebar_tab 字段恢复活动的侧边栏页面，接受旧的 Folders/Tags 值。进程突然终止仍可能在下一个保存点之前丢失最近的 file_manager 或侧边栏变化。

## 检视器宽度兼容性（2026-10-14）

SessionState.inspector_width:f32 使用 serde(default)；零值使用 300pt。已有的 sidebar_width、selection 以及所有源字段保留其名称/语义。Shell 在分隔条 mouse-up 时保存两个宽度；不执行任何用户文件迁移/重置。兼容性测试加载不含 inspector_width 的旧 JSON，保留 folder 7/media 12/Tags，并往返一次 350pt 的右侧宽度。MCP 重启在一个隔离 profile 中恢复了实际拖拽的宽度 304/340。

## 增量式 file_manager 布局偏好（2026-10-14）

`SessionState.file_manager_list_layout: bool` 的 serde 默认为 false；较旧的 FileAtlas JSON 仍可读取并保留网格默认值。FileManager 随已有的完整会话对象恢复并保存它。兼容性回归现在会连同遗留的 folder/selection/sidebar 字段一起往返一个列表偏好。SQLite schema 与其他已有路径不变。

## 分组兼容性（2026-10-14）

`SessionState.grouping_mode: Option<String>` 对于旧 JSON 的 serde 默认为 None；Grouping 消费者解析 none/month/file_type/modified/created/folder/subfolder。已有的 group_by_month 与 collapsed_months 字段保持兼容。FileManager 首先使用新模式，回退到原有的 settings/session 布尔值，并映射旧的英文月份折叠标签。当前模式的折叠状态会被持久化；切换模式会清除它。兼容性测试保留选择并往返 file_type。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与已有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报挂钟（wall-clock）span 时长与错误。领域服务契约、事件负载与源码操作顺序保持不变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。


## 浏览器运行时更新 — 2026-10-15

增量字段：`show_descendants:bool`（false）、`thumbnails_disabled:bool`（false）、`before_descendants_grouping:Option<String>`、`selected_file_paths:Vec<String>`。浏览器选择在目录处理完成之后解析路径；旧的 selected_media_ids 仍保持 JSON 兼容，但在浏览器缓存重启之间不被信任。偏好保留已接受的视图布局。


## 评审修正 — 2026-10-16

增量式的 serde 默认 `sidebar_collapsed_sections:HashSet<String>` 与 `sidebar_collapsed_groups:HashSet<String>` 恢复 Explorer 的分区/分组折叠。`SessionService::replace` 跳过相等的状态，在持久化期间持有写锁，并在失败时保持其上一次快照不变。这防止被动地保存相同内容时生成被监视的文件事件。测试覆盖无操作、保存失败与往返。源码对照：vendor/photag-source/src/session.rs；其直接的状态保存仍是历史参考，而 Cordis 服务如今拥有有序的发布。
