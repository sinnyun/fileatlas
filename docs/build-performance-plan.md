# Rust 构建与增量开发优化清单

## 目标与适用范围

本文记录 FileAtlas Rust workspace 的构建提速方案，供后续开发和 CI 调整时按证据逐项评估。项目是 Windows 桌面应用，使用 GPUI/GPUI Kit、Cordis 和 14 个 workspace crate；不是 Rust WASM 网页应用。优化目标分为：

1. 缩短源码修改后的本地反馈时间。
2. 缩短 CI 冷构建和重复构建时间。
3. 控制 Windows 原生最终链接耗时。
4. 保持构建可复现、MSRV 兼容及现有插件边界。

不要把“编译”“链接”“测试执行”和“应用热更新”混作一个指标。Rust/Cargo 没有可直接替代 Vite 并为 GPUI 应用提供模块热替换的通用方案；改动 Rust 代码仍要编译，桌面应用通常还需重启。

## 当前基线

- **本机工具链前置规则**：默认 Rust 1.94.1 低于项目要求，但已安装可用的 1.96.0/1.97.1。先运行 `rustup toolchain list`，本机统一使用 `cargo +1.97.1 ...`；下文通用 `cargo` 命令在本机执行时均需加此工具链选择。版本拦截不能作为构建性能或代码编译结果。详见[工具链说明](rust-toolchain.md)。

- 根 `Cargo.toml` 将 `fileatlas-shell` 设为 `default-members`，日常根目录 `cargo check` 只选择 shell 及其依赖图；Shell 会带入注册的插件。
- `[profile.dev]` 已设置 `debug = 0`、`incremental = true`。`codegen-units` 未设置，沿用 Cargo 开发 profile 默认值（当前默认 256）。
- `[profile.release]` 使用 `opt-level = 3`、`lto = "thin"`、`codegen-units = 1`；这是发布优化配置，不适合作为日常迭代构建。
- `.github/workflows/rust-checks.yml` 已使用 `Swatinem/rust-cache@v2`，设置 `CARGO_INCREMENTAL=1`、`CARGO_BUILD_JOBS=2`、`CARGO_PROFILE_DEV_DEBUG=0`，并运行格式检查及 `cargo check -p fileatlas-shell --locked`。
- workspace 有多个 GPUI 相关 crate；修改 shell 或公共 crate 时，下游重编译可能覆盖增量编译带来的收益。
- 已有 validation report 的云端验证包含 Linux 编译结果；Windows MSVC GUI/链接行为仍须在 Windows 工具链上测量。

## 优化清单（按优先级）

### P0：建立可比较的构建基准

**动作**

- 在相同机器、相同 Rust 工具链、相同锁文件和相同目标平台下，分别记录首次构建、无改动复跑、单个叶子插件改动、公共接口改动的耗时。
- 使用 Cargo `--timings` 保存 HTML 报告，关注最慢 crate、可并行区间、Rust 编译与最终链接阶段。
- 至少分别测量本地 Windows MSVC 和现有 Linux CI；不要从 Linux 数值推断 Windows 链接器收益。

**建议命令**

```powershell
cargo check -p fileatlas-shell --locked --timings
cargo check -p fileatlas-plugin-file-manager --locked --timings
cargo build -p fileatlas-shell --locked --timings
```

完整 `cargo build` 用于评估产物生成和链接，不作为每次编辑后的默认检查。保留 `target/cargo-timings/` 报告用于前后对比；每项实验一次只改变一个变量。

### P1：保持快速开发路径和细粒度命令

**动作**

- 日常先使用 `cargo check -p <当前 crate>`；需要检查应用集成时使用 `cargo check -p fileatlas-shell`。
- 保留 `[profile.dev] incremental = true` 和 `debug = 0` 作为当前快速编译基线。`debug = 0` 会减少调试信息；若需要更好的本机调试体验，再单独比较 `debug = "line-tables-only"` 的耗时、磁盘占用及调试器体验。
- 迭代时避免 `--release`；release profile 的 LTO 和单 codegen unit 会增加构建工作量。
- 保留 workspace `default-members = ["crates/fileatlas-shell"]`；它已让根目录命令面向主应用，而非默认构建所有成员。

**命令参考**

```powershell
cargo check -p fileatlas-plugin-file-manager --locked
cargo check -p fileatlas-shell --locked
```

Cargo 增量编译主要帮助 workspace 成员和 path dependency 的后续重编译；依赖接口或 feature 配置变化仍可能使多个 crate 重编。增量缓存放在 Cargo 构建目录中，清理 `target` 会丢失它。

