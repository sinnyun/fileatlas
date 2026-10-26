# 画布插件数据存储位置（2026-10-27）

## 实现

设置 → 插件设置 → FileAtlas Canvas → 数据存储位置现在展示：

- `whiteboards/library.json`：版本化 JSON，保存画板/场景图层、文本、资源与 Eagle 链接、模板、颜色和图片预览引用。画布数据不是 SQLite 数据库。
- `whiteboards/previews/`：画板专属图片缩略图副本。
- `whiteboards/pasted-images/`：粘贴进画布的独立图片文件。

路径以当前 `SettingsService.path()` 的数据根目录组合，隔离 MCP profile 会显示 profile 下的真实目录。每行还显示对应文件/目录的占用大小；统计通过 `ReadModel` 在后台读取并缓存，刷新按钮可重新统计。目录递归统计但不跟随符号链接；SQLite 行将 `-wal` 和 `-shm` 计入；缺失路径显示 `0 B`，部分不可读取时标注“至少”及原因。渲染不执行文件系统 IO，也不改写数据。

## 同进程验证

命令：`node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --plugins whiteboard --profile .local/validation/profiles/plugin-storage-sizes-20261027 --access-level standard --scenario scripts/validate-whiteboard-storage-settings.cjs`。

场景检查所有运行时插件设置页均可打开并断言页面 ID，最后回到 `fileatlas.whiteboard`；截图确认 JSON 数据文件、图片预览目录和粘贴图片目录及其内容说明与大小均显示。截图为 Windows 同进程 FileAtlas MCP 窗口；几何审查色框为诊断覆盖层。

`cargo +1.97.1 fmt --all -- --check`、Settings/Whiteboard 锁定 crate 检查、Shell build、UI theme self-test/守卫、UI refresh、architecture、feature-ledger 及场景脚本语法检查通过。Shell 第一次构建因工作区的 FileAtlas 进程占用 EXE 未能替换；核对路径后按工作区授权正常关闭该进程，随后标准目标构建成功，只有既有 `LNK4098` 警告。

截图和报告位于 `.local/validation/profiles/plugin-storage-sizes-20261027/`。该界面是只读估算；目录可能与其他行重叠，不能把各行简单相加。由于访问权限及运行期间文件变化，数据可能是部分统计或短暂滞后。
