# tag（标签）

```javascript
// 取得所有标签
const tags = await eagle.tag.get();

// 按名称筛选标签
const designTags = await eagle.tag.get({ name: "design" });

// 取得最近使用标签
const recents = await eagle.tag.getRecentTags();

// 取得常用标签
const starred = await eagle.tag.getStarredTags();
```

## 方法 <a href="#z1a5y" id="z1a5y"></a>

## get(options) <a href="#x9nu2" id="x9nu2"></a>

获取标签，可通过选项进行筛选。

* `options` Object (可选) - 查询条件
  * `name` string (可选) - 按标签名称进行模糊搜索，不区分大小写
* 返回 `Promise<tags: Object[]>` - tags 查询结果。

```javascript
// 获取所有标签
const tags = await eagle.tag.get();

// 按名称筛选标签
const filteredTags = await eagle.tag.get({
    name: "design"
});
```

{% hint style="info" %}
提示：`name` 参数需要 Eagle 4.0 build12 以上版本支持。
{% endhint %}

***

## getRecentTags() <a href="#dwsxw" id="dwsxw"></a>

获取最近使用的的标签。

* 返回 `Promise<tags: Object[]>` - tags 查询结果。

```javascript
const recents = await eagle.tag.getRecentTags();
```

***

## getStarredTags() <a href="#starred" id="starred"></a>

获取常用标签（用户收藏的标签）。

* 返回 `Promise<tags: Object[]>` - tags 查询结果。

```javascript
const starred = await eagle.tag.getStarredTags();
```

{% hint style="info" %}
提示：`getStarredTags()` 方法需要 Eagle 4.0 build18 以上版本支持。
{% endhint %}

***

## merge(options) <a href="#merge" id="merge"></a>

合并标签：将来源标签重新命名为目标标签，所有使用来源标签的素材都会自动更新。

* `options` Object - 选项参数
  * `source` string - 来源标签名称（将被移除）
  * `target` string - 目标标签名称（合并后保留）
* 返回 `Promise<Object>` - 合并结果
  * `affectedItems` number - 受影响的素材数量
  * `sourceRemoved` boolean - 来源标签是否已移除

```javascript
// 将所有 "UI Design" 标签合并为 "UI"
const result = await eagle.tag.merge({
    source: 'UI Design',
    target: 'UI'
});

console.log(`已合并 ${result.affectedItems} 个素材的标签`);
```

{% hint style="info" %}
提示：`merge()` 方法需要 Eagle 4.0 build18 以上版本支持。
{% endhint %}

{% hint style="warning" %}
注意：合并操作会更新所有使用来源标签的素材、标签群组、收藏标签和历史标签。此操作不可逆。
{% endhint %}

***

## 类：Tag <a href="#tag-class" id="tag-class"></a>

由 Eagle API `get` 返回的 Object 类型，提供修改、保存功能。

{% hint style="success" %}
**🦄 最佳实践：** 为了确保数据安全性，请使用 Tag 实例提供的 `save()` 方法进行数据的修改，应避免直接修改 Eagle 资源库底下的标签数据。
{% endhint %}

***

### 实例方法 <a href="#instance-methods" id="instance-methods"></a>

#### **save()**

保存标签的修改。目前仅支持修改标签名称。

* 返回 `Promise<result: boolean>` - `result` 是否修改成功

```javascript
// 获取所有标签
const tags = await eagle.tag.get();

// 找到要修改的标签
const tag = tags.find(t => t.name === 'old-name');

// 修改标签名称
tag.name = 'new-name';

// 保存修改
await tag.save();
```

{% hint style="info" %}
提示：`save()` 方法需要 Eagle 4.0 build12 以上版本支持。
{% endhint %}

{% hint style="warning" %}
注意：修改标签名称后，所有使用该标签的文件都会自动更新为新的标签名称。
{% endhint %}

***

### 实例属性 <a href="#instance-properties" id="instance-properties"></a>

#### **`name` string**

标签名称。可修改此属性并通过 `save()` 方法保存。

#### **`count` number**

只读，使用此标签的文件数量。

#### **`color` string**

标签颜色。

#### **`groups` string\[]**

只读，标签所属的分组。

#### **`pinyin` string**

只读，标签名称的拼音（用于搜索和排序）。
