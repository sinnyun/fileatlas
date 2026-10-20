# 右侧信息面板完整状态 v2（2026-10-16）

后续用户修订以[统一预览与集中确认](right-inspector-unified-edit-2026-10-16.md)为准：历史恢复完整预览，标签编辑改为单次确认。

概念图：[`right-inspector-complete-2026-10-16-v2.png`](right-inspector-complete-2026-10-16-v2.png)。使用内置 image_gen 生成，作为此前右栏概念的补全版。原程序深色稿与历史树稿继续定义局部结构。本稿明确五种完整状态：程序、标签、文件夹、文件信息、文件历史。

## 实现约束

- 石墨面板、深色身份卡与内容卡；16 px 内边距、8–12 px 分区间隙、浅色细分隔线。标题、标签、数值和帮助文本分层。
- 程序页为身份卡、2×2 概览、当前位置、绿色就绪状态和提示；不展示具体标签名、媒体库容量、扫描任务。
- 标签页展示实际名称、色值、父级、关联数和直接子标签数；名称输入及保存、颜色色板、可见性开关和子项操作菜单均连接已有 Navigation 服务。
- 文件夹页使用当前浏览快照的文件结果和子文件夹数；未知计数展示待打开状态，不伪造为0，不沿用索引/扫描面板。
- 文件页保留真实缩略图、评分、彩色标签和完整可滚动信息/EXIF/路径；缺失字段不伪造。
- 历史页采用紧凑文件身份栏、双按钮、分支选择、主线/弯曲侧分支、固定尺寸缩略图与选中版本详情。正常启动保留当前工作副本；用户要求的完整模拟分支仅在隔离 dev-test 且 `FILEATLAS_DEMO_INSPECTOR=1` 时启用。模拟创建/切换只更新内存，底部明确提示；历史快照后端仍按用户前述决定暂缓。

## 原始生成提示

```text
Use case: ui-mockup. Create a precise high fidelity desktop UI design reference sheet for FileAtlas, a local file/photo browser. Five adjacent tall inspector panels on a dark charcoal background, flat native desktop UI, crisp readable Simplified Chinese typography. Every panel same width approximately 340 logical pixels, generous 16 px panel padding, 12 px vertical gaps, 1px subtle light gray dividers, 8px rounded dark inset cards. Palette graphite #2b2f34 panel, #1e2126 cards, white #e9ebee titles, #a9aeb5 labels, blue #399bec accents, yellow folders, colored tags. No decorative outer headings. Panel 1 header 程序信息 with close X. Identity card blue image icon FileAtlas 本地照片浏览与管理工具. 浏览概览 header and exact 2x2 metrics cards 当前结果 18, 可见文件夹 5, 标签数 8, 版本 0.1.0. 当前浏览位置 bordered icon card 收藏集合, 程序状态 green dot 就绪 and 可见子文件夹 5, 使用提示 bordered info card. Never show specific tag names in this program panel, no media library or disk capacity or scan status. Panel 2 header 标签信息, tall identity red tile with white tag icon 旅行, 18 个文件, 所有标签 > 旅行. 标签详情 icon rows 标签名称 旅行, 颜色 red swatch #E36B6B, 父级标签 —, 媒体数量 18, 子标签 5. 编辑标签 inset card containing labeled editable 名称 input 旅行, full blue 保存名称 button, seven round colored swatches with red ring selected, 在缩略图上显示 toggle enabled, 更多标签设置 link. 子标签 heading with plus, five separated colored rows 日本 6 blue, 欧洲 4 pink, 城市 3 orange, 自然 3 green, 美食 2 purple each right aligned count and visible ellipsis button. Panel 3 header 文件夹信息, yellow folder identity 2026-秋日旅行 and D:\Photos\2026-秋日旅行. 文件夹详情 icon rows 文件夹名称, 当前结果 18, 子文件夹 5. 位置 card full readable wrapped path with 复制路径 button. 浏览状态 green 就绪. 操作 full width 打开文件夹 and 刷新 buttons. Do not invent indexed library or scan concepts. Panel 4 文件信息 header. Landscape preview thumbnail of vivid diagonal rainbow image, centered DSC08812.jpg and 4 blue rating stars, 信息 / 历史 / 扩展 tabs with 信息 blue underline. 基本信息 file name, JPG, 32.0 KB, 320×200, modification date. 标签 five colored pills 旅行 日本 自然 风光 富士山, EXIF available camera/lens rows, 文件路径 lower section. Panel 5 版本历史 header, compact file name strip without huge file preview. 信息/历史/扩展 tabs with 历史 active. Full width horizontal 创建版本 bright blue and 切换版本 secondary buttons. Full width branch selector 全部分支 chevron. Main blue branch vertical line with round nodes and actual diagonal rainbow thumbnail cards v1 timestamp selected outlined blue and v0 timestamp; curved purple branch from v0 bends right to indented v0.1 thumbnail card. Version names and timestamps secondary to thumbnail and branch structure. Below tree empty continuation card 暂无更多历史版本 and bottom bordered 当前选中版本 card with thumbnail, v1, 修改时间, 文件大小 31.9 KB, 尺寸 320×200. Small unobtrusive 模拟版本 note bottom. Keep all controls within panels, no clipped right text, no excessive blank space, no faux disabled gray all over. This is visual direction for implementation, show fully populated representative states, hierarchy separation and comfortable spacing.
```

