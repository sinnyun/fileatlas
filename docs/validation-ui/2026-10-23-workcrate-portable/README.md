# WorkCrate 项目数据随原生文件保存（2026-10-23）

用户最终要求：新建原生文件时就在旁边保存项目数据，后续直接读写这份数据；旧项目可迁移，
移动/改名整个文件夹后重新选择文件夹恢复。图片一般就在项目文件夹内部，链接与更新继续
使用 VectorCraft 自带机制，不增加自己的图片复制、重链接或原生文件改写流程。

## 存储与兼容

继续 Projects 的版本1 JSON。程序目录 projects.json 增加 defaulted locations，只作最近项目
及定位目录；权威项目数据是 `<原生文件夹>/.workcrate/<项目UUID>/project.json`，旁边保存
原有 previews、native-previews、boards-cache 和画布配置。ProjectDocument 增加 portable
及 project 快照，保留原始项目、绑定、画板身份、默认信息、封面、自定义预览和引用记录。
程序全局默认信息与官方可执行文件/端口配置不迁移。无 SQLite、Shell 产品逻辑或新插件管理器。

新建/复制原生项目自动建立相邻数据。旧项目先写相邻临时目录，发布完整目录后更新定位；
保留原程序目录旧数据作为恢复副本，后续写入相邻目录。相同文件夹内的项目按UUID隔离。
导入所有清单校验完成后才发布索引；重名、身份不符、路径越界、版本不符均拒绝。
文件夹暂时失踪不删除最近项目记录，也不阻止插件启动；需要重新选择文件夹后继续修改。
删除只移除管理记录并留存数据，不删除原生文件、源图片和相邻数据。

引用注册表中项目内路径相对项目根保存，既有公共 API 为消费者解析绝对路径；项目外引用
仍是原引用。原生图片链接完全交给官方 VectorCraft，迁移及恢复的冷路径不开官方进程。
如有旧画布会话，文件夹已移动时绝不保存回消失的旧路径，避免重建原目录。

## 验证

Windows、150% DPI，Rust1.97.1 MSVC，正常 target/debug/fileatlas-shell.exe；所有运行验证
使用 FileAtlas 同进程 MCP stdio、隔离 Full profile；原生编辑采用官方0.14.0 headless。
图谱 WorkCrate 符号无覆盖后用定向源码检索；此功能无 PhoTag 对应模块，映射已登记。

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 build --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-ui -- --test-threads=1
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-layering.cjs
node scripts/check-feature-ledger.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-portable-2026-10-23-final --access-level full --scenario scripts/validate-workcrate-portable.cjs
$env:FILEATLAS_PORTABLE_SOURCE=(Resolve-Path .local/validation/profiles/wc-portable-2026-10-23-final/portable-summary.json).Path
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-portable-reopen-2026-10-23-final --access-level full --scenario scripts/validate-workcrate-portable-reopen.cjs
node scripts/prepare-workcrate-legacy-fixture.cjs .local/validation/profiles/wc-portable-legacy-2026-10-23-final
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-portable-legacy-2026-10-23-final --access-level full --scenario scripts/validate-workcrate-legacy-migration.cjs
```

静态检查与治理全部通过；Projects12、Boards28、UI12，共52项通过，4项既有官方集成测试
仍 ignored。实际官方路径另外由本次 MCP 场景覆盖。构建仅有既存 LNK4098 警告。

- 主流程36个归档步骤：创建时相邻数据即存在；默认信息、封面、双页及字段保存；真链接
  图片；禁用CLI后的幂等迁移及文件夹移动恢复；原生文件/图片哈希不变；恢复后官方链接
  状态ok、图片ID不变；原图蓝色改红色后链接更新可见；仍有画布会话时再改名文件夹，
  重新打开不重建旧目录；项目改名写入相邻清单；3.5秒静置reload不增长；小窗显示。
- 新程序档案4步：原先零项目，未配置VectorCraft，仅选择原项目文件夹就恢复原ID、改名
  标题、默认信息、两页目录及已更新预览。
- 旧格式9步：先在隔离profile生成真实旧版绝对路径/程序目录数据，实际点击GPUI项目
  更多菜单→迁移项目数据；未配置CLI，完整迁移、原生哈希不变、旧数据保留；再改名文件夹，
  恢复两页与预览。迁移操作不是只测试新建项目上的空操作。
- Projects 单元测试还覆盖：全新注册表恢复、独立项目同目录、越界清单导入不部分发布、
  复制封面、删除保留原文件、显式改关联文件后相邻数据跟随、旧版普通项目接口兼容。

## 界面对照

实际打开活动项目用户稿 `docs/reference-ui/concepts/workcrate-projects-user-2026-10-21.png`，
按三层说明中的本次局部清单比较同进程截图。保持顶部层级、标题/搜索操作横行、卡片层级、
封面和底部入口。新打开按钮位于新建旁，迁移入口复用现有更多菜单。初版菜单长文案被截断，
已缩为“迁移项目数据”，重新构建并实测菜单文字完整。

| 检查项 | 实际表现 | 证据 |
|---|---|---|
| 更多菜单迁移 | 文案完整，真实点击执行，原有主图/归档仍在 | [菜单](../../../.local/validation/artifacts/2026-10-23-workcrate-portable/legacy-migration-menu.png) |
| 新档案恢复 | 双页及红色更新图片恢复，身份和预设不变 | [恢复后](../../../.local/validation/artifacts/2026-10-23-workcrate-portable/portable-fresh-profile-pages.png) |
| 原生图片更新 | 改名移动后更新原图，纸内显示红图 | [编辑页](../../../.local/validation/artifacts/2026-10-23-workcrate-portable/portable-link-updated.png) |
| 小窗 | 1600×1100打开/新建按钮完整，长卡片使用既有滚动区 | [小窗](../../../.local/validation/artifacts/2026-10-23-workcrate-portable/portable-small-window.png) |

本轮新控件复用原生主题按钮及busy禁用。未单独重测新按钮的更换强调色/键盘焦点/禁用截图，
也未在原生系统文件夹选择器内注入输入；选择文件夹后的同一公共服务通过MCP实测。
跨磁盘真实移动、权限失败、断电时刻、官方GUI打开中的移动，以及嵌套原生链接文件未覆盖。
未把这些边界标为通过。所有生成清单、原生文件、日志、报告、截图仅位于上述隔离profile，
截图与报告副本归档于 `.local/validation/artifacts/2026-10-23-workcrate-portable/`，不纳入Git。
