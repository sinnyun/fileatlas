# dialog（对话框）

下面是一个选择多个文件的对话框示例：

```javascript
let result = await eagle.dialog.showOpenDialog({ 
    properties: ['openFile', 'multiSelections'] 
});
```

***

#### 方法 <a href="#z1a5y" id="z1a5y"></a>

## showOpenDialog(options) <a href="#tkp0d" id="tkp0d"></a>

显示打开文件对话框。

* `options` Object
  * `title` string (可选) - 对话框窗口的标题
  * `defaultPath` string (可选) - 对话框的默认展示路径
  * `buttonLabel` string (可选) - 「确认」按钮的自定义标签, 当为空时, 将使用默认标签。
  * `filters` [FileFilter](https://www.electronjs.org/zh/docs/latest/api/structures/file-filter)\[] (可选)
    * `name` string
    * `extensions` string\[]
  * `properties` string\[] (可选) - 包含对话框相关属性。 支持以下属性值:
    * `openFile` - 允许选择文件
    * `openDirectory` - 允许选择文件夹
    * `multiSelections`- 允许多选。
    * `showHiddenFiles`- 显示对话框中的隐藏文件。
    * `createDirectory` `macOS` - 允许你通过对话框的形式创建新的目录。
    * `promptToCreate` `Windows`- 如果输入的文件路径在对话框中不存在, 则提示创建。 这并不是真的在路径上创建一个文件，而是允许返回一些不存在的地址交由应用程序去创
  * `message` string (可选) `macOS` - 显示在输入框上方的消息。
* 返回 `Promise<result: Object>`
  * `result`Object
    * `canceled` boolean - 对话框是否被取消
    * `filePaths` string\[] - 用户选择的文件路径的数组. 如果对话框被取消，这将是一个空的数组。

```javascript
{
  filters: [
    { name: 'Images', extensions: ['jpg', 'png', 'gif'] },
    { name: 'Movies', extensions: ['mkv', 'avi', 'mp4'] },
    { name: 'Custom File Type', extensions: ['as'] },
    { name: 'All Files', extensions: ['*'] }
  ]
}
```

```javascript
let result = await eagle.dialog.showOpenDialog({
    properties: ['openFile', 'openDirectory']
});
```

{% hint style="info" %}
备注：此功能与 Electron API 的 [dialog.showOpenDialog](https://www.electronjs.org/zh/docs/latest/api/dialog#dialogshowopendialogbrowserwindow-options) 功能类似。
{% endhint %}

***

## showSaveDialog(options) <a href="#g872m" id="g872m"></a>

显示保存文件对话框。

* `options` Object
  * `title` string (可选) - 对话框窗口的标题
  * `defaultPath` string (可选) - 对话框的默认展示路径
  * `buttonLabel` string (可选) - 「确认」按钮的自定义标签, 当为空时, 将使用默认标签。
  * `filters` [FileFilter](https://www.electronjs.org/zh/docs/latest/api/structures/file-filter)\[] (可选)
    * `name` string
    * `extensions` string\[]
  * `properties` string\[] (可选) - 包含对话框相关属性。 支持以下属性值:
    * `openDirectory` - 允许选择文件夹
    * `showHiddenFiles`- 显示对话框中的隐藏文件。
    * `createDirectory` `macOS` - 允许你通过对话框的形式创建新的目录。
* 返回 `Promise<result: Object>`
  * `result`Object
    * `canceled` boolean - 对话框是否被取消
    * `filePath` string - 如果对话框被取消，该值为 `undefined`。

```javascript
{
  filters: [
    { name: 'Images', extensions: ['jpg', 'png', 'gif'] },
    { name: 'Movies', extensions: ['mkv', 'avi', 'mp4'] },
    { name: 'Custom File Type', extensions: ['as'] },
    { name: 'All Files', extensions: ['*'] }
  ]
}
```

```javascript
let result = await eagle.dialog.showSaveDialog({
    properties: ['openDirectory']
});
```

{% hint style="info" %}
备注：此功能与 Electron API 的 [dialog.showSaveDialog](https://www.electronjs.org/zh/docs/latest/api/dialog#dialogshowsavedialogbrowserwindow-options) 功能类似。
{% endhint %}

***

## showMessageBox(options) <a href="#grq5h" id="grq5h"></a>

显示讯息对话框。

* `options`Object
  * `message` string - 对话框主要内容
  * `title` string (可选) - 对话框标题
  * `detail` string (可选) - 额外信息
  * `buttons` strings\[] (可选) - 按钮文本数组
  * `type` string (可选) - 可以为 `none`、 `info`、 `error`、`question` 或者 `warning`
* 返回 `Promise<result: Object>`
  * `result`Object
    * `response` Interger - 点击按钮的索引

```javascript
let result = await eagle.dialog.showMessageBox({
    title: "Messagebox title",
    message: "Message from the Plugin process",
    detail: "Ultra message here",
    buttons: ["确定", "取消"],
    type: "info"
});

console.log(result);		// {response: 0}
```

{% hint style="info" %}
此功能与 Electron API 的 [dialog.showSaveDialog](https://www.electronjs.org/zh/docs/latest/api/dialog#dialogshowsavedialogbrowserwindow-options) 功能类似。
{% endhint %}

***

## showErrorBox(title, content) <a href="#erokr" id="erokr"></a>

显示错误讯息的对话框。

* `title` string - 显示在错误框中的标题
* `content` string - 显示在错误框中的文本内容
* 返回 `Promise<void>`

{% code overflow="wrap" %}

```javascript
await eagle.dialog.showErrorBox("Error box title", "Error message from the Plugin process");
```

{% endcode %}

{% hint style="info" %}
备注：此功能与 Electron API 的 [dialog.showSaveDialog](https://www.electronjs.org/zh/docs/latest/api/dialog#dialogshowsavedialogbrowserwindow-options) 功能类似。
{% endhint %}

### &#x20;<a href="#nptwx" id="nptwx"></a>
