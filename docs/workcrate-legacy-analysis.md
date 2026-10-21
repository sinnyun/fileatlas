# 旧版 WorkCrate 实现与本次重构对照

检查日期：2026-10-21。旧项目位于仓库外的 `../old/`，作为布局参考保留；本仓库不导入其网页运行时或历史数据。

## 旧项目实际开发方式

旧项目由 React 19、TypeScript、Vite、Tailwind 和浏览器组件构成，入口为 `src/App.tsx`。项目列表、页面矩阵、装配工作台、侧边画布目录和右侧信息栏分别由独立 React 组件实现。`useVectorCraftStore.ts` 使用 React state 和浏览器 localStorage 保存整份项目数组，页面和素材是自己定义的 TypeScript 对象。新项目默认插入示例名称、尺寸及矩形素材，复制项目主要是 JSON 深拷贝。

工作台的素材位置、旋转、层级等由网页自己的模型与交互维护，不等于直接操作官方程序中的原生画板。`VectorCraftFullScreenEditor.tsx` 另开 iframe，地址为静态目录 `/vectorcraft-engine/index.html`；已检查的组件没有把当前项目、当前画板和物料栏同步到 iframe 的消息协议。界面显示“官方引擎运行中”本身不能证明业务数据已经接入官方文档。

静态入口引用特定版本的 JS 和 WASM 文件。当前旧目录有对应 JS，未发现该入口所引用的 WASM 文件，因此仅凭这些项目文件无法确认网页编辑器可运行。

`utils/vectorcraftDoc.ts` 自行组装一个带 `version`、`materialHeader`、`placed_image` 等字段的 JSON，再通过浏览器下载为 `.vectorcraft` 文件。源码注释声称兼容官方格式，但这条下载链路没有调用官方保存或格式验证，不能据此认定其输出是可被官方程序正确回读的原生工程。

`utils/exportEngine.ts` 使用 HTML Canvas 重新绘制固定 720 × 460 的页面、物料文字、网格和代表素材的色块/文件名。图片导出来自这张浏览器画布；PDF 使用 jsPDF，把该画布的 JPEG 贴入 A4 横向页面。它并非导出当前官方编辑器中的真实矢量文档，也没有调用官方 PDF 导出能力。

## 可以保留的设计参考

项目卡片 → 页面矩阵 → 左侧页面列表、中央画布、右侧物料表单，这组三层导航与空间分配符合管理工作台的需求。可以参考其卡片、标题、间距和信息组织方式，并使用 FileAtlas 的 GPUI Kit 原生组件及主题重新实现。

## 本次替换方式

| 职责 | 旧实现 | FileAtlas 内的新实现 |
| --- | --- | --- |
| 程序与界面 | 浏览器 React 页面 | Rust / GPUI Kit / Cordis 插件 |
| 项目 | localStorage 对象数组 | FileAtlas 项目服务，关联真实原生工程 |
| 页面 | 自建页面对象 | 官方工程内部真实 Artboard |
| 内容编辑 | 自建网页操作 + 独立 iframe | 打开、定位并聚焦官方已编译程序 |
| 物料信息栏 | 网页模型和绘制文字 | 官方原生文字、形状和分组；备注标记所属字段 |
| 画布预览 | 网页模型重绘 | 官方程序渲染 PNG，由 FileAtlas 显示 |
| 工程保存 | 自制 JSON 下载 | 官方程序保存原生 `.vectorcraft` |
| PNG / JPEG / PDF | HTML Canvas + jsPDF | 官方程序导出，等待实际完成并校验输出 |
| 编辑器升级 | 固定网页静态构建 | 配置外部官方程序，连接时验证所需能力 |

实施范围与当前验收状态以 [VectorCraft 程序重构](workcrate-vectorcraft-refactor.md) 为准。
