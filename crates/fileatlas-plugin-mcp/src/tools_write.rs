use super::*;

#[tool_router(router = write_tool_router, vis = "pub(super)")]
impl super::FileAtlasMcpServer {
    #[tool(description = "截取已验证属于 QuickLook.exe 的可见查看窗口，用于外部查看器集成验收。")]
    async fn screenshot_quicklook(&self) -> Result<CallToolResult, McpError> {
        #[cfg(windows)]
        {
            let Some(window) = self.quicklook.visible_windows().into_iter().next() else {
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
}
