use super::*;

#[tool_router(router = read_tool_router, vis = "pub(super)")]
impl super::PhoTagMcpServer {
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
}
