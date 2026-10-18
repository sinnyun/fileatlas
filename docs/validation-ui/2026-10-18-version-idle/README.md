# 普通浏览中的 Lore 被动读取与无关写入失效

用户报告静置时大量 `opentelemetry_sdk / MeterProvider.ExistingMeterReturned /
urc.store.immutable.local`。追踪当前固定依赖与工作区调用后发现：Gallery 的可见文件
“未保存版本修改”提示会自动调用 VersionsService::file_state，即使没有打开 History；
Lore 初始化本地不可变存储时构造多项统计指标，每项取得 meter 都可能记录此 DEBUG。
该日志不代表版本切换，也不代表一次读取耗时等于 elapsed 时间。

## 定位与修复范围

单文件测试完成首次读取后，12秒静置计数95不变；82文件/12已建版本、打开调试面板
的20秒采样中，该事件计数3583保持不变。因此没有把“无写入的空闲无限循环”描述为
已复现，也不能证明本轮定位就是用户现场唯一触发源。

发现并复现的具体问题：`version_badges::StateKey` 包含全库 data_revision，而该值
来自 DatabaseService::write_revision，包含剪贴板/启动台等无关写入。已缓存的版本
检查在下一次普通重绘会重新执行。修复前，没有打开版本页，只写入一条剪贴板记录
并触发正常 hover 重绘，统计日志95→127，Gallery snapshot revision仍为3。

本轮移除这一无关依赖，保留 snapshot_generation/history_generation/可见文件路径。
文件监听/索引快照及显式版本动作仍会刷新。共享ReadModel继续缓存结果/失败与合并在途
请求，未屏蔽或降低任何日志级别，未更改Lore库、SQLite模型、版本ID或内容比较语义。

依赖追踪：固定Lore `da73d9f` 的 lore-storage/local/immutable_store.rs 中
StoreInstruments/CompactionInstruments → lore-telemetry InstrumentProvider::meter →
opentelemetry_sdk0.31.0 meter_provider.rs::meter_with_scope 的已有meter分支。
工作区调用：Gallery flush_visible_window → request_version_states → VersionsService::file_state。
vendor/photag-source没有版本历史/未保存提示对应项；沿用本项目现有固定Lore实现。

## 验证

Windows，150% DPI，Rust1.97.1 MSVC，同进程GPUI+MCP隔离profile
`target/mcp-version-idle-fixed`，进程23248，2026-10-18T07:39:51.208Z启动。
EXE SHA256 `DE192332DA29984E4AEB7B955C6F51C60B1A1A483481D7D24F17F3AAC2D431AB`。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-gallery -p photag-plugin-versions --lib -j 2
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
node scripts/check-ui-theme.cjs
git diff --check
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-version-idle-fixed --allow-mutations --scenario scripts/validate-version-idle.cjs
```

构建/格式/治理通过；Gallery24与Versions5共29项测试通过。现有LNK4098与历史孤立
目录提示保留。场景只修改隔离fixture，未使用用户文件或常规配置。

- 未打开History，连续3次无关剪贴板写入+普通重绘，事件计数均95；额外静置2.2秒仍不变。
- 真实文件修改经监听自动显示Modified/未保存徽标，事件计数增到127。
- 创建第二个版本自动变回Saved，读完后2秒静置计数207不变；版本列表仅01/02。
- [精简结果](version-idle-summary.json)、[未打开版本页的静置截图](quiet-no-history.png)、
  [真实文件修改](real-file-modified.png)。已打开前一张确认实际窗口与调试面板状态。

初次加载已建版本的文件、真实版本动作及实际文件变化仍会产生有限统计日志；本轮没有
消除这些有实际工作对应的日志。未在用户原始82文件目录及其后台事件环境重放，若更新后
仍持续增长，需要结合当时目录、是否有文件/缓存写入及调试面板状态继续追踪其他失效源。
