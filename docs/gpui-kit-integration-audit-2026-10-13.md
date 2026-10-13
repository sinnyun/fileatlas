# GPUI Kit 接入检查（2026-10-13）

## 结论

后续修复状态：2026-10-13 工作区已加入 Shell 的 Assets 注册和 Settings 的
`apply_accent_components` 同步调用。此报告下方保留发现时的证据作为历史；
最新验证结果见 validation-report.md 的修复记录。不是 Windows 交互验收。

项目确实使用 longbridge/gpui-kit，核心依赖与启动方式正确，锁定依赖的 Shell 检查通过。发现两项明确的接入遗漏：组件 SVG 资源未注册；运行时强调色未同步 GPUI Kit 全局主题。此外，自绘控件的键盘能力、UI 交互测试和上游版本文档有待完善。

本次为检查，未修改应用代码。此前工作区已有用户修改，均保留。不是完整视觉/交互验收。

## 依据与检查范围

优先查询 codebase-memory-mcp。图谱返回的 Shell 符号范围与当前文件不一致，对 apply_components/sliders 查询没有结果；随后对当前源码及实际 Cargo registry 源码逐项核对。审查官方仓库 README、Kit facade、资产说明与可执行应用示例。

- [官方仓库](https://github.com/longbridge/gpui-kit)
- [Kit 入口与 Root 包装源码](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/src/lib.rs)
- [资产注册说明](https://github.com/longbridge/gpui-kit/blob/main/crates/assets/src/lib.rs)
- [官方可执行示例与交互验证范围](https://github.com/longbridge/gpui-kit/blob/main/examples/ai_recipes/README.md)

以 Cargo.lock 及本机下载的版本为实际 API 依据，main 分支仅作官方用法参照：
gpui-kit0.7.1、gpui-base0.7.2、gpui-component0.7.2、gpui-kit-assets0.7.2、gpui-pre/platform0.3.8。这是当前锁文件允许的版本组合，不能仅因补丁版本不同判定错误。

## 正确使用的部分

| 项目 | 证据 |
|---|---|
| 单一应用入口依赖 | Cargo.toml:44 精确声明 gpui-kit =0.7.1，GUI crate 使用 workspace 依赖；未在新应用 Cargo manifest 中引入 egui/eframe |
| 正确平台与初始化顺序 | photag-shell/src/main.rs:444-446 使用 application → init → shared theme |
| Root 自动包装 | main.rs:479 使用 Kit open_window；实际 Kit lib.rs 的实现创建 base::Root，不能误判项目“缺少 Root” |
| 使用真实组件 | Gallery Input/InputState、DatePicker；Navigation DropdownMenu/PopupMenuItem；Playback Slider/SliderState；Settings/Preview/Button/Input |
| 保留状态与订阅 | Gallery/Navigation 输入状态为 Entity，Playback 订阅存在 _playhead_subscription/_volume_subscription/_speed_subscription 字段，未在每次 render 重建 |
| 共享主题 | photag-ui-bridge/src/theme.rs:151-188 设置 Theme::global_mut 的字体、表面、文本、primary 等 |
| Cordis 所有权 | UiBridge 的目录与视图注册配合 plugin disposal；本次3项现有测试通过 |

GPUI Kit 官方允许以 gpui-base 加自定义展示构建设计系统。Gallery 的 Div/canvas、自绘范围滑块和自定义层本身不是错误，也不要求为了使用 Kit 而全部改为现成组件。

## 明确发现

### P2：组件 SVG 资产未注册

位置：crates/photag-shell/src/main.rs:444。

application() 后直接 run，没有 with_assets；全项目源码未找到其他 AssetSource 注册。实际平台 application() 创建 GPUI Application，默认资产源为 Arc::new(())。Kit init() 初始化组件/Root，未注册 SVG 资产。

项目虽然主要通过 Segoe MDL2 字体绘制图标，但 DatePicker 本身使用 IconName::Calendar，菜单内部也使用 Check/ChevronRight 等 SVG 图标。依赖编译包含 assets feature 不等于运行时安装资产源。因此这些内置图标没有正确的资源来源。具体可见缺图形态仍需 Windows 交互确认。

建议启动时注册 gpui_kit::assets::Assets，例如：
```rust
gpui_kit::application()
    .with_assets(gpui_kit::assets::Assets)
    .run(move |cx| {
        gpui_kit::init(cx);
        // existing application setup
    });
```
保留当前 Cordis 和窗口结构，不增加额外直接依赖。

### P2：运行时强调色只更新自绘 token

位置：crates/photag-plugin-settings/src/view.rs:257-293；crates/photag-ui-bridge/src/theme.rs:151-188,266-274。

Settings edit 成功后调用 set_accent_name；后者只写入 AtomicU32。apply_components 只在 Shell 启动时调用一次。Theme::global_mut 中的 primary/accent/selection 保留旧颜色，自绘 token 与原生组件主题会分离。Settings 的 cx.notify() 也不等于重新设置 Theme。

建议在 GPUI UI 上下文内统一更新 Kit Theme 中的强调色相关字段，并刷新需要的窗口/视图；沿用共享 UI Bridge 主题边界，避免每个插件独立设置颜色。验证切换颜色后 DatePicker、菜单、Slider 与 Gallery 高亮一致，且重启持久化正确。

## 完善项与证据边界

- 自绘缩略图/评分滑块：gallery/src/view/sliders.rs:46-126 只有鼠标拖拽，无独立 FocusHandle、键盘处理或滑块语义。source 样式还原不能证明键盘/辅助功能完整。可复用 Base 行为或为自绘层补充；并非 API 编译错误。
- Settings 的 toggle_button 使用 On/Off 文本 Button（view.rs:325），未使用 Checkbox/Switch。功能可以成立，但状态语义与标准组件有差异，后续样式优化应与源码规则一起评审。
- 自定义 Tools 弹层、Settings 层不等同 Kit PopupMenu/Dialog；不能从编译通过推断焦点陷阱、点击外部关闭、Escape 与层级都正确。Settings 源本身是 movable window，改成标准模态框前应确认兼容性。
- 项目未发现 gpui_kit::test 或 test-support 依赖。此次3项测试为 UI ownership/service 层测试，不是按钮、输入、焦点、菜单或绘制测试。官方示例明确区分交互测试与 native review。
- SOURCE_PINS.md 宣称 UI 上游基线1e7ffaa6d1c0c230cb84099584e6c057531d5a5d；实际 gpui-kit0.7.1 的 .cargo_vcs_info.json 标识87d10ae5e1299d1be18670a37570c5402f952e34，锁文件来自 crates.io。应说明审查基线与实际运行依赖的区别，不能声称 Cargo 构建使用该 Git 提交。
- 本机默认 stable rustc1.94.1，小于 workspace rust-version1.95；仓库没有 rust-toolchain.toml。本次用已安装1.97.1显式检查成功。建议另行统一构建工具链，不把默认环境失败误报为 Kit API 不兼容。
- 本次未启动或操控 GUI，未验证系统窗口、SVG实际画面、原生对话框、鼠标、键盘、Mica、codec 或音频设备。

## 验证记录

环境：Windows，PowerShell，F:\AI\geren\pho_cordis，2026-10-13（Asia/Shanghai），bounded jobs2，dev incremental=true。

| 命令 | 结果 |
|---|---|
| cargo fmt --all -- --check | 通过 |
| cargo check --locked -p photag-shell -j 2 | 默认 rustc1.94.1 因项目最低1.95要求停止；未进入有效编译 |
| rustup toolchain list | stable(default)、1.96.0、1.97.1 已安装 |
| cargo +1.97.1 check --locked -p photag-shell -j 2 | 通过 |
| cargo +1.97.1 test --locked -p photag-ui-bridge --lib -j 2 | 通过，3 passed |
| cargo +1.97.1 tree --locked -p gpui-kit --depth 1 --prefix none | 版本组合如上 |

