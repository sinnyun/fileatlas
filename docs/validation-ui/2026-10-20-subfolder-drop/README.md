# 子文件夹分组拖放验证（2026-10-20）

后续更正：本记录只证明最初的标题目标/移动控制入口。用户随后要求整体框架，
当前目标已经扩展为标题与文件内容共同所属的分组框架；新的实际 GPUI 指针验证见
[整体框架证据](../2026-10-20-group-frame-drop/README.md)。

## 行为

FileManager 的子文件夹分组标题接收文件卡片的拖放。拖动未选中卡片时只移动该文件；
拖动当前已选中的卡片时移动当前选择。移动复用 File Operations 批量目录移动，
保留冲突跳过、文件名、索引事件和 FileManager 刷新/选择修复。其他分组模式不接受该拖放。

MCP `control_file_manager` 增加 `move_files_to_subfolder`，要求 1–100 个唯一、已索引媒体 ID，
并验证目的路径存在且是当前可见的子文件夹分组。它经 FileManager UI 请求触发同一个移动处理器。

## 运行验证

- 环境：Windows；仓库构建的 `target/debug/fileatlas-shell.exe`；FileAtlas MCP stdio 子进程与 GPUI 窗口同进程。
- 隔离 profile：`.local/validation/profiles/drag-subfolder-final2`；隔离数据库、设置、缓存、样例库和 MCP 报告均留在该目录。
- Fixture：两个子目录 `alpha`、`beta`；索引器使用生产 IndexingService 扫描各一张图。输入小图复制自 `test-data/fileatlas-fixtures`。
- 命令：

  ```powershell
  cargo +1.97.1 fmt --all
  cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
  cargo +1.97.1 build --locked --bin fileatlas-shell -j 2
  $profile = Join-Path (Resolve-Path .local/validation/profiles).Path 'drag-subfolder-final2'
  $fixture = Join-Path $profile 'library'
  New-Item -ItemType Directory -Force -Path (Join-Path $fixture 'alpha'), (Join-Path $fixture 'beta') | Out-Null
  Copy-Item test-data/fileatlas-fixtures/01-small/tiny.jpg (Join-Path $fixture 'alpha/drag-source.jpg') -Force
  Copy-Item test-data/fileatlas-fixtures/01-small/small.png (Join-Path $fixture 'beta/existing-target.png') -Force
  $env:FILEATLAS_TEST_LIBRARY = $fixture
  node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile $profile --plugins all --access-level standard --scenario scripts/validate-subfolder-drop.cjs
  ```

- 结果：通过。MCP 实测分组显示 `alpha`/`beta`，调用 FileManager 移动后源文件消失、
  目标文件出现，`file_manager.search_media` 返回目标目录下的更新路径。测试也覆盖
  `\\?\` 扩展路径与常规路径的等价匹配。
- MCP 拒绝访问首先以 profile 默认 Off 正常阻止写操作；显式将隔离 profile 设为 Standard 后重跑通过。
- 失败重试发现 FileManager 的异步 UI 请求单槽会允许过快连续请求互相覆盖；可复现场景现等待
  分组状态落实后再导航。该序列化是测试操作的时序要求，不影响实际点击后的正常交互。

## 验收边界

- 功能行为：同进程 MCP 验证了对应的 FileManager 移动入口及生产索引器/文件操作服务，真实本地文件移动成功。
- 分组画面：检查本次运行生成的 `screenshots/subfolder-groups.png`。两张组标题横跨内容区，alpha 与 beta 项目分别处在对应标题下；没有改动现有布局。
- 拖放 UI：分组标题具备 FileDragPayload 的 GPUI drop handler，并只在 Subfolder 模式启用。
  MCP 合成指针不模拟 Windows 按住鼠标期间的原生 OLE 拖放，因此实际拖放手势仍待人工/原生输入验证。
- 视觉：布局未改；未执行概念图截图对比、小窗口与悬停画面验收。
- 原始 MCP 报告、截图、日志和 profile 均位于 `.local/validation/profiles/drag-subfolder/`。
