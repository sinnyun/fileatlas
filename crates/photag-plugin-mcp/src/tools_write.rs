use super::*;

impl PhoTagMcpServer {
    fn observe_scan<T: Send + 'static>(&self, progress: std::sync::mpsc::Receiver<Option<T>>) {
        let context = self.context.clone();
        let runtime = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            while let Ok(update) = progress.recv() {
                if update.is_none() {
                    break;
                }
            }
            // Like the Sidebar scan observer, notify every consumer after the
            // production indexing stream completes. Do not clear UI filters.
            if let Err(error) = runtime.block_on(
                context.emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ()),
            ) {
                tracing::error!("MCP scan completion dispatch failed: {error}");
            }
        });
    }
}

#[tool_router(router = write_tool_router, vis = "pub(super)")]
impl super::PhoTagMcpServer {
    #[tool(
        description = "Rename a folder favorite alias (action=rename,name) or reorder it (action=move,delta). Requires mutations enabled. Does not rename local files."
    )]
    fn manage_favorite(&self, Parameters(request): Parameters<FavoriteRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        let result = match request.action.as_str() {
            "rename" => self
                .navigation
                .rename_favorite(request.folder_id, request.name.as_deref().unwrap_or("")),
            "move" => self
                .navigation
                .move_favorite(request.folder_id, request.delta.unwrap_or(0)),
            _ => return error_json("unsupported favorite action"),
        };
        match result {
            Ok(()) => {
                let _ = self
                    .navigation
                    .request_ui(photag_plugin_navigation::NavigationUiRequest::CloseModal);
                json!({"updated":request.folder_id}).to_string()
            }
            Err(error) => error_json(&error.to_string()),
        }
    }
    #[tool(
        description = "Inspect missing tagged-file records (action=list_missing); explicitly relink a record (action=relink, record_id,path), or delete missing records (action=delete_missing,record_ids,confirm=true). Mutations require PHOTAG_MCP_ALLOW_MUTATIONS=1. No local file is deleted."
    )]
    async fn manage_tag_records(
        &self,
        Parameters(request): Parameters<TagRecordRequest>,
    ) -> String {
        if request.action != "list_missing" {
            if let Err(error) = self.mutation_guard() {
                return error_json(&error);
            }
        }
        let mutating = request.action != "list_missing";
        let tagging = self.tagging.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let result = match request.action.as_str() {
                "list_missing" => tagging
                    .missing_records()
                    .map(|items| json!({"records":items})),
                "relink" => match (request.record_id, request.path) {
                    (Some(id), Some(path)) => tagging
                        .relink_record(id, &path)
                        .map(|_| json!({"relinked":id})),
                    _ => Err("record_id and path are required".into()),
                },
                "delete_missing" if request.confirm == Some(true) => tagging
                    .delete_missing_records(&request.record_ids.unwrap_or_default())
                    .map(|count| json!({"deleted":count})),
                _ => Err("Invalid action or missing confirmation".into()),
            };
            match result {
                Ok(value) => value.to_string(),
                Err(error) => error_json(&error),
            }
        })
        .await
        .unwrap_or_else(|error| error_json(&error.to_string()));
        if mutating
            && serde_json::from_str::<serde_json::Value>(&outcome)
                .is_ok_and(|v| v.get("error").is_none())
        {
            let _ = self
                .context
                .emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
                .await;
        }
        outcome
    }

    #[tool(
        description = "将 PhoTag 当前完整元数据备份为源格式 JSON。需要启用修改权限，并要求 confirm=true。"
    )]
    async fn export_metadata(&self, Parameters(request): Parameters<ExportRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("导出元数据需要 confirm=true。 ");
        }
        let path = std::path::PathBuf::from(request.path);
        if let Err(error) = self.metadata_export.set_destination(path.clone()) {
            return error_json(&error.to_string());
        }
        if let Err(error) = self.metadata_export.request_export() {
            return error_json(&error.to_string());
        }
        loop {
            match self.metadata_export.status() {
                Ok(photag_plugin_metadata_export::MetadataExportStatus::Succeeded {
                    path,
                    records,
                }) => {
                    return json!({"ok": true, "path": path, "records": records}).to_string();
                }
                Ok(photag_plugin_metadata_export::MetadataExportStatus::Failed(error)) => {
                    return error_json(&error);
                }
                Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
                Err(error) => return error_json(&error.to_string()),
            }
        }
    }

    #[tool(description = "为最多 100 个媒体设置 0 到 5 星评分。修改工具默认关闭。")]
    fn set_rating(&self, Parameters(request): Parameters<RatingRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !(1..=100).contains(&request.media_ids.len()) || request.rating > 5 {
            return error_json("media_ids 数量必须为 1 到 100，rating 必须为 0 到 5。");
        }
        match self
            .gallery
            .apply_rating(&request.media_ids, request.rating)
        {
            Ok(()) => "{\"ok\":true}".into(),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "为最多 100 个媒体分配最多 50 个标签。此操作也会按 PhoTag 语义安排 XMP/IPTC 写回。修改工具默认关闭。"
    )]
    fn assign_tags(&self, Parameters(request): Parameters<TagMutationRequest>) -> String {
        self.mutate_tags(request, true)
    }

    #[tool(
        description = "从最多 100 个媒体移除最多 50 个标签。此操作也会按 PhoTag 语义安排 XMP/IPTC 写回。修改工具默认关闭。"
    )]
    fn remove_tags(&self, Parameters(request): Parameters<TagMutationRequest>) -> String {
        self.mutate_tags(request, false)
    }

    #[tool(
        description = "重命名一个已索引媒体文件。需要启用修改权限；执行前请由 MCP 客户端向用户展示文件名变更。"
    )]
    async fn rename_media(&self, Parameters(request): Parameters<RenameRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("重命名需要 confirm=true。 ");
        }
        if request.new_filename.trim().is_empty() || request.new_filename.contains(['/', '\\']) {
            return error_json("new_filename 必须是非空文件名，不能包含路径分隔符。");
        }
        match self
            .file_operations
            .rename(request.media_id, request.new_filename)
            .await
        {
            Ok(()) => "{\"ok\":true}".into(),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "将最多 100 个媒体移动到指定文件夹。需要启用修改权限；这是不可轻易撤销的文件操作。"
    )]
    async fn move_media(&self, Parameters(request): Parameters<MoveRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("移动文件需要 confirm=true。 ");
        }
        if let Err(error) = validate_batch(&request.media_ids) {
            return error_json(&error);
        }
        let outcomes = self
            .file_operations
            .bulk_move_into_folder(request.media_ids, request.destination_folder.into())
            .await;
        match outcomes {
            Ok(outcomes) => {
                serde_json::to_string(&json!({ "items": outcomes.iter().map(|item| json!({
                "media_id": item.media_id, "moved": item.moved, "error": item.error
            })).collect::<Vec<_>>() }))
                .unwrap_or_else(|error| error_json(&error.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "将最多 100 个媒体复制到指定文件夹。需要启用修改权限。")]
    async fn copy_media(&self, Parameters(request): Parameters<CopyRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("复制文件需要 confirm=true。 ");
        }
        if let Err(error) = validate_batch(&request.media_ids) {
            return error_json(&error);
        }
        let outcomes = self
            .file_operations
            .bulk_copy_to_folder(request.media_ids, request.destination_folder.into())
            .await;
        match outcomes {
            Ok(outcomes) => {
                serde_json::to_string(&json!({ "items": outcomes.iter().map(|item| json!({
                "media_id": item.media_id, "copied": item.copied, "error": item.error
            })).collect::<Vec<_>>() }))
                .unwrap_or_else(|error| error_json(&error.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "将最多 100 个媒体移入 Windows 回收站。需要启用修改权限；执行前请由 MCP 客户端向用户展示待删除清单。"
    )]
    async fn delete_media(&self, Parameters(request): Parameters<DeleteRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("移入回收站需要 confirm=true。 ");
        }
        if let Err(error) = validate_batch(&request.media_ids) {
            return error_json(&error);
        }
        match self
            .file_operations
            .bulk_delete_to_recycle_bin(request.media_ids)
            .await
        {
            Ok(outcomes) => {
                serde_json::to_string(&json!({ "items": outcomes.iter().map(|item| json!({
                "media_id": item.media_id,
                "removed_from_database": item.removed_from_database,
                "recycled": item.recycled,
                "error": item.error
            })).collect::<Vec<_>>() }))
                .unwrap_or_else(|error| error_json(&error.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "注册现有文件夹并开始 PhoTag 递归索引。需要启用修改权限。")]
    fn add_folder(&self, Parameters(request): Parameters<AddFolderRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("注册文件夹并开始索引需要 confirm=true。 ");
        }
        match self.navigation.add_folder(&request.path) {
            Ok((folder, progress)) => {
                self.observe_scan(progress);
                serde_json::to_string(&json!({ "ok": true, "folder": folder }))
                    .unwrap_or_else(|error| error_json(&error.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "通过本机已安装的 QuickLook 查看原始文件。需要启用修改权限；不安装软件。")]
    async fn open_media(&self, Parameters(request): Parameters<MediaIdRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        let media = match self.gallery.media_by_id(request.media_id) {
            Ok(Some(media)) => media,
            Ok(None) => return error_json("没有找到该文件记录。"),
            Err(error) => return error_json(&error.to_string()),
        };
        let quicklook = Arc::clone(&self.quicklook);
        match tokio::task::spawn_blocking(move || quicklook.open(media.full_path)).await {
            Ok(Ok(())) => {
                json!({"ok":true,"viewer":"quicklook","media_id":request.media_id}).to_string()
            }
            Ok(Err(error)) => error_json(&error.to_string()),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "读取 QuickLook 最近调用结果；不代表外部窗口已完成显示。")]
    fn get_view_state(&self) -> String {
        json!({"quicklook": self.quicklook.snapshot()}).to_string()
    }

    #[tool(
        description = "只读检查本机 QuickLook.exe 的可见窗口标题和尺寸，区别进程派发与实际显示。"
    )]
    fn inspect_quicklook(&self) -> String {
        #[cfg(windows)]
        {
            json!({"dispatch":self.quicklook.snapshot(),"windows":photag_plugin_quicklook::windows::visible_windows(),"higher_integrity_process":photag_plugin_quicklook::windows::higher_integrity_quicklook_process(),"pipe_probe":photag_plugin_quicklook::windows::probe()}).to_string()
        }
        #[cfg(not(windows))]
        {
            error_json("QuickLook requires Windows")
        }
    }

    #[tool(description = "截取已验证属于 QuickLook.exe 的可见查看窗口，用于外部查看器集成验收。")]
    async fn screenshot_quicklook(&self) -> Result<CallToolResult, McpError> {
        #[cfg(windows)]
        {
            let Some(window) = photag_plugin_quicklook::windows::visible_windows()
                .into_iter()
                .next()
            else {
                return Ok(CallToolResult::error(vec![ContentBlock::text(
                    "No visible QuickLook window",
                )]));
            };
            match screenshot::capture_png(window.handle as isize) {
                Ok((png, _, _)) => Ok(CallToolResult::success(vec![
                    ContentBlock::text(window.title),
                    ContentBlock::image(
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png),
                        "image/png",
                    ),
                ])),
                Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error)])),
            }
        }
        #[cfg(not(windows))]
        {
            Ok(CallToolResult::error(vec![ContentBlock::text(
                "QuickLook requires Windows",
            )]))
        }
    }

    #[tool(description = "创建标签，可选指定父标签。修改工具默认关闭。")]
    fn create_tag(&self, Parameters(request): Parameters<CreateTagRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if request.name.trim().is_empty() || request.name.len() > 256 {
            return error_json("标签名不能为空，且最多 256 字节。");
        }
        match self.navigation.create_tag(&request.name, request.parent_id) {
            Ok(tag) => {
                serde_json::to_string(&tag).unwrap_or_else(|error| error_json(&error.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "重命名标签。修改工具默认关闭。")]
    fn rename_tag(&self, Parameters(request): Parameters<RenameTagRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if request.name.trim().is_empty() || request.name.len() > 256 {
            return error_json("标签名不能为空，且最多 256 字节。");
        }
        match self.navigation.rename_tag(request.tag_id, &request.name) {
            Ok(()) => "{\"ok\":true}".into(),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "删除标签及其关联。调用方必须将 confirm 设为 true；修改工具默认关闭。")]
    fn delete_tag(&self, Parameters(request): Parameters<DeleteTagRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("删除标签需要 confirm=true。 ");
        }
        match self.navigation.delete_tag(request.tag_id, false) {
            Ok(_affected_media_ids) => "{\"ok\":true}".into(),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "从 PhoTag 库移除已注册文件夹及其索引记录，不会删除文件夹中的媒体文件。调用方必须将 confirm 设为 true；修改工具默认关闭。"
    )]
    fn remove_folder(&self, Parameters(request): Parameters<FolderRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("移除文件夹需要 confirm=true。 ");
        }
        match self.navigation.remove_folder(request.folder_id) {
            Ok(folder) => serde_json::to_string(&json!({ "ok": true, "folder": folder }))
                .unwrap_or_else(|error| error_json(&error.to_string())),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "刷新收藏目录并打开其浏览视图。浏览模式不清理标签，不递归建立媒体索引。需要 confirm=true 及修改权限。"
    )]
    fn rescan_folder(&self, Parameters(request): Parameters<FolderRequest>) -> String {
        if let Err(error) = self.mutation_guard() {
            return error_json(&error);
        }
        if !request.confirm {
            return error_json("重新扫描需要 confirm=true。 ");
        }
        match self.navigation.rescan(request.folder_id) {
            Ok(progress) => {
                self.observe_scan(progress);
                "{\"ok\":true,\"started\":true}".into()
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "取消已注册文件夹的当前扫描，不改变现有媒体文件。")]
    fn cancel_scan(&self, Parameters(request): Parameters<FolderIdRequest>) -> String {
        match self.navigation.cancel_scan(request.folder_id) {
            Ok(()) => "{\"ok\":true}".into(),
            Err(error) => error_json(&error.to_string()),
        }
    }
}