### P1：编辑时自动反馈

**候选工具**

- `rust-analyzer`：编辑器内持续提供诊断和语义信息，通常是首选反馈方式。
- `bacon`：可监视源码并持续运行指定 Cargo 命令，例如 `bacon -- cargo check -p fileatlas-shell`；适合偏好终端界面的开发者。
- `cargo-watch`：类似的文件监视器，可重跑 Cargo 命令；与 bacon 功能重叠，按个人工作流选一个即可。

这些工具自动触发 Cargo 检查，不替代 Cargo 增量编译，也不提供 GPUI 应用的 Rust 模块热替换。首次检查仍需编译依赖。

### P1：优化 CI 缓存策略（先比较，再替换）

**当前方案**

CI 已使用 `Swatinem/rust-cache` 缓存 Cargo/target 内容。先观察连续 workflow 的缓存恢复、缓存保存和构建耗时；避免再叠加重复缓存同一批 `target` 文件的 action。

**候选方案：Mozilla `sccache`**

- 用于跨 CI 运行复用 rustc 编译结果，适合冷启动比例高、构建成本明显的场景。
- Rust 增量编译与 sccache 有取舍：sccache 的 Rust cache 文档要求关闭 incremental 才能缓存兼容的 rustc 调用。因此应与现有 `CARGO_INCREMENTAL=1` + rust-cache 做独立 CI 基准对比，而不是无测量地一起打开。
- 可先在单独分支/临时 workflow 中测试 `CARGO_INCREMENTAL=0`、`RUSTC_WRAPPER=sccache`、可用的远程缓存后端，并记录 cache hit/miss 和总耗时。只有稳定变快时才纳入主 workflow。
- sccache 主要改善重复/冷构建；它不会让同一 crate 内小改动像按源文件粒度那样独立编译。

### P2：按证据评估 Windows 链接器

**候选方案：LLVM `lld-link`**

- 仅当 Windows `cargo build --timings` 显示最终链接占显著时间时评估。
- MSVC 目标对应 `lld-link` 风格；`-fuse-ld=lld` 是 GCC/Clang 风格选项，不应照搬到 `x86_64-pc-windows-msvc` 配置。
- 在隔离的本地配置/分支中，以 MSVC `link.exe` 与 LLVM `lld-link` 对相同干净和增量构建做比较，并验证 GPUI、SQLite bundled、图片/视频及其他原生依赖的链接结果。
- 在通过 Windows 验证前，不提交为所有开发者默认启用的链接器设置。

### P2：定位泛型与依赖 feature 成本

**动作与工具**

- 若 timings 显示某个 Rust crate 本身的编译/codegen 时间突出，可用 `cargo-llvm-lines` 检查泛型单态化是否过多；它用于定位代码生成规模，不会自动优化，也不能完整代表所有依赖的成本。
- 用 `cargo tree -e features -p fileatlas-shell` 检查实际构建的依赖 feature；只有发现未使用且明显昂贵的 feature，才评估 `default-features = false` 或可选依赖。共享依赖的 feature 会合并，需核对完整依赖图及功能兼容性。
- workspace 继续保持插件公共接口稳定，避免无关插件频繁依赖变化的公共 crate。新 crate 拆分只在能形成稳定依赖边界且实测减少重编时采用，避免为提速而增加过度碎片化。
- `cargo-hakari` 可在大型 workspace 中生成 workspace-hack crate，统一共享依赖的 feature 集合，减少同一依赖因不同 feature 集合而重复编译。当前先通过 timings/依赖构建信息确认存在重复编译，再评估引入；它会增加生成文件和维护流程。

### P3：测试执行提速（与编译提速分开）

`cargo-nextest` 是测试运行器，可改善大量测试的调度和执行体验；它主要加速测试运行，不是 `cargo check`/Rust 编译加速器。测试套件增长后可评估，但不纳入本清单的首轮编译优化实验。当前项目的验证阶段约束仍优先适用。

## 不适用于本项目的建议

- **Trunk、`cargo-leptos`、Dioxus CLI**：服务于各自的 WASM/web 工作流，FileAtlas 当前是 GPUI Windows 桌面应用。
- **Linux `mold`、macOS `zld`**：不是当前 Windows MSVC 目标的链接器方案；现有 Linux CI 也不是交付 Windows 桌面二进制的链接验证。
- **`-C link-arg=-fuse-ld=lld` 的 MSVC 示例**：链接器参数风格不匹配，不能原样采用。
- **`[profile.dev.package."*"] opt-level = 2`**：目的是提高开发运行时依赖的执行速度，通常会增加依赖编译时间；只有明确以运行性能换编译时间并测量收益时才考虑。
- **`codegen-units = 256`**：当前 dev profile 没有覆盖该项，Cargo 的开发 profile 默认已使用 256；显式重复设置不会自动提速。
- **“提升 50%–90%”等固定提速承诺**：没有针对本仓库、当前工具链、缓存状态和 Windows 机器的可重复基准数据支撑，不作为预期或验收标准。

