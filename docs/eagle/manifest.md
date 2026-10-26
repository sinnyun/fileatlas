# manifest.json 完整配置

每个插件都必须包含一个 `manifest.json` 文件。这个文件定义了插件执行方式及插件的基本信息，例如插件的名称、版本号和执行代码的入口点。

以下是 `manifest.json` 文件支持的所有字段：

```json
{
    "id": "LBCZE8V6LPCKD",
    "version": "1.0.0",
    "platform": "all",
    "arch": "all",
    "name": "窗口插件",
    "logo": "/logo.png",
    "keywords": [],
    "devTools": false,
    "main":
    {
        "url": "index.html",
        "width": 640,
        "height": 480,
        "minWidth": 640,
        "minHeight": 480,
        "maxWidth": 640,
        "maxHeight": 480,
        "alwaysOnTop": false,
        "frame": true,
        "fullscreenable": true,
        "maximizable": true,
        "minimizable": true,
        "resizable": true,
        "backgroundColor": "#ffffff",
        "childWindow": false,
        "followCursor": false,
        "multiple": false,
        "runAfterInstall": false
    }
}
```

## 字段说明：

* `id` - 插件 ID
* `version` - 插件版本
* `platform` - 支持平台
  * `all` - 所有平台
  * `mac` - mac 系统
  * `win` - Windows 系统
* `arch` - CPU 架构
  * `all` - 所有架构
  * `arm` - 仅支持 arm 架构
  * `x64` - 仅支持 x64 架构
* `name` - 插件名
* `logo` - 插件代表图标（仅支持 `png`, `jpg`, `webp` 格式）
* `keywords` - 插件关键字，除了插件名外用户还可以使用这些关键字快速搜索到这个插件
* `devTools` - 是否打开开发调试窗口
* `main` - 插件窗口程序入口设置
  * `url` - 入口页面
  * `width` - 窗口宽度
  * `height` - 窗口高度
  * `minWidth` - 窗口最小宽度
  * `minHeight` - 窗口最小高度
  * `maxWidth` - 窗口最大宽度
  * `maxHeight` - 窗口最大高度
  * `alwaysOnTop` - 窗口是否永远在别的窗口的上面， 默认值为 `false`。
  * `frame` - 默认值为 `true`，当为 `false` 时，将使用[无边框窗口](https://developer.eagle.cool/plugin-api/zh-cn/tutorial/frameless-window)，这是一种特殊的窗口模式，它不带有外壳（包括窗口边框、标题栏、工具栏等），只含有网页内容。
  * `fullscreenable` - 窗口是否可以进入全屏状态，默认值为 `true`。
  * `maximizable` - 窗口是否最大化，默认值为 `true`。
  * `minimizable` - 窗口是否可最小化，默认值为 `true`。
  * `resizable` - 窗口大小是否可调整，默认值为 `true`。
  * `backgroundColor` - 窗口背景色，默认值为`#FFF`。
  * `childWindow` - 是否以子窗口形式打开（附属于主窗口），默认值为 `false`。
  * `followCursor` - 打开时窗口自动定位在光标附近，适合需要快速交互的工具型插件，默认值为 `false`。需要 Eagle 4.0 Build22 以上版本。
  * `multiple` - 窗口是否可以多開，默認為 `false`。
  * `runAfterInstall` - 安装后自动打开，默认为 `false`
