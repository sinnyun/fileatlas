# WorkCrate 离线浏览与插件配置实施计划

> **For agentic workers:** Use subagent-driven-development with task review. Follow the checked user requirements below, use official compiled programs only, and verify actual no-GUI operation.

**Goal:** 程序路径在插件设置保存应用；项目创建先选择原生文件保存文件夹；FileAtlas 内部持久化项目关联、主图、画板目录和预览，断开/不启动编辑器时照常浏览。用户明确允许创建、物料栏和导出通过后台官方 CLI，仅内容编辑显示 GUI。

**Architecture:** Boards 保存独立于官方进程的项目画板快照及稳定预览文件；普通查询只读本地。显式创建/同步/修改/导出使用官方 CLI headless 会话，GUI 仅在 open_editor 启动。项目原生文件保存在用户指定文件夹，管理元数据与缓存保存在 FileAtlas feature root。UI 设置复用插件设置入口，业务面板不显示路径/端口连接工具条。

**Tech Stack:** Rust / GPUI Kit / Cordis，官方 VectorCraft.exe 与同目录 vectorcraft-cli.exe，官方 JSON-RPC MCP stdio 适配（不链接/修改上游）。

## Global constraints

- 浏览项目、页面、画布预览不启动 GUI 或 CLI，也不连接端口；缺少预览用明确占位，不偷偷启动。
- 用户目录 native文件；FileAtlas 内部主图、目录、快照与缓存；不导入旧网页数据。
- 当前设置持久化配置复用Boards.configure，配置更改后后续动作直接使用。
- 后台官方程序用 Windows hidden，无原始 stdio 污染；有界超时、错误状态、生命周期清理。
- 后台写入不得覆盖官方 GUI 中尚未保存的同一文档；GUI编辑结束显式同步到缓存，离线浏览始终保留最后成功快照。
- Cargo +1.97.1 --locked -j2，产物只进 .local/validation，UI测试同进程 MCP。

## Tasks

- [x] Backend：缓存目录/预览与缓存读取；官方 CLI headless；新增 create_project_document(project, folder)、refresh_project(project)，保留关联绑定及原生画板ID。
- [x] Native UI：插件设置保存程序路径；项目新建/复制先选文件夹；浏览读取本地缓存；显式同步调用后台，打开编辑动作启动 GUI。
- [x] MCP：目录读取/打开预览离线；创建/复制项目需要 directory；显式 refresh 工具；配置只持久化，编辑才启动 GUI。
- [ ] Full runtime：官方 CLI 创建/画板/物料栏/预览/导出及退出后重启 FileAtlas 浏览仍待确认。
- [x] Code review/docs are complete; commit, feature-branch push, and merge/push to `main` are in progress on `codex/workcrate-vectorcraft`; no force push.

验证边界：Cargo 锁定模式相关 crate 测试在最后一次 CLI 单根目录修正前通过（17 项通过，1 项按条件忽略）；架构/分层/UI/功能守卫和格式检查通过。实际 CLI 流程先暴露官方 CLI 每类权限只允许一个根目录，代码已按此修正。按用户要求，此后不再运行测试或场景，因此这次根目录修正后的原生运行流程尚未复验。
