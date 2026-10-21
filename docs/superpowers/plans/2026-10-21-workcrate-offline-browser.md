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
- [x] Full runtime：官方 CLI 创建/画板/物料栏/预览/导出及退出后重启 FileAtlas 浏览已复验通过。
- [x] 离线分支已合并进 `main`（`341e744`）；本轮复验产生的修复与文档尚未提交，等待用户授权提交。

验证边界：同进程 MCP 离线场景 `scripts/validate-workcrate-offline.cjs` 首轮与重启复浏览轮均通过（7 项断言），运行前后官方 GUI/CLI 进程计数为 0；官方 headless 往返用例 `official_native_round_trip` 已改为无 GUI 并通过。复验暴露并修复三处缺陷：headless PDF 导出参数被官方 CLI 拒绝、重新关联原生文件仍依赖 GUI TCP 通道、原生往返用例长期失效。精确命令、实测产物尺寸、证据目录与未覆盖项见 `docs/validation-ui/2026-10-21-workcrate-offline-cli/README.md`；`open_editor` 的 GUI 跳转与画布内编辑、官方版本差异、原生弹窗与跨进程竞态仍不在该轮覆盖内。
