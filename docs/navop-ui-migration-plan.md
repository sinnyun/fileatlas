# Navop 与 gpui-kit-material 界面迁移调研和规划

状态：方案阶段（2026-11-06）。本文仅记录调研和迁移规划，不包含程序代码修改。

## 结论

Navop 展示了一套成熟的 GPUI 桌面工作区产品，但其仓库 UI 与应用业务、状态和自有 GPUI Kit fork 紧密结合。适合借鉴视觉组织和交互模式，不能假定整页源码可直接放入 FileAtlas。

gpui-kit-material 是 Material 3 / M3 Expressive 的 GPUI 主题、token、动效和组件适配层，建立在 GPUI Kit 控件之上。技术方向与 FileAtlas 相近，MIT 许可也比 Navop 作者代码更适合作为组件候选；但它当前标注为 0.1 prerelease，组件尚无独立 conformance review，README 明确称没有达到 production standard。更关键的是它要求 GPUI Kit 与多个 GPUI 底层 crate 使用同一特定 fork/revision，并需 workspace 级 `[patch]`。FileAtlas 当前已有 GPUI Kit 原生 Theme、UI Bridge 唯一主题入口和八类语义色规则，故不应直接引入第二套 Material Theme runtime 或整体替换 GPUI 栈。

推荐方案：将 gpui-kit-material 作为设计参考和可选组件来源，先审查其 MIT 许可、依赖补丁与 FileAtlas 当前版本兼容性；优先提炼 Material 视觉 token/模式并映射到 FileAtlas 原生主题。如果需要直接复用组件，先选一个独立控件做兼容性验证，禁止在生产 UI 大范围采用尚未达到生产标准的组件。Navop 继续作为工作区布局与视觉参考，其补充许可下的作者代码在用途许可明确前不复制。

## 项目调研

### Navop

- 仓库：<https://github.com/feigeCode/navop>。README 称其为 Rust + GPUI 原生应用，无 WebView，并具备主页工作区、树形导航、连接管理、标签和多业务面板。
- 根 manifest：<https://github.com/feigeCode/navop/blob/main/Cargo.toml>。workspace 使用 feigeCode 的 gpui-kit 固定 revision 和 GPUI Fast；业务 view 分布于多个 crate，与 Navop 领域模型耦合。
- README 许可证说明：<https://github.com/feigeCode/navop#license>。Apache-2.0 之外，作者代码还受 [Navop Supplementary License](https://github.com/feigeCode/navop/blob/main/NAVOP_LICENSE) 约束；其限制包括竞争产品/服务及收费分发。不可仅凭 Apache-2.0 文件认定作者代码可自由用于 FileAtlas。
- 视觉迁移的合适对象是工作区层级、导航和内容的关系、标签与面板组织、控件密度、空状态及状态反馈。Navop 的数据库/SSH 等页面不对应 FileAtlas 文件浏览行为。

### gpui-kit-material

- 仓库：<https://github.com/mizorewww/gpui-kit-material>。README 将其定义为 GPUI Kit 控件的 Material 3 / M3 Expressive 视觉适配层：GPUI Kit 负责控件行为、焦点、无障碍和 overlay；该 crate 提供 Material 外观与动效。
- 包含内容：独立颜色角色/调色板 crate、主题运行时、Material tokens、motion、Material Symbols 图标，以及按钮、选择、输入、导航、容器、反馈、PaneScaffold 等组件；另有组件 Gallery。
- 依赖结构：使用 Git revision 分发。README 的集成示例要求 gpui-kit 与 gpui-kit-material 指向配套 revision，并在 workspace 根 manifest 对 gpui-base、gpui-pre、gpui-fast 等添加多个 `[patch]`。这会影响整个 Cargo workspace 的解析，不是仅加一个 UI crate 就能隔离的依赖。
- 成熟度：README 标记 pre-release (0.1)，称组件尚未稳定，没有组件通过独立 conformance review，且没有达到 production standard；详细差距在该项目的 COVERAGE、ROADMAP 和 conformance 文档中。因 API/revision 动态，实施前应重新锁定 commit 并查看对应版本状态。
- 许可：README 声明主体 MIT，但移植自 Apache-2.0 的文件保留原通知；Material Symbols 是 Apache-2.0，Gallery 字体有 SIL OFL 1.1 约束。采用时需保留 notices，并按实际进入产品的文件/资源确认许可范围。

## 和 FileAtlas 的结合判断

