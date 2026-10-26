# Zorite 原生画板托管验证（2026-10-26，历史阶段）

> 更正（2026-10-26）：本记录验证的是早期直接托管上游完整 UI 的方案，之后按用户要求改为将 MIT 源码复制进 FileAtlas 本地引擎 crate，并按 tldraw 实际布局调整产品界面。当前参考与结论以[FileAtlas Canvas tldraw 布局验证](../2026-10-26-fileatlas-canvas/README.md)为准；以下结果只证明旧阶段当时的行为，不代表当前界面。

## 实现边界和参考

按用户决定，FileAtlas 只提供外框和文件管理。新增 Cordis 编译期插件
`fileatlas.whiteboard`，左侧“画板”入口打开既有通用悬浮面板；内部直接挂载未经修改的
`gpui-whiteboard = 0.5.1`，没有另写画布、选择、文字布局、变换或撤销历史。
上游固定提交 `d94ade3eb162c041b71994681356fcbcb4dcfd8e`；独立 crate 为 MIT，
归属和依赖字体许可证见 [PORTS](../../../crates/fileatlas-plugin-whiteboard/PORTS.md)。

编码前查看了[固定上游截图及实现清单](../../reference-ui/concepts/zorite-whiteboard-adoption-2026-10-26.md)。
最终截图逐项对照：原生点阵主体、紧凑居中工具栏、文字/图形/图片、原生选择工具保留；
宿主顶行仅管理文件，左侧列管理画板，底行显示保存状态。
本次是用户指定的原生组件托管，不替换上游控件设计。内部主题只通过必需的样式回调映射已有 Theme 角色。

| 行为 | 原生实现/公开接缝 | 宿主职责 |
| --- | --- | --- |
| 渲染、图形、文字、选择、变换、历史 | `WhiteboardView::new`、原生输入与工具栏 | 挂载完整 view |
| 场景保存 | `set_on_change` / `Scene` | 原样存储场景 JSON，250ms 合并写入 |
| 图片 | `set_on_image`、`add_image_at`、图片/文件拖放回调 | 通过 Media Resources URI 链接文件、后台解码缓存、RGBA→BGRA、显式刷新 |
| 剪贴板 | 原生 Element JSON、copy/paste 回调 | 系统剪贴板；无来源位图单独落 PNG |
| 模板、颜色、字体、工具栏 | 对应公开 setter / host hooks | 版本化 JSON、字体路径和原生模板 JSON |
| 原生卡片打开 | `add_embed` / `set_on_open` | 全局文件 ID 映射，复用 QuickLook 服务 |

这是一项新增能力，没有 PhoTag 对应功能或数据库迁移。存储选择为小型整文档版本化 JSON；
正常路径、隔离路径、备份、损坏恢复及未来版本拒绝策略均记录在 PORTS。
后续白板改为消费公共 URI，白板清单从 v1 迁移至 v2，旧 `path` 字段以及场景中的旧绝对图片路径继续可读；结构和插件接入图见[媒体资源目录 PORTS](../../../crates/fileatlas-plugin-media-resources/PORTS.md)。

## 环境、命令和结果

Windows x86_64 MSVC，Rust 1.97.1，GPUI 0.3.8，GPUI Kit 0.7.1，主机显示缩放 150%。
复用 `target/debug/fileatlas-shell.exe`。所有运行验证由 FileAtlas MCP 启动 `--mcp` 同进程子程序，
隔离 profile `.local/validation/profiles/zorite-native-acceptance`，Full 权限仅用于测试导出；
导出同时传入 `confirm:true`。隔离设置使用 dark 模式，关闭审查描边/文字审查色。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 test --locked -p fileatlas-plugin-whiteboard -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/zorite-native-acceptance --plugins whiteboard --access-level full --scenario scripts/validate-whiteboard.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/zorite-native-acceptance --plugins whiteboard --access-level full --scenario scripts/validate-whiteboard-restart.cjs
node scripts/check-ui-theme.cjs
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
node scripts/check-ui-geometry-coverage.cjs
```

全部通过。5 个单测覆盖场景/模板/颜色/工具栏重启保存、停止后拒绝旧回调、备份恢复、
未来版本保护、导入/导出不改源文件、失败写入重试，以及图片通道/alpha。
Shell 构建只有已有的 `LNK4098 LIBCMT` 链接警告；本次最终运行 stderr 为空。

同进程原生输入场景通过 10 项：矩形拖绘、撤销/重做、选择移动、文字输入、链接图片放置、
源图片 SHA256 不变、原生 JSON 导出再导入、挂载状态下静置 revision/文件 mtime 不变、
画板切换/改名，以及通用面板缩至 770×550。第二次子程序启动通过 4 项：场景恢复、
Cordis 停用清除 MCP/面板/侧栏入口、重新启用恢复场景、实际点击侧栏打开画板。
绘图通过原生指针/键盘事件完成；MCP 图片入口调用与宿主图片 hook 共用的原生放置方法。

本地证据（git-ignored，不提交生成图片/日志/数据库）：

- [完整画板截图](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/screenshots/whiteboard-final.png)：工具栏、外部管理和保存状态完整；图片与源文件比对颜色正确。
- [770×550 面板](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/screenshots/whiteboard-small.png)：管理行、原生工具栏和保存状态可见；画面缩小时已有内容可能在视口外，沿用原生平移查看。
- [重启并点击入口](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/screenshots/whiteboard-rail-click.png)、[停用状态](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/screenshots/whiteboard-disabled.png)。
- [主场景结果](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/whiteboard-result.json)、[生命周期结果](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/whiteboard-restart-result.json)。
- [主场景报告](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/whiteboard-main-report.json)、[重启报告](../../../.local/validation/artifacts/2026-10-26-zorite-whiteboard/whiteboard-restart-report.json)。

## 覆盖限制和剩余差异

- 上游没有内嵌视频/音频播放器或时间码标记，本轮不新增替代媒体引擎。
- 原生图片/字体/模板文件对话框、系统剪贴板图片、QuickLook 实际打开、OS 文件拖入和 FileManager 内部拖入尚未实际交互验证；hook 已接入，不能据此声称 Windows 路径全部通过。
- 模板/颜色/工具栏持久化经单测验证，但原生所有菜单、字体上传、中文 IME、连接线/思维导图等未逐一验收。默认内部英文文案保持上游。
- 图片解码最长边限制为 4096px；广泛格式/缩略图 fallback 未全覆盖。链接为本地路径，未实现 HTTP URL 下载。
- 原生 Scene JSON 导出不包含宿主模板库、字体路径及文件 ID 注册表；完整恢复应保留 `whiteboards/library.json` 和关联文件。
- 既有 Navigation 在插件重新启用时将侧栏贡献追加到缓存列表，画板入口会移到 WorkCrate 后；本轮记录该宿主排序差异。被动 MCP 侧栏快照已补入注册 ID 签名，能及时反映入口增删。
- 原生控件未消费 Escape 时，按通用面板行为关闭宿主浮层；文档仍保存。本轮没有改变宿主键盘规则。
- 更换应用强调色、浅色主题、hover/focus/disabled 全状态及更小主窗口未专项验证；没有把静态主题守卫当作这些状态的视觉验收。
