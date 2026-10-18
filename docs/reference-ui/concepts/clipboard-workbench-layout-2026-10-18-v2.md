# 剪贴板工作台：参考还原与开源工具调研 v2

2026-10-18。用户指出上一版只实现三栏结构，未还原参考样式，且分类操作不正确。
本稿覆盖上一版剪贴板面板视觉方案；不改变已接受的照片工作台主窗口。

## 调研与取舍

| 一手来源 | 观察 | 本次采用 |
|---|---|---|
| [EcoPaste 项目](https://github.com/EcoPasteHub/EcoPaste)、[官方暗色截图](https://www.ecopaste.cn/home/ecopaste-hero-dark.png) | 内容类型和用户分组独立；卡片突出类型、时间、置顶与快速操作 | 自动类型入口、真实类型图标、时间/字数、分组徽标、卡片置顶/菜单 |
| [CopyQ 功能与截图](https://hluk.github.io/CopyQ/)、[分组文档](https://copyq.readthedocs.io/en/latest/tabs-and-items.html) | 自定义标签页组织内容，支持移动与编辑；树状分组是可选能力 | 明确的用户分组、未分类、即时移动、分组管理，与正文保存分离 |
| [Ditto 官方项目](https://github.com/sabrogden/Ditto) | 以快捷唤醒、搜索、回用历史为核心；支持直接粘贴到先前窗口 | 搜索和复制入口易达；当前保留已实现的复制到系统剪贴板行为 |
| [ZClipboardManager](https://github.com/TheHolyOneZ/ZClipboardManager) | GPL-3.0 的完整 Tauri/Rust 剪贴板管理器；分别读取文本与图片并按内容分类 | 借鉴内容类型分类与图像独立预览的行为；该仓库不是可嵌入库，不复用其 Tauri/React 界面或代码 |

本轮参考交互与视觉模式，没有复制这些项目的代码。没有把它们的文件捕获、
来源程序识别、直接跨程序粘贴等功能描述成本项目已实现。

## 实现目标

- [概念图 v2](clipboard-workbench-layout-2026-10-18-v2.png)：内置 ImageGen 生成，检查后保存到项目。
- 深蓝底 `#0e1823`，区域 `#152230`，卡片 `#1b2a3b`，输入区 `#101b27`，边框 `#304357`，主色 `#2787ff`。
- 面板默认 1160×760、最小 1000×660；区域间 10px，内部 12–16px，圆角 7–8px。
  左栏 220px，右栏 324px，中间自适应；标题 18–23px，正文 14px，辅助 11–13px。
- 使用 GPUI Kit 的 Lucide 图标和真实数据数量；不再用空格对齐计数或以字符充当图标。
- 自动内容类型与用户分组独立；分组立即保存，正文明确显示是否保存。链接和代码仅是文本内容的派生筛选。
- 选中卡片为蓝色底/边框，非选中悬停轻提亮；卡片显示真实分组。编辑分组表单固定在导航滚动区外，避免保存按钮被裁切。
- 概念中的示例数量/文本是视觉示意；运行截图只使用隔离 fixture。没有虚构来源程序、自动正文保存或嵌套分组。

## ImageGen 最终提示

内置工具编辑参考图 `clipboard-workbench-layout-2026-10-18.png`，提示如下：

> Use case: ui-mockup. Refine the attached clipboard workbench reference, preserving its rich dark navy palette, blue selection edges, generous 14-16px typography, white outline icons, three clearly separated panels, 12px gaps, 8px corners. A native desktop floating panel, straight-on full screenshot, landscape 1120x720 logical px. Left rail 210px: title 剪贴板, bordered search 搜索内容, menu 全部历史 / 置顶 / 未分类 with right-aligned counts; section 内容类型 with 文本 / 链接 / 代码 counts; section 我的分组 with 工作 / 常用 snippets and small plus button, no nested folders; capture live green dot and pause button at bottom. Center history area around 530px: top title 全部历史 and 新建文本 blue button; compact date sections 今天/昨天; 5 representative cards with type icons, 2-3 readable lines of Chinese sample text, HH:mm metadata, char count, category pill on lower right, pin icon top right. Selected first card has darker rich blue fill and clear bright blue border/accent edge; hovered other row uses subtle lighter navy. Right inspector 320px: 编辑内容, real large multiline textarea with selected text and footer char count; 分组 dropdown 工作 with adjacent note 即时保存; 置顶 compact control; created date metadata only; bottom blue 保存内容 and outlined 复制到剪贴板 button. Use flat navy/slate surfaces (#141f2b,#192735,#223344), blue #2787ff, secondary blue-gray text #9fafc4, clear border #35485e. No fake source apps, no auto-save content, no unsupported image/file controls, no gradients, no gigantic empty blank space. Match attached reference's typography, card density, icons and hierarchy more faithfully.
