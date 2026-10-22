# WorkCrate 物料信息条紧凑横条与单页字段保存同进程实测

日期：2026-10-22。环境：Windows x64，Rust 1.97.1（显式 `cargo +1.97.1`），官方
VectorCraft 0.14.0（`C:\Program Files\VectorCraft\vectorcraft.exe` 与同目录
`vectorcraft-cli.exe`），基线提交 `46c22b3`，dev profile EXE
`target/debug/fileatlas-shell.exe`。

活动参考：[2026-10-21 装配工作台用户稿](../../reference-ui/concepts/workcrate-studio-user-2026-10-21.png)
与[检查器概念](../../reference-ui/concepts/workcrate-inspector-2026-10-21.png)，
登记于[界面参考入口](../../reference-ui/README.md)的「WorkCrate 装配工作台」行；
本轮要求写在[三层清单](../../reference-ui/concepts/workcrate-three-level-2026-10-20.md)
的「Material header / page fields follow-up」一节。

## 结论

1. **物料信息条是紧凑浅色横条**：左侧页码 + 醒目项目名称，右侧同一基线上以中点
   分隔符排布物料/尺寸/数量；附加字段落到主行下方的「值 + 标签」次要行。尺寸标记
   仍是页面元数据，与画板实际尺寸无关；升级信息条只移动 FileAtlas 自有节点，页面
   原有机件与画板尺寸不动。
2. **一次字段保存只影响这一页**：显式打开的官方编辑会话被复用（同一会话
   `revision` 前进，覆盖层直接拿到保存帧），未打开会话时也只重渲染该页；其它页面
   的预览文件名与磁盘文件保持不变，卡片不退成占位图。
3. **面板不再整项目重载**：保存期间 `busy` 始终为 `false`，只有该行显示「保存中」
   标记；草稿行、左侧页面栏与滚动位置保留；内容未变的草稿既不产生官方步骤也不重
   渲染。被拒绝的写入保留草稿并给出中文可执行提示，界面不出现内部角色名。

## 实现要点

- 几何与官方分工：`header_layout.rs` 是 crate 内部纯几何，官方程序通过
  `document.node` 报告每个文字框，几何模块只把这些度量换算成落位目标框，位置由
  `object.transform` 增量 `[1,0,0,1,dx,dy]` 应用。两遍排布：第一遍排主行与分隔符并
  判断是否放得下（放不下时 `group_in_primary` 为 false，物料组降级），第二遍为每个
  次要行同时落位值与标签（都用 `SUB_SIZE`）。
- 角色契约：`page_number`、`project_name`、`material|size|quantity`、`size_label`、
  `extra:<key>`、`label:<role>`、`band`，分隔符按各自索引 `separator:<index>` 测量，
  裸 `separator` 从不落位。归属信息写在原生文本节点的
  `attributes.set` 版本化备注 `fileatlas.workcrate` 里，回读时遍历 `document.json`。
- 保存三条路径：`update_info` 先比对草稿与已存记录（相同即 `changed:false`，不写不
  渲染），再判断该页是否正被显式会话持有（复用并把 `frame` 交给覆盖层），否则走路
  由会话写入并只 `store_page` 一页。ID 中段必须等于持久化绑定 UUID，绑定轮换后直接
  拒绝。

## 缺陷与修复

1. 分隔符度量按裸角色索取（功能级）。几何 `group_extent`/`build` 用 `separator` 查
   表，而驱动逐个落位 `separator:<index>`，第一次带分隔符的保存就报
   `header text size missing for 'separator'`。现按索引取用，并加
   `each_separator_is_measured_under_its_own_index` 断言「一次度量不能复用于所有分隔符」。
2. 第二遍只落位标签，附加字段的值没有度量，保存 `extra:工艺` 报
   `header text size missing for 'extra:工艺'`。现次要行的值与标签都按 `SUB_SIZE`
   落位，同时删掉降级时重复重排整组的代码。
3. 无变化草稿仍排队重渲染。`InfoSave{frame:None}` 被 UI 当成「需要重读该页」，与
   「未变内容不得重渲染」冲突。`InfoSave` 增加 `changed`，UI 在 `!changed` 时直接返回。
4. 官方智能参考线把测试拖拽吸走。`canvas_edits_land_in_the_native_document` 的正文
   拖拽落在页面上方，实测终点 x=67.640625 而 `67.640625 + 200 = 267.640625` 恰为信息条
   `project_name` 右边缘，即约 2.4 pt 的官方吸附。这是官方行为不是缺陷，测试改为在页面
   中部（600,300）落点，使界框四边与中心都远离吸附引导线。
5. 界面泄漏英文内部原因。信息条的拒绝文案原本带 `separator`、`header node not sited`
   等角色名与英文短句。现 `check_request` 拆成中文分支（页面名称为空/过长、页面内容
   过多、页面尺寸太小、字段过长或含控制字符），`describe` 把 `HeaderLayoutError` 映射
   成中文提示，其余官方回执缺失一律「请重新同步页面」措辞。
6. 长文本注入不再是可用的失败用例。官方 `text.create` 会把过长文字自动折行到画板宽
   度，1000 字符的物料值被正常保存（`已保存：物料名称`），`TooWide` 因此是兜底分支。
   场景改用确定性的空页面名称拒绝（`保存失败：页面名称不能为空，请填写后再保存`），
   并用键盘 `ctrl-a` + `backspace` 清空输入。
