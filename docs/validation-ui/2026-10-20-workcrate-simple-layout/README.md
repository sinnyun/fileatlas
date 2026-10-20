# 固定 A3 图片排版与链接更新验证

## 本轮依据

用户澄清：真实页面是固定 A3 的“顶部页面内容信息条 + 下方图片放置区”。
页外尺寸只是资料标记。图片只需要移动、等比大小和可恢复裁剪；图片本地链接与
PSD 等扩展源文件链接各自独立。这个要求覆盖此前通用画布/整页平移/可变页面尺寸的理解。

结构参照原用户编辑页草图和三层概念的栏目关系；最新行为清单见
[固定 A3 简化排版](../../reference-ui/concepts/workcrate-simple-layout-2026-10-20.md)。

## 实现与已验结果

| 项目 | 实现/同进程实际操作结果 |
|---|---|
| 固定纸张 | A3 横向 420×297 mm；移除页面尺寸调整、整页平移及旋转入口 |
| 尺寸标记 | 自由文字 `size_label`；修改为 210cm×100cm 后，源文档 A3 几何不变 |
| 内容范围 | 原生 DesignCraft 纸张/信息条作为静态背景；下方图片区单独接受图片和编辑 |
| 实时移动 | 原图缓存图层即时移动；按下/移动期间清单没有写入，UI 没有进入项目重载 busy |
| 等比缩放 | 真实点击四角手柄，向不同比例方向拖动，最终宽高比例仍保持不变 |
| 可恢复裁剪 | 手柄修改 frame，content 不变；原文件字节不变；恢复后 frame 与原 content 边界一致 |
| 手柄可见 | 独立顶层选择框/四角手柄，不因其他图片覆盖而消失；不改变图像层叠顺序 |
| 主界面拖入 | 正常模式、关闭内部拖拽调试开关，从真实 FileManager 卡片拖入图片区并保存第二张图片 |
| 项目批量更新 | 外部覆盖链接 JPG，点击可见“更新项目图片”，两页都从 900×600 更新为实际 24×16 |
| 扩展源文件 | 独立 PSD 路径持久化，图片刷新后仍保留；编辑入口复用 File Operations 系统关联打开 |
| 导出 | DesignCraft + PdfCraft；两页 PDF 的页尺寸 1191×842 pt，Poppler 识别为 A3；PNG 同时导出 |
| 静止状态 | 1.2 秒间隔的 UI 快照一致 |

刷新样例刻意将图片换成 24×16 的极小测试 JPG，因此更新后的大图模糊是源文件实际像素，
不是将缩略图当作原图。移动/缩放/裁剪截图使用原 900×600 合成图片。
没有以此声称相机级/超大设计稿的帧率或全部 GPU 尺寸覆盖。

## 复现

Windows MSVC，150% DPI，Rust 1.97.1，同进程 FileAtlas MCP 隔离 stdio 子进程。

```powershell
cargo +1.97.1 build --locked --offline -j 2 -p fileatlas-shell
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/workcrate-simple-layout-2026-10-20-r9 --plugins mcp,workcrate-ui --access-level full --scenario scripts/validate-workcrate-simple-layout.cjs
```

成功场景输出：`.local/validation/profiles/workcrate-simple-layout-2026-10-20-r9/`。
本地 `simple-layout-result.json` 记录 13 项行为检查；`screenshots/` 包含 selected-handles、
image-moving、image-scaled、image-cropped、project-links-refreshed、main-file-dropped。
`artifacts/` 保存 fixed-a3.pdf / fixed-a3.png。均为 Git 忽略的本地产物。
脚本使用真实 GPUI 指针事件、输入框和按钮，后台放图仅用于第二页场景准备，不替代可见交互验收。

回归新增 `fixed_a3_labels_uniform_scale_crop_restore_and_project_refresh`：验证标签不改 A3、
拒绝拉伸、裁剪不改内容及源字节、恢复、跨页刷新、扩展链接保留、源比例改变仍等比，
以及缺失文件逐项报告。主题、刷新、架构、分层和功能台账守卫通过。

## 边界

实际 Photoshop 打开 → 编辑 PSD → 由 Photoshop 导出覆盖 JPG 的外部过程没有自动执行。
本轮使用独立 PSD 路径夹具验证链接保留，真实源文件覆盖和项目刷新已经实际执行。
Windows Browse 原生对话框点击、外部 Explorer OLE、跨显示器、超大图像/大量图层帧率仍未验。
旧版旋转/斜切图形不属于本轮简化手柄的视觉覆盖，旧输出仍使用 DesignCraft 文档渲染。
不能把这些未验路径描述为已通过。

最终静态结果：相关六个 crate 共 16 项测试通过，locked shell 构建、fmt、六项治理守卫及 diff whitespace 检查通过。
