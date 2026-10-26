# Eagle 文件链接转换到 FileAtlas 画布

Eagle 官方 Plugin API 的 Item 对象提供只读 `fileURL`（`file:///`）与
`filePath`。应用内的 `eagle.item.getById()` 可查询条目；`eagle.item.open(id)`
用于在 Eagle 中打开条目。FileAtlas 画布通过共享 `ResourceProvider` 将本地文件
规范化为 `file://` URI，并在资源清单和画布 Scene 中复用同一链接。

新增 `fileatlas-plugin-eagle-links` Cordis 插件，服务
`fileatlas/eagle-links` 的 `EagleLinkService::convert()` 接收官方 Item 的
`fileURL` 或 `filePath`，交给共享本地文件 Provider 校验、规范化并生成画布
`ResourceLink`。源文件仍由 Eagle 资源库持有，FileAtlas 不复制、不修改媒体或
Eagle 元数据。缺失或非本地文件会返回错误。

升级后的 Eagle 实测支持 Web API V2。转换服务现在可用 V2 `item/get?id=...`
读取条目 ID、扩展名、文件大小及 `noThumbnail`，并用 V1 `item/thumbnail` 取得
缩略图位置。缩略图和原图都在对应 `.info` 目录时，服务在同目录查找扩展名和
字节大小均匹配的唯一原图，并返回画布统一 `ResourceLink`；Eagle 标记
`noThumbnail` 时，接口返回的文件自身就是可用图像，直接引用该文件。Eagle 未运行、
条目在回收站、文件缺失或匹配不唯一时会返回错误，不猜测候选文件。

本机只读验证确认 V2 条目查询可用，V1 缩略图路径可经 URL 解码访问，并抽查到
同一 `.info` 目录内的原图与缩略图。扩展名和字节大小筛选已纳入实现。Eagle 官方
Web API 文档没有把该 `.info` 同目录原图匹配规则定义为稳定接口，因此遇到不符合
规则的项目会安全失败；更长期兼容的原图路径方案仍是 Eagle Plugin API 公开的
`fileURL`。

官方依据：

- [Eagle Plugin API：Item](https://developer.eagle.cool/plugin-api/api/item)
- [Eagle Plugin API：Retrieve Data](https://developer.eagle.cool/plugin-api/tutorial/get-eagle-data)
- [Eagle Web API：Overview](https://api.eagle.cool/)

画布原生图片拖放会调用该服务；确认来源后，把原始文件的统一链接放入画布
Scene，并将 Eagle 条目 ID、名称、文件夹、原图路径/URI、缩略图路径及 API 地址
保存到当前画板的属性数据中。Eagle 服务插件/发送按钮不是此方案的前置条件。
详情见 [Eagle 链接插件 PORTS.md](../../crates/fileatlas-plugin-eagle-links/PORTS.md)
和[画布 PORTS.md](../../crates/fileatlas-plugin-whiteboard/PORTS.md)。
