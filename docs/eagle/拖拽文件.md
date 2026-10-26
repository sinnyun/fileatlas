# drag（拖拽文件）

#### 方法 <a href="#z1a5y" id="z1a5y"></a>

## startDrag(filePaths) <a href="#tkp0d" id="tkp0d"></a>

显示通知窗口

* `filePaths` string\[] - 欲拖拽文件路径
* 返回 `Promise<>`

```javascript
await eagle.drag.startDrag([
    "C:\\Users\\User\\Pictures\\drag1.jpg",
    "C:\\Users\\User\\Pictures\\drag2.jpg",
]);
```

{% hint style="info" %}
备注：此功能与 Electron API 的 [webContents.startDrag(item)](https://www.electronjs.org/zh/docs/latest/tutorial/native-file-drag-drop) 功能类似。
{% endhint %}

#### &#x20;<a href="#p4ult" id="p4ult"></a>

### &#x20;<a href="#nptwx" id="nptwx"></a>
