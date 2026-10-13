use super::*;

#[tool_router(router = write_tool_router, vis = "pub(super)")]
impl super::PhoTagMcpServer {
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
        let service = Arc::clone(&self.metadata_export);
        let path = std::path::PathBuf::from(request.path);
        match tokio::task::spawn_blocking(move || service.write_to(path)).await {
            Ok(Ok(())) => "{\"ok\":true}".into(),
            Ok(Err(error)) => error_json(&error.to_string()),
            Err(error) => error_json(&error.to_string()),
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
                std::thread::spawn(move || while progress.recv().is_ok() {});
                serde_json::to_string(&json!({ "ok": true, "folder": folder }))
                    .unwrap_or_else(|error| error_json(&error.to_string()))
            }
            Err(error) => error_json(&error.to_string()),
        }
    }

    #[tool(
        description = "在 PhoTag 中打开媒体：图片进入图片预览，视频进入视频播放界面。只改变当前应用视图。"
    )]
    fn open_media(&self, Parameters(request): Parameters<MediaIdRequest>) -> String {
        let media = match self.gallery.media_by_id(request.media_id) {
            Ok(Some(media)) => media,
            Ok(None) => return error_json("没有找到该媒体记录。"),
            Err(error) => return error_json(&error.to_string()),
        };
        let view = match media.media_type.as_str() {
            "image" => {
                if let Err(error) = self.preview.open(&media.full_path) {
                    return error_json(&error.to_string());
                }
                "preview"
            }
            "video" => {
                if let Err(error) = self.playback.open(&media.full_path) {
                    return error_json(&error.to_string());
                }
                "playback"
            }
            "audio" => return error_json("当前 PhoTag 版本尚未实现音频预览。"),
            _ => return error_json("PhoTag 不支持该媒体类型的预览。"),
        };
        serde_json::to_string(&json!({ "ok": true, "view": view, "media_id": media.id }))
            .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(description = "读取当前图片预览和视频播放的视图状态。")]
    fn get_view_state(&self) -> String {
        let preview = self.preview.snapshot();
        let playback = self.playback.snapshot();
        serde_json::to_string(&json!({
            "preview": {
                "source": preview.source,
                "quarter_turns": preview.quarter_turns,
                "flip_horizontal": preview.flip_horizontal,
                "flip_vertical": preview.flip_vertical,
                "fit_to_window": preview.fit_to_window,
                "zoom_percent": preview.zoom_percent,
                "zoom_scale": preview.zoom_scale,
                "show_info": preview.show_info,
                "revision": preview.revision,
            },
            "playback": {
                "path": playback.path,
                "playing": playback.playing,
                "position_seconds": playback.position_seconds,
                "duration_seconds": playback.duration_seconds,
                "width": playback.width,
                "height": playback.height,
                "speed": playback.speed,
                "audio_available": playback.audio_available,
                "error": playback.error,
                "revision": playback.revision,
            }
        }))
        .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(description = "控制当前图片预览的旋转、翻转、适配、缩放和信息面板。")]
    fn control_preview(&self, Parameters(request): Parameters<PreviewControlRequest>) -> String {
        match request.action.as_str() {
            "rotate_clockwise" => self.preview.rotate_clockwise(),
            "rotate_counter_clockwise" => self.preview.rotate_counter_clockwise(),
            "flip_horizontal" => self.preview.flip_horizontal(),
            "flip_vertical" => self.preview.flip_vertical(),
            "fit" => self.preview.set_fit(),
            "toggle_info" => self.preview.toggle_info(),
            "zoom" => {
                let Some(percent) = request.zoom_percent else {
                    return error_json("zoom 操作需要 zoom_percent。");
                };
                if ![25, 50, 100, 125, 150, 200].contains(&percent) {
                    return error_json("缩放值只支持 25、50、100、125、150 或 200。");
                }
                self.preview.set_zoom_percent(percent);
            }
            _ => return error_json("不支持的预览操作。"),
        }
        self.get_view_state()
    }

    #[tool(description = "控制当前视频播放：播放/暂停、定位、逐帧步进、倍速或关闭。")]
    fn control_playback(&self, Parameters(request): Parameters<PlaybackControlRequest>) -> String {
        let result = match request.action.as_str() {
            "play_pause" => self.playback.toggle_play_pause(),
            "seek" => match request.seconds {
                Some(seconds) if seconds.is_finite() && (0.0..=86_400.0).contains(&seconds) => {
                    self.playback.seek(seconds)
                }
                _ => return error_json("seek 需要 0 到 86,400 秒之间的有限 seconds。"),
            },
            "step" => match request.frames {
                Some(frames) if (-300..=300).contains(&frames) => self.playback.step_frames(frames),
                _ => return error_json("step 需要 -300 到 300 之间的 frames。"),
            },
            "set_speed" => match request.speed {
                Some(speed) if speed.is_finite() && (0.1..=8.0).contains(&speed) => {
                    self.playback.set_speed(speed)
                }
                _ => return error_json("set_speed 需要 0.1 到 8 之间的有限 speed。"),
            },
            "close" => {
                self.playback.close();
                return self.get_view_state();
            }
            _ => return error_json("不支持的播放操作。"),
        };
        match result {
            Ok(()) => self.get_view_state(),
            Err(error) => error_json(&error.to_string()),
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
        description = "重建一个已注册文件夹的索引；会先清理该文件夹现有数据库媒体行，不会删除源文件。调用方必须将 confirm 设为 true；修改工具默认关闭。"
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
                std::thread::spawn(move || while progress.recv().is_ok() {});
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