## 推荐实施顺序与验收

1. 保存当前状态下的冷构建、热构建、单插件增量检查、Shell 检查和 Windows 链接 timings。
2. 为本地开发选择 rust-analyzer 或 bacon/cargo-watch 自动检查，不调整编译语义。
3. 用 CI 数据比较现有 rust-cache 与 sccache 冷构建策略；保留更快且稳定的方案。
4. 仅当 Windows 链接耗时有证据时试验 `lld-link`。
5. 仅当 timings/依赖分析显示真实热点时处理 feature、泛型或 workspace feature 统一。
6. 每项采纳的改动记录工具链版本、平台、命令、缓存状态、测量前后时间及功能/调试限制；按仓库验证约束更新 `docs/validation-report.md`。

## 已采集基准（2026-10-12）

- 本机为 Windows MSVC，`rustc 1.96.0`；当前默认 `stable` 是 1.94.1，低于 workspace 声明的 1.95，故须显式使用已安装的 `+1.96.0` 工具链。未修改 MSRV。
- `cargo +1.96.0 fmt --all -- --check` 通过。
- 按 CI 的增量编译、2 个 build jobs、关闭 dev debug 设置运行 `cargo +1.96.0 check -p fileatlas-shell --locked --timings`：完整依赖图首次编译用时 2 分 54 秒，检查通过。报告：`target/cargo-timings/cargo-timing-20261012T155111622Z-0de67308caf8ada2.html`。
- 无源码变化时重复运行同一检查：Cargo 报告 1.26 秒，PowerShell 外层计时 1.42 秒；作为当前本机热构建基线。
- 首次检查覆盖 681 个 dirty units，包括 GPUI、图像/文本依赖和所有 Shell 插件。数据说明当前冷构建主要成本来自首次编译完整依赖图；热检查反馈已较快。由于验证只运行 `cargo check`，并未构建最终二进制，尚无链接阶段数据，暂不采纳 Windows linker 改动。
- 首次报告中最长的单元是 `windows 0.62.2 (check)`，39.43 秒；其后是 `gpui-pre 0.3.8` 9.58 秒、`gpui-component 0.7.2` 8.76 秒及多个 Windows/GPUI 依赖。`cargo tree -e features -i windows@0.62.2 -p fileatlas-shell --locked` 显示 Windows API feature 由 `gpui-pre`/`gpui-pre-platform` 的 Windows 后端引入，并经 GPUI Kit 进入 shell。当前应用没有直接控制这些 feature 的安全开关；需要先确认 GPUI 上游可裁剪范围和功能覆盖，再决定是否改。
- 编译期间有现存警告：session/settings 的 `ServicePublication` 未使用，以及 metadata-export 的 `FluentBuilder` 未使用。它们未使检查失败；需在对应代码变更中按生命周期语义处理，不作为本次性能调整。
- 直接使用默认 Rust 1.94.1 执行 Shell 检查时，Cargo 因 workspace `rust-version = 1.95` 在编译前拒绝。以后本机验证继续使用满足 MSRV 的工具链。

## 参考资料

- [Cargo Profiles：增量编译、codegen units、dev/release profile](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [Cargo Workspaces：default-members 与包选择](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo Timing Reports](https://doc.rust-lang.org/cargo/reference/timings.html)
- [Cargo Features：共享依赖 feature 合并](https://doc.rust-lang.org/cargo/reference/features.html)
- [Cargo Build Scripts：`rerun-if-changed`](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache)
- [Mozilla sccache Rust 支持与限制](https://github.com/mozilla/sccache/blob/main/docs/Rust.md)
- [cargo-llvm-lines](https://github.com/dtolnay/cargo-llvm-lines)
- [cargo-hakari](https://docs.rs/cargo-hakari/latest/cargo_hakari/)
- [bacon](https://github.com/Canop/bacon)
- [cargo-watch](https://github.com/watchexec/cargo-watch)
- [rustc linker option：MSVC 与 lld-link](https://doc.rust-lang.org/rustc/codegen-options/index.html#linker)
- [cargo-nextest](https://github.com/nextest-rs/nextest)
