# folder（文件夾）

```javascript
// 取得 Eagle 应用当前被选中的文件夹
let folder = (await eagle.folder.getSelected())[0];

// 修改属性
folder.name = 'New Folder Name';
folder.description = 'New description...';

// 保存修改
await folder.save();
```

{% hint style="success" %}
**🦄 最佳实践：** 为了确保数据安全性，请使用 API 提供的 `save()` 方法进行数据的存取与修改，应避免直接修改 Eagle 资源库底下的 `metadata.json` 或任意文件。
{% endhint %}

## 方法 <a href="#z1a5y" id="z1a5y"></a>

## create(options) <a href="#xnzds" id="xnzds"></a>

建立文件夾

* `options` Object
  * `name` string - 文件夾名
  * `description` string (可选) - 文件夾描述
  * `parent` string (可选) - 父文件夹 ID，带此参数等同 `createSubfolder(parentId, options)`
* 返回 `Promise<folder: Folder>` - `folder` 成功创建的文件夹

```javascript
let newFoler = await eagle.folder.create({
    name: 'New Folder',
    description: 'Folder\'s description.',
});
```

***

## createSubfolder(parentId, options) <a href="#rys4i" id="rys4i"></a>

建立子文件夾

* `parentId` string - 父文件夹 ID
* `options` Object
  * `name` string - 文件夾名
  * `description` string (可选) - 文件夾描述
* 返回 `Promise<folder: Folder>` - `folder` 成功创建的文件夹

```javascript
let parentFolder = await eagle.folder.getById('folder_id');
let subFolder = await eagle.folder.createSubfolder(parentFolder.id, {
    name: 'Subfolder',
    description: 'Subfolder description.',
});
```

***

## get(options) <a href="#x9nu2" id="x9nu2"></a>

获取指定条件的文件夹。

* `options` Object - 查询条件
  * `id` string (可选) - 文件夾 id
  * `ids` string\[] (可选) - 文件夾 id 数组
  * `isSelected` boolean (可选) - 正在被选中的文件夹
  * `isRecent` boolean (可选) - 近期存取的文件夹
* 返回 `Promise<folders: Folder[]>` - `folders` 查询结果

```javascript
// 取得指定 id 对应的文件夹
let folders = await eagle.folder.get({
    ids: ['folder_id1', 'folder_id2']
});

// 取得应用当前被选中的文件夹
let folders = await eagle.folder.get({
    isSelected: true
});
```

***

## getAll() <a href="#fbdzh" id="fbdzh"></a>

获取所有文件夹。

* 返回 `Promise<folders: Folder[]>` - `folders` 查询结果

```javascript
let folders = await eagle.folder.getAll();
```

***

## getById(folderId) <a href="#sy5fz" id="sy5fz"></a>

获取对应 `folderId` 的文件夹。

* `folderId` string - 文件夾 id
* 返回 `Promise<folder: Folder>` - `folder` 查询结果

```javascript
let folder = await eagle.folder.getById('folder_id');
```

***

## getByIds(folderIds) <a href="#n0gjq" id="n0gjq"></a>

获取对应 `folderIds` 的文件夹数组。

* `folderIds` string\[] - 文件夾 id 数组
* 返回 `Promise<folders: Folder[]>` - `folders` 查询结果

```javascript
let folders = await eagle.folder.getByIds(['folder_id1', 'folder_id2']);
```

***

## getSelected() <a href="#dsbgj" id="dsbgj"></a>

获取当前应用选中的文件夹

* 返回 `Promise<folders: Folder[]>` - `folders`

```javascript
let folders = await eagle.folder.getSelected();
```

***

## getRecents() <a href="#dwsxw" id="dwsxw"></a>

获取最近使用的的文件夹

* 返回 `Promise<folders: Folder[]>` - `folders`

```javascript
let folders = await eagle.folder.getRecents();
```

***

## open(folderId) <a href="#gjdst" id="gjdst"></a>

Eagle 将打开对应 `folderId`文件夹。

* 返回 `Promise<void>`

```javascript
await eagle.folder.open('folder_id');

// 等价于
let folder = await eagle.folder.getById('folder_id');
await folder.open();
```

{% hint style="info" %}
提示：你也可以直接呼叫 folder 实例的 `open()` 方法打开文件夹。
{% endhint %}

***

## 類：Folder <a href="#uezi0" id="uezi0"></a>

由 Folder API `get`返回的 Object 类型，提供修改、保存功能。

