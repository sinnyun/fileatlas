# 工程治理守卫（2026-10-16 起）

借鉴 PdfCraft（storytold/pdfcraft，浅克隆于 `%TEMP%/pdfcraft` 供查阅）的分层守卫、
命令/清单一致性与 lint 棘轮做法。本文档是本批次的唯一事实源：令牌、基线数字、
阶段复选框与进度台账都写在这里；脚本文件只承载执行逻辑，不另写说明。

## P1 前现场快照（2026-10-16）

以下记录的是 P1 lint 收紧前的现场状态；P1 完成后的权威计数见阶段记录和
`.governance/architecture-baseline.json`。脚本基线是当前棘轮判断依据，旧人工估数仅作历史参考。

- 根 `Cargo.toml` 已定义 `[workspace.lints]`：`todo`/`dbg_macro` deny、
  `unsafe_op_in_unsafe_fn` deny。22 个 crate 有 `[lints] workspace = true`；
  `photag-contracts`、`photag-shell` 没有该段。
- `crates/photag-plugin-preview`、`crates/photag-plugin-media-playback` 是无
  Cargo.toml、无源文件的空目录，不在 workspace members 中。
- `src/` 内 `.unwrap()`/`.expect()` 曾人工估计约 660 处，按后续守卫脚本统计，
  P1 完成后的基线为 686 处。当前文件分布曾记录为 database(54)、
  debug(36)、dev-test(15)、file-operations(17)、gallery(105)、grouping(6)、
  i18n(20)、indexing(50)、navigation(50)、session(13)、settings(21)、
  tagging(82)、thumbnails(17)、updater(26)、watcher(29)、shell(67)、
  ui-bridge(50)。quicklook=5（全部为 expect）。
  contracts、ai、drag-drop、history、mcp、metadata-export 为 0。
- 依赖 gpui/gpui-kit 的 crate（8 个）：shell、ui-bridge、gallery、navigation、
  settings、i18n、debug、dev-test；另 metadata-export 依赖 gpui-kit。
  即 UI 栈 crate 共 9 个（含 metadata-export）。
- 历史上 feature crate 直接导入 Database 的 `db::queries`、Indexing 的 `scan`、
  Tagging 的 `writeback` 与 QuickLook 的 `windows` 模块。2026-10-16 首轮收敛已将
  Database 查询门面移到 crate 根、隐藏 `db` 实现模块，并将写回/QuickLook 诊断
  访问收回各自服务；现存 Database 查询门面调用仍按精确消费 crate 列入守卫，后续
  继续评估是否迁成更窄的数据库服务方法。

## 守卫令牌表

| 令牌 | 规则 | 载体 | 违规处置 |
|---|---|---|---|
| A1 工作区完整性 | 检查 `crates/*` 下含 `Cargo.toml` 的 crate 是否为 workspace member；检查 member 目录及其 `src` 是否存在；无 `Cargo.toml` 的目录只发警告 | scripts/check-architecture.cjs | 不一致失败；孤儿目录警告不失败 |
| A2 跨插件依赖基线 | 词法扫描 `crates/<a>/src/**/*.rs` 中的 `photag_plugin_b::` 文本（a≠b），按 crate 对冻结；基线只减不增 | .governance/architecture-baseline.json | 新 crate 对失败 |
| A3 UI 栈基线 | 词法扫描 Cargo.toml 中直接声明的 GPUI/UI 栈依赖，冻结使用这些依赖的 crate 集合；新增 UI 消费 crate 失败 | 同上 | 集合新增失败 |
| A4 unwrap/expect 棘轮 | 每 crate `src/**/*.rs` 中字面匹配 `.unwrap()` 与 `.expect(` 的数量只减不增；**包含** `#[cfg(test)]` 代码 | .governance/architecture-baseline.json | 超过 crate 基线失败；当前错误信息会列出 crate 和计数，不定位增量文件 |
| A5 lint deny | 冻结基线中的 deny crate 清单及 workspace-lints 清单；检查 deny 是否仍存在、是否与基线一致、deny crate 债务是否为零 | check-architecture.cjs + 各 crate lib.rs | 与基线不符失败；目前正则只确认 `unwrap_used` 或 `expect_used` 至少出现其一，不保证两项都存在，也不检查属性位置 |
| A6 crate 分层 | 检查 Cargo workspace 本地 path 依赖不得向较低层反向依赖；检查 UI 栈依赖只允许在 L3+；检查 SQLite 连接只由 database crate 打开；跨 crate 深模块路径须列入显式例外 | scripts/check-layering.cjs + rust-checks.yml | 非法依赖或越界模块导入失败；本地路径依赖解析限当前 Cargo.toml 常见写法，深模块检查是源码词法规则 |
| L1 清单一致性 | 检查功能计划中的阶段行数量至少为 4、状态前缀在允许词表内、文档相对链接目标存在；有进展的行只要求包含 Markdown 链接 | scripts/check-feature-ledger.cjs | 检查失败时退出 1；不验证证据与声明是否相符，也不验证链接锚点 |

