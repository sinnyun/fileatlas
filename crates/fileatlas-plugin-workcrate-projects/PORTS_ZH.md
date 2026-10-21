# WorkCrate Projects 对接

Cordis 插件/服务，以及项目 CRUD、引用登记表、预览和字段预设，都保留其现有的公开 API。项目记录是位于特性根目录 projects.json 的带版本 JSON；每个项目有 projects/<uuid>/project.json 以及自己拥有的预览/原生文件。原子写入保留 json.bak 恢复能力。这个规模不大的元数据登记表有意使用带版本的 JSON，而不是再复制一个数据库服务。

新增的 `document_path(project_id) -> Result<Option<PathBuf>,WorkCrateProjectsError>` 和 `set_document_path(project_id,&Path) -> Result<(),...>` 用于关联一个规范的原生文档路径。关联本身不存储画板设计副本。原生扩展名/打开校验由 Boards 拥有。`duplicate_project_document(id,name,directory)` 会把原生文件复制到显式选定的目录而不覆盖已有文件，携带 FileAtlas 拥有的项目元数据/预览，并轮换新项目的关联绑定。外部源文件保持不变。新的原生项目创建流程应当请求一个文件夹并调用 Boards 的 `create_project_document`；较早的 `duplicate_project` 为现有调用方保留。删除操作会把特性拥有的数据保留在回收站中，并且从不删除外部的原始文件。

引用仍然是原始路径，自定义预览仍然是特性自己拥有的；预设沿用现有的校验/存储方式。服务版本号与共享的刷新 Wake 用于广播变更。不新增事件或基础设施；错误会暴露 IO/JSON/校验/依赖方面的失败。没有实现遗留的 board-JSON 迁移，因为用户已授权直接替换。
