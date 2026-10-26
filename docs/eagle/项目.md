# item（项目）

```javascript
eagle.onPluginCreate(async (plugin) => {
    // 取得 Eagle 应用当前被选中的文件
    let items = await eagle.item.getSelected();
    let item = items[0];
    
    // 修改属性
    item.name = 'New Name';
    item.tags = ['tag1', 'tag2'];
    
    // 保存修改
    await item.save();
});
```

{% hint style="success" %}
**🦄 最佳实践：** 为了确保数据安全性，请使用 `item` API 提供的 `save()` 方法进行数据的存取与修改，应避免直接修改 Eagle 资源库底下的 `metadata.json` 或任意文件。
{% endhint %}

***

## 方法 <a href="#z1a5y" id="z1a5y"></a>

## get(options) <a href="#bdcw2" id="bdcw2"></a>

万用搜索方法，可获取指定条件的文件。

* `options` Object - 查询条件
  * `id` string (可选) - 文件 id
  * `ids` string\[] (可选) - 文件 id 数组
  * `isSelected` boolean (可选) - 正在被选中的文件
  * `isUntagged` boolean (可选) - 尚未标签
  * `isUnfiled` boolean (可选) - 尚未分类
  * `keywords` string\[] (可选) - 包含关键字
  * `tags` string\[] (可选) - 包含标签
  * `folders` string\[] (可选) - 包含文件夹
  * `ext` string (可选) - 格式
  * `annotation` string (可选) - 注释
  * `rating` Interger (可选) - 评分，`0 ~ 5`
  * `url` string (可选) - 来源链接
  * `shape` string (可选) - 形状，`square`、`portrait`、`panoramic-portrait`、`landscape`、`panoramic-landscape`
  * `fields` string\[] (可选) - 指定返回的字段，仅返回需要的数据以提升性能
* 返回 `Promise<items: Item[]>` - `items` 查询结果

```javascript
let items = await eagle.item.get({
    ids: [],
    isSelected: true,
    isUnfiled: true,
    isUntagged: true,
    keywords: [""],
    ext: "",
    tags: [],
    folders: [],
    shape: "square",
    rating: 5,
    annotation: "",
    url: ""
});


let selected = await eagle.item.get({
    isSelected: true
});

let jpgs = await eagle.item.get({
    ext: "jpg"
});

// 仅获取特定字段以提升性能
let itemsWithFields = await eagle.item.get({
    tags: ["Design"],
    fields: ["id", "name", "tags", "modifiedAt"]
});
```

{% hint style="info" %}
提示：使用 `fields` 参数可以显著提升性能，特别是在处理大量文件时只需要部分信息的场景。
{% endhint %}

***

## getAll() <a href="#na8ve" id="na8ve"></a>

返回所有文件

* 返回 `Promise<items: Item[]>` - `items` 所有文件

```javascript
let items = await eagle.item.getAll();
console.log(items);
```

{% hint style="success" %}
**🦄 最佳实践：** 如果资源库文件数量非常多（例：20W+），避免无限制的呼叫此方法，避免造成应用性能的降低。
{% endhint %}

***

## getById(itemId) <a href="#katrb" id="katrb"></a>

返回指定 ID 之文件

* `itemId` string
* 返回 `Promise<item: Item>` - `item` 对应 ID 的文件

```javascript
let item = await eagle.item.getById('item_id');
console.log(item);
```

## getByIds(itemIds) <a href="#by1ek" id="by1ek"></a>

返回指定 IDs 之文件

* `itemIds` string\[]
* 返回 `Promise<items: Item[]>` - `items` 对应 IDs 的文件

```javascript
let items = await eagle.item.getByIds(['item_id_1', 'item_id_2']);
console.log(items);
```

***

## getSelected() <a href="#ffgvj" id="ffgvj"></a>

返回应用当前选中的文件

* 返回 `Promise<items: Item[]>` - `items` 选中之文件

```javascript
let selected = await eagle.item.getSelected();
console.log(selected);
```

***

## getIdsWithModifiedAt() <a href="#getidswithmodifiedat" id="getidswithmodifiedat"></a>

快速获取所有文件的 ID 和最后修改时间

