# 添加文件夹点击退出：复现与修复

2026-10-14，Windows、150% DPI、Rust1.97.1 MSVC。Git起点67b079a。

旧版通过同进程PhoTag MCP点击左侧标题“＋”(GPUI逻辑坐标215,74)后退出，日志为main线程 `RefCell already borrowed`，位置gpui-pre-0.3.8/src/app/async_context.rs:66。同步rfd目录选择器的Windows消息循环在GPUI实体/App借用未释放时重入。见crash-stderr.log。

修复仅调整调用时机：改为rfd::AsyncFileDialog，在其Windows STA线程上打开；GPUI任务await期间释放借用，返回路径后再this.update调用原add_folder_path注册／扫描。取消不变更媒体库。共享lease阻止重复打开，按钮pending时禁用，任务释放时清理状态并请求关闭已创建的所属native窗口。

验证命令：

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -p photag-shell -j 2
cargo +1.97.1 build --locked -p photag-shell -j 2
cargo +1.97.1 test --locked -p photag-plugin-navigation -p photag-plugin-mcp -j 2
node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-folder-fixed --plan docs/validation-ui/2026-10-14-folder-picker/plan.json --allow-mutations
```

相关5项测试通过，0失败；包括single in-flight lease/reset及原有目录注册／rescan／remove、Cordis清理测试。Node客户端语法和git diff whitespace检查通过。summary.json保存实测exe SHA256。

15步MCP结果见report.json：真实按钮down/up→实际查找到同进程原生dialog→screenshot_folder_picker截图→dialog打开期间主窗口继续响应→cancel→pending/native_open归零→再次点击打开→取消。所有list_library结果保持folders=[]、tags=[]，修复后的stderr无panic。native-picker.png是实际原生选择器，before/after-cancel/main-while-picker-open是同一子进程GPUI窗口。

所有操作在target/mcp-folder-fixed的隔离DB／settings／session／LOCALAPPDATA运行。取消通过项目MCP调用Navigation公开接口，识别本进程、#32770类及当前标题的窗口后请求WM_CLOSE；不注入OS鼠标键盘，也不使用generic computer-use。

本轮验收针对点击退出、原生窗口打开和取消重开。真实选择路径的Accept按钮未自动化操作；返回路径仍接原后端流程，有service回归但不据此宣称native接受已验。任务取消和窗口尚未创建之间的竞态也未逐项验证。页面保持已接受sidebar-pages-preview-v2布局，系统选择器沿用Windows原生UI。
