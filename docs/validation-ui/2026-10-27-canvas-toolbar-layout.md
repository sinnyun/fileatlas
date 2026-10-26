# 画布标题、状态与底部工具栏布局（2026-10-27）

## 实现核对

按用户标注截图和[本轮概念清单](../reference-ui/concepts/fileatlas-canvas-toolbar-revision-2026-10-27.md)调整：移除画布插件内顶部标题带及底部整宽状态带；左轨顶部显示画布和当前画板名，画板/资源按钮在其下；底部工具行按撤销/重做、绘图、颜色/字体/模板三块独立背景排列；保存反馈以无框状态放在工具行下方。引擎的颜色、字体和形状扩展弹层统一位于底部工具行上方，避免继续使用隐藏的顶部工具栏锚点。

## 同进程验证

- 命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --plugins whiteboard --profile .local/validation/profiles/canvas-toolbar-layout-20261027 --access-level standard --scenario scripts/validate-canvas-toolbar-layout.cjs`
- 环境：Windows 桌面 FileAtlas，同进程隔离 MCP 子进程，Standard 权限，截图画面为 3286×1831；系统显示缩放比例未由截图接口报告。使用隔离 profile 并新建临时验证画板，不触碰普通用户数据。
- 场景确认临时画板处于活动状态并截取默认布局；随后通过 `whiteboard.tool_group` 打开形状扩展面板，并通过新增的 `whiteboard.toolbar_menu` 调用引擎现有颜色、字体、模板切换方法，分别截图。形状、颜色与字体面板显示在底部工具行上方；模板库保持居中模态。截图确认左侧标题位于按钮上方、底部三块面板各自分离、保存状态显示在面板下方；无整宽标题/状态框。
- 截图带有项目当前开启的组件/几何审查边界颜色，这些是诊断覆盖层，不属于产品边框。MCP 场景通过同进程动作调用视图/引擎同一切换方法，未通过原生鼠标逐个点击工具按钮。

## 静态检查

`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-plugin-whiteboard -p fileatlas-canvas-engine -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、UI theme self-test/守卫、UI refresh、architecture、feature-ledger 守卫及 `node --check scripts/validate-canvas-toolbar-layout.cjs` 通过。Shell 链接仍有既有 Windows `LNK4098` 默认库冲突警告，构建成功。

完整截图与 MCP 报告位于 `.local/validation/profiles/canvas-toolbar-layout-20261027/`。小窗换行、长画板名截断、保存失败/重试状态、键盘焦点及实际用户画板内容未在本次场景覆盖。