* 返回 `Promise<items: Object[]>` - 包含 `id` 和 `modifiedAt` 的对象数组

```javascript
let idsWithTime = await eagle.item.getIdsWithModifiedAt();
console.log(idsWithTime);
// 输出示例：
// [
//   { id: "ITEM_ID_1", modifiedAt: 1625123456789 },
//   { id: "ITEM_ID_2", modifiedAt: 1625123456790 },
//   ...
// ]

// 可用于增量同步或检测文件变化
let changedItems = idsWithTime.filter(item => 
    item.modifiedAt > lastSyncTime
);
```

{% hint style="info" %}
提示：此方法专门优化用于获取文件 ID 和修改时间，比使用 `get()` 方法获取完整数据要快得多。
{% endhint %}

***

## count(options) <a href="#count" id="count"></a>

计算符合条件的文件数量，支持与 `get()` 方法相同的查询条件。

* `options` Object - 查询条件（与 `get()` 方法相同）
  * `id` string (可选) - 文件 id
  * `ids` string\[] (可选) - 文件 id 数组
  * `isSelected` boolean (可选) - 正在被选中的文件
  * `isUntagged` boolean (可选) - 尚未标签
  * `isUnfiled` boolean (可选) - 尚未分类
  * `keywords` string\[] (可选) - 包含关键字
  * `tags` string\[] (可选) - 包含标签
  * `folders` string\[] (可选) - 包含文件夹
  * `ext` string (可选) - 格式
  * `annotation` string (可选) - 注释
  * `rating` Interger (可选) - 评分，`0 ~ 5`
  * `url` string (可选) - 来源链接
  * `shape` string (可选) - 形状，`square`、`portrait`、`panoramic-portrait`、`landscape`、`panoramic-landscape`
* 返回 `Promise<count: number>` - `count` 符合条件的文件数量

```javascript
// 计算 JPG 格式文件数量
let jpgCount = await eagle.item.count({
    ext: "jpg"
});

// 计算带有特定标签的文件数量
let taggedCount = await eagle.item.count({
    tags: ["Design", "Illustration"]
});

// 计算未分类文件数量
let unfiledCount = await eagle.item.count({
    isUnfiled: true
});
```

{% hint style="info" %}
提示：当只需要获取文件数量时，使用 `count()` 比 `get()` 性能更好。
{% endhint %}

***

## countAll() <a href="#countall" id="countall"></a>

快速返回资源库中所有文件的总数

* 返回 `Promise<count: number>` - `count` 所有文件数量

```javascript
let totalCount = await eagle.item.countAll();
console.log(`资源库共有 ${totalCount} 个文件`);
```

{% hint style="info" %}
提示：`countAll()` 针对性能进行了优化，比 `getAll()` 后计算数组长度要快得多。
{% endhint %}

***

## countSelected() <a href="#countselected" id="countselected"></a>

返回应用当前选中的文件数量

* 返回 `Promise<count: number>` - `count` 选中的文件数量

```javascript
let selectedCount = await eagle.item.countSelected();
console.log(`当前选中了 ${selectedCount} 个文件`);
```

***

## select(itemIds) <a href="#select" id="select"></a>

选中指定的文件

* `itemIds` string\[] - 要选中的文件 ID 数组
* 返回 `Promise<result: boolean>` - `result` 是否选中成功

```javascript
// 选中单个文件
await eagle.item.select(['ITEM_ID_1']);

// 选中多个文件
await eagle.item.select(['ITEM_ID_1', 'ITEM_ID_2', 'ITEM_ID_3']);

// 清空选中
await eagle.item.select([]);
```

{% hint style="info" %}
提示：调用此方法会替换当前的选中状态，而不是追加到现有选中项。
{% endhint %}

{% hint style="info" %}
提示：`select()` 方法需要 Eagle 4.0 build12 以上版本支持。
{% endhint %}

***

## addFromURL(url, options) <a href="#tg9ak" id="tg9ak"></a>

将图片链接添加至 Eagle

