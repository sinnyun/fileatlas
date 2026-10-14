# 不分组：目录浏览与文件夹卡片（2026-10-14）

用户本轮要求：不分组时显示选中目录直属文件和内部子文件夹，增加可双击进入的文件夹卡片，类似系统文件管理器。保留sidebar-pages-preview-2026-10-14-v2及final-v5主界面结构；新增卡片以本轮指示为设计依据。网格和列表均采用普通文件夹图标、名称、选中蓝边；没有为目录伪造媒体ID、评分、标签或缩略图。已查看final-browser-root、folder-list、empty-folder截图。

## 最终行为

- 不分组＋选中目录：文件夹在前，后面只显示当前目录直属的索引文件。文件夹来自真实文件系统，包括空目录和.hidden；文件沿用数据库筛选与排序、200条完整记录缓存。目录名称排序（NameDesc时反向），其余文件排序选项不伪造目录时间/评分。
- 未选目录时为登记根目录卡片；无目录范围的全库搜索和标签视图继续提供原递归文件结果。目录内搜索匹配直属文件和子文件夹名称/路径。其他分组模式保持递归展示。
- 单击文件夹选择，双击或Enter进入；上级按钮/Alt+Up返回，根目录再上一级回登记根列表。刷新按钮/F5重新读取目录。路径和布局复用原session保存；文件夹选择是临时UI状态。
- 导航通过公开NavigationService请求队列进入；侧栏显示当前根路径选中态，并展开所进入目录的祖先。文件选择/预览/框选只用实际文件ID；当前目录批量来源使用直属文件条件，根列表未选目录时拒绝该来源。WholeLibrary来源仍明确指整个资料库。
- 当前文件夹卡片仅提供浏览选择，不参加媒体标签/评分/文件批量动作。目录读取错误会显示，并可刷新重试；不会把失败当作正常空目录。

## 数据与兼容

`MediaQuery.direct_directory: Option<String>`是内存查询字段，公共filter builder用参数化精确目录前缀＋后续无路径分隔符条件过滤直属文件；目录名中的百分号/下划线不作为LIKE通配符。SQLite schema及普通PhoTag资料路径未更改。`GalleryService::load/load_page`仍是原递归数据API；`load_grouped(..., None)`供当前显示投影使用。Source对应：vendor/photag-source/src/db/queries.rs build_filters:145与path_prefix条件，src/scan.rs list_subdirectories:466及app.rs get_subdirs:819调用；上游没有这种文件夹卡片视图。目录列表沿用上一轮已获要求的全文件/隐藏目录策略；新增IndexingService::try_list_subdirectories报告错误，原best-effort list_subdirectories的跳过错误行为保留。

## 环境、fixture与命令

Windows MSVC/Rust1.97.1，150%DPI，1942×1256；已检查rustup toolchain list。EXE target/debug/photag-shell-folder-browser.exe，独立输出避免覆盖现有运行窗口。MCP与GPUI同一个--mcp子进程；没有通用computer-use或OS输入注入。普通PhoTag数据库/设置/会话和用户既有窗口未修改。

隔离profile：target/mcp-folder-browser-validation。fixture含250个普通文件：根目录2（真实PNG及TXT），.hidden1，01-Photos直属PNG1与nested/deep.txt1，03-Many的245TXT；02-Empty为空。fixture.py只生成target/mcp-folder-browser-fixture内的固定样本。

```powershell
python docs/validation-ui/2026-10-14-folder-browser/fixture.py
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-database -p photag-plugin-indexing -p photag-plugin-i18n -j 2
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-navigation -j 2
cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-folder-browser.exe
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-folder-browser.exe --profile target/mcp-folder-browser-validation --plan docs/validation-ui/2026-10-14-folder-browser/plan.json --allow-mutations
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-folder-browser.exe --profile target/mcp-folder-browser-validation --plan docs/validation-ui/2026-10-14-folder-browser/restart-plan.json
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-folder-browser.exe --profile target/mcp-folder-browser-empty-restart --plan docs/validation-ui/2026-10-14-folder-browser/empty-restart-plan.json
```

最后的empty-restart profile通过SQLite只读backup复制测试库，session将selected_path设为已有02-Empty、gallery_scroll设0；只为验证空列表初始滚动恢复。两次重启均未开启MCP修改权限；后台生产Watcher仍按原逻辑核对这个隔离样本库。

## 证据

- fmt、locked Shell check和独立Shell完整编译通过，仅rustc显式-o的多输出提示。相关自动测试36项通过：Gallery19、Database5、Indexing3、i18n4、Navigation5；原FFmpeg测试1项ignored，不计通过。
- 新回归验证直属245文件/两个目录、空目录、缺失目录错误、根列表、标签和全库搜索例外、目录名_%字面匹配、跨页ID一致，以及文件夹/文件共用一行时的独立索引范围。i18n新增计数参数覆盖中英文。
- 82步最终构建同进程MCP＋普通目录/空目录两次只读重启，共84步；报告report.json、restart-report.json、empty-restart-report.json。报告计数/路径/选择/缓存/预览另作断言复查。
- 真实GPUI单击选中文件夹（selected_directory有值、selected_media_ids为空）；网格双击进入.hidden、Photos、nested，列表双击进入.hidden；点击上级按钮返回当前根。MCP新增pointer_double_down/up产生click_count=2的同进程GPUI事件，验证命中和双击handler，不宣称OS双击时间阈值测试。
- 根显示4目录＋2文件，Photos显示nested＋1直属图片，Empty为0/0；Many245文件的末页offset45/cache200。文件夹/文件混排框选正确得到photo.png的媒体ID，双击图片打开Preview。
- 原分组仍能查询250文件；无范围搜索photo.png仍得到1文件，清除后恢复根目录卡片。非法跳过可见子目录和无上级请求均被拒绝。
- 重启恢复None/grid及根目录4＋2；空目录重启无崩溃并显示“此文件夹为空”。

## 边界

键盘Enter/Alt+Up/F5已接入，未单独通过MCP注入键盘验收；鼠标单双击、上级和刷新按钮已验。新增空目录且未产生媒体行变更时可用刷新/F5更新；不宣称所有目录事件均自动刷新。真实权限拒绝、网络目录/海量子目录/链接目录、原生拖出文件夹及目录复制移动删除不是本轮验收。文件仍由生产索引服务提供，扫描进行时沿用原进度反馈；不是绕过索引的系统资源管理器替代品。原生文件对话框/媒体codec等F0未验项仍保留。
