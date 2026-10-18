use std::sync::Arc;

use photag_plugin_settings::McpAccessLevel;
use photag_plugin_settings::SettingsService;
use photag_ui_bridge::settings_page::{
    self, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
};

pub(super) fn contribution(settings: Arc<SettingsService>) -> photag_ui_bridge::UiViewContribution {
    let snapshot_service = Arc::clone(&settings);
    let update_service = settings;
    settings_page::contribution(
        "mcp/settings",
        "MCP",
        "控制 AI 通过当前 PhoTag MCP 连接执行的操作。",
        move || {
            let saved = snapshot_service
                .snapshot()
                .map_err(|error| error.to_string())?;
            Ok(SettingsPage::new(
                "AI 控制",
                "逐级开放界面控制、常规信息修改和高影响操作。",
                vec![SettingsSection::new(
                    "AI 操作权限",
                    vec![SettingRow::new(
                        "mcp.access_level",
                        "MCP 控制级别",
                        SettingControl::Segments {
                            value: saved.mcp_access_level.key().into(),
                            options: vec![
                                photag_ui_bridge::settings_page::Choice::new("off", "关闭")
                                    .description("仅允许读取状态"),
                                photag_ui_bridge::settings_page::Choice::new(
                                    "standard",
                                    "常规控制",
                                )
                                .description("允许控制界面，以及新增、编辑和删除常规信息"),
                                photag_ui_bridge::settings_page::Choice::new("full", "完全控制")
                                    .description("另外开放文件操作、卸载、恢复版本等高影响操作"),
                            ],
                        },
                    )
                    .description("完全控制档中的高影响操作仍需单次 confirm=true 确认。级别立即生效并自动保存。")],
                )],
            ))
        },
        move |id, value| match (id, value) {
            ("mcp.access_level", SettingValue::Choice(level)) => {
                let Some(level) = (match level.as_str() {
                    "off" => Some(McpAccessLevel::Off),
                    "standard" => Some(McpAccessLevel::Standard),
                    "full" => Some(McpAccessLevel::Full),
                    _ => None,
                }) else {
                    return Err("Unsupported MCP access level".into());
                };
                update_service
                    .update(|settings| settings.mcp_access_level = level)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            _ => Err("Unsupported MCP setting".into()),
        },
    )
}