* `url`string - 欲添加图片链接，支持 `http`、 `https`、 `base64`
* `options` Object
  * `name` string (可选) - 文件名
  * `website` string (可选) - 来源网址
  * `tags` string\[] (可选) - 标签
  * `folders` string\[] (可选) - 所属文件夹 IDs
  * `annotation` string (可选) - 注释
* 返回 `Promise<itemId: string>` - `itemId`成功创建的项目 ID

```javascript
const imgURL = 'https://cdn.dribbble.com/userupload/3885520/file/original-ee68b80a6e10edab6f192e1e542da6ed.jpg';
const itemId = await eagle.item.addFromURL(imgURL, { 
    name: 'Camping', 
    website: 'https://dribbble.com/shots/19744134-Camping-2', 
    tags: ["Dribbble", "Illustration"],
    folders: [],
    annotation: 'add from eagle api',
});
```

***

## addFromBase64(base64, options) <a href="#zmwst" id="zmwst"></a>

添加 base64 图像至 Eagle

* `base64`string - base64 格式图像
* `options` Object
  * `name` string (可选) - 文件名
  * `website` string (可选) - 来源网址
  * `tags` string\[] (可选) - 标签
  * `folders` string\[] (可选) - 所属文件夹 IDs
  * `annotation` string (可选) - 注释
* 返回 `Promise<itemId: string>` - `itemId`成功创建的项目 ID

```javascript
const base64 = 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNDAgMjM0IiBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCAyNDAgMjM0Ij48cGF0aCBmaWxsLXJ1bGU9ImV2ZW5vZGQiIGNsaXAtcnVsZT0iZXZlbm9kZCIgZmlsbD0iIzI2MTMwMCIgZD0iTTEwIDEwaDIyMHYyMTMuOTk5aC0yMjB6Ii8+PHBhdGggZD0iTTAgMHYyMzRoMjQwLjAwMXYtMjM0aC0yNDAuMDAxem0xMCAxMGgyMjAuMDAxdjIxNGgtMjIwLjAwMXYtMjE0em03My4yNTIgMTIyLjUwMWwtNy45MiAyOS45ODJjLS4xNjUuODI0LS40OTUgMS4wMTgtMS40ODUgMS4wMThoLTE0LjY4N2MtLjk4OCAwLTEuMTUyLS4zMy0uOTg4LTEuNDg1bDI4LjM4LTk5LjQ0OGMuNDk1LTEuODE1LjgyNS0zLjM3Ny45OS04LjMyOCAwLS42Ni4zMy0uOTkuODI1LS45OWgyMC45NTVjLjY2IDAgLjk5LjE2NSAxLjE1NS45OWwzMS44NDUgMTA3Ljk0Yy4xNjUuODI0IDAgMS4zMi0uODI1IDEuMzJoLTE2LjVjLS44MjQgMC0xLjMxOS0uMTkzLTEuNDg0LS44NTRsLTguMjUtMzAuMTQ2aC0zMi4wMTF6bTI3Ljg4NS0xNi4yNWMtMi44MDUtMTEuMDU2LTkuNDA1LTM1LjI4Ni0xMS44OC00N2gtLjE2NWMtMi4xNDYgMTEuNzE1LTcuNDI1IDMxLjQ5LTExLjU1IDQ3aDIzLjU5NXptNDQuOTkzLTU1LjU3OGMwLTYuNDM1IDQuNDU1LTEwLjIzIDEwLjIzLTEwLjIzIDYuMTA1IDAgMTAuMjMgNC4xMjUgMTAuMjMgMTAuMjMgMCA2LjYtNC4yOSAxMC4yMy0xMC4zOTUgMTAuMjMtNS45NCAwLTEwLjA2NS0zLjYzLTEwLjA2NS0xMC4yM3ptMS4xMiAyMi43MzJjMC0uODI1LjMzLTEuMTU1IDEuMTU1LTEuMTU1aDE1LjY4OWMuODI1IDAgMS4xNTUuMzMgMS4xNTUgMS4xNTV2NzguOTM5YzAgLjgyNi0uMTY1IDEuMTU2LTEuMTU1IDEuMTU2aC0xNS41MjRjLS45OSAwLTEuMzItLjQ5Ni0xLjMyLTEuMzJ2LTc4Ljc3NXoiIGZpbGwtcnVsZT0iZXZlbm9kZCIgY2xpcC1ydWxlPSJldmVub2RkIiBmaWxsPSIjRkY3QzAwIi8+PC9zdmc+';
const itemId = await eagle.item.addFromBase64(base64, { 
    name: 'Illustation Logo', 
    website: 'https://www.eagle.cool/', 
    tags: ["Adobe", "Logo"],
    folders: [],
    annotation: 'ai logo form api',
});
```

