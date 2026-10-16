# 右侧面板：统一预览与集中确认（2026-10-16）

用户本轮明确要求：历史顶部恢复与信息/插件相同的预览；标签编辑全部展开，依次为名称、颜色、图标、显示开关、确认按钮。

使用内置 image_gen、imagegen skill 生成并审阅[概念图](right-inspector-unified-edit-2026-10-16.png)，再实现本轮布局。它取代上一版概念的历史紧凑身份栏与逐字段立即保存方式；其他上下文仍沿用完整v2。生成图仅指导布局和层级，其中照片、文字、图标不作为产品数据或资源。

- 三个文件页共用 `inspector_preview` 的180逻辑像素预览、文件名、评分与统一“文件信息”标题，页签位置固定。
- 标签编辑主卡放在身份卡下方；每个字段明确标题与间隙，蓝色确认按钮在最后，详情与子标签是次级分区。
- 色板、图标和开关只更新当前标签草稿；确认时验证名称与目标，并复用Navigation现有rename/appearance/display服务。已有显示规则继续传播至子标签。
- 图标采用项目已支持的14个持久化标识；生成图里的相机/太阳/地球不意味着引入新的图标标识。色板7色保留现有方案；不操作色板/图标时保留已有自定义值。
- 普通历史无版本后端的边界不变；演示仅在明确的隔离dev-test模式出现。

## 完整提示（内置工具，无CLI/API）

```text
Use case: ui-mockup. Create a precise high fidelity dark desktop PhoTag right inspector design reference. Two adjacent 360px wide panels on charcoal background, Chinese UI. Left: 标签信息 identity card coral tag icon and 旅行 title, compact metadata parent/root and 18 files/5 children. Main edit card ALWAYS EXPANDED with vertical sequence: 名称 outlined single text input 旅行; 颜色 seven colored round swatches with coral selected white ring; 图标 a 2-row compact grid of familiar line icons (tag, star, heart, folder, camera, sun, globe, mountain), tag selected with coral tinted background; 在缩略图上显示 labeled blue toggle; ONE full width blue 确认修改 button at the BOTTOM after all fields. No individual Save Name button, no more settings disclosure link. Child tags below separate section colored rows 日本6 and 欧洲5. Right: 文件信息 top photo preview occupying fixed 180px, filename DSC08812.jpg then blue four stars, consistent tabs 信息 历史 插件 all horizontally aligned below preview. 历史 selected. Below tabs compact create/switch buttons, purple/blue git-like branch tree with small photo thumbnails and primary version titles v1/v0/v0.1; secondary metadata smaller. Explicit 模拟版本 note. Visual hierarchy clear, dark #292e34 outer surface #1f2328 inset cards, #dce3eb primary text #97a5b4 secondary, subtle borders, 16px side padding, 12px card spacing, 8px field spacing, restrained 7px corner radius. Aim implementable native GUI with readable text and stable repeated header geometry. No giant whitespace, no invented metrics or media library.
```

同进程截图、交互和布局比较见[本轮证据](../../validation-ui/2026-10-16-inspector-unified-edit/README.md)。
