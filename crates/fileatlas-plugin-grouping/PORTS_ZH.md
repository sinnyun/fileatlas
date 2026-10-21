# Grouping 插件对接

Cordis 插件 `fileatlas.grouping` 发布 `fileatlas/grouping`（`GroupingService`）。Shell 在 FileManager 之前启动它，并在 FileManager 之后停止它。没有平行的生命周期管理器、动态加载、数据库 schema 或新的用户数据路径。模式元数据贡献 `grouping/modes` 归 Cordis UI 桥接层所有；处置（disposal）会同时移除它与该服务。FileManager 提供已有的原生过滤弹出框与 Tools 选择列表适配器，并消费公开的 modes/arrangement/rows 契约。

## 输入与输出

- `GroupingMode::{None, Month, FileType, Modified, Created, Folder, Subfolder}`；`key/parse/label/ALL/is_date` 暴露稳定的字符串、菜单选项与日期语义。未知的已保存模式回退到旧设置。
- `GroupingService::arrange(&self, &mut Vec<MediaSkeleton>, &[MediaGroupingFields], GroupingMode, SortOrder) -> Vec<Option<String>>` 返回与重排后的骨架对齐的分组标签。稳定分组在每组内保留查询顺序。日期分组按最新优先排序，或对 OldestFirst 按最旧优先；不可用的日期仍排在最后。Subfolder 把当前目录的直接分组放在最前，随后是后代路径按字母序；其他分组按字母序排序。None 保留查询顺序。
- Month 使用原始的 `COALESCE(date_taken, modified_at)` 时间戳。Modified 使用被索引的 `modified_at`；Created 读取文件系统 `metadata.created()`，绝不以拍摄/修改时间替代。所有日期模式使用本地日历月 `YYYY-MM`。缺失/离线/不可用的创建时间戳变为 Unknown date。创建元数据在分组/重载时读取，绝不在每一帧卡片渲染时读取；大型/离线的文件系统会延迟这一同步操作，且未经性能认证。
- FileType 对不区分大小写的转大写扩展名分组；没有任何扩展名拥有专属分组。这并不意味着支持预览/解码器。
- Folder 按已注册的数据库根路径分组；Subfolder 按完整的直接父路径分组，包括直接位于根中的文件。完整路径用于消歧重复的目录名；标题对长文本以省略号截断。
- `GroupingService::rows(labels, columns, collapsed) -> Vec<GroupRow>` 计算虚拟分区、计数、列边界与折叠范围。GroupRow 为 FileManager 兼容性保留遗留的以月份命名的渲染字段，但承载任意分组模式。
- `restore_collapsed(keys, mode)` 为旧会话把遗留的英文月份键映射为 YYYY-MM。FileManager 使用已有的会话存储，附带增量可选的 `grouping_mode`；旧的 `group_by_month`、settings `show_date_dividers` 与折叠字段仍可读取。切换模式会清除上一模式的折叠集合；当前模式/折叠在重启时恢复。

## 消费者、事件与失败

FileManager 通过 `DatabaseService` / `queries::get_media_grouping_fields` 获取紧凑的已过滤输入，把它们传给本服务，并按排列后的 ID 拉取丰富记录，从而让 200 行缓存、选择与 Preview 保持一致。已有的 MediaIndexChanged 会刷新快照；本插件不添加事件，也不写入文件。FileManager 通过其已有的错误路径报告数据库/依赖错误。未知的创建日期与无扩展名文件是显式的分区，而非错误。

MCP 使用 FileManagerUiRequest::Grouping/ToggleGroup 以及原始的 MonthGrouping 兼容动作。get_ui_state 从活动的 FileManager 发布 mode、有序的 [label,count] 分组、collapsed_groups 以及窗口计数。变更操作需要已有的 MCP 守卫。生命周期与分组/窗口/过滤测试连同隔离的 Windows MCP 证据位于 docs/validation-ui/2026-10-14-grouping/README.md。

## 当前目录优先（2026-10-14）

`arrange_in_scope(&self, &mut Vec<MediaSkeleton>, &[MediaGroupingFields], GroupingMode, SortOrder, Option<&str>) -> Vec<Option<String>>` 接受活动目录。为兼容起见，`arrange` 以 None 委托给它。Subfolder 把活动目录的直接文件分组排为最前；没有作用域时，所有已注册根的直接分组先于后代分组。缺失的直接文件不会产生空分组。分组键/标签保持为实际的父路径；词法层面的 Windows 路径规范化仅用于排名比较（不进行文件系统 IO）。在每个分组内保留稳定的查询顺序。FileManager 传入 MediaQuery.path_prefix。Windows 证据与限制见 docs/validation-ui/2026-10-14-root-first。

FileManager 的 None 消费者现在提供一个带文件夹卡片的原生直接目录浏览器投影；这不改变 GroupingService::arrange/rows 的顺序契约或源码的递归数据库 API。分组模式仍会排列递归匹配的文件。见 FileManager 的 PORTS 与 docs/validation-ui/2026-10-14-folder-browser。

## 运行时诊断（2026-10-14）

运行时诊断：插件启动与已有的错误/诊断输出现在会发出由 fileatlas.debug 捕获的 tracing 记录；被插桩的公开操作会上报挂钟（wall-clock）span 时长与错误。领域服务契约、事件负载与源码操作顺序保持不变。捕获范围与限制见 fileatlas-plugin-debug/PORTS.md。
