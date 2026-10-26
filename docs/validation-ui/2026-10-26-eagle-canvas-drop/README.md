# Eagle 图片拖入 FileAtlas 画布验证（2026-10-26）

## 实现与结果

白板画布的 `set_on_drop_files` 原生文件回调与图片插入工具共用 `Host::place_images`。插入流程保留 Eagle 库里的原图，不复制媒体；当原图位于 Eagle 的 `<item-id>.info` 目录并能由当前 Eagle Web API 确认时，服务读取条目名称及 folder ID，并把条目 ID、API URL、原图路径和统一 `file://` URI、缩略图路径、文件夹 ID/名称写入当前画板的 `eagle_sources`。选中该图片后，右侧属性面板显示这些来源字段。旧版画板读取时缺少该字段会按空列表处理。

## 验证

- 环境：Windows，Rust `1.97.1-x86_64-pc-windows-msvc`；Eagle 4.0.0 正在运行，本机 Web API `127.0.0.1:41595` 可用；FileAtlas MCP 以隔离 profile `.local/validation/profiles/eagle-canvas-drop-final-20261026` 启动同一子进程中的 `fileatlas-shell --mcp`，AI control 为 Standard。
- 构建：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked --offline -p fileatlas-plugin-eagle-links -p fileatlas-plugin-whiteboard -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked --offline -p fileatlas-shell -j 2` 通过。链接阶段有既有 `LNK4098` 警告。
- 运行：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/eagle-canvas-drop-final-20261026 --plugins whiteboard --access-level standard --scenario scripts/validate-eagle-canvas-drop.cjs` 通过。用运行中 Eagle API 找到本地原图 `WILDWOOD - LAIKA.jpg`，经 `whiteboard.add_images` 排队并走和原生拖放共用的 `place_images` 回调；Scene 图片 URI 指向原图，`eagle_sources` 记录名称、文件夹“代号-一生”、路径、URI、缩略图及 `http://127.0.0.1:41595/api/v2/item/get?id=MUF1LNTTIIFPG`，画板保存后状态一致。
- 详情截图来自同进程 `screenshot_window`，可见图片已选中，右侧属性列出 Eagle 来源信息：[选中 Eagle 图片的详情面板](../../../.local/validation/artifacts/2026-10-26-eagle-canvas-drop/eagle-image-properties.png)。截图启用了调试组件边框；详情列较窄，长路径按行折叠显示，字段可滚动查看。
- Eagle Web API 的 folder/get 返回树形分页数据，插件递归查找条目提供的 folder ID，以免把全部库文件夹误显示为该图片所属项。Eagle 原始文件与缩略图同置 `.info` 目录时，插件按扩展名和字节数唯一匹配原图；无法唯一匹配时会安全报错。此原图匹配规则来自当前库的实测目录结构，不是 Eagle 对内部磁盘布局的稳定承诺。

## 覆盖边界

这次 MCP 通过白板公共 `add_images` 工具触发与系统拖放相同的宿主放置回调，验证了 Eagle 原图解析、画板持久化和属性展示。尚未用真实 Windows 指针从 Eagle 跨应用拖到 FileAtlas，也未检查不同 Eagle 库位置、不可访问库、Eagle 关闭后的拖入行为；这些系统路径仍待现场交互验证。完整布局参考为 [FileAtlas tldraw 布局稿](../../reference-ui/concepts/fileatlas-canvas-tldraw-layout-2026-10-26.md)。

原始运行报告与 profile 在 `.local/validation/profiles/eagle-canvas-drop-final-20261026/`，不进入版本控制。可复用运行脚本位于 `scripts/validate-eagle-canvas-drop.cjs`。
