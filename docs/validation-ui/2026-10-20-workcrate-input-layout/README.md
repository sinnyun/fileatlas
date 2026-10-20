> 后续用户已纠正产品语义：固定 A3 图片排版，不是通用画布。此文保留旧路径修正历史，
> 其中整页平移等行为已撤销，最新参考为 `workcrate-simple-layout-2026-10-20.md`。

# WorkCrate：概念布局与图片入口纠正（2026-10-20）

## 更正上一轮结论

用户明确指出布局、位置、框架不符，添加图片和主界面拖入也不能使用。
上一轮后台 `place_image` 成功不能作为可见入口的验收。本记录保留原三张概念图，
不重新生成接近现状的图来降低标准。

活动参考：`docs/reference-ui/concepts/workcrate-{projects,pages,editor}-concept-2026-10-20.png`
及同目录三个 `*-user-*` 草图；逐项清单见
[三层参考](../../reference-ui/concepts/workcrate-three-level-2026-10-20.md)。

## 原因与修正

1. Windows 原生选择路径与引用服务返回的规范路径直接比较，导致找不到刚添加的引用。
   现在选择器、应用内路径录入、所有拖入都走规范化后的原图导入。支持多文件。
2. 画布没有注册主界面的公共拖拽载荷接收器。现在接收 FileManager、暂存篮及平台路径；
   只接收 JPG/JPEG/PNG，解码与放置仍委托 DesignCraft。
3. FileManager 进入拖动就启动 OLE，抢走同窗口接收器事件。现在窗口内保留 GPUI 拖拽，
   离开窗口才排队调用已有 DragDropService。每手势的原子标记防止重绘后的多卡片重复启动；
   导出路径来自实际载荷。GPUI Windows 的外部提升默认不支持，因此没有依赖该空实现。
4. 原界面工具条仅占中央列、画布留白过多、项目封面比例过扁、检查器图片卡片过重。
   默认浮窗改为 1560×980，工具条跨左/中区域，180 页面栏 / 弹性画布 / 310 检查器。
   项目恢复三列高封面卡片，页面名/尺寸/像素同行，整卡选中，默认六页可见且导出栏固定。
5. 页内灰色信息带由 DesignCraft 的 swatch/frame/fill/text 命令生成，属于导出内容。
   应用配色仍完全由原生主题拥有。选择/平移工具可用，适应页面会清除平移偏移。
6. 重新装载时不能用未填入图片字节的清单短暂覆盖现有可渲染文档；忙碌时也不触发
   新的适配渲染。修复“已显示图片但仍误报无法解码”的时序问题。

## 复现入口与环境

Windows MSVC，Rust `1.97.1-x86_64-pc-windows-msvc`，DPI 150%。
使用刚构建的 `target/debug/fileatlas-shell.exe`，每轮由 FileAtlas MCP 启动隔离 stdio
子进程，与 GPUI 窗口同进程。没有通用桌面鼠标注入。

```powershell
cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-workcrate-contracts -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-export
cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-input-layout-2026-10-20-r7 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-input-layout.cjs
```

三层目录/六页/PDF/PNG/JPEG 场景：`scripts/validate-workcrate-three-level.cjs`；
本轮目录复验 profile 为 `.local/validation/profiles/workcrate-three-level-2026-10-20-r3`。
报告、截图、清单、数据库和导出文件均保留在上述忽略目录内，不纳入 Git。

## 对照与边界

| 检查 | 实际观察 | 边界 |
|---|---|---|
| 项目管理 | 三列标题+高封面，邻接菜单，新增卡片；无页面字段 | 原生主题默认强调色，不另造概念图蓝色调色板 |
| 页面管理 | 三列两行六页全部可见，单行页信息，灰色页内信息带，固定导出栏 | 素材使用合成渐变，概念中的活动插画不是产品数据 |
| 页面编辑 | 跨左/中工具带、左页面栏、中央 A3、右检查器，页信息位于导出区外 | 复用宿主浮窗标题栏；非独立应用标题栏 |
| 可见添加入口 | 点击添加，真实 GPUI 输入中文/空格/大写扩展名路径，点击添加并保存首图与 900×600 元数据 | Windows Browse 选择器点击另列未验证 |
| 主界面拖入 | 真实文件卡片 pointer down/drag/up，关闭内部拖拽调试开关，保存第二张 DesignCraft 图片 | 不以后台放置工具替代该检查 |
| 平移与适配 | 点击平移并拖动，偏移改变；点击适配，偏移归零 | 不修改源文档坐标或导出内容 |
| 小窗口 | A3 适配，侧栏独立滚动，主要命令保留 | 图片列表底部需滚动，不声称小窗全部同时可见 |

剩余差异/未验：概念中的图片缩放/旋转拖动手柄和裁剪尚未实现（仍使用既有变换按钮）；
源文档外部“编辑”未接入；此次不宣称完整设计工具对等。
原生 Browse、Explorer 外部拖入/拖出、跨窗口/多显示器 OLE 边界尚未实际验收。
上一次静态/服务验收不能覆盖这些路径。原生边界变更保留为待覆盖项。
