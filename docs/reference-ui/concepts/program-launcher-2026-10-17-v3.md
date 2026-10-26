# 程序启动台：桌面图标布局（2026-10-17 v3）

用户提供的桌面快捷方式截图覆盖前两版宽卡片设计。本版使用内置 imagegen 工具生成概念图 `program-launcher-2026-10-17-v3.png`，参考图仅定义视觉方向，数据与程序图标来自 Windows。

- 左侧分类栏宽218逻辑像素，分类图标、名称与数量分列。
- 中部工具栏为搜索/添加/扫描，下一行为分类标题、匹配数量、排序字段和升降序。
- 网格为112×132逻辑像素的图标单元，64像素图标，名称居中最多两行，间距12像素。默认透明背景，悬停浅亮底，选中为青色细边框和淡青底；失效程序名称减淡。
- 左键选中，双击启动，右键复用 GPUI Kit PopupMenu；移动分类为二级菜单。
- 右侧250逻辑像素信息区，固定预留空间以保证双击时图标位置稳定。未选中显示提示；选中后提供名称、分类、隐藏草稿及统一保存。增加“程序说明”单句只读行，位于显示名称后；安装日期/系统登记大小/来源/路径只读，较长内容在信息区内滚动查看。
- 名称、安装时间、大小支持升降序；未知值始终排在末尾，同值使用名称和ID稳定排序。
- 右键菜单对有 Windows 卸载命令的已安装程序显示“卸载”，执行前确认；安装器/卸载器本身、卸载快捷方式和帮助/许可/教程入口不作为启动项目。
- 深灰底、细分隔线、青色强调；沿用宿主浮动面板的标题栏、拖动、缩放和生命周期。

## 批量管理扩展概念（2026-10-26）

批量管理保留左侧分类导航和外层浮窗；中间程序区切换为可搜索的紧凑列表，列为勾选框、程序名、当前分类、显示状态和路径。顶部显示全选与所选数量；右侧检查器切换为批量修改，提供目标分类、名称前缀/后缀和显示/隐藏/保持不变选项。底部显示所选数量并提供清空选择与应用。选择以稳定程序 ID 保存；筛选变化不清除选择。应用为单次 JSON 持久化事务，名称只增加非空前后缀，空分类/可见性选项代表不修改。沿用现有主题组件、间距和生命周期。

## 生成提示

Use case ui-mockup. High fidelity implementation reference of a Chinese desktop application launcher floating panel ONLY, landscape 1500x950, refined flat charcoal GPUI native design with cyan accent, no gradients. User wants desktop shortcut style tiles: large real application icon and text name below, NO big bordered card body, NO launch button on tiles, NO category/status/path displayed on each tile. Thin host titlebar 程序启动台. Left fixed sidebar 190px, header 启动台 and small subtitle 个人程序工作台. All programs 398 selected, Hidden tools 0, category rows with glyph icons names and right-aligned numbers, 分类, 常用, 开发工具, 设计创作, 办公软件, 通讯社交, 系统工具, 其他; bottom add category controls. Middle flexible browser area toolbar with bordered 搜索程序名称或路径 search field, 添加程序, 扫描程序. Below 全部程序 and 398 个程序; right-aligned sort dropdown 名称 and arrow 升序; dropdown options 名称 / 安装时间 / 大小. The main grid resembles Windows desktop icon arrangement with 5 columns of 112x122 tiles: generous 56px to 64px colorful real program icons with names centered below wrapping to two lines, uniform 16px gaps. Programs VS Code Photoshop PowerShell WeChat Blender 7-Zip Chrome Notion WPS Office. One tile Photoshop selected with subtle translucent cyan rectangle and 1px blue outline, one tile hover muted surface. A context menu opened on Blender contains 启动, 重命名, 移动分类 with submenu arrow, 隐藏, 卸载, 移除. Right side narrow 260px information inspector with border separator and title 程序信息 with close X, selected large Photoshop icon 64px and editable name Adobe Photoshop 2025. Fields 显示名称 editable input, 分类 dropdown 设计创作, 隐藏 switch; section 安装信息 with 安装时间 2025-06-18, 大小 2.4 GB (系统登记), 来源 已安装程序; 路径 wrapping at bottom. Explicit primary 保存修改, secondary 启动程序. Small hint 单击选择 · 双击启动 · 右键管理 along bottom. Footer total programs and green scan complete. Match modern Windows shortcut affordances, clear alignment and typography, 18px padding, 10px corner controls, faint separators. Keep this feasible using standard flex layout and icon images.