生成/更新基线：`node scripts/check-architecture.cjs --update-baseline`（人工审查 diff 后提交）。

## 阶段

- [x] P0 文档：本文件。
- [x] P1 lint 收紧：contracts、shell 补 `[lints] workspace = true`（wsLints
      现为 24 crate）；对 contracts、ai、drag-drop、history、mcp、metadata-export
      六个零债务 crate 以 `#![cfg_attr(not(test), deny(clippy::unwrap_used,
      clippy::expect_used))]` crate 属性开启 deny；
      `cargo +1.97.1 clippy --locked` 六个 crate 与 shell/contracts 全部通过。
- [x] P2 结构守卫：scripts/check-architecture.cjs + .governance/architecture-baseline.json
      初版（75 条跨插件边、9 个 UI 栈 crate、686 处 unwrap/expect 债务、6 个 deny
      crate、24 个 workspace-lints crate）。棘轮实测：注入伪边
      `grouping -> updater` 即失败（exit 1），git restore 后恢复通过。
      quicklook 5 处 expect 保留、不 deny；preview/media-playback 空目录以 A1
      警告列出（未被 git 跟踪，是否删除属用户决定）。
- [x] P3 清单校验：scripts/check-feature-ledger.cjs。计划表 F0–F2 行原先无行内
      证据链接被规则拦下，已核实 sidebar-review/inspector-complete/panels 记录
      内容后补链，F3 补 redesign-validation.md；现 4 行全部通过。
- [x] P4 CI 接入：rust-checks.yml 在 check-ui-refresh 后新增
      check-architecture、check-feature-ledger 两步（ubuntu job）。
- [x] P5a 分层守卫：scripts/check-layering.cjs 编码当前 crate 层级、peer 边、
      UI 栈边界、SQLite 连接边界和精确的既有深模块调用方；CI 每次运行。
- [x] P5b UI/MCP 命令目录：UI Bridge `Command` 元数据含 `CommandNeeds`；Shell
      UI 和 MCP `command_list`/`command_run` 共用生命周期注册表及 Cordis 命令事件。
      目前只覆盖已注册 UI 命令；通用 CLI 未实现，MCP 专用工具仍保留自身参数契约。
- [ ] P5 后续：预算型滚动基准测试；债务 crate 逐批清零后补 deny。

## 守卫边界与已知偏差

- A1 不查询 Git 跟踪状态；当前脚本对所有无 Cargo.toml 的 crate 子目录统一打印
  `untracked`，该词不是脚本核实的事实。它们只产生警告，不阻断 CI。
- A2 是正则文本扫描，不是 Rust AST 或 Cargo 依赖图分析；注释/字符串可能造成
  误报，非 `src` 目录、别名等写法可能漏报。它冻结 crate 对；A6 另以精确消费者
 例外冻结当前深模块调用方，新增消费 crate 或模块路径会失败。
