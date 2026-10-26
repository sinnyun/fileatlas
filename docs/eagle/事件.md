# event（事件）

## onPluginCreate(callback) <a href="#gylpl" id="gylpl"></a>

插件窗口建立时，Eagle 会主动调用这个方法，你可以使用此方法初始化插件需要的模块。

* `callback` Function
  * `plugin` Object - 插件属性
    * `manifest` Object - 插件 manifest.json 完整配置。
    * `path` String - 插件所在路径

```javascript
eagle.onPluginCreate((plugin) => {
    console.log(plugin.manifest.name);
    console.log(plugin.manifest.version);
    console.log(plugin.manifest.logo);
    console.log(plugin.path);
});
```

{% hint style="info" %}
提示：如果插件不需要 manifest 信息就可以运行，那么你也可以使用 `window.onload` 来进行开发。
{% endhint %}

## onPluginRun(callback) <a href="#gylpl" id="gylpl"></a>

当用户点击插件面板的插件时，Eagle 会主动调用这个方法。

* `callback` Function

```javascript
eagle.onPluginRun(() => {
    console.log('eagle.onPluginRun');
});
```

## onPluginBeforeExit(callback) <a href="#z1a5y" id="z1a5y"></a>

插件窗口关闭前 Eagle 会主动调用这个方法。

* `callback` Function

```javascript
eagle.onPluginBeforeExit(() => {
    console.log("插件将退出");
});

// 阻止窗口关闭
window.onbeforeunload = (event) => {
    return event.returnValue = false;
};
```

{% hint style="info" %}
提示：如果你想要阻止窗口被关闭，你可以注册 `window.onbeforeunload`方法避免窗口被关闭。
{% endhint %}

## onPluginShow(callback) <a href="#w2vxi" id="w2vxi"></a>

插件窗口显示时时，Eagle 会主动调用这个方法。

* `callback` Function

```javascript
eagle.onPluginShow(() => {
    console.log("插件窗口显示");
});
```

## onPluginHide(callback) <a href="#zgvst" id="zgvst"></a>

插件窗口隐藏时时，Eagle 会主动调用这个方法。

* `callback` Function

```javascript
eagle.onPluginHide(() => {
    console.log("插件窗口隐藏");
});
```

## onLibraryChanged(callback) <a href="#g3tny" id="g3tny"></a>

当用户切换资源库时，Eagle 会主动调用这个方法。

* `callback` Function
  * `libraryPath` String - 当前资源库路径。

```javascript
eagle.onLibraryChanged((libraryPath) => {
    console.log(`侦测到资源库切换，新的资源库路径: ${libraryPath}`);
});
```

{% hint style="info" %}
提示：如果你需要获取更完整的资源库信息，你可以使用 `eagle.library.info()` 方法取得。
{% endhint %}

{% hint style="warning" %}
**注意：** 如果插件执行过程必须依赖相对的资源库路径，你可能需要透过注册此方法，在资源库切换时，做出对应的调整，避免程序执行过程发生错误。
{% endhint %}

## onThemeChanged(callback) <a href="#xlf6z" id="xlf6z"></a>

Eagle 主程序主题配色改变是，Eagle 会主动调用这个方法，如果插件支持多种配色主题，你可以使用此方法做出对应的 UI 调整。

* `callback` Function
  * `theme` String - 当前主题配色的名称，如 `Auto`、`LIGHT`、`LIGHTGRAY`、`GRAY`、`DARK`、`BLUE`、`PURPLE`。

```javascript
eagle.onThemeChanged((theme) => {
    console.log(`配色主题以改为: ${theme}`);
});
```

### &#x20;<a href="#nptwx" id="nptwx"></a>
