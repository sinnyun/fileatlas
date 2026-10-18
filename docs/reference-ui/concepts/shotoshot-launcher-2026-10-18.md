# Shotoshot 快捷启动管理面板清单

日期：2026-10-18。依据用户提供的 Shotoshot GitHub 截图及明确说明。

## 功能边界

- PhoTag 只做已安装/可运行状态、Shotoshot 可执行文件选择、专用截图目录设置、立即启动按钮和快捷键入口。PhoTag 启动时只扫描一次配置路径和常见安装位置，将结果缓存；工具条渲染和每次截图不重复扫描。未检测到时显示“获取 Shotoshot”和“选择程序”，由用户安装并指向 `shotoshot.exe`。
- 截图选择层、截屏、图像预览、所有标注工具、撤销/重做、剪贴板和文件保存窗口全部使用 Shotoshot 自带界面与操作。
- PhoTag 不画截图画布，不实现标注对象，也不在自身面板内复制一套截图工具栏。
- 启动命令调用 Shotoshot 的 `--capture` 模式；全局启动快捷键由现有 Shortcuts 插件注册与分发。

## PhoTag 管理面板结构

- 使用现有通用浮窗外壳和统一深蓝主题；标题为“截图工具”。
- 正文为紧凑纵向设置卡片：上方显示 Shotoshot 可执行文件状态/路径与“选择程序”操作；中部显示截图目录与“更改目录”操作；底部给出“启动截图”主操作和实际快捷键提示。
- 启动按钮在启动时检测不到程序时禁用，并显示明确提示和上游安装页；文件选择取消不改设置；用户可重新定位路径。捕获时向已运行实例转发请求，没有运行实例时启动 Shotoshot。
- 保存目录默认为 `%LOCALAPPDATA%\\PhoTag\\Screenshots`。选择后由 Shotoshot 官方 `Settings::Store::save_destination` 接口写入 Shotoshot 设置，并保留其余设置字段；PhoTag 自己的两个配置值写入既有 `SettingsService`。
- 专用目录不存在时创建；创建失败或 Shotoshot 设置写入失败时保留旧设置并显示错误。

## 来源、许可与限制

- 上游仓库：[Chmyke/shotoshot](https://github.com/Chmyke/shotoshot)，仓库 GPL-3.0-only。
- PhoTag 不复制或编译 Shotoshot 源码；用户单独安装官方程序，PhoTag 只保存可执行文件路径并通过启动参数/Windows 消息请求截图。
- 这是用户个人使用的软件；许可不设白名单。若分发组合程序，需按 GPL-3.0 及依赖许可证处理，不因本集成自动修改工作区 MIT 声明。
- Shotoshot 的截图保存对话框记忆其上次目录。PhoTag 只更新 Shotoshot 版本化设置中的输出目录字段，并通过同名 Windows 互斥锁和原子替换保留其他设置。
- Shotoshot 自带界面目前为英文；界面语言和工具行为由上游程序管理。
- PhoTag 默认把 Shotoshot 最终保存的 PNG 放入 `%LOCALAPPDATA%\\PhoTag\\Screenshots`。该目录属于截图插件的专用存储位置；当前不会把这些 PNG 自动导入 PhoTag 照片数据库。
