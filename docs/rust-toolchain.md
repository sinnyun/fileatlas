# 本地 Rust 工具链选择

本机已经安装满足项目要求的 Rust。默认版本较旧，不代表缺少可用工具链。

2026-10-13 核对结果：

| 项目 | 版本 |
|---|---|
| Cargo.toml 的 workspace rust-version | 1.95 |
| 默认 stable（Windows MSVC） | 1.94.1 |
| 其他已安装工具链 | 1.96.0、1.97.1（Windows MSVC） |
| 本机验证选择 | 显式使用 1.97.1 |

## 执行规则

每次开始构建或验证时，先检查已安装工具链，再显式选择满足当前项目最低版本的工具链。当前本机使用以下命令：

```powershell
rustup toolchain list
rustc +1.97.1 --version
cargo +1.97.1 --version
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-ui-bridge --lib -j 2
```

对 run、clippy、tree 和其他 Cargo 操作也使用 `cargo +1.97.1`。测试目标应按具体改动选择，示例不是每次都必须运行的固定测试集合。保留锁定依赖、有界并行与已有增量编译配置。

若以后工具链列表或项目最低版本变化，重新选取已安装、满足要求的 Windows MSVC 工具链。只有找不到合适工具链时才考虑安装。

不要先重复执行裸 `cargo check` / `cargo test`，再把旧默认版本导致的 MSRV 拦截报告为“没有 Rust”“项目无法编译”或“GPUI Kit 不兼容”。也不要为了绕过拦截降低 Cargo.toml 的 rust-version、使用忽略版本检查选项，或擅自更改用户的全局默认工具链。

## EXE 被运行进程占用时（用户明确要求，2026-10-18）

如果构建因 PhoTag 正在运行而无法覆盖 EXE，直接关闭占用该目标的 PhoTag 进程，
然后重新构建正常的 `target/debug/photag-shell.exe`。用户已授权这一操作，无需再次确认。
先核对进程的可执行文件绝对路径确实属于本工作区，优先正常关闭；无法退出时，
只结束已核实的对应进程，不按模糊名称批量结束其他程序。

继续复用当前 `target` 目录、依赖缓存和增量编译产物：

```powershell
cargo +1.97.1 build --locked -p photag-shell -j 2
```

不得为了绕开文件占用另取 EXE 名称、设置新的 target 目录、执行 `cargo clean`
或进行全量重编译。此前独立命名 EXE 的处理不作为后续工作范例。

## MCP 子进程

MCP 客户端通过 Cargo 启动 Shell 时，也必须在 args 中传入工具链选择：

```json
{
  "command": "cargo",
  "args": ["+1.97.1", "run", "--locked", "-p", "photag-shell", "-j", "2", "--", "--mcp"],
  "env": {
    "PHOTAG_DEV_PLUGINS": "dev-test"
  }
}
```

已构建的 photag-shell.exe 不需要 Cargo 工具链选择。其他命令包装器和自动化任务应确保传入同样的工具链参数；本次只更新文档，未修改脚本、工具链安装或全局默认配置。

## 验证记录

GPUI Kit 审查中，`cargo +1.97.1 check --locked -p photag-shell -j 2` 和 UI Bridge 的3项测试已通过。此前默认1.94.1的失败保留为历史记录，不是项目编译结论。详细证据见 [GPUI Kit 接入报告](gpui-kit-integration-audit-2026-10-13.md)。
