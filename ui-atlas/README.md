# FileAtlas 界面图谱（ui-atlas）

把 FileAtlas 的程序界面用 SVG 逐个拆解成可浏览、可编辑、可追溯版本的组件图谱。
这里是**描摹与评审用的图谱**，不是产品实现：产品界面仍由 Rust/GPUI 渲染。

## 事实源

1. 颜色：实机 GPUI Kit 主题的角色解析值，由 FileAtlas MCP `get_ui_state.theme.roles`
   在同一进程内导出（暗色 `Default Dark`、亮色 `Default Light` 各一次）。
2. 几何：`crates/fileatlas-ui-bridge/src/theme.rs` 的公开常量，由脚本直接解析，不手抄。
3. 图标：扫描 `crates/` 中全部 `IconName::` 引用，从产品实际依赖的
   `gpui-kit-assets`（Lucide，ISC）复制同名 SVG 到 `icons/`。**不手绘图标**。
4. 结构与状态：实机窗口截图（`.local/validation/profiles/ui-tokens-*/window-*.png`）
   与对应 crate 的渲染代码。四者冲突时，以实机为准并回写清单。

刷新令牌与截图：

```bash
cargo +1.97.1 build -p fileatlas-shell -j 2
node ui-atlas/tools/capture.mjs
```

## 目录

```
index.html            画廊（file:// 直接打开为只读）
app/                  画廊脚本与样式；tokens.css 为生成文件
tokens/               geometry.json + theme.light.json + theme.dark.json
icons/                产品实际使用的 Lucide 图标（gpui-kit-assets 原样复制）+ index.json
catalog/inventory.json 全量条目清单（唯一人工维护的登记表）
catalog/manifest.json  版本登记（生成）
catalog/data.js        画廊离线数据（生成）
catalog/sources.js     全部 SVG 源（生成）
svg/<分类>/<id>.svg    描摹产物
tools/                tokens / icons / build / serve / capture
```

## 编写规范

- 每个文件是一个自包含 SVG 文档，必须有 `viewBox`、`width`、`height` 与中文 `aria-label`。
- 一律使用令牌变量：`fill="var(--fa-accent)"`、`stroke="var(--fa-separator)"`。
  禁止写死十六进制颜色，`build.mjs` 会把它当作缺陷报出来。
- 变量名 = 角色名把下划线换成连字符并加 `--fa-` 前缀。产品角色见 `tokens/theme.*.json`
  的 `roles`；`native_` 前缀的是 GPUI Kit 原生字段（标题条、状态带、滑杆、滚动条等）。
  几何常量同样可用，如 `--fa-space-3`、`--fa-round-md`、`--fa-action-height`。
- 尺寸取自 theme.rs：控件高 32/36/38/40，图标按钮 30，圆角 4/6/8/10，间距 4/8/12/16，
  字号 11/12/14/15/18，侧栏 264，检查器 300，标题带 56，状态带 40，浮窗标题栏 36。
- 图标一律引用产品同源的 Lucide 库，不手绘近似图形：
  `<use href="#fa-icon-search" x="12" y="14" width="16" height="16" color="var(--fa-text-muted)"/>`。
  可用名称见 `icons/index.json` 的 `icons` 或 `icons/*.svg` 文件名；构建时会把所用 symbol
  注入每个 SVG 的 `FA:ICONS` 块，引用不存在的图标会直接报错。
- 组件必须画全实际存在的状态（默认/悬停/按下/选中/禁用/焦点中适用者），并在图内用小字标注状态名。
- 一个条目一个文件；同一控件的多状态放在同一张图里并排展示，不拆成多文件。

## 版本规则

- 新文件首次入库：`1.0.0`。
- 内容哈希变化（不含机器管理的 `FA:TOKENS`、`FA:ICONS` 块）：修订号 `+0.0.1`。
- `viewBox` 尺寸变化：次版本号 `+0.1.0`。
- 主题令牌或图标库刷新只改写文件头部的机器块，不触发版本变化；改动 `<use>` 引用才算内容变化。
- 每次有任何变化，`catalogRevision` 自增，条目 `history` 追加一条（版本、哈希、时间、说明）。
- 触发方式：网页内保存（`POST /api/save`）、`node tools/build.mjs`、或 `npm run check` 后手动构建。

## 常用命令

```bash
npm run tokens     # 由 theme.rs 与实机转储重新生成令牌与 tokens.css
npm run icons      # 按 crates/ 的 IconName 引用同步 gpui-kit-assets 图标
npm run build      # 嵌入令牌/图标块、校验、升版本、刷新 catalog
npm run check      # 只校验不写盘（CI/自检用）
npm run serve      # http://127.0.0.1:8181/ ，开启网页内编辑回写
npm run capture    # 跑实机 MCP 导出令牌与截图，然后同步图标并重建
```

## 网页能做什么

- 按「完整界面 / 区域部件 / 原子控件 / 对话框 / 状态与反馈」分类浏览，支持搜索与状态过滤。
- 明暗切换：SVG 只引用令牌，切换即整库换肤。
- 条目详情显示归属 crate、源文件、规格、尺寸、哈希、更新时间与完整版本历史。
- 全屏查看：卡片悬停出现「全屏」，详情页也有「全屏查看」；单张 SVG 撑满窗口，
  可切换明暗、进入浏览器原生全屏，Esc 或「关闭」退出。
- 待描摹条目以虚线卡片显示其规格与归属，覆盖进度实时统计，未描摹的一律不标成完成。
- 编辑区直接改 SVG 源码，保存后写回磁盘并自动升版本，页面即时反映新版本。

## 批次台账

| 批次 | 范围 | 状态 |
|---|---|---|
| 0 | 工程骨架、令牌导出、版本机制、画廊 | 已完成并实测 |
| 1 | 首批样例：主按钮、媒体网格卡、主工作台整窗 | 已完成 3 项 |
| 2 | 原子控件其余 29 项 | 进行中 |
| 3 | 区域部件 25 项 | 进行中 |
| 4 | 完整界面 25 项（设置 12 页、WorkCrate、各工具面板） | 待办 |
| 5 | 对话框 14 项 + 状态 9 项 | 待办 |
| 6 | 逐屏与实机截图比对校色 | 待办 |

清单共 108 项，登记在 `catalog/inventory.json`；`catalog/manifest.json` 的 `stats`
是覆盖率的实时来源。产品界面变更后：先 `npm run capture` 刷新令牌，再更新受影响 SVG，
网页会自动出现新版本与变更历史。
