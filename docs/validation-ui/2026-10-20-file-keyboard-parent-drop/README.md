# FileManager 快捷复制与主目录拖放（2026-10-20）

## 参考与检查清单

按活动参考 `docs/reference-ui/concepts/multi-column-workspaces-2026-10-19.png`
及同名说明中 2026-10-20 子文件夹分组清单执行。实际概念图已打开核对：每个分组按
标题、文件区组成连续框架；子目录之间保留明确间距；主内容仍按行虚拟化，主题控制边框和
选中反馈。补充检查主目录是子分组外层放置范围，空白区域可接收文件且反馈主题状态；
文件管理视图焦点时 Ctrl+C 暂存本栏选择，Ctrl+V 命中鼠标下的文件夹组或主目录空白区。

## 实现范围

- `Ctrl+C` 在当前 FileManager 栏聚焦且有选择时保存所选 ID；MCP 状态提供缓冲区计数。
- `Ctrl+V` 复用 `FileOperationsService::bulk_copy_to_folder`，复制到鼠标下的子目录组；若鼠标在组外网格空白处则复制至当前浏览目录。
- 文件卡片拖动先交给应用内 GPUI typed drag/drop。在子文件夹分组模式中不启动同步平台 OLE 拖出，以免内部回拖被系统拖动交接阻塞；其他分组模式保留系统拖出。
- 子组仍由自身框架接收命中；未命中子组且位于视口的网格空白时回退当前主目录。拖动悬停主目录时网格底层以 Theme selection 角色反馈。

## 同进程验证

Windows FileAtlas shell，Rust `1.97.1-x86_64-pc-windows-msvc`，150% DPI；MCP child 与 GPUI 窗口同进程，Standard access。隔离 profile 和小型真实图片夹具均位于 `.local/validation/profiles/file-keyboard-empty-parent-final2-20261020/`。

执行命令：

```powershell
$env:FILEATLAS_TEST_LIBRARY = (Resolve-Path .local/validation/profiles/file-keyboard-empty-parent-final2-20261020/library).Path
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/file-keyboard-empty-parent-final2-20261020 --scenario scripts/validate-file-keyboard-and-parent-drop.cjs --access-level standard
```

场景启动生产 `dev-test.scan_fixture`，切到子文件夹分组，以同进程 GPUI 指针选择 alpha 的 `alpha.png`，发出 `ctrl-c` 并检查 `file_copy_buffer_count = 1`；鼠标移到 beta 组后发出 `ctrl-v`，确认 `beta/alpha.png` 出现。该夹具在主目录没有直连文件组，随后从 alpha 卡片拖至子分组之外、主网格空白处，确认 `alpha.png` 移到主目录且子目录源消失；再选择 beta 内文件，Ctrl+V 到主网格空白处，确认 `beta.jpg` 复制到主目录。没有调用直接移动控制路由。

功能结论：以上两个真实文件系统操作均通过。截图由同进程 `screenshot_window` 获得并已查看：`.local/validation/profiles/file-keyboard-empty-parent-final2-20261020/screenshots/parent-drop-hover.png` 和 `file-keyboard-parent-drop.png`。`parent-drop-hover.png` 可检查空白主目录目标的悬停反馈。

静态检查：`cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`、UI theme/refresh、architecture 和 feature-ledger 守卫通过。链接器保留既有 `LNK4098` 库冲突警告。

物理键盘和平台 OLE 拖出未在本场景覆盖；MCP 注入的按键通过真实 GPUI focus/key handler，拖放通过 typed GPUI handler。窗口截图布局与当前参考的分组结构一致；窄窗口、四栏下键盘操作、Windows 系统拖出仍需单独覆盖。
