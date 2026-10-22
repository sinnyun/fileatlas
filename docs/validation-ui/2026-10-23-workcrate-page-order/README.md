# WorkCrate 页面排序、自动标题和连接保持（2026-10-23）

## 当前设计和功能

本轮用户明确覆盖旧标题与页面信息编辑方式；已打开实际
workcrate-pages-user-2026-10-21.png、workcrate-inspector-2026-10-21.png，
并沿用三层入口、三列矩阵、左页面栏、官方画布及右侧检查器。
局部实现清单在 reference-ui/concepts/workcrate-editing-tools-2026-10-20.md。

- 页面矩阵与编辑侧栏均有上移、下移图标及拖动手柄；拖到目标页面即采用其位置，支持两个方向。
- 首页上移、末页下移禁用；保存、画布任务或字段草稿期间禁用排序。悬停/选中/焦点使用原生主题。
- 页面标题按实际顺序自动生成“页面01、页面02…”，不再手动编辑。物料栏只显示数字“01、02…”。
- 页面信息仅保留只读自动标题。移除尺寸标记、画板点数/像素、旧尺寸占位及对应编辑键。
  页面内容里的物料尺寸仍可编辑。旧尺寸元数据保留兼容，不做破坏性迁移。
- 真实顺序写入原有 .vectorcraft；没有额外的 UI 顺序文件或数据库。旧缓存标题只在读取时
  投影为自动名，不因为浏览就改写用户文档；新建、显式换序写入官方画板标题。

## 连接和批量更新

初版换序复用了整项目 perform/重载路径，用户指出其会造成重连和卡顿，本次已替换：

1. 编辑页换序复用现有官方 MCP/CLI 会话。按稳定 native_id 更新预览索引，保留 session_id、
   当前选中页、官方对象选区、工具、画布图像及前端缩放。
2. artboard.reorder、自动标题及数字物料编号在一个官方 undo group 中完成，最后保存一次。
   同一批次共用 OwnedLayout；不移动任何已有画板/正文，也不重建 layout 2 的物料栏对象。
3. 只渲染编号/标题实际变化的页面。选中页的同一帧用于 UI 与缓存，未移动页保留 PNG 路径。
   其余预览的渲染与图片解码均在后台，前端按页面 ID 一次合并记录/图像，不整项目 reload。
4. 旧缩略图读取和画布探针通过已有 generation 失效机制丢弃；较旧排序完成也不能覆盖较新结果。
   批处理期间画布保持可见，结束后恢复同一会话的被动探针。纯浏览/默认信息仍在本地服务完成。

排序、标题和纸内编号属于真实原生文件元数据，仍需要官方保存；将其留在前端会让重新打开或导出
的顺序不一致。本轮分离的是界面状态/图像交付与后台事务，不是另建一份互相冲突的文档内容。

## 发现并修复

- 官方 artboard.new 默认放在“列表最后一页”右侧。排序后最后页可能在中间，继续新增会重叠其他页。
  现在向官方命令明确传入全部画板最右边界 + 20pt 和最上边 y；已有画板不移动。
- 矩阵原宽度未充分扣除左右边距、间隙及滚动条，会意外只排两列。已保留三列并修正可用宽度。
- 无界面官方会话不支持 view.goToArtboard；保持画布时只更新内部预览索引，和原切页机制一致，
  也不移动用户 GUI 的视口。
- 被动刷新曾清除上一条排序/保存错误，现在保留操作反馈；失败不会通过静默清空提示来掩盖。

## 环境、命令及证据

Windows，150% DPI，Rust 1.97.1 MSVC，官方 VectorCraft 位于 C:/Program Files/VectorCraft。
使用正常 target/debug/fileatlas-shell.exe，所有输入/界面与运行数据均经同进程 FileAtlas MCP
stdio 子进程，Full 只用于隔离验证。官方绘图操作使用 headless，会话 ID 断言针对该路径。
没有外部鼠标注入、直接修改原生文档或独立启动 GUI 来代替 MCP 交互验证。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -- --test-threads=1
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-page-order-2026-10-23-efficient-final4 --access-level full --scenario scripts/validate-workcrate-page-order.cjs
```

测试 TEMP/TMP 在 .local/validation/profiles/wc-page-order-efficient-tests-final/temp。
40 项 Boards/UI 回归通过，4 项原有官方门控测试仍 ignored；不将 ignored 当通过。
fmt、locked shell check/build 与治理检查通过。正常 EXE 曾被工作区已核实进程占用，
按 AGENTS 授权先正常关闭该进程后复用原 target 重新链接；无新 EXE/target 或全量清理。
链接器仍有现有 LNK4098 警告。

同进程脚本实际创建三页并设置不同物料，按钮和拖动分别验证矩阵及侧栏，
比较换序前后的会话 ID、reload 计数、选区、工具和未移动页 PNG 路径；
原生保存重开后比较真实顺序、页面标题、纯数字物料编号、文本 ID、右侧内容及背景分组。
新增第四页验证名为页面04，并逐对检查官方返回的画板矩形不重叠；
重复 ID 排序和已移除页面信息字段编辑被拒绝。静置 3.5s 检查目录重载安静。
更换原生主题种子、查看悬停、选中、字段焦点和排序禁用态；文档文字保持黑色。

本轮测试中的等待必须匹配预期页面/屏幕，不能仅看到 busy=false 就认为异步导航已结束；
此前场景的“点击无效/quiet reload”失败曾由读取旧场景状态或目标被滚动区域裁切导致，
已在可复用场景中改为按实际状态/可见控件等待。产品排序、会话与文件结果由独立断言验证。

所有生成数据、报告、样本与运行截图留在 .local/validation，不提交。
归档目标为 .local/validation/artifacts/2026-10-23-workcrate-page-order。
最终图像对照：

- [矩阵](../../../.local/validation/artifacts/2026-10-23-workcrate-page-order/page-order-matrix.png)：保留三列、预览、多选和导出；每页排序图标/手柄可见，边界按钮禁用。
- [编辑页](../../../.local/validation/artifacts/2026-10-23-workcrate-page-order/page-order-editor.png)：原三栏、自动标题及纯数字纸内编号，正文对象选区保持；选中对象面板较长，页面信息需向下滚动。
- [主题/悬停](../../../.local/validation/artifacts/2026-10-23-workcrate-page-order/page-order-accent-hover.png)：原生主题变化与选中/悬停状态。
- [焦点/禁用](../../../.local/validation/artifacts/2026-10-23-workcrate-page-order/page-order-focus-disabled.png)：只读页面标题、无尺寸/像素编辑；物料输入仍可编辑，草稿期间排序控件禁用。
- [小窗](../../../.local/validation/artifacts/2026-10-23-workcrate-page-order/page-order-small-window.png)：1600×1100，控件保留，更多页面/检查器内容滚动访问。

## 边界

一次拖动移动一页；没有声称支持批量拖动、多页键盘排序或拖到屏外自动滚动。
原生格式由官方整体保存，不声称磁盘层是局部字节写入。新建页面、关联、显式重新同步等
真实目录/文件操作仍使用各自原有事务路径；本轮连接保持覆盖排序与原有单字段保存/切页，
不把未测试的所有功能都称为不会重连。真实 GUI 连接上的排序交互没有在本轮单独验证。

最终场景 passed=true，共 83 个归档步骤（额外读取/等待断言不计入），全部连接保持、内容保持与不重叠断言通过。
