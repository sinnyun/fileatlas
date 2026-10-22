# WorkCrate 每页只保留一帧预览（2026-10-23）

用户报告：每个 WorkCrate 项目数据目录里 `native-previews/` 与 `canvas/` 两个文件夹的画板图片
完全相同。要求按方案一修复——官方渲染直接落到持久预览文件，取消 `canvas/` 镜像；并特别要求
确认"不会每次渲染都新生成一个图片文件而不替换原有文件"。

## 重复来源与存储现状

审计本机 148 个项目的相邻数据：119 对 `canvas/<画板号>.png` 与 `native-previews/<token>.png`
字节完全相同，共 23.1 MB 重复，`native-previews` 内无未被引用文件（脚本
`.local/audit-workcrate-previews.cjs`，只读）。原因是旧实现把一次渲染写两份：编辑会话的渲染
目标是 `canvas/<native_id>.png`，发布时再 `std::fs::copy` 到 `native-previews/<时间戳>-<序号>.png`；
`canvas/` 既不能被缓存校验器引用，也不在回收范围内，迁移复制还会把它一起带走。

当前形态（本次改动后）：`native-previews/` 是唯一帧目录，文件名
`page<画板原生号>-<关联UUID前8位>-1600.png` 只由页面与原生文档关联决定，与渲染次数无关。
`make_previews`、页面编辑 `canvas_open`、字段保存、页面排序发布都写这一个路径；
`mcp::McpSession::render_preview` 用同目录临时文件加原子改名原地替换，因此重复渲染覆盖自己
的文件，不产生新文件。`write_cache` 独占回收：新映射丢弃的帧在写入成功后删除、写入失败时删除
新写的帧，并删除旧版遗留的 `canvas/` 镜像目录。缓存键仍是 `{board_id}@1600`，schema 不变，
无迁移、无新存储。改关联（`attach_document`）会轮换 binding，旧帧全部作废并由新关联命名。

## 验证

Windows、150% DPI，Rust 1.97.1 MSVC，正常 `target/debug/fileatlas-shell.exe`；运行验证使用
FileAtlas 同进程 MCP stdio 与隔离 Standard profile；原生编辑使用官方 0.14.0 headless 会话。
验证前确认无 `vectorcraft.exe`/`vectorcraft-cli.exe` 残留进程，验证后同样为空（未结束任何用户 GUI）。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-boards
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-ui
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-dedup-2026-10-23 --access-level standard --scenario scripts/validate-workcrate-preview-dedup.cjs
```

静态与自动化通过：fmt、四项治理脚本、Boards 31 通过（4 项既有官方集成测试仍 ignored）、
Projects 12、UI 12；构建仅既存 LNK4098 警告。Boards 新增三项回归：帧名按页与关联稳定且互不
相同、发布后旧帧与 `canvas/` 镜像被回收且二次渲染只改字节不增文件、目录外路径（含 `canvas/`）
一律拒绝发布。

同进程 MCP 场景（`scripts/validate-workcrate-preview-dedup.cjs`，退出码 0）实测：新建两页后目录里只有 `native-previews/` 两个
`page1-…-1600.png`/`page2-…-1600.png`，无 `canvas/`；页面矩阵两卡片读缓存帧且文件数不变；
`canvas_open` 回报的渲染路径就是该缓存帧（同一文件，非副本）；连续两次字段保存后帧文件哈希
变化但文件名与文件清单完全不变；切页两次不新增文件；手工写入旧版 `canvas/1.png` 后下一次发布
即删除该镜像且两帧仍在；缓存引用集合与目录列举完全相等；`open_board` 重开后叠加层仍指向同一
帧；静置 3.5 秒 `reloads` 不增长。

## 界面对照

本轮不改界面结构，只验证既有页面矩阵与编辑叠加层仍正常读取缓存帧，故沿用活动编辑工作台用户稿
的既有对照结论，未新增概念图。截图为同进程窗口实拍。

| 检查项 | 实际表现 | 证据 |
|---|---|---|
| 页面矩阵读缓存帧 | 两卡片显示官方帧，未额外渲染 | [页面矩阵](../../../.local/validation/artifacts/2026-10-23-workcrate-preview-dedup/dedup-page-grid.png) |
| 编辑会话帧 | 编辑层显示保存后的"第二版物料"内容 | [编辑帧](../../../.local/validation/artifacts/2026-10-23-workcrate-preview-dedup/dedup-session-frame.png) |
| 重开后仍是一帧 | 左侧两卡片与右侧叠加层同源，帧文件未增加 | [重开编辑](../../../.local/validation/artifacts/2026-10-23-workcrate-preview-dedup/dedup-editor-reopened.png) |

## 剩余缺陷与未覆盖项

- 场景调试中稳定复现一个既有面板缺陷（与本次存储改动无关）：从页面矩阵经远程选页
  （`workcrate-ui.navigate` 后立刻 `workcrate-ui.open_board`）进入编辑页时，若上一次离开的
  目录刷新仍占着 `busy`，`ensure_canvas_session` 直接返回且不重试，编辑层停在无会话状态，
  需要再一次交互才会打开。场景改为等待面板空闲后再进入，故本轮通过；未修改该进入路径，
  留作后续单独修复与验证。
- 首轮实现按"渲染临时文件＋改名收养"设计，实测发现官方会话回报的帧路径是带 `\\?\` 前缀的
  `/` 分隔字符串，`strip_prefix` 判定为目录外而拒绝发布；已改为在边界还原原生分隔符并由单元
  测试覆盖两种写法。
- 未覆盖：真实用户手工点击卡片进入编辑的连续路径、GUI 与 headless 混合会话下的帧覆盖时序、
  只读/占用文件的删除失败分支、跨磁盘移动与断电时刻。旧数据集里已存在的 `canvas/` 镜像只在
  下一次发布时清理，未发布过的项目仍保留原目录（不主动删除用户文件）。
- 生成数据、profile、日志与截图仅位于 `.local/validation/`；截图与清单副本在
  `.local/validation/artifacts/2026-10-23-workcrate-preview-dedup/`，不纳入 Git。