- A3 冻结 UI 栈消费者集合，不限制已在集合中的 crate 增加更多 UI 栈依赖，也不检查
  所有 crate 的分层合法性。当前 exception 名单只是产品服务 crate 的设计约束说明，
  不会被脚本逐项解析。
- A4 债务计数包含 crate 源码内 `#[cfg(test)]` 模块的 unwrap（测试惯用法）；因此
  测试代码增长也可能触发棘轮。脚本只比较总量，不排除测试，也不定位具体新增行。
- A5 的零债务 deny 范围由基线清单指定，并非脚本自动要求所有零债务 crate 启用 deny；
  当前实现也未分别确认 `unwrap_used` 和 `expect_used` 两项 lint 均存在。
- L1 只验证链接路径存在，不验证 Markdown 锚点、证据内容或证据与阶段声明的对应关系；
  阶段行数量至少为 4，但脚本不要求恰好是 F0–F3。

## P5 实施可行性核查（2026-10-16）

| 项目 | 可行性 | 当前基础 | 建议边界 |
|---|---|---|---|
| UI/MCP/CLI 命令目录 | 可实施，但需先定公共契约 | UI 已通过 `UiCommandInvoked` 事件传递命令 ID；MCP 将 read/write/dev-test/UI/Gallery/debug 工具路由组合。尚无统一的产品命令目录或通用 CLI 自动化层 | 先盘点真实 UI 命令和 MCP 工具的重叠、权限、参数与生命周期；只统一产品动作，不把 debug/dev-test 工具自动暴露成用户命令。若没有 CLI 产品需求，可先做 UI/MCP 命令元数据目录，不必新建 CLI |
| 滚动/导航预算基准 | 可实施，先建立测量基线 | dev-test 支持 1k/10k/100k 虚拟记录；已有真实目录导航测量和 100k 文件验证脚本，可记录首批/全量时间及进程 WorkingSet/PeakWorkingSet | 固定隔离 profile、数据集、机器和重复次数；先报告首批时间、完成时间、峰值内存及失败/过期任务数，再确定阈值。初期作为手动/定期基准，数据稳定后再考虑 CI gate，避免把单机波动当回归 |
| unwrap/expect 债务下降 | 可实施，适合分批 | 当前字面计数基线为 686；6 个零债务 crate 已启用 deny；架构脚本已有只增不减棘轮 | 按 crate 或高风险边界逐批减少，并在归零后加入 deny。先明确测试代码是否计入；当前脚本计入 `src` 下测试模块，因此计数下降可能与生产安全性无关 |
| 守卫精度修复 | 可实施，成本低到中 | 当前守卫通过，但 A1 的 `untracked` 标签、A5 的 lint 正则和 A2 的文本扫描有上述边界 | 后续可让 A1 查询 Git 状态、A5 分别验证两项 lint、L1 要求明确 F0–F3 并验证本地证据链接锚点；A2 若要减少误报/漏报，再评估 AST 或 `cargo metadata`，不必为了更复杂而重写全部守卫 |

以上可行性判断现已由 P5a/P5b 中的实现更新；预算基准和债务清理仍未实施。
分层脚本是可审查的静态守卫，不等同于 Rust AST/Cargo 完整依赖分析。命令目录
覆盖 UI Bridge 注册命令，不会把 MCP 所有 debug/dev-test/read/write 工具自动包装
成通用命令；`command_run` 回报事件入队，不承诺异步 handler 已完成。

P5 验证（2026-10-16）：`node scripts/check-layering.cjs` 通过（24 crates、8 层、
13 条 peer 例外）；架构基线审查只增加 MCP→History 依赖边和 UI Bridge 两处测试
unwrap。`cargo +1.97.1 check --locked -p photag-plugin-mcp -j 2` 与 UI Bridge 13
项单测通过。隔离 Profile 的同进程 MCP 运行返回 4 个注册命令；Settings 命令成功
入队，History Undo 在 mutation 默认关闭时被拒绝。Shell 隔离构建使用
`target/command-registry-validation`，随后 MCP 计划通过
`scripts/photag-mcp-debug.cjs` 执行。该脚本检查的是 Cargo.toml 常见单行依赖写法和
Rust 源码路径文本；数据库守卫禁止在 database crate 外直接打开 SQLite 连接，并
冻结当前已审查的深模块入口，但它不证明所有查询都经过某一个服务方法，也不替代
完整 Rust AST/Cargo 元数据分析。