***

## addFromPath(path, options) <a href="#lnsox" id="lnsox"></a>

从本地文件路径添加文件至 Eagle

* `path`string - 欲添加文件路径
* `options` Object
  * `name` string (可选) - 文件名
  * `website` string (可选) - 来源网址
  * `tags` string\[] (可选) - 标签
  * `folders` string\[] (可选) - 所属文件夹 IDs
  * `annotation` string (可选) - 注释
* 返回 `Promise<itemId: string>` - `itemId`成功创建的项目 ID

```javascript
const filePath = 'C:\\Users\\User\\Downloads\\ai.svg';
const itemId = await eagle.item.addFromPath(filePath, { 
    name: 'Illustation Logo', 
    website: 'https://www.eagle.cool/', 
    tags: ["Adobe", "Logo"],
    folders: [],
    annotation: 'ai logo form api',
});
```

***

## addBookmark(url, options) <a href="#atulp" id="atulp"></a>

添加书签链接至 Eagle

* `url`string - 欲添加书签链接
* `options` Object
  * `name` string (可选) - 书签名
  * `base64` string (可选) - 自订缩图 base64 格式
  * `tags` string\[] (可选) - 标签
  * `folders` string\[] (可选) - 所属文件夹 IDs
  * `annotation` string (可选) - 注释
* 返回 `Promise<itemId: string>` - `itemId`成功创建的项目 ID

```javascript
const bookmarkURL = 'https://www.google.com/';
const itemId = await eagle.item.addBookmark(bookmarkURL, { 
    name: 'Eagle', 
    tags: ["Eagle", "Site"],
    folders: [],
    annotation: 'bookmark form api',
});
```

```javascript
const bookmarkURL = 'https://www.google.com/';
const base64 = 'data:image/png;base64,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';
const itemId = await eagle.item.addBookmark(bookmarkURL, { 
    name: 'Eagle', 
    base64: base64,
    tags: ["Eagle", "Site"],
    folders: [],
    annotation: 'bookmark form api',
});
```

***

## open(itemId, options) <a href="#yxkul" id="yxkul"></a>

在全部列表显示 `itemId` 对应的文件

* `itemId`string - 欲显示文件 ID
* `options` Object (可选) - 开启选项
  * `window` boolean (可选) - 是否在新窗口中开启文件，默认为 `false`
* 返回 `Promise<result: boolean>`

```javascript
// 在当前窗口开启
await eagle.item.open("item_id");

// 在新窗口开启
await eagle.item.open("item_id", { window: true });
```

{% hint style="info" %}
提示：`window` 参数需要 Eagle 4.0 build12 以上版本支持。
{% endhint %}

{% hint style="info" %}
提示：你也可以直接呼叫 item 实例的 `open()` 方法打开文件。
{% endhint %}

***

## 類：Item <a href="#uezi0" id="uezi0"></a>

由 Eagle API `get`返回的 Object 类型，提供修改、保存功能。

{% hint style="success" %}
**🦄 最佳实践：** 为了确保数据安全性，请使用 Item 实例提供的 `save()` 方法进行数据的存取与修改，应避免直接修改 Eagle 资源库底下的 `metadata.json` 或任意文件。
{% endhint %}

***

#### 实例方法 <a href="#sihmc" id="sihmc"></a>

### **save()**

保存所有修改

* 返回 `Promise<result: boolean>` - `result`是否修改成功

```javascript
let item = await eagle.item.getById('item_id');
item.name = 'New Name';
item.tags = ['tag_1', 'tag_2'];

// 保存修改
await item.save();
```

***

### moveToTras&#x68;**()**

将文件丢到垃圾桶

* 返回 `Promise<result: boolean>` - `result`是否成功删除