```javascript
let folder = await eagle.folder.getById('folder_id');

console.log(folder.id);
console.log(folder.name);

folder.name = 'new name';
console.log(folder.name);

await folder.save();
```

{% hint style="success" %}
**🦄 最佳实践：** 为了确保数据安全性，请使用 Folder 实例提供的 `save()` 方法进行数据的存取与修改，应避免直接修改 Eagle 资源库底下的 `metadata.json` 或任意文件。
{% endhint %}

***

#### 实例方法 <a href="#sihmc" id="sihmc"></a>

### **save()**

保存所有修改

* 返回 `Promise<void>`

```javascript
let folder = await eagle.folder.getById('folder_id');
folder.name = 'New Fodler Name';

// 保存修改
await folder.save();
```

***

### **open()**

Eagle 将打开此文件夹。

* 返回 `Promise<void>`

```javascript
let folder = await eagle.folder.getById('folder_id');
await folder.open();

// 等价于
await eagle.folder.open('folder_id');
```

{% hint style="info" %}
提示：你也可以直接呼叫 `eagle.folder.open(folderId)`方法打开文件夹。
{% endhint %}

***

#### 实例属性 <a href="#woenk" id="woenk"></a>

`Folder` 实例包含以下属性：

### **`id` string**

只读，文件夹 id。

### **`name` string**

文件夹名称。

### **`description` string**

文件夹描述、介绍。

### **`icon` string**

只读，文件夹图标。

### **`iconColor` string**

文件夹图标颜色。

```javascript
let folder = await eagle.folder.getById('folder_id');

// 设置文件夹颜色为红色
folder.iconColor = eagle.folder.IconColor.Red;

// 或直接使用字符串值
folder.iconColor = 'red';

// 保存修改
await folder.save();
```

{% hint style="info" %}
提示：在 Eagle 4.0 build12 版本之前，此属性为只读状态，不支持修改。从 Eagle 4.0 build12 版本开始，支持修改此属性。
{% endhint %}

### **`createdAt` Interger**

只读，文件夹创建时间(timestamp)。

```javascript
let date = new Date(folder.createdAt);
```

### **`parent` string**

父文件夾 ID。

```javascript
let folder = await eagle.folder.getById('folder_id');

// 获取父文件夹 ID
console.log(folder.parent);

// 更改父文件夹（将文件夹移动到另一个父文件夹下）
folder.parent = 'parent_folder_id';
await folder.save();

// 移动到根目录（设为 null 或 undefined）
folder.parent = null;
await folder.save();
```

{% hint style="info" %}
提示：在 Eagle 4.0 build12 版本之前，此属性为只读状态，不支持修改。从 Eagle 4.0 build12 版本开始，支持修改此属性，可以通过更改此属性来移动文件夹到不同的父文件夹下。
{% endhint %}

### **`children` Folder\[]**

只读，子文件夹数组。

```javascript
let children = folder.children;

console.log(children[0]);
await children[0].open();
```

***

## 靜態屬性 <a href="#static-properties" id="static-properties"></a>

### **`IconColor` Object**

提供预定义的文件夹图标颜色常量，用于设置文件夹的 `iconColor` 属性。

```javascript
// 可用的颜色常量
eagle.folder.IconColor.Red      // 'red'
eagle.folder.IconColor.Orange   // 'orange' 
eagle.folder.IconColor.Yellow   // 'yellow'
eagle.folder.IconColor.Green    // 'green'
eagle.folder.IconColor.Aqua     // 'aqua'
eagle.folder.IconColor.Blue     // 'blue'
eagle.folder.IconColor.Purple   // 'purple'
eagle.folder.IconColor.Pink     // 'pink'
```

**使用示例：**

```javascript
let folder = await eagle.folder.getById('folder_id');

// 使用颜色常量设置文件夹颜色
folder.iconColor = eagle.folder.IconColor.Blue;
await folder.save();

// 批量设置多个文件夹颜色
let folders = await eagle.folder.getAll();
for (let i = 0; i < folders.length; i++) {
    if (i % 2 === 0) {
        folders[i].iconColor = eagle.folder.IconColor.Green;
    } else {
        folders[i].iconColor = eagle.folder.IconColor.Purple;
    }
    await folders[i].save();
}
```

{% hint style="success" %}
**🦄 最佳实践：** 建议使用 `eagle.folder.IconColor` 常量而非直接使用字符串值，这样可以获得更好的代码提示和类型安全。
{% endhint %}

### &#x20;<a href="#nptwx" id="nptwx"></a>
