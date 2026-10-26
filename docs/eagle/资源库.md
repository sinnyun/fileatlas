# library（资源库）

## 方法 <a href="#z1a5y" id="z1a5y"></a>

## info() <a href="#s7pkf" id="s7pkf"></a>

取得当前资源库详细信息，包含文件夹、智能文件夹、标签群组等

* 返回 `Promise<data: Object>`
  * `data` Object - 资源库各项属性

```javascript
console.log(await eagle.library.info());
```

***

## 屬性 <a href="#adtwq" id="adtwq"></a>

`library` 模块包含以下属性：

## `name` string <a href="#qxggi" id="qxggi"></a>

返回当前资源库名称

```javascript
console.log(eagle.library.name)
// test
```

## `path` string <a href="#qxggi" id="qxggi"></a>

返回当前资源库所在路径

```javascript
console.log(eagle.library.path);
// C:\Users\User\Pictures\Design.library
```

## `modificationTime` Integer

返回最后修改时间 (timestamp)

```javascript
console.log(eagle.library.modificationTime);
// 1681281134495
```
