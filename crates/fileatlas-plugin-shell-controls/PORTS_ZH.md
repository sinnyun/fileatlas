# Shell 控件插件对接

- Cordis 身份：`fileatlas.shell-controls`；没有公开服务，也不持久化数据。
- 启动依赖：宿主必须在本插件启动之前发布 `fileatlas-ui-bridge::UiBridgeService`。
- Cordis 拥有的视图：侧栏切换 → `shell.title-left-action`；inspector、workspaces 和 Tools → `shell.toolbar-actions`。
- 视图操作会更新 `UiSurfaceState` 中的 `sidebar.visible`、`inspector`、`workspace-manager` 和 `tools`；Shell 观察这些值并组合出可见性。
- 没有事件、载荷、数据库调用或 IO。诊断点击问题时请查看 surface 值以及 Shell 的 surface 观察者。
- 限制：窗口框架、标题品牌、槽位挂载、面板几何和尺寸调整行为仍由 Shell 保留。通用的插件命令贡献由宿主根据贡献目录渲染。