```javascript
await item.moveToTrash();
```

***

### **replaceFile(filePath)**

使用指定文件替换原文件，将自动刷新缩略图，无须再次呼叫 `refreshThumbnail()`。

{% hint style="success" %}
**🦄 最佳实践：** 直接对要更改的文件进行操作是具有风险的，若过程中出现错误或异常，都有可能造成文件损毁且无法复原。因此，先将新版本文件保存在电脑其它路径，确定无误后，再使用 `replaceFile()`方法来替换是更稳健的作法。
{% endhint %}

* `filePath`string - 欲替换文件之路径
* 返回 `Promise<result: boolean>` - `result`是否替换成功

```javascript
let item = await eagle.item.getById('item_id');
let result = await item.replaceFile('new_file_path');

console.log(result);
```

***

### **refreshThumbnail()**

重新刷新文件缩略图，同时也会重新获取文件大小、颜色分析、尺寸等属性。

* 返回 `Promise<result: boolean>` - `result`是否成功

```javascript
let item = await eagle.item.getById('item_id');
let result = await item.refreshThumbnail();

console.log(result);
```

***

### **setCustomThumbnail(thumbnailPath)**

为文件设置自定缩略图。

* `thumbnailPath`string - 欲设置缩略图的路径
* 返回 `Promise<result: boolean>` - `result`是否替换成功

```javascript
let item = await eagle.item.getById('item_id');
let result = await item.setCustomThumbnail('thumbnail_path');

console.log(result);
```

***

### **addComment(commentData)** (Eagle 4.0 build22+)

向此条目添加一个标注。支持图片框选标注和视频时间轴注解。

* `commentData` Object - 标注数据
  * `annotation` string (可选) - 标注文字
  * `x` number (可选) - 框选 X 位置（图片标注）
  * `y` number (可选) - 框选 Y 位置（图片标注）
  * `width` number (可选) - 框选宽度，必须 > 0（图片标注）
  * `height` number (可选) - 框选高度，必须 > 0（图片标注）
  * `duration` number (可选) - 视频时间戳（秒），必须 >= 0（视频标注）
* 返回 `Promise<comment: Object>` - 新创建的标注对象

```javascript
let item = await eagle.item.getById('item_id');

// 添加图片框选标注
let comment = await item.addComment({
    x: 350, y: 480, width: 380, height: 400,
    annotation: "脸部区域"
});

// 添加视频时间轴注解
let comment = await item.addComment({
    duration: 65.5,
    annotation: "重要场景"
});
```

{% hint style="info" %}
备注：此方法需要 Eagle 4.0 build22 或更高版本。
{% endhint %}

***

### **updateComment(commentId, updateData)** (Eagle 4.0 build22+)

更新现有的标注。仅更新提供的字段。

* `commentId` string - 要更新的标注 ID
* `updateData` Object - 要更新的字段
  * `annotation` string (可选) - 新的标注文字
  * `x` number (可选) - 新的 X 位置（仅图片标注）
  * `y` number (可选) - 新的 Y 位置（仅图片标注）
  * `width` number (可选) - 新的宽度（仅图片标注）
  * `height` number (可选) - 新的高度（仅图片标注）
  * `duration` number (可选) - 新的时间戳（仅视频标注）
* 返回 `Promise<comment: Object>` - 更新后的标注对象

```javascript
let item = await eagle.item.getById('item_id');
let updated = await item.updateComment('comment_id', {
    annotation: "更新后的文字"
});
```

{% hint style="info" %}
备注：此方法需要 Eagle 4.0 build22 或更高版本。
{% endhint %}

***

### **removeComment(commentId)** (Eagle 4.0 build22+)

从此条目中移除一个标注。

* `commentId` string - 要移除的标注 ID
* 返回 `Promise<result: boolean>` - 是否移除成功

```javascript
let item = await eagle.item.getById('item_id');
await item.removeComment('comment_id');
```

{% hint style="info" %}
备注：此方法需要 Eagle 4.0 build22 或更高版本。
{% endhint %}

***

### **open(options)**

在全部列表显示此文件

