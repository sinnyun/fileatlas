# 统一文件预览与标签一次确认

2026-10-16，Windows、150% DPI；[本轮内置生图概念与完整提示](../../reference-ui/concepts/right-inspector-unified-edit-2026-10-16.md)。用户明确要求覆盖上一版的紧凑历史身份栏和逐字段保存。

## 修改与真实截图

- 文件信息、历史和扩展共用相同标题、180逻辑像素预览、文件名、评分和页签行，只更换下方内容。
  [信息](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/file-information.png) / [历史](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/file-history.png) / [扩展](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/file-extensions.png)。
- 标签编辑直接展开：名称 → 颜色 → 图标 → 显示开关 → 确认修改。
  [全部展开](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/tag-expanded.png) / [未确认草稿](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/tag-draft.png) / [一次确认后](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/tag-confirmed.png) / [恢复样例](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/tag-restored.png)。
- 窄面板300、1920×1080：图标自动换行，编辑主卡及确认按钮完整可见；历史正文独立滚动。
  [标签小窗口](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/tag-small.png) / [历史小窗口](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/history-small.png)。

截图均由同进程FileAtlas MCP `screenshot_window`直接取得，无重绘或拼接。
文件和标签继续使用[上一轮准备的真实隔离样例](../2026-10-16-inspector-complete/README.md)，
没有修改用户FileAtlas档案。历史多版本仍是显式隔离演示；实际持久快照不属于本轮。

## 验证

`verify.cjs`完成62步同进程MCP调用，真实pointer操作色板、图标、开关和确认按钮：

1. 修改名称为“旅行 · 统一确认”、选蓝色、Star图标、关闭显示后，`list_library`完整标签对象与修改前相同。
2. 点击一次确认后，真实服务快照同时反映四项变化，父标签显示规则继续传播到子标签；身份卡和侧栏同步显示新颜色/图标/名称。
3. 再统一确认恢复旅行、珊瑚红、Tag图标、显示开启；子标签显示恢复。
4. 同一真实文件依次切换信息/历史/扩展，选择ID不变。对原始1920×1560截图的右栏区域
   `(1370,130,1908,553)`进行RGB逐像素比较，三页顶部标题、预览、文件名及评分的差异边界均为空。
   比较排除会改变选中态的页签本身，结果见[results.json](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/results.json)。
5. 小窗口300宽度截图；静止日志latest_id为798→798。

随后无演示环境变量启动同一最终构建，复用上一轮 `verify-normal.cjs` 完成20步；
普通历史为真实当前副本，demo enabled=false、count=null；静止日志448→448。
原始普通历史图见[普通历史](../../../.local/validation/artifacts/2026-10-16-inspector-unified-edit/screenshots/history-normal.png)。

初次指针脚本100ms间隔触发MCP已有“pointer request is still pending”并停止；
调整为450ms且在服务明确拒绝排队时有界重试后，完整62步通过。重跑只恢复本隔离样例。

命令：

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p fileatlas-shell -j 2
cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-inspector-unified.exe
cargo +1.97.1 test --locked -p fileatlas-plugin-file-manager -p fileatlas-plugin-i18n -j 2
node scripts/check-ui-refresh.cjs
git diff --check
```

检查通过，FileManager 21项、i18n 4项测试通过。直接默认build曾因正在运行的
`target/debug/fileatlas-shell.exe`被Windows占用而返回os error5；使用上述独立输出成功构建并完成全部运行验证，未终止现有正常应用。
独立输出的两个rustc警告来自显式`-o`与多种输出类型，不是源代码编译错误。

复现前先按上一轮证据准备隔离fixture。使用dev-test及该profile启动独立输出，
`--scenario docs/validation-ui/2026-10-16-inspector-unified-edit/verify.cjs`；
演示环境变量及缩略图路径设置沿用上一轮。无演示检查需清除两个演示变量。
指针坐标按1920×1560、150%DPI和360宽度记录，任意机器复用前需用`window_status.geometry`核对。

## 边界

确认复用三个现有Navigation写命令，依次执行，未新增跨命令原子事务；遇错保留反馈并刷新实际模型。
仅显示开关变化时传播显示规则，单改名称/颜色/图标不覆盖子标签显示。未改SQL/schema。
图标使用现有14个标识，不引入生成图里的新图标资源。其他关联/合并/移动等结构操作继续在既有标签管理器中。
未覆盖非150%DPI、所有图标逐一点击、写入失败磁盘、全部格式以及真实持久版本；不扩大为全项目验收。
