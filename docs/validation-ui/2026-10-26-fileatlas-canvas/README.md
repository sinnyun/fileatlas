# FileAtlas 画布布局与运行验证（2026-10-26）

## 参考和实现

实际打开 [tldraw.com](https://www.tldraw.com/) 查看空白编辑器，并阅读其[官方 UI 组件布局说明](https://tldraw.dev/sdk-features/ui-components)。可见结构是主画布占据中间绝大区域，左上文档/页面操作，右上样式入口，底部中间工具栏，左下缩放，撤销/重做紧邻工具栏。FileAtlas 在这套布局上保留自己的板列表、资源列表和图层检查器。

实现概念是[FileAtlas tldraw 布局稿](../../reference-ui/concepts/fileatlas-canvas-tldraw-layout-2026-10-26.md)。MCP 截图中检查了近全幅面板、文档操作、左侧画板和资源、中心点阵画布、右侧图层/属性，以及底部工具和历史按钮。选择图片后，右侧行和属性显示文件名、位置与尺寸；画布显示引擎提供的选择控制点。

![空白画布布局](../../../.local/validation/artifacts/2026-10-26-fileatlas-canvas/canvas-empty.png)

![插入并选择图片](../../../.local/validation/artifacts/2026-10-26-fileatlas-canvas/canvas-selected-image.png)

## 运行验证

- 环境：Windows，Rust `1.97.1-x86_64-pc-windows-msvc`；FileAtlas MCP 启动同一子进程中的 `fileatlas-shell --mcp`；profile：`.local/validation/profiles/fileatlas-canvas-final-pointer`，AI control 为 isolated Full。
- 构建：`cargo +1.97.1 build --locked --offline -p fileatlas-shell -j 2`，成功；链接器有既有 `LNK4098` 库冲突警告。
- 同进程场景：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/fileatlas-canvas-final-pointer --plugins whiteboard --access-level full --scenario scripts/validate-fileatlas-canvas.cjs`，通过。创建画板、读取动态面板 bounds（2060×1150）、从隔离 fixture 复制小图并经 `whiteboard.add_images` 放入场景、确认引擎选择与图层属性、确认保存 revision 与 saved revision 相等并 flush。
- 截图来自上述运行的同进程 `screenshot_window`。原始日志、数据库、profile 留在 `.local/validation/`；概念稿不是产品截图。
- 源码：画布算法由本地 `fileatlas-canvas-engine` 保存的 `gpui-whiteboard 0.5.1` MIT 源码承担；Cargo.lock 无 `gpui-whiteboard` 包依赖。源码 revision、许可证与改动见 `crates/fileatlas-canvas-engine/PORTS.md`。

## 未覆盖

本轮未验证画布上通过真实指针绘制新形状、各工具弹层/颜色/字体/模板交互、撤销重做点击、窄窗口与变更主题；也未验证 Windows 原生拖放、剪贴板、文件对话框和视频播放。图层分组、隐藏/锁定及可编辑属性仍未实现。截图确认当前布局在测试窗口下可见，不代表这些状态已完整。
