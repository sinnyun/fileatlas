# 源码基准版本

以下上游提交共同构成本仓库的迁移基线。选定的 PhoTag 源码完整镜像位于 `vendor/photag-source/`。

| 项目 | 地址 | 提交 | 用途 |
|---|---|---|---|
| PhoTag | https://github.com/OleksandrShabaldas/PhoTag | `36cee242a78c0a8cd9ac6b0d63b64265aa0add90` | 功能与行为基准 |
| cordis-rs | https://github.com/dshbox/cordis-rs | `d77cf919964f657b122b5eac9a7f723c0b002ca8` | 生命周期、服务、事件和依赖管理 |
| GPUI Kit | https://github.com/longbridge/gpui-kit | `1e7ffaa6d1c0c230cb84099584e6c057531d5a5d` | 原生 GPUI 组件和应用界面 |

PhoTag 基准提交日期：2026-08-04。运行时和 UI 工具包于 2026-10-12 从各自默认分支克隆，并固定在此版本以供 API 审查。
