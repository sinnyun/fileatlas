# 工程治理守卫（2026-10-16 起）

借鉴 PdfCraft（storytold/pdfcraft，浅克隆于 `%TEMP%/pdfcraft` 供查阅）的分层守卫、
命令/清单一致性与 lint 棘轮做法。本文档是本批次的唯一事实源：令牌、基线数字、
阶段复选框与进度台账都写在这里；脚本文件只承载执行逻辑，不另写说明。

## 现场事实（2026-10-16 实测）

- 根 `Cargo.toml` 已定义 `[workspace.lints]`：`todo`/`dbg_macro` deny、
  `unsafe_op_in_unsafe_fn` deny。22 个 crate 有 `[lints] workspace = true`；
  `photag-contracts`、`photag-shell` 没有该段。
- `crates/photag-plugin-preview`、`crates/photag-plugin-media-playback` 是无
  Cargo.toml、无源文件的空目录，不在 workspace members 中。
- `src/` 内 `.unwrap()`/`.expect()` 合计约 660 处，全部集中在 database(54)、
  debug(36)、dev-test(15)、file-operations(17)、gallery(105)、grouping(6)、
  i18n(20)、indexing(50)、navigation(50)、session(13)、settings(21)、
  tagging(82)、thumbnails(17)、updater(26)、watcher(29)、shell(67)、
  ui-bridge(50)。quicklook=5（全部为 expect）。
  contracts、ai、drag-drop、history、mcp、metadata-export 为 0。
- 依赖 gpui/gpui-kit 的 crate（8 个）：shell、ui-bridge、gallery、navigation、
  settings、i18n、debug、dev-test；另 metadata-export 依赖 gpui-kit。
  即 UI 栈 crate 共 9 个（含 metadata-export）。
- 跨插件深模块 import 存在（非纯 service 面）：gallery 使用
  `photag_plugin_database::db::queries::*`、`photag_plugin_indexing::scan::ScanProgress`、
  `photag_plugin_tagging::writeback::*`、`photag_plugin_quicklook::windows::*`、
  `photag_plugin_grouping::restore_collapsed` 等；settings→navigation 亦有直接依赖。
  本批次不重构这些耦合，只冻结现状、禁止新增。

## 守卫令牌表

| 令牌 | 规则 | 载体 | 违规处置 |
|---|---|---|---|
| A1 工作区完整性 | `crates/*` 含 Cargo.toml 的目录必须与 members 双向一致；缺 Cargo.toml 的目录报"孤儿目录" | scripts/check-architecture.cjs | 失败退出 1 |
| A2 跨插件依赖基线 | `crates/<a>/src` 内出现的 `photag_plugin_b::…`（a≠b）边必须 ∈ 基线快照；只减不增 | .governance/architecture-baseline.json | 新增边即失败 |
| A3 UI 栈基线 | 依赖 gpui/gpui-kit 的 crate 集合冻结；数据/服务插件（database、thumbnails、indexing、watcher、tagging、session、file-operations、quicklook、drag-drop、grouping、history、ai、updater、i18n 除外）禁止新增 UI 依赖 | 同上 | 新增即失败 |
| A4 unwrap/expect 棘轮 | 每 crate `src/**`（排除 `#[cfg(test)]`）的 `.unwrap()`/`.expect(` 计数只减不增 | .governance/debt-baseline.json | 上升即失败并列出增量文件 |
| A5 lint deny | 债务为 0 的 crate 在 `src/lib.rs` 顶部加 `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]`（`[lints] workspace=true` 不允许本地 lints 键，故走 crate 属性）；清单由脚本核对，"已清零但未 deny"与"有 deny 但有债务"都算漂移 | check-architecture.cjs + 各 crate lib.rs | 漂移即失败 |
| L1 清单一致性 | functional-development-plan 阶段表状态词 ∈ {已完成, 进行中, 随开发执行, 未开始}；文中相对链接（docs 内）目标必须存在 | scripts/check-feature-ledger.cjs | 失败退出 1 |

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
- [ ] P5 后续（本批次不做，仅登记）：命令注册表统一 UI/MCP/CLI；预算型滚动
      基准测试；债务 crate 逐批清零后补 deny。

## 已知偏差

- A4 债务计数包含 crate 源码内 `#[cfg(test)]` 模块的 unwrap（测试惯用法）；
  棘轮按总数只减不增，未来若测试增长导致误报，再实现 cfg 感知的行排除。
- A2 只冻结 crate 级边，不约束 `database::db::queries` 这类深模块路径；深耦合
  收敛属 P5 之后的独立改造。

## 本地命令

- `node scripts/check-architecture.cjs`
- `node scripts/check-feature-ledger.cjs`
- `node scripts/check-ui-refresh.cjs`
- `cargo +1.97.1 check -p photag-plugin-mcp --locked -j 2`（受影响 crate）

## 进度台账

| 日期 | 阶段 | 结果 | 证据 |
|---|---|---|---|
| 2026-10-16 | P0 | 文档建立，现场事实实测写入 | 本文件；grep/find 计数见上 |
| 2026-10-16 | P1–P4 | 全部完成：8 处源码改动、2 脚本、1 基线、CI 2 步；clippy(fmt/ledger/refresh/architecture) 全绿；伪边注入探针验证棘轮生效 | `node scripts/check-architecture.cjs` → passed (edges=75, debt=686, ui crates=9)；`check-feature-ledger.cjs` → passed；`cargo +1.97.1 clippy -p 六denycrate / -p shell -p contracts` → Finished 无 error |
