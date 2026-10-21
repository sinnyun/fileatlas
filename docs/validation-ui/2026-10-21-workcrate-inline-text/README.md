# WorkCrate 画布内多行输入修正 — 2026-10-21

## 范围与参考

按用户本轮要求修正已选布局的交互，不重新设计三层页面。参考
[三层布局](../../reference-ui/concepts/workcrate-three-level-2026-10-20.md)、
[文本概念](../../reference-ui/concepts/workcrate-editing-tools-2026-10-20.png)及
[逐项清单](../../reference-ui/concepts/workcrate-editing-tools-2026-10-20.md)。
已打开概念和本次同进程截图对照。概念中的旧缩放/旋转控件继续不作为产品需求。

## 实际窗口

Windows，Rust 1.97.1，正常 `target/debug/fileatlas-shell.exe`，同进程 stdio MCP 子进程；
未使用外部鼠标注入。隔离 profile `.local/validation/profiles/workcrate-inline-final-2026-10-21`。
截图原始 3286×1831；当前原生暗色主题，页面纸张/用户黑字属于内容，控件/手柄遵循主题。

| 检查 | 实际表现 | 证据 |
|---|---|---|
| 三栏与固定 A3 | 左缩略图、中央信息条+图片区、右检查器保留；新增只进入文本输入态 | inline-live-input.png |
| 直接输入 | 新增自动获得原生输入焦点，输入中文两行，无需保存即持久化；双击已有文本继续输入 | inline-live-input.png、inline-double-click-edit.png |
| 尺寸和位置 | 输入时可拖动四角改变框宽高；拖动上沿移动；字号24→36即时显示并保存，字形没有拉伸 | inline-resized-no-page-reload.png |
| 右侧输入 | 点击右侧文本条目切换检查器输入，修改后画布单文本层同步显示 | inspector-live-text.png |
| 保存范围 | load_generation不变；A3背景canvas_image_id和照片display_image_ids不变；没有 perform/reload、重新解码照片 | 脚本真实 UI 状态断言 |
| 大字号多行 | 原默认 rem 行高会在36pt重叠；现随字号×1.2，无行重叠，框内正常显示 | inline-double-click-edit.png |
| 空闲 | 写入队列排空后shared poll停止；两次1秒间隔UI状态一致 | 脚本quiet断言 |

截图和 JSON 都是本机生成文件，未加入版本控制：
`../../../.local/validation/profiles/workcrate-inline-final-2026-10-21/screenshots/`，
结果 `../../../.local/validation/profiles/workcrate-inline-final-2026-10-21/inline-result.json`。
先前两次新增后的背景 ID 断言曾捕获初始布局尚未稳定的异步渲染；脚本改为等待背景就绪和
输入态布局稳定后记录基线。后续新增、输入、缩放、移动、保存检查全部通过。

## 可复现命令

```powershell
rustup toolchain list
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked --offline -j 2 -p fileatlas-plugin-workcrate-boards -p fileatlas-plugin-workcrate-projects -p fileatlas-plugin-workcrate-ui -p fileatlas-plugin-workcrate-export
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-inline-final-2026-10-21 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-inline-text.cjs
node scripts/check-ui-theme.cjs --self-test
node scripts/check-ui-theme.cjs
node scripts/check-ui-refresh.cjs
node scripts/check-architecture.cjs
node scripts/check-feature-ledger.cjs
node scripts/check-layering.cjs
```

四个相关crate共16个测试通过。Shell正常编译通过，只有已存在的LNK4098默认CRT警告。
图中的“保存文本”作为立即提交入口保留；正文/字号/颜色/尺寸自动提交，不再要求先点保存。
实际中文字符串注入与原生光标可见，但未模拟物理键盘中文IME候选窗/组合输入；本轮也未重复
全部小窗口、改主题、PSD系统关联或极大文本/极端尺寸验证，不将其标为通过。