7. 绑定守卫仍回英文。字段保存与 `target()` 的三处 `native document association changed;
   refresh pages` 会原样出现在 `保存失败：…` 里。现统一为「本页所属原生工程的关联已
   变化，请先同步工程与预览后再保存」；`BoardsError` 各变体自身的英文 `#[error(...)]`
   仍服务日志与 MCP 回执，未改动。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -j 2
$env:FILEATLAS_VECTORCRAFT_EXE='C:\Program Files\VectorCraft\vectorcraft-cli.exe'
$env:WORKCRATE_NATIVE_TEST_ROOT='F:\AI\geren\pho_cordis\.local\native\header5'
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-boards -j 2 -- --ignored --test-threads=1
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-header-fields-20261022d --access-level full --scenario scripts/validate-workcrate-field-header.cjs
node scripts/check-ui-theme.cjs; node scripts/check-ui-refresh.cjs; node scripts/check-architecture.cjs; node scripts/check-feature-ledger.cjs; node scripts/check-layering.cjs
```

结果：fmt 退出 0；shell 构建退出 0；非忽略测试 boards 22 通过 / 3 忽略、ui 9 通过，
无失败；三个官方门控用例全部通过（19.04 s，含 `canvas_edits_land_in_the_native_document`）；
五个守卫退出 0（架构守卫的 A4 unwrap/expect 债务在检查器单元测试改写为
`assert_eq!(…, Ok(()))` 后回到基线 0）；`git diff --check` 干净。

同进程场景 82 次 MCP 调用、11 个断言段全部成立，输出
`{"headerFields":"passed","project":"a11d8921-…","board":"…:1","savingMarkersSeen":3}`：

- 端口由 `freePort()` 现取（本轮 64336）且未监听，因此走静默 headless 会话
  （`canvas.state.live == false`）；全程未启动、未连接、也未影响用户自己的
  `vectorcraft.exe --control 17979` 实例，运行前后官方进程基线一致。
- 连续三个字段保存（物料、数量、尺寸）每次都断言：会话仍打开且仍指向该页、官方
  `revision` 前进、卡片 `canvas_revision` 等于该 revision、`preview_pending == 0`、
  `busy == false`、另一页 `preview_ready` 仍为真且预览文件名与磁盘文件未变；
  其中 3 次保存期间观察到该行自己的在途标记。
- 附加字段：`+ 添加字段` 填「工艺」→ `已保存：字段 工艺`，再填值「模切」写回。
- 继承与重置：本页专用名称写入 `project_name_override`，`恢复继承` 后为 `null`，
  提示 `已保存：项目名称`。
- 无变化草稿：官方 `revision` 不变、`preview_pending == 0`、预览文件名不变。
- 失败保留：空页面名称 → `保存失败：页面名称不能为空，请填写后再保存`，草稿行仍
  打开（`editing == "<board>#title"`）、会话 revision 不变、页面不重渲染、卡片标题
  不变，提示中无 ASCII 内部标识；`取消` 后草稿关闭。
- 原生持久化：`.vectorcraft` 文本含 `fileatlas.workcrate`、`page_number`、
  `project_name`、`material`、`quantity`、`band`、`label:extra:`；显式
  `refresh_project` 从磁盘读回全部字段且附加名称保持继承。
- 静默：稳定面板两次读取（间隔 2.5 s）快照完全相同。

截图（本地 `.local/validation/`，不入库）：
`profiles/wc-header-fields-20261022e/screenshots/header-band-before-fields.png`、
`header-band-after-fields.png`、`header-field-save-failure.png`、
`header-pages-final.png`；结果 JSON
`profiles/wc-header-fields-20261022e/header-fields-result.json`，运行日志
`.local/wc-header-run5.log`。上一轮运行（`…20261022d`，日志 `.local/wc-header-run4.log`）
同样通过；改完中文提示与 clippy 清理后重新构建并复跑，最终证据以上面这份为准。

## 与手绘稿的差距

对照 [2026-10-21 装配工作台用户稿](../../reference-ui/concepts/workcrate-studio-user-2026-10-21.png)：
横条结构、左右分区、同一基线的物料组与下方次要行都已实现（见
`header-band-after-fields.png`、`header-field-save-failure.png` 的「工艺 模切」行）。
手绘稿里项目名称字号更大、页码与名称间距更紧，实测按官方报告的度量排布，字号令牌为
页码 12 pt、名称 20 pt、物料组 13 pt、次要行 11 pt，与稿面观感一致但不做像素对齐。

## 未覆盖项

- 实时 GUI 路由下的会话复用：本轮场景强制空闲端口走 headless，用户 GUI 在自己
  文档上被面板写入的路径仍只有 10 月 21/22 日画布场景的覆盖。
- 窄窗口与超长字段的信息条观感：只断言放得下与拒绝文案，未做小窗口截图验收。
- 附加字段超过 12 行（`MAX_SUB_ROWS`）与 `TooWide`/`TooManyRows` 分支：官方自动折行
  使这些分支在真实文档上难以触发，仅有单元测试覆盖。
- 信息条在 PNG/JPEG/PDF 导出结果里的一致性：本轮只验证原生文档与预览位图，
  导出链路仍沿用 Export 端口既有验证。
- 主行降级（物料组落到次要行）未在真实文档上实测，仅 `group_in_primary` 单元测试。