| 方案 | 判断 | 依据及约束 |
|---|---|---|
| 借鉴 Navop 的桌面工作区视觉与交互 | 推荐 | 以参考图和界面清单迁移；保留 FileAtlas 0.2 多栏文件浏览行为 |
| 直接搬 Navop 整页 UI 或应用 shell | 不推荐 | 业务状态耦合、GPUI fork 依赖差异、补充许可限制 |
| 借鉴 Material 3 token、形状、动效和层级 | 可行 | 将设计意图映射到 FileAtlas 原生 Theme 角色与 UI Bridge 组件，不增加私有主题系统 |
| 直接依赖 gpui-kit-material 组件 | 有条件可行 | 先验证精确 revision 的 API、底层 patches、Windows 编译/运行、主题接管方式和组件成熟度；先单组件试点 |
| 复制 gpui-kit-material 源码 | 许可上相对可行，工程上仍待审 | MIT/第三方 notices 清楚，但须验证组件依赖和当前 FileAtlas GPUI Kit 兼容性；复制后维护成本由 FileAtlas 承担 |
| 套用完整 Material 3 主题运行时 | 当前不建议 | FileAtlas 的主题唯一由 GPUI Kit ThemeRegistry/ThemeConfig/Theme 管理；Material 运行时若形成第二个颜色/主题真相源即违反现有约束 |
| 加入 Material Symbols | 候选 | 检查 Apache-2.0 notices、所需字形和打包体积；图标须进入共享基础设施而非各插件私自绘制 |

FileAtlas 已有 `fileatlas-plugin-material-theme`，它负责依据种子/风格生成 GPUI 原生 ThemeSet JSON，不应用自己的 paints。名字相近不代表两项目职责相同：gpui-kit-material 是另一套运行时 token/控件风格库；FileAtlas Material 插件是现有主题 JSON 生成器。必须保持后者生成、GPUI Kit 解析、UI Bridge 作为应用入口的既有管线。

## 推荐迁移方案

### 阶段 0：锁定证据

- Navop 和 gpui-kit-material 各选定 commit/tag，并保存源码定位与截图来源。
- 阅读 gpui-kit-material 对应版本 `docs/ARCHITECTURE.md`、`docs/PRODUCTION.md`、`docs/COVERAGE.md`、`docs/ROADMAP.md`，记录目标控件成熟度和已知缺口。
- 检查 FileAtlas GPUI Kit 精确 git revision、ThemeRegistry 公共 API、UI Bridge 组件接口，与 gpui-kit-material 需要的版本逐项比对。
- 对目标 crate 做依赖树/patch 影响评估，尤其是 gpui-fast、gpui-base、gpui-pre 是否会造成全 workspace 版本统一或冲突。
- 建立组件/资源许可清单，记录源文件、版权通知、依赖、许可证、拟用方式和决定。

### 阶段 1：设计语言映射

- 从 Navop 选择用户喜爱的工作区/主窗口画面作为视觉参考，从 gpui-kit-material 选择具体 Material 视觉特征作为组件参考。
- 对照 FileAtlas 活动参考图与 0.2 基线，写明哪些视觉属性要吸收、哪些业务结构必须保持。
- 将 Material 颜色意图映射到最多八个 FileAtlas 语义色族和 GPUI 原生角色；不增加 role-by-role 色彩设置、独立 palette 或第二份 Theme state。
- 对圆角、层级、动效、导航密度、表面分层和状态层建立 checklist，不能把全 Material 默认密度直接套到高密度多栏文件浏览区。

### 阶段 2：单控件/单区域试点

- 选一个风险可控、可独立验证的区域，例如工作区卡片、按钮组或导航行。
- 首选复用 FileAtlas UI Bridge 的 Kit 控件及共享几何/组件接口；如引入上游组件，先做独立依赖原型并固定 revision，评估 patch 影响，不立即扩展到全 workspace。
- 若集成必须替换 FileAtlas 全部 GPUI Kit/GPUI 底层 revision，则暂停该直接依赖方案，评估按设计意图适配现有 Kit 控件，或在兼容许可下仅提取最小组件实现。
- 通过同进程 FileAtlas MCP 检查默认/变化主题、选中/悬停/焦点/禁用、窗口变窄和真实操作行为；截图按活动参考逐项核对。

### 阶段 3：逐屏推广

- 依据试点结果再选择主窗口、导航、检查器、工作区管理器等下一屏；每屏独立维护参考图、布局清单和验证记录。
- 设计语言进入共享 UI Bridge 时才作为公共组件推广；功能插件继续只提供内容与行为，不维护自己的视觉主题。
- 记录每个被采纳组件的上游 revision、许可证、API 差异、FileAtlas 本地补丁及升级办法。

## 验收条件与未决项