* `options` Object (可选) - 开启选项
  * `window` boolean (可选) - 是否在新窗口中开启文件，默认为 `false`
* 返回 `Promise<void>`

{% hint style="info" %}
提示：你也可以直接呼叫 `eagle.item.open(itemId, options)`方法打开文件夹。
{% endhint %}

```javascript
let item = await eagle.item.getById('item_id');
// 在当前窗口开启
await item.open();

// 在新窗口开启
await item.open({ window: true });

// 等价于
await eagle.item.open('item_id');
await eagle.item.open('item_id', { window: true });
```

{% hint style="info" %}
提示：`window` 参数需要 Eagle 4.0 build12 以上版本支持。
{% endhint %}

***

### **select()**

选中此文件

* 返回 `Promise<result: boolean>` - `result` 是否选中成功

```javascript
let item = await eagle.item.getById('item_id');
await item.select();

// 等价于
await eagle.item.select([item.id]);
```

{% hint style="info" %}
提示：调用实例方法 `select()` 会清空当前选中并仅选中此文件。如需批量选中多个文件，请使用静态方法 `eagle.item.select(itemIds)`。
{% endhint %}

{% hint style="info" %}
提示：`select()` 方法需要 Eagle 4.0 build12 以上版本支持。
{% endhint %}

***

#### 实例属性 <a href="#woenk" id="woenk"></a>

### **`id` string**

只读，文件 ID。

### **`name` string**

文件名。

### **`ext` string**

只读，文件扩展名。

### **`width` Interger**

图像宽度。

### **`height` Interger**

图像高度。

### **`url` string**

来源链接。

### **`isDeleted` boolean**

只读，文件是否在垃圾桶。

### **`annotation` string**

文件注释。

### **`tags` string\[]**

文件标签。

### **`folders` string\[]**

所属文件夹 ids。

### **`palettes` Object\[]**

只读，色票信息。

### **`comments` Object\[]** (Eagle 4.0 build22+)

只读，条目标注。使用 `addComment()`、`updateComment()`、`removeComment()` 方法来修改。

```javascript
let item = await eagle.item.getById('item_id');

// 读取标注
console.log(item.comments);
// [{ id: "abc", x: 324, y: 810, width: 194, height: 208, annotation: "脸部", lastModified: 1774282485915 }]
```

{% hint style="info" %}
备注：此属性需要 Eagle 4.0 build22 或更高版本。
{% endhint %}

### **`size` Interger**

只读，文件大小。

### **`star` Interger**

评分信息，`0 ~ 5`。

### **`importedAt` Interger**

导入时间（时间戳）。可读写，修改后需调用 `save()` 保存。

```javascript
// 读取导入时间
let date = new Date(item.importedAt);

// 修改导入时间（需要 Eagle 4.0 build18+）
item.importedAt = Date.now();
item.importedAt = new Date('2024-01-01').getTime();
await item.save();
```

{% hint style="info" %}
备注：设置值必须为正整数时间戳，无效值将被忽略。此功能需要 Eagle 4.0 build18 或更高版本。
{% endhint %}

### **`modifiedAt` Interger**

只读，最后修改时间。

```javascript
let modifiedDate = new Date(item.modifiedAt);
console.log(`文件最后修改于: ${modifiedDate.toLocaleString()}`);
```

### **`noThumbnail` boolean**

只读，文件是否有缩略图，无缩略图文件将以原始文件进行预览。

### **`noPreview` boolean**

只读，文件是否支持双击预览。

### **`filePath` string**

只读，返回文件所在路径。

### **`fileURL` string**

只读，返回文件所在路径之链接（`file:///`）。

### **`thumbnailPath` string**

只读，返回缩略图路径。

### **`thumbnailURL` string**

只读，返回缩略图链接（`file:///`），如需在 HTML 显示该文件，可以使用这个属性。

### **`metadataFilePath`string**

只读，该文件 `metadata.json` 所在位置。

{% hint style="success" %}
**🦄 最佳实践：** 为了确保数据安全性，请使用 `item` API 提供的 `save()` 方法进行数据的存取与修改，应避免直接 `metadata.json`。
{% endhint %}

### &#x20;<a href="#nptwx" id="nptwx"></a>
