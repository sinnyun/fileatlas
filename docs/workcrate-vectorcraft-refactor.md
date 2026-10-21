# WorkCrate 官方 VectorCraft 程序重构

> 2026-10-21 用户进一步纠正：程序路径改在“设置 → 插件 → WorkCrate”持久化；新项目先选择文件夹保存原生工程。项目关联、主图、页面目录和预览缓存由 FileAtlas 保存，浏览不启动或连接官方程序。新建、物料栏和导出允许后台官方 CLI；只有画布内容编辑启动 GUI。当前修正计划见[离线浏览计划](superpowers/plans/2026-10-21-workcrate-offline-browser.md)，下文首轮 TCP 工作台验收为历史阶段。


2026-10-21 用户明确要求：在本项目内拉取 FileAtlas，在 FileAtlas 仓库中开发并最终推送至 sinnyun/fileatlas。WorkCrate 仍在开发中，旧历史数据无需保留，不实现旧 DesignCraft 画板 JSON 迁移。日常 FileAtlas 其他插件数据不属于本次重置范围。

## 实施边界

- FileAtlas 保留 Rust / GPUI Kit / Cordis 插件架构、主题与公共 UI Bridge，不引入 React、Tauri 或 egui。
- WorkCrate 是文档管理、真实画板预览和通用物料栏工作台。完整画布内容编辑交给已编译的官方 VectorCraft.exe。
- 一个项目关联一份真实 `.vectorcraft` 工程，页面来自其中的原生 Artboard。新建和复制项目都先选择原生工程保存目录。
- 项目关联、主图、画板目录与 PNG 预览缓存保存在 FileAtlas 数据目录；普通项目/页面/预览浏览只读取本地数据，不启动 GUI/CLI，也不连接本机端口。编辑器外保存后由用户显式同步缓存。
- 官方程序路径在“设置 → 插件 → WorkCrate”保存；后台默认发现同目录的 `vectorcraft-cli.exe`。新建、同步、物料栏更新、置入文件与导出使用官方 CLI headless 模式。只有打开画布内容编辑时才启动或聚焦 VectorCraft GUI。
- WorkCrate 不链接或修改 VectorCraft 源码。GUI 编辑使用官方 TCP 控制通道；后台工作使用官方 MCP stdio 接口。
- 管理器显示真实画板预览与物料表单，点击画布编辑打开/聚焦官方窗口；不承诺外部窗口嵌入。
- 字段值以原生文字为准；对象备注标识物料栏与字段。画板身份由项目、关联绑定和原生 ID 共同构成。
- 旧 WorkCrate 数据无需迁移，不递归删除用户应用目录。隔离数据与运行证据位于 `.local/validation/`。

旧版网页项目的源码行为及替换对照见[旧实现分析](workcrate-legacy-analysis.md)。界面布局可以参考旧版，但使用 GPUI 原生组件实现。

## 模块安排

1. `workcrate-projects`：沿用项目 CRUD、文件引用、字段预设与原子持久化，新增原生工程关联。
2. `workcrate-boards`：替换 DesignCraft 模型与渲染，改为官方程序配置、离线画板/预览缓存、CLI headless 原生物料栏命令与 TCP 编辑器交接。
3. `workcrate-export`：替换 PdfCraft 图像转 PDF，调用官方 PNG/JPEG 与多画板 PDF；保留任务进度和错误。
4. `workcrate-ui`：沿用 Cordis/UI Bridge 的入口、面板、设置和原生组件；项目管理 → 页面矩阵 → 预览与物料栏。去掉自建图片手柄、裁切、全文编辑等第二套编辑器逻辑，提供官方编辑入口。
5. MCP：保留 FileAtlas 现有权限门槛，提供项目/画板/字段/配置/导出工具；浏览不触发官方程序，只有编辑动作启动 GUI。

## 验收与执行顺序

- [x] 拉取官方 FileAtlas，origin 为 sinnyun/fileatlas，建立 codex/workcrate-vectorcraft 分支。
- [x] 下载官方 VectorCraft 0.14.0 Windows portable 并核对官方 SHA256；样本 CLI 报告 3 个画板、287 个对象。应用打开诊断另发现外部图片链接缺失，不能把 CLI 的空 warnings 当成资源完整证明。
- [x] 新协议与模型首轮测试通过，后续审阅问题修复后复测。
- [x] 替换 Boards/Export 服务，通过官方程序保存回读、物料栏、PNG 与 PDF 验证。
- [x] 接入原生 GPUI 项目/页面/物料表单及官方编辑入口，保留项目管理和预设功能。
- [x] 构建与相关测试、架构/主题/刷新守卫。
- [x] 使用同进程 FileAtlas MCP 完成隔离 profile 下的真实 UI 操作和截图；未运行项明确标记。
- [x] 代码审阅及修复复核完成；分支为 `codex/workcrate-vectorcraft`，提交推送状态以 Git 远端结果为准。

## 初始验证状态

旧版最小契约测试在离线模式下因缺失 Craft-Libs/Krilla 固定依赖而无法启动；不将此状态记录为测试通过。该依赖由即将替换的 DesignCraft/PdfCraft 链路引入，重构后更新锁文件并重新运行。

此前宿主库级集成、独立 egui 管理器、React/Tauri 候选和旧数据迁移方案，均由本文明确的 FileAtlas 插件重构要求替代。

## 使用与调试入口

在 WorkCrate 顶部配置已安装的官方 VectorCraft 可执行程序路径和本机控制端口，然后连接。连接不到服务时，由管理器尝试用 `--control <端口>` 启动该官方程序。程序无需放入 FileAtlas 源码，也不在 FileAtlas 编译时重新构建。升级程序后重新连接并检查所需命令是否可用；不假定任意未来版本都兼容。

项目可以关联现有 `.vectorcraft`，也可以在首次新建页面时由官方程序建立工程。页面是该工程的真实画板。选择页面后显示官方渲染的预览，右侧编辑通用物料字段；内容编辑按钮定位到官方编辑器。完成外部编辑后，保存并刷新管理器预览。多页 PDF 的页面必须属于同一个原生工程。

开发验证入口：`scripts/validate-workcrate-vectorcraft.cjs`，由 `scripts/fileatlas-mcp-debug.cjs` 驱动同进程 FileAtlas MCP。验证前设置 `FILEATLAS_VECTORCRAFT_EXE`，启动独立、隔离配置的官方程序；FileAtlas profile、官方配置目录和测试输出路径均放入 `.local/validation/profiles/`。官方配置用 `VECTORCRAFT_CONFIG_DIR` 隔离；手工启动受限实例时另传官方自动化读写根目录参数。旧 `validate-workcrate-editing`、`inline-text` 等脚本针对已替换的自建编辑器，不作为本次原生程序集成的验收依据。

最终构建、14 项测试、1 项官方程序集成测试、窗口操作与输出文件检查均通过，详见[验收记录](validation-ui/2026-10-21-workcrate-vectorcraft/README.md)。
