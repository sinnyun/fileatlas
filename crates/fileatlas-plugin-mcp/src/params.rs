//! Typed host-level MCP input contracts.

use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct WindowSizeRequest {
    /// Requested outer window width, in physical pixels.
    pub(super) width: u32,
    /// Requested outer window height, in physical pixels.
    pub(super) height: u32,
}