跨插件边界首轮收敛（2026-10-16）：Database 的 `db` 实现模块改为私有，查询门面
经 crate 根公开；Gallery、MCP 等调用路径不再暴露 `database::db`。Indexing 的
`scan` 实现模块也改为私有，仅公开根层进度/分类契约；Gallery 的日期解析改为根层
API。Gallery/MCP 通过 TaggingService 读写回失败状态，MCP 通过 QuickLookService
读取 Windows 诊断；守卫不再豁免 Tagging/QuickLook 内部模块，并按具体消费者冻结
剩余 Database 查询门面调用。该步收紧了实现模块可见性和服务边界，但尚未把现存
Database 查询调用逐个改成高层领域方法。

跨插件边界续迁（2026-10-16）：缩略图 `thumbs` 实现模块改为私有。Indexing 通过
`ThumbnailService::video_thumbnail_path` 获取视频缓存路径，通过
`generate_micro_thumbnail` 生成卡片微缩图；原始缓存维护时机保持不变。层级守卫中
相应的深模块豁免已删除，源码扫描未发现跨 crate 的私有实现模块路径。Database 的
`queries` 仍是 crate 根公开的兼容查询门面，业务调用均在 DatabaseService 的读写锁
回调内执行；当前迁移没有把这些调用全部改写为领域专用方法。

Database 服务边界完成（2026-10-16）：Gallery、Tagging、History、Navigation、
Indexing、Watcher、File Operations、Metadata Export 及 opt-in Dev Test 的生产
读写已迁至具名 `DatabaseService` 方法。Gallery 快照、扫描/缩略图批次、Watcher
协调和虚拟基准数据保留数据库锁及事务范围；浏览器模式下压力数据仅写会话缓存，
并清除旧版本遗留在持久库中的保留路径。产品 SQL/schema/行语义未主动变更。
`check-layering.cjs` 现在禁止 database crate 之外的生产源码调用
`with_reader`、`with_writer`、`with_cache_writer` 或直接打开 SQLite；inline 单元
测试 fixture 不在生产守卫范围内；同时禁止消费者 crate 在普通运行依赖中引入
`rusqlite`。测试 crate 可以把它留在 `dev-dependencies`。`queries` 仍公开以承载共享模型类型及兼容入口，
但禁止新增生产查询函数调用迁移到该门面；现有剩余 `queries` 调用均位于
Database 实现或测试。`cargo +1.97.1 fmt --all -- --check`、分层守卫及数据库、
开发测试、Gallery、Indexing、Tagging、Watcher 相关 crate 测试通过。Windows 原生
UI/文件系统环境行为仍不由这些自动化结果证明。

## 本地命令

- `node scripts/check-architecture.cjs`
- `node scripts/check-layering.cjs`
- `node scripts/check-feature-ledger.cjs`
- `node scripts/check-ui-refresh.cjs`
- `cargo +1.97.1 check -p photag-plugin-mcp --locked -j 2`（受影响 crate）

## 进度台账

| 日期 | 阶段 | 结果 | 证据 |
|---|---|---|---|
| 2026-10-16 | P0 | 文档建立；该日期的早期现场快照与 P1 后基线分开记载 | 本文件 |
| 2026-10-16 | P1–P4 | 全部完成：8 处源码改动、2 脚本、1 基线、CI 2 步；历史记录称 clippy 与治理检查通过，伪边注入探针验证 crate 对棘轮 | 当前复核：`node scripts/check-architecture.cjs` → passed (edges=75, debt=686, ui crates=9)，另有两个无 Cargo.toml 目录警告；`node scripts/check-feature-ledger.cjs` → passed (4 stage rows, links resolve)。Clippy 未在本次复核中重跑 |
