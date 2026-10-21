# WorkCrate Export 对接

Cordis 插件 `fileatlas.workcrate-export` 发布 `fileatlas/workcrate-export`，需要 Boards 服务。公开构造函数 `new(Arc<WorkCrateBoardsService>)`、`progress() -> Option<String>`、export_pages_pdf、export_pages_images、export_page_image、export_page_pdf 保留既有的 shell/服务集成方式。

所有文件生成都通过 Boards 使用官方 VectorCraft CLI 的无头 MCP 导出。PDF 选择使用当前的原生索引；所有被选中的页面必须属于同一个已关联的原生文档。图片支持 300dpi 的 PNG/JPG。ExportedPage 保留既有的 path、dpi、width_px、height_px 字段。原生输出会先生成一个全新的临时文件，检查文件是否完整，然后原子地交付到请求的路径。计数结果表示已完成的输出；成功时原生告警会保留在 progress 中。PDF 的失败进度会被持久化；图片失败返回 ExportError。

不使用 PdfCraft、图片编码器、自研渲染器或自研位图 PDF 生成。除了底层客户端的生命周期之外不拥有或清理任何进程。原生错误与进度信息用于诊断；没有事件生产者/消费者。集成运行时仍需通过那个被忽略的 official_native_round_trip 测试单独验证。
