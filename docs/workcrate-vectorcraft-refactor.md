# WorkCrate 官方 VectorCraft 程序重构

2026-10-21 用户明确要求：在本项目内拉取 FileAtlas，在 FileAtlas 仓库中开发并最终推送至 sinnyun/fileatlas。WorkCrate 仍在开发中，旧历史数据无需保留，不实现旧 DesignCraft 画板 JSON 迁移。日常 FileAtlas 其他插件数据不属于本次重置范围。

## 实施边界

- FileAtlas 保留 Rust / GPUI Kit / Cordis 插件架构、主题与公共 UI Bridge，不引入 React、Tauri 或 egui。
- WorkCrate 是文档管理、真实画板预览和通用物料栏工作台。完整内容编辑交给已编译的官方 VectorCraft.exe。
- 项目与一份真实 `.vectorcraft` 工程关联，页面来自其 Artboard，不再每页创建 DesignCraft 文档。
- 官方控制通道负责打开、查询、原生对象修改、历史、保存、渲染与 PNG/PDF。WorkCrate 不链接官方源库。
- 官方程序路径/端口可配置，升级按能力检查连接，不将编辑器重新编译进 FileAtlas。
- 管理器显示预览和物料表单，点击编辑打开/聚焦官方窗口；不承诺外部窗口嵌入。
- 字段值以原生文字为准；对象备注标识物料栏与字段。画板身份使用原生 ID，命令执行时解析当前索引。
- 旧项目/画板数据无需迁移，但不递归删除用户应用目录。隔离测试数据均位于 `.local/validation/`。

旧版网页项目的源码行为及替换对照见[旧实现分析](workcrate-legacy-analysis.md)。界面布局可以参考旧版，但使用 GPUI 原生组件实现。

## 模块安排

1. `workcrate-projects`：沿用项目 CRUD、文件引用、字段预设与原子持久化，新增原生工程关联。
2. `workcrate-boards`：替换 DesignCraft 模型与渲染，改为官方程序配置、TCP 客户端、会话核验、画板快照、原生物料栏命令和官方预览缓存。
3. `workcrate-export`：替换 PdfCraft 图像转 PDF，调用官方 PNG/JPEG 与多画板 PDF；保留任务进度和错误。
4. `workcrate-ui`：沿用 Cordis/UI Bridge 的入口、面板、设置和原生组件；项目管理 → 页面矩阵 → 预览与物料栏。去掉自建图片手柄、裁切、全文编辑等第二套编辑器逻辑，提供官方编辑入口。
5. MCP：保留 FileAtlas 现有权限门槛，提供实际会话、画板、字段、配置与导出工具；停止发布已移除的自建编辑 API。

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
