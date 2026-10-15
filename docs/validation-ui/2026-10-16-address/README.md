# 顶部面包屑地址栏（2026-10-16）

依据用户本轮圈定顶部区域的[参考截图](user-reference.png)及文字要求实施；保留 accepted sidebar-pages-preview-v2 与 Explorer 侧栏结构。此次仅整理顶部位置区域，右侧重复的 Up 移到地址栏前。用户所说“上一页”按“上一级文件夹”实现，后退/前进使用独立导航历史。

## 已实现

- 按“后退、前进、上一级、地址栏”排列。地址从盘符/共享根至当前文件夹逐级展示，点击任一级切换；实际目录不再显示虚构的“收藏集合”路径前缀。
- 点击地址空白区或末尾“…”编辑完整地址，自动全选；Enter 跳转、Escape 取消，也可用输入框旁的跳转/取消按钮。支持引号包裹、中文、空格、相对地址；非法位置显示红框/错误提示并保留原位置。
- 文件夹卡片、侧栏、收藏入口与标签视图进入同一份窗口内导航历史，最多 200 项；相同 Windows 路径去重，后退后新导航清除前进分支。历史目标消失时保留当前范围和历史指针；盘符根和 UNC 共享根无 Up 目标。点击当前面包屑不重复加载。
- 当前目录仍沿用 session 保存；新窗口从已恢复位置重新建立历史。启动仅发送一个对应路径/收藏/标签范围事件，修复原先空路径恢复事件会覆盖标签/收藏恢复的问题。
- 键盘：地址栏聚焦时支持 Ctrl+L、Alt+Left/Right/Up，Gallery 聚焦时转发 Ctrl+L 与 Alt+Left/Right，原 Alt+Up 保留。长路径可横向滚动，完整文本可在编辑模式查看。此轮没有加入自动补全或兄弟目录下拉功能。

实现归属 Navigation 的既有 command.breadcrumb view；Shell 仅挂载和投递调试输入。没有新数据库或第二份文件操作 undo 历史。路径展示与历史比较是纯词法操作；文件存在性仅在用户导航动作时检查。

## 验证与结果

Windows / 150% DPI，Rust 1.97.1 MSVC，debug、locked、-j2。同进程 PhoTag MCP 子进程拥有被操作窗口，隔离 profile 均在 target；未使用通用 OS 输入注入。常用程序文件仍可能被用户窗口占用，因此本轮独立输出 `target/debug/photag-shell-address.exe`，包含此前重复刷新修复。

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-address.exe
cargo +1.97.1 test --locked -p photag-plugin-navigation -p photag-plugin-gallery -p photag-plugin-i18n -p photag-plugin-mcp -j 2
node scripts/check-ui-refresh.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-address.exe --profile target/mcp-address3 --allow-mutations --scenario scripts/validate-address-bar.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-address.exe --profile target/mcp-address3 --allow-mutations --scenario scripts/validate-address-pointer.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-address.exe --profile target/mcp-address3 --scenario scripts/validate-address-restart.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-address.exe --profile target/mcp-address3 --allow-mutations --scenario scripts/prepare-address-tag-restart.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-address.exe --profile target/mcp-address3 --scenario scripts/validate-address-restart.cjs
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-address.exe --profile target/mcp-address-refresh --allow-mutations --scenario scripts/validate-browser-refresh.cjs
```

全部通过。34 项自动测试通过、0 失败（target/address-bar-tests-final.log），新增历史去重/分支/边界、完整 Windows/UNC 面包屑测试。构建的两个输出文件名提示来自显式 rustc -o，不是源码警告。初次场景把未规范化路径传给“已可见卡片”接口而被正确拒绝，后改为使用实际卡片发布的路径，没有放宽生产校验。

11 项运行场景覆盖路径/编辑/分支/失效历史/标签往返；Enter、Escape、Ctrl+L 走实际 GPUI KeyDown/InputEvent，文本准备使用真实 InputState 的 set_text，不宣称这证明了全部输入法/剪贴板交互。4 项 pointer 场景实际点击后退/前进/Up、父级面包屑、空白编辑以及当前层级不重载。指针坐标基于当前 150% 窗口截图的逻辑位置，其他窗口尺寸应重新测量。

文件夹及标签视图都在只读重启后等待约 3 秒核验；地址与内容仍一致，导航历史从恢复点开始，修改请求被只读保护拒绝。此前窗口切换重复刷新回归也通过：三次实际最小化/恢复不改变枚举/查询版本、选择和滚动，真实外部新增/删除仍生效。[摘要](summary.json)保留场景清单与地址数据；完整本机日志与报告留在 target 隔离 profile。

对照参考与实际截图已检查顶部位置、按钮顺序、完整层级、地址输入/错误状态及原右侧工具区保留：[面包屑](address-breadcrumb.png)、[编辑](address-editing.png)、[非法路径](address-invalid.png)、[实际点击后](address-pointer.png)。

## 仍未实机覆盖

UNC 根路径的构造/Up 边界有自动测试，离线共享、所有凭据/权限拒绝和超长路径的各类设备组合没有逐项实机覆盖；这些不能由本地盘点击证明。没有为地址自动补全、历史持久化、兄弟目录下拉新增承诺。既有 F0 系统对话框/全部媒体 codec 等未验项继续保留。

最终确认常用程序未被占用后，`cargo +1.97.1 build --locked -p photag-shell -j 2` 已成功更新 target/debug/photag-shell.exe（12.54s），并用该常用构建对 mcp-address2 运行相同只读重启检查通过。下次正常启动无需选择独立地址栏构建。
