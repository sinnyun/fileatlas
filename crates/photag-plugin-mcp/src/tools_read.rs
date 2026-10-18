use super::*;

#[tool_router(router = read_tool_router, vis = "pub(super)")]
impl super::PhoTagMcpServer {
    #[tool(description = "读取程序启动台的分类、程序、隐藏状态和扫描状态。")]
    fn get_launcher_state(&self) -> String {
        let Some(launcher) = &self.launcher else {
            return error_json("程序启动台插件未启用。");
        };
        let snapshot = launcher.snapshot();
        serde_json::to_string(&json!({
            "ui": self.surfaces.value("launcher.ui").and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok()),
            "scanning": snapshot.scanning,
            "revision": snapshot.revision,
            "categories": snapshot.categories.iter().map(|item| json!({"id":item.id,"name":item.name,"position":item.position})).collect::<Vec<_>>(),
            "entries": snapshot.entries.iter().map(|item| json!({"id":item.id,"display_name":item.display_name,"launch_path":item.launch_path,"category_id":item.category_id,"hidden":item.hidden,"pin_scope":match item.pin_scope { photag_plugin_launcher::PinScope::None => "none", photag_plugin_launcher::PinScope::Category => "category", photag_plugin_launcher::PinScope::AllPrograms => "all" },"source":item.source,"available":item.available,"has_uninstaller":item.uninstall_command.is_some(),"installed_on":item.installed_on,"size_bytes":item.size_bytes})).collect::<Vec<_>>()
        })).unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(
        description = "Capture this PhoTag process's currently open native folder picker. No external window attachment."
    )]
    async fn screenshot_folder_picker(&self) -> Result<CallToolResult, McpError> {
        let Some(handle) = self.navigation.folder_picker_window() else {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "No native folder picker is open",
            )]));
        };
        match screenshot::capture_png(handle) {
            Ok((png, width, height)) => Ok(CallToolResult::success(vec![
                ContentBlock::text(format!("Folder picker, {width} × {height} pixels")),
                ContentBlock::image(
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png),
                    "image/png",
                ),
            ])),
            Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error)])),
        }
    }
    #[tool(
        description = "调整当前 PhoTag 原生窗口外框尺寸（像素），用于隔离档案的布局验证。范围：640×480 至 4096×2160。"
    )]
    fn resize_window(&self, Parameters(request): Parameters<WindowSizeRequest>) -> String {
        match screenshot::resize_window(self.capture.get(), request.width, request.height) {
            Ok((actual_width, actual_height)) => serde_json::to_string(&json!({
                "ok": true,
                "requested_width": request.width,
                "requested_height": request.height,
                "actual_width": actual_width,
                "actual_height": actual_height,
            }))
            .unwrap_or_else(|error| format!("{{\"error\":\"{error}\"}}")),
            Err(error) => serde_json::to_string(&json!({"error": error}))
                .unwrap_or_else(|_| "{\"error\":\"window resize failed\"}".into()),
        }
    }

    #[tool(description = "截取当前 PhoTag 原生窗口的真实画面，并以 MCP 图片内容返回。")]
    async fn screenshot_window(&self) -> Result<CallToolResult, McpError> {
        let handle = self.capture.get();
        if handle == 0 {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "PhoTag 窗口尚未就绪。",
            )]));
        }
        match screenshot::capture_png(handle) {
            Ok((png, width, height)) => Ok(CallToolResult::success(vec![
                ContentBlock::text(format!("PhoTag 窗口截图，{width} × {height} 像素。")),
                ContentBlock::image(
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png),
                    "image/png",
                ),
            ])),
            Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error)])),
        }
    }

    #[tool(description = "列出 PhoTag 已注册文件夹、标签树和对应媒体数量。")]
    fn list_library(&self) -> String {
        match self.navigation.snapshot() {
            Ok(snapshot) => serde_json::to_string(&json!({
                "folders": snapshot.folders.iter().map(|node| json!({
                    "id": node.folder.id,
                    "path": node.folder.path,
                    "name": node.folder.name,
                    "last_scan": node.folder.last_scan,
                    "media_count": node.media_count,
                })).collect::<Vec<_>>(),
                "tags": snapshot.tags,
            }))
            .unwrap_or_else(|error| format!("{{\"error\":{error}}}")),
            Err(error) => format!("{{\"error\":\"{error}\"}}"),
        }
    }

    #[tool(
        description = "在 PhoTag 图库中搜索媒体，支持路径、标签、类型、分页和排序；每页最多 100 条。"
    )]
    fn search_media(&self, Parameters(request): Parameters<SearchMediaRequest>) -> String {
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit.unwrap_or(50);
        if offset > 1_000_000 || !(1..=100).contains(&limit) {
            return error_json("offset 必须在 0 到 1,000,000 之间，limit 必须在 1 到 100 之间。");
        }
        if request.tag_ids.as_ref().is_some_and(|ids| ids.len() > 50) {
            return error_json("每次搜索最多指定 50 个标签。");
        }
        let sort = match request.sort.as_deref().unwrap_or("newest") {
            "newest" => Some(SortOrder::NewestFirst),
            "oldest" => Some(SortOrder::OldestFirst),
            "largest" => Some(SortOrder::BiggestFirst),
            "smallest" => Some(SortOrder::SmallestFirst),
            "name_asc" => Some(SortOrder::NameAsc),
            "name_desc" => Some(SortOrder::NameDesc),
            "rated" => Some(SortOrder::HighestRated),
            "unrated_first" => Some(SortOrder::LowestRated),
            _ => return error_json("不支持的排序方式。"),
        };
        if request
            .media_type
            .as_ref()
            .is_some_and(|value| !["image", "video", "audio", "file"].contains(&value.as_str()))
        {
            return error_json("media_type 只支持 image、video、audio 或 file。");
        }
        let query = photag_plugin_database::MediaQuery {
            folder_id: request.folder_id,
            tag_ids: request.tag_ids,
            search_in_filenames: request.search.is_some(),
            search_in_paths: request.search.is_some(),
            search_in_tags: request.search.is_some(),
            search: request.search,
            media_type: request.media_type,
            sort,
            ..Default::default()
        };
        let result = self.gallery.load(query.clone()).and_then(|snapshot| {
            let search_provider = snapshot.search_provider;
            let page = if offset == 0 {
                snapshot.media.into_iter().take(limit).collect()
            } else {
                self.gallery
                    .load_page(query, offset)
                    .map(|page| page.media.into_iter().take(limit).collect::<Vec<_>>())?
            };
            Ok(json!({ "total": snapshot.media_count, "offset": offset, "items": page, "search_provider": search_provider }))
        });
        match result {
            Ok(value) => {
                serde_json::to_string(&value).unwrap_or_else(|e| error_json(&e.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "读取单个已索引媒体的详情以及 PhoTag 当前关联的标签。")]
    fn get_media(&self, Parameters(request): Parameters<MediaIdsRequest>) -> String {
        if request.media_ids.len() != 1 {
            return error_json("该工具要求 media_ids 恰好包含一个 ID。");
        }
        let id = request.media_ids[0];
        let media = match self.gallery.media_by_id(id) {
            Ok(Some(media)) => media,
            Ok(None) => return error_json("没有找到该媒体记录。"),
            Err(error) => return error_json(&error.to_string()),
        };
        let tags = match self.tagging.for_media(id) {
            Ok(tags) => tags,
            Err(error) => return error_json(&error.to_string()),
        };
        serde_json::to_string(&json!({ "media": media, "tags": tags }))
            .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(description = "Read the persistent Lore version history for one indexed media file.")]
    async fn list_file_versions(&self, Parameters(request): Parameters<MediaIdRequest>) -> String {
        let media = match self.gallery.media_by_id(request.media_id) {
            Ok(Some(media)) => media,
            Ok(None) => return error_json("没有找到该媒体记录。"),
            Err(error) => return error_json(&error.to_string()),
        };
        match self.versions.list(&media.full_path).await {
            Ok(versions) => serde_json::to_string(
                &json!({"media_id":request.media_id,"path":media.full_path,"versions":versions}),
            )
            .unwrap_or_else(|error| error_json(&error.to_string())),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "Compare file content with immutable saved revisions; distinguish unsaved edits, an unchanged historical revision, and content already stored in another version. No mutation."
    )]
    async fn get_file_version_state(
        &self,
        Parameters(request): Parameters<MediaIdRequest>,
    ) -> String {
        let media = match self.gallery.media_by_id(request.media_id) {
            Ok(Some(media)) => media,
            Ok(None) => return error_json("没有找到该媒体记录。"),
            Err(error) => return error_json(&error.to_string()),
        };
        match self.versions.file_state(&media.full_path).await {
            Ok(state) => json!({"media_id":request.media_id,"path":media.full_path,"state":state})
                .to_string(),
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(description = "读取单个文件的 Lore 分支、分支版本链和待处理合并冲突。")]
    async fn list_file_branches(
        &self,
        Parameters(request): Parameters<BranchGraphRequest>,
    ) -> String {
        let media = match self.gallery.media_by_id(request.media_id) {
            Ok(Some(media)) => media,
            Ok(None) => return error_json("没有找到该媒体记录。"),
            Err(error) => return error_json(&error.to_string()),
        };
        match self
            .versions
            .branch_graph(&media.full_path, request.include_archived)
            .await
        {
            Ok(graph) => serde_json::to_string(
                &json!({"media_id":request.media_id,"path":media.full_path,"graph":graph}),
            )
            .unwrap_or_else(|error| error_json(&error.to_string())),
            Err(error) => error_json(&error.to_string()),
        }
    }
}
