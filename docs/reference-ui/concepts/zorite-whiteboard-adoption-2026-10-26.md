# Zorite 画板原生托管（2026-10-26 用户决定）

第一阶段完整采用 `gpui-whiteboard = 0.5.1` 的 WhiteboardView：界面、工具栏、
颜色面板、图形/连接线/文字/图片、输入、命中、选择、变换和撤销重做均由上游负责。
FileAtlas 只提供通用原生浮动面板、画板清单、文件和公开 host hooks；以后另行决定
是否将内部控件接入 FileAtlas UI 管理。这一用户决定明确允许此第三方视图保留
原生自绘 chrome；本轮不重画画板内部，也不为上游控件生成替代概念。

## 活动参考及实现清单（编码前已检查上游实际截图）

- [固定上游截图](https://github.com/packetThrower/zorite/blob/d94ade3eb162c041b71994681356fcbcb4dcfd8e/docs/public/screenshots/zorite-whiteboard-dark.png)
  本机只读参考：`.local/validation/profiles/zorite-adoption/references/upstream-dark.png`。
- 原生点阵背景占主体；顶部居中可拖动的紧凑单排工具栏；子工具在其下方弹出。
- 原生卡片、笔迹、文字和连接线保持统一层级；选框、旋转和缩放句柄沿用上游。
- 原生颜色选择器、右键菜单、模板库和字体选择入口完整保留。
- FileAtlas 外框继续使用通用面板；外部顶行只管理名称、新建、打开/导出和移除。
  左侧窄列管理多个画板，主体为完整上游 view；底行反馈自动保存/文件错误。
- 外部文件管理控件继续复用 UI Bridge。上游必需的样式回调只映射原生 Theme 角色；
  不新增独立配色管理器、不更改上游组件或原生英文文案。
- 画板内用户选定的笔迹/填充/高亮颜色是文档内容数据，不作为应用界面配色。
- 小窗口允许管理顶行折行，左列滚动；画板工具栏和所有菜单保持上游行为。

## 范围

`gpui-whiteboard` 没有视频播放器；首版不新增自己的视频元素或标注引擎。
图像路径保持链接，只有无原始路径的剪贴板图片生成文件。页面卡片的 host 目标
在 FileAtlas 中对应用户选择的文件，打开走既有 QuickLook 服务。模板、字体、
保存颜色和工具栏位置的持久化全部通过上游公开回调补齐。

未把 Zorite 笔记应用、SQLite 页面系统或 GPL 应用层复制到 FileAtlas。
采用的独立 MIT crate 来源、hash 和限制见插件 PORTS。
