# 全文件与格式角标验证（2026-10-14）

参考：`docs/reference-ui/concepts/sidebar-pages-preview-2026-10-14-v2.png`、其说明及 final-v5 主界面。保持布局结构，按本轮用户要求替换卡片角落 More 为格式角标。长格式在紧凑列表角标中省略，完整名称仍在文件名/Inspector中可见。

环境：Windows MSVC，Rust 1.97.1，窗口1500×1000物理像素，150%显示缩放，中文；FileAtlas MCP server/GPUI 同进程，由项目客户端启动；profile=`target/mcp-all-files-final-20261014`，仅测试库及隔离设置/缓存，修改权限显式开启。正常FileAtlas数据库及正在运行的旧窗口未修改。

命令：

```powershell
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 check --locked -j 2 -p fileatlas-shell
cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-indexing -p fileatlas-plugin-database -p fileatlas-plugin-watcher -p fileatlas-plugin-dev-test -p fileatlas-plugin-file-manager
node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell-all-files.exe --profile target/mcp-all-files-final-20261014 --plan target/mcp-all-files-final-plan.json --interactive --allow-mutations
```

默认`cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`因用户运行中的`target/debug/fileatlas-shell.exe`无法覆盖（os error 5）。为验证，临时在shell Cargo.toml声明同src/main.rs的`fileatlas-shell-all-files`及原`fileatlas-shell` bin，执行：

```powershell
cargo +1.97.1 rustc --locked -j 2 -p fileatlas-shell --bin fileatlas-shell-all-files -- -C link-arg=/STACK:8388608
```

随后还原Cargo.toml，无产品配置差异。最终编译成功。第一次备用bin没有继承build.rs只针对原bin的8 MiB stack参数，启动stack overflow；显式加相同stack参数后正常，最终stderr无崩溃。初次客户端过早resize也返回window-handle未就绪；最终plan先等待窗口启动。这些是验证启动问题，不计为通过。

## 已验

- 16个普通文件全部索引显示：5图片（含损坏图及视频封面）、1音频、1视频、9通用文件；包含PDF、ZIP、Rust、PSD、字体、未知长格式、无扩展名、隐藏文件及隐藏目录内文件。Fixture manifest记录真实文件；模拟视频仅用于封面配对，未声称可播放。
- 所有文件视图数量16，图片过滤仍为5；通用`file`查询9。Grid/List均显示类型图标及格式角标。损坏JPG、WAV、文档、压缩包及任意未知格式可见；Inspector文档与通用图标可见。无右上ellipsis按钮，selection边框仍正常。
- Native GPUI右键输入在`.config`打开真实菜单；移动至评分及子菜单，点击3星成功。前后MCP查询评分3，随后Add同已注册目录再次扫描，评分仍3且ID不变、目录无重复、总数仍16。
- 真实Windows watcher收录新增`15-live.custom`为通用文件，删除后消失；`empty.custom`目录不会被索引为文件。Scanner/Watcher自动回归同时覆盖无扩展名、隐藏与音频以及目录排除。
- 27项自动测试通过，1项FFmpeg外部工具测试保持原ignored状态。扫描回归在两次扫描间新增文件，确认旧ID、rating、date_taken、modified_at和media_tags都保留；所有源文件通过数据库服务访问，schema未变。

## 诊断与边界

旧Scanner使用INSERT OR REPLACE，重复扫描会换ID且删除评分/关联，测试中确实复现；现改用scanner专用数据库upsert并完成最终重测。`pre-upsert-retention-report.json`是修复前的诊断记录，保留旧请求ID；修复前的保留评分断言失败，不作为最终覆盖。显式Navigation Rebuild/Rescan仍沿用清空媒体行的原语义；补入旧版遗漏文件使用Add同目录的增量扫描。未自动启动正常库迁移。

此修改不增加预览codec支持。未知格式的inline preview仍按原逻辑提示不支持，右键Open使用默认应用。图片异步加载失败的fallback已实现并编译，失效缓存路径没有单独注入实测。网络目录/权限错误/大规模未知格式库/目录rename竞态未逐项测试。最终截图验证针对本轮功能，不声称完整mockup像素或全应用验收。


最终缓存补充：扫描thumbnail cache命中可能返回0尺寸/空micro。扫描批量UPDATE改用COALESCE保留已有尺寸/micro；Indexing回归再次通过并断言32×20保留。重编译同备用bin后，在`target/mcp-all-files-cache-final-20261014`执行`cache-retention-plan.json`的32步，全部通过；再次验证右键评分、重复扫描不换ID/评分，并读取已缓存JPG仍为24×16。`cache-retention-report.json`及对应截图记录最后源码的同进程结果，原45步记录覆盖其他界面与watcher路径。
