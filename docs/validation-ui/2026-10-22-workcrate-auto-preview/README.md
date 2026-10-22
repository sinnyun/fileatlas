# WorkCrate 预览自动获取官方保存改动同进程实测

日期：2026-10-22。环境：Windows x64，Rust 1.97.1（显式 `cargo +1.97.1`），官方
VectorCraft 0.14.0，基线提交 `46c22b3`，dev profile EXE
`target/debug/fileatlas-shell.exe`。

用户要求：预览页面始终没有正确刷新，必须先切到编辑页再点“同步工程与预览”才能
看到 VectorCraft 的改动。本轮要让浏览态自动获得官方程序保存后的更改，不再需要
切换页面或手动同步。

## 结论

1. **浏览态自动发现保存改动**：不打开画布时，面板每 1.5 s 只做一次本地 stat，
   把已保存 `.vectorcraft` 的 `(modified_ns, len)` 与目录缓存记录的 source 戳比较；
   相同则保持安静，不启动进程、不打开端口。
2. **只有真正变化才启动官方静默会话**：戳过期时在后台执行一次
   `auto_refresh_project`，只通过 `mcp --headless` 读取已保存文件，重建目录与
   预览并原子发布；绝不使用实时 GUI 通道，绝不驱动、聚焦或激活用户窗口。
3. **界面不整项目重载**：成功后只补丁页面矩阵，已解码图像作为占位保留，选中页
   优先重新解码，选择与编辑态保留，不打开编辑会话；底部显示
   “已自动同步 VectorCraft 的更改”。
4. **失败可恢复**：官方 CLI 不可用时保留旧预览，记录错误并显示一次中文提示，
   30 秒后重试，不反复启动进程或刷屏。
5. **缓存自写不触发循环**：所有缓存写入者（显式同步、新建文档、单页保存、画布帧
   发布）都在自己的原生写入之后盖 source 戳，因此自身写入不会被误判为外部改动。

## 实现要点

- `crates/fileatlas-plugin-workcrate-boards/src/autosync.rs`：
  `DocumentStamp { modified_ns, len }`、`PreviewState::stale()`、
  `preview_state(project)`（本地 stat + 缓存戳）、
  `auto_refresh_project(project)`（匹配即 `synced:false`，否则静默 headless 重建）
  与 `rebuild_cache`。
- `BoardCache` 新增 `source: Option<DocumentStamp>`（`serde(default)`，旧缓存补同步
  一次）；`refresh_project`、`refresh_cache_from_client`、
  `create_project_document`、`store_page`、画布帧发布都会更新它。
- `crates/fileatlas-plugin-workcrate-ui/src/view/autosync.rs`：1.5 s
  `refresh::poll` 观察器；`busy`、画布会话/画布模式、字段保存、未完成状态读取
  期间跳过；失败 30 s 退避；成功时补丁卡片并递增 `load_generation`，重新排队
  预览解码。
- 状态输出新增 `ui.auto_sync { syncs, pending, error }`，供 MCP 与验证脚本断言。

## 验证命令与结果

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-plugin-workcrate-ui -j 2
cargo +1.97.1 test --locked -p fileatlas-plugin-workcrate-boards -j 2
cargo +1.97.1 build --locked -p fileatlas-shell -j 2
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/wc-auto-preview --access-level full --scenario scripts/validate-workcrate-auto-preview.cjs
```

结果：fmt/check/build 退出 0；boards 31 通过 / 3 忽略；四个守卫退出 0。
架构守卫因 `autosync.rs` 新增 6 处仅测试用 `.expect()` 而报告 A4 债务
17 → 23，按既定流程复核 diff 后更新基线；同一次重生成还删除了 export crate
已不存在的 `workcrate-projects` 边和过期的 9 项债务（现为 0），基线只缩小或记录
真实现状。

自动同步场景输出：

```json
{"autoPreview":"passed","passed":true,"port":61929,
 "project":"2df3219e-e23c-4af4-80f7-875e777a1772",
 "board":"2df3219e-e23c-4af4-80f7-875e777a1772:9c9523c0-de71-461d-9b10-5e2879c8e803:1",
 "autoSyncs":1,
 "initialHash":"9f4d0fb50eba3a99bfad6988916ab3394fb80b444c8db91e00c787b36b2c2e91",
 "refreshedHash":"8ed9fc3ea6d3be2d0eb47e2f85cc43681e70d8c7cea699bfcaf33c3200d961dc"}
```

场景断言（`scripts/validate-workcrate-auto-preview.cjs`）：

- 初始缓存构建完成后 `auto_sync.syncs == 0`，浏览态 `canvas.state == null`，
  4 秒安静窗口内不触发同步。
- 独立官方 headless 会话打开同一 `.vectorcraft`、添加红色
  “AUTO-SYNC 自动同步”文本并保存；FileAtlas 未收到任何手动同步命令。
- 观察器自动完成 1 次同步：缓存 source 戳变为新保存文件、预览键与 PNG 哈希改变、
  旧预览文件被删除、选中页保持选中、未打开任何编辑会话、消息以“已自动同步”开头。
- 同步完成后状态连续 5.5 秒不再变化（`auto_sync.syncs` 不增长），空闲时不重复
  启动官方会话。

截图（本地 `.local/validation/`，不入库）：
`profiles/wc-auto-preview/screenshots/auto-preview-after-external-save.png`；
结果 JSON `profiles/wc-auto-preview/auto-preview-result.json`。截图中预览画布与
左侧页面缩略图都显示红色外部文本，底部状态为“已自动同步 VectorCraft 的更改”。

## 未覆盖项与限制

- 只读取已保存文件：用户在官方 GUI 中尚未保存的改动不可见；本轮用独立官方
  headless 会话保存来模拟外部程序写入，未在用户自己的实时 GUI 上复测保存路径
  （该 GUI 当时运行在控制端口 17979，本轮未触碰）。
- 画布会话打开期间观察器保持空闲：当前页由既有画布探针负责，其它页在离开画布时
  刷新；画布打开期间的外部保存不会立即反映到左侧缩略图。
- 需要 WorkCrate 面板处于打开状态；首次打开面板时若缓存已过期，会先显示旧预览，
  随后由观察器自动替换。
- 快速连续多次保存未单独压测；若同步过程中文件再次变化，缓存戳会保持过期并在下
  一采样周期再同步一次。
- 自动路径只走静默 headless 会话，不验证实时 GUI 路由；这是为了避免驱动用户窗口
  的有意设计。
