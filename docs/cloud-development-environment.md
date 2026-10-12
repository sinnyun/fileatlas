# 云端开发环境基础说明

本文供接手 `pho_cordis` 的 AI 在云端开发环境中快速检查和配置环境。目标项目是 Rust + Cargo Workspace 的 Windows 本地素材管理软件；云端 Linux 主要用于格式检查、非 GUI 插件测试和 GPUI Shell 编译，不能代替 Windows GUI 与 Windows Shell 实机验证。

## 1. 开始前先确认工作区

不要假设会话已经挂载仓库。先在终端运行：

```bash
pwd
ls -la
git rev-parse --show-toplevel
git status --short --branch
git remote -v
```

如果 `git rev-parse` 失败，当前目录不是仓库。先使用云端环境提供的仓库挂载/克隆能力获取 `https://github.com/sinnyun/pho_cordis.git`，再进入仓库根目录。不要在空目录里新建一套同名 Cargo 项目，也不要仅凭先前会话摘要推断源码状态。

确认当前分支、未提交修改和远端后，再阅读：

- `AGENTS.md`：项目开发约束与任务要求。
- `README.md`：当前迁移范围和仍未实现的功能。
- `SOURCE_PINS.md`：PhoTag 源码基准版本。
- `docs/source-feature-audit.md`：原项目功能与迁移映射。
- `docs/validation-report.md`：实际验证记录与已知问题。
- `Cargo.toml`：workspace 成员、MSRV 和共享依赖。

不得覆盖或丢弃工作区中已有的用户修改。项目要求保留 PhoTag 原有功能；新增实现应沿用 `vendor/photag-source` 的行为并更新功能审计/验证记录。

## 2. 工具链和依赖

仓库根 `Cargo.toml` 声明 Rust edition 2024、最低 Rust 版本 1.88，并使用稳定版工具链。至少准备：

- Rust stable、Cargo、rustfmt（`rustup component add rustfmt`）。
- Git。
- Linux x86_64 系统开发包：`libxcb1-dev`、`libxkbcommon-dev`。

Ubuntu/Debian 配置命令：

```bash
sudo apt-get update
sudo apt-get install --yes git build-essential pkg-config libxcb1-dev libxkbcommon-dev
rustup toolchain install stable --component rustfmt
rustup default stable
rustc --version
cargo --version
```

CI 已明确安装 `libxcb1-dev` 和 `libxkbcommon-dev`。如果构建明确报告其他缺失的原生库，再根据错误补装对应开发包，并把经过验证的环境变化同步到 CI；不要无依据地堆叠系统依赖。

Linux 首次解析依赖需要网络。仓库有 `Cargo.lock`，开发/CI 命令应优先使用 `--locked`。依赖已缓存时可使用 `--offline` 排查，但离线失败本身不能证明代码有问题。

## 3. 推荐验证顺序

每次改动后在仓库根目录运行：

```bash
cargo fmt --all -- --check
cargo test --workspace --exclude photag-shell --locked
cargo check -p photag-shell --locked
```

如果要自动修复格式，再运行 `cargo fmt --all`，并检查差异。修改单个插件时也运行该插件的测试，例如：

```bash
cargo test -p photag-plugin-gallery --locked
cargo check -p photag-shell --locked
```

当前 CI（`.github/workflows/rust-checks.yml`）执行格式检查、排除 Shell 的 workspace 测试和 Shell 编译。仓库验证记录还说明，完整 `cargo test --workspace` 曾在 GPUI 归档阶段遇到 Rust 的对象文件映射错误；应保留非 GUI 测试与 Shell 编译分开执行的诊断路径，只有复现并定位后才调整测试策略。

不要把“编译通过”表述为“功能已完成”或“与 PhoTag 等价”。同步更新 `docs/validation-report.md`：只记录本次真实执行的命令、环境、结果和未覆盖项目。

## 4. Windows 验证边界

应用涉及 Windows 原生窗口、Shell 操作、拖放和回收站等能力。Linux 环境不能验证：

- Windows GUI 实际运行和交互。
- Windows 文件操作、Shell 拖出/拖入和回收站行为。
- Windows 文件通知行为及真实素材库兼容性。

这些项目只能标记为“待 Windows 实机验证”，不能用 Linux 编译结果代替。GPUI 相关构建失败时，先保存完整错误、区分 Rust 代码问题与系统库/链接/内存资源问题，再采取最小修复。

## 5. AI 接手与提交规范

1. 以当前仓库文件和 Git 状态为准，重新核对最近提交及尚未完成的功能；不要重复已完成的功能，也不要按摘要臆造 API。
2. 一次聚焦一个可审查的功能切片。先追踪 PhoTag 原实现及功能审计，再迁移行为到对应插件，并保持 Cordis 的插件、Service、Event 和依赖生命周期。
3. 代码按仓库既有 crate 边界组织；避免超大文件，补充能覆盖行为的测试。
4. 运行与改动相称的验证，检查 `git diff --check`、`git diff` 和文件行数，更新审计/验证文档。
5. 按用户已授权的节奏提交并推送。提交前确认目标分支、提交内容和身份验证可用；推送失败时报告具体原因，不得声称已推送。
6. 最终交接需列出实现内容、提交号、实际执行的验证、未通过项以及仍需 Windows 实机验证的内容。

## 6. 本仓库当前事实（2026-10-12）

- 默认分支：`main`；工作区根目录包含 10 个 Cargo 成员。
- Rust 最低版本：1.88；依赖版本和锁文件以仓库当前内容为准。
- CI 使用 Linux Ubuntu、Rust stable 和 rustfmt。
- 本地验证优先采用“非 Shell workspace 测试 + Shell check”的两阶段方式。
- 迁移仍未达到 PhoTag 全功能等价；以最新 `README.md` 和功能审计为准，不要把该状态固定为未来事实。
