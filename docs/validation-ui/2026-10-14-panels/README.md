# 弹出面板、列表布局与菜单交互验证

2026-10-14，Windows／150% DPI／Rust1.97.1 MSVC。本轮按用户明确要求对已接受基线作局部功能优化：通用浮动面板、设置点击隔离、媒体预览面板化、Grid/List切换和卡片按钮菜单稳定性。

## 环境和命令

静态检查使用 `cargo +1.97.1 fmt --all -- --check`、`cargo +1.97.1 check --locked -p fileatlas-shell -j 2`、`cargo +1.97.1 build --locked -p fileatlas-shell -j 2`。测试为 `cargo +1.97.1 test --locked -p fileatlas-ui-bridge -p fileatlas-plugin-file-manager -p fileatlas-plugin-session -p fileatlas-plugin-mcp -p fileatlas-plugin-i18n -j 2`：26项通过，0失败。最后临时几何清理变更另重跑UI Bridge5项，全部通过。Node客户端语法检查通过。

最终构建是 `target/debug/fileatlas-shell.exe`；SHA256见summary.json。中间构建使用 `cargo +1.97.1 rustc --locked -p fileatlas-shell --bin fileatlas-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/fileatlas-shell-panels-final.exe` 等命名输出。初次标准build遇到删除旧exe时os5，随后标准build已通过；不是当前阻塞。初次截图早于窗口就绪，加入启动等待后重测成功。

实测均由 `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-panels-final.exe --profile target/mcp-panels-v3 --allow-mutations --interactive` 启动同进程MCP/GPUI窗口；最后清理回归直接使用最新标准exe和cleanup-plan.json。profile及LOCALAPPDATA均指向workspace target，正常FileAtlas DB／设置／会话未用于测试。只对隔离DB作扫描、注册样本和虚拟行操作。真实测试媒体为现有9项fixture及复制到target隔离目录的320×180／8秒MP4。

已保存52步交互报告、73步最终媒体／列表报告、12步最新exe清理回归、4步只读拒绝报告，共141步。工具参数和结果在相应JSON中；旧互动报告未保存等待字段，使用150–600ms交互等待与媒体2.5s等待。最新客户端开始记录wait_ms/until，cleanup-plan可直接重放。每次capture都是调用该子进程screenshot_window，未注入OS鼠标键盘、未附着外部应用。

## 已观察到的结果

| 检查 | 运行证据 |
|---|---|
| 设置内点击与主界面隔离 | 设置空白区及遮罩上的GPUI down/up没有改变FileManager选中ID9或左侧folders页；设置Appearance页实际点击成功 |
| 设置拖动／缩放 | 190,60,900,680 → 290,110,900,680 → 290,110,800,580；主界面选择保留。见final-settings-drag-resize.png |
| 单／多实例 | Single连续open返回同一status@1；Multiple产生@2/@3。@3移至100,74，其他实例保留各自520×300／600×360默认尺寸。点击@3关闭仅移除它；最新清理回归关闭全部后open_panel_ids和panel_instances为空，重复close报告未知实例 |
| 列表与顶部按钮 | 真实列表选择得到ID9并更新Inspector；顶部List按钮实际鼠标点击切换；下一进程恢复list。见final-list-selected.png／final-topbar-list-click.png |
| 卡片按钮菜单 | 鼠标移出卡片进入菜单后仍存在；Ratings子菜单展开；点击Preview得到真实tiny.jpg／ID6。直接右键菜单仍能移入并高亮条目。见card-menu-hovered.png／right-menu-hovered.png及交互报告 |
| 图片／音频 | 图片面板1100×760移动至170,40；音频面板960×640移动至260,100；原内容／媒体身份和控件保留 |
| 视频 | MP4探测320×180、8秒，播放／seek3秒后实际解码画面可见。播放面板最大化0,0,1280,800，再还原260,100,960,640。初次600ms截图尚无解码画面，不能用它声称解码成功；成功截图为final-video-decoded.png |
| 10万列表 | 注入100000虚拟行；当前fixture范围匹配100009项，scroll99999后window_offset99809／loaded_count200，末页真实列表可见；之后通过dev_test_clear_virtual清理100000行。见final-list-100k-confirmed.png |
| 只读策略 | 未设置mutation标志时panel_open和FileManager layout均返回明确拒绝，未打开面板 |

对照的是final-v5/01、02、03、05中的内容控件与状态，以及已接受sidebar-pages-preview-v2；本轮标题栏拖动／缩放和列表是用户新要求，按用户要求扩展，不追求旧截图的全屏窗口几何。设置导航、自动保存页脚，图片信息／AI／EXIF／工具栏，媒体transport，原卡片菜单操作都保留。新面板公共接口见../../floating-panels.md。

## 当前边界

四边／四角缩放路径共用同一实现，本轮指针实测的是右下角；其他七个把手、窗口外捕获、不同DPI、持续复杂输入、列表Ctrl/Shift/框选/原生拖出未逐项验收。图片／音视频各覆盖本轮样本，不代表完整codec或音频听感验收。通用目录支持多实例内容；既有Preview/Playback服务仍各为单媒体会话，未宣称多媒体并行解码。面板尺寸不跨程序重启保存。

旧设计中已有未实现状态继续沿用baseline记录；本轮不补出原生系统对话框、音频装饰波形或多媒体并行解码等未承诺能力。首次视频解码前仍沿用现有等待行为。报告和截图是运行证据，不是正式发布包。