- 设计验收：固定上游版本和参考画面，写明层级、密度、控件位置及交互状态；同进程 MCP 截图核验，不以编译通过代替视觉验收。
- 主题验收：仅 GPUI Kit 原生 Theme 是 resolved color source；Material generator 只输出原生 ThemeSet JSON；UI Bridge 继续是应用主题 gateway。
- 依赖验收：明确 Cargo patch 对全 workspace 的影响、Windows MSVC 构建可行性、feature 选择和升级路径；固定 Git revision。
- 许可验收：gpui-kit-material 依 MIT 及文件内保留的 Apache/OFL 等 notices；Navop 作者源码先确认补充许可对具体使用方式的覆盖性。
- 组件成熟度：gpui-kit-material 当前自身声明没有组件满足生产标准。若选用仍须 FileAtlas 独立做功能、键盘/焦点、无障碍、主题、性能和平台验证，不能把上游测试等同 FileAtlas 验收。
- 待选试点：建议先以 FileAtlas 主窗口导航/工具栏为目标，是否采用 Material 组件依赖则在阶段 0 的兼容评估后决定。

## 本规划采用的项目约束

FileAtlas 0.2 主窗口和多栏工作区保持当前已接受结构；产品行为归现有 Cordis feature crates；Shell 只做顶层组合；UI 注册与主题通过 UI Bridge；GPUI Kit 原生 Theme 是唯一颜色系统。任何后续界面实现须遵循 `docs/ui-reference-workflow.md`、`docs/reference-ui/README.md`、`docs/ui-theme.md` 和 `AGENTS.md`，先形成参考图布局清单，再编码与同进程 MCP 验证。

## Argus 参考补充（2026-11-06）

### 调研结论

Argus 是 Rust + GPUI Kit 构建的现代 Windows 任务管理器，强调信息密集但清晰的系统监控界面。README 展示进程页、资源曲线、多类系统页面，支持列表/树/应用视图、搜索排序和详情面板；Windows 11 用 Mica，系统强调色与明暗主题可跟随系统。它与 FileAtlas 同属 GPUI Kit 原生 Windows 桌面 UI，因此其组件组织、信息层级、表格/列表呈现和原生主题接入比单纯网页视觉参考更接近可迁移经验。

Argus 的页面仍属于任务管理器产品：资源曲线、进程指标、服务列表等数据结构不应直接替换 FileAtlas 文件浏览结构。可借鉴的是信息密度、列对齐、轻量分隔、顶栏/导航层级、搜索与筛选组织、详情面板和选中状态反馈；将其转译到 FileAtlas 的文件列表、多栏浏览和检查器。

### 复用与许可证

Argus 仓库标注 `AGPL-3.0-or-later`。文件Atlas 用户接受 GPL-3.0 依赖，但这不自动等同于接受 AGPL-3.0；Argus 整体代码或组件代码复制进 FileAtlas，可能带来分发和网络交互场景下的额外义务。复制之前必须按具体文件核对版权、贡献者和第三方 notices，并判断 AGPL 与 FileAtlas 当前声明许可证的兼容方式。当前建议直接复用其设计经验，源码层复用保持“许可专项审查后再决定”；不因同样使用 GPUI Kit 就推断代码可以无条件复制。

Argus README 将 GPUI Kit 作为其原生组件、窗口、表格和曲线来源。是否与 FileAtlas 当前 GPUI Kit 精确 revision 相同、组件 API 是否兼容，需锁定 Argus commit 后检查其 Cargo manifest、依赖树和 UI crate。没有完成该检查前，不计划替换 FileAtlas GPUI Kit 或直接加入 Argus UI crate。

### 和 Navop / gpui-kit-material 的分工

- Navop：参考综合工作台和多业务面板的组织方式；作者补充许可证使直接复制作者代码风险较高。
- gpui-kit-material：参考 Material 3 token、动效和控件，组件库处于 pre-release，依赖 workspace 级底层 revision patch。
- Argus：参考 Windows 原生任务管理器的信息密度、数据表格和性能图表表达；整体 AGPL-3.0-or-later，直接复制需单独评估许可义务。
- FileAtlas：继续以 0.2 的文件浏览、多栏工作区为产品基准；GPUI Kit 原生 Theme 和 UI Bridge 是实际集成入口，不将三套参考应用的页面拼成一套新信息架构。

### 迁移方向与试点建议

综合三项参考，建议采用“Argus 式清晰密度 + Navop 式工作区层级 + 适量 Material 3 组件语言”的方向，但以 FileAtlas 现有布局、八类语义色和多栏行为为边界。第一候选试点为 FileManager 文件列表与工具栏：评估列信息层级、选中/悬停呈现、搜索筛选的分组和右侧检查器关系；不复制进程指标/曲线等无关产品功能。第二候选为导航树和工作区卡片，观察 Navop 的工作区组织和 Material 组件是否适合现有密度。

后续正式设计前，应分别打开并核对 Navop、Argus 的实际目标截图/窗口状态，并读取 FileAtlas 对应活动参考；随后写屏幕级布局 checklist。只有试点概念和行为清单收敛后才实施，按 AGENTS.md 使用同进程 FileAtlas MCP 做状态和截图验收。
