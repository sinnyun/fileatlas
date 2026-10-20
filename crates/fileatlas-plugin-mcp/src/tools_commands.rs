use super::*;
use fileatlas_ui_bridge::{CommandNeeds, RegisteredCommand, ShortcutScope, UiCommandInvoked};
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(super) struct CommandRunRequest {
    /// Stable command identifier returned by command_list.
    command_id: String,
    /// Required only for commands registered as ConfirmedMutation.
    confirm: Option<bool>,
}

#[tool_router(router = commands_tool_router, vis = "pub(super)")]
impl super::FileAtlasMcpServer {
    #[tool(
        description = "List lifecycle-registered FileAtlas UI commands, menu placements, keybindings, and their invocation requirements."
    )]
    fn command_list(&self) -> String {
        let commands = match self.command_catalog.commands() {
            Ok(commands) => commands,
            Err(error) => return error_json(&error.to_string()),
        };
        let overrides = self
            .context
            .try_service::<fileatlas_plugin_settings::SettingsService>()
            .ok()
            .and_then(|service| service.snapshot().ok())
            .map(|settings| settings.keybindings)
            .unwrap_or_default();
        serde_json::to_string(&json!({
            "commands": commands.iter().map(|command| {
                    let (enabled, reason) = self.command_availability(command.needs);
                json!({
                    "id": command.id,
                    "owner": command.owner,
                    "label": command.label,
                    "needs": needs_name(command.needs),
                    "enabled": enabled,
                    "disabled_reason": reason,
                    "menu_items": command.menu_items.iter().map(|item| json!({
                        "id": item.id, "label": item.label, "menu": item.menu, "order": item.order,
                    })).collect::<Vec<_>>(),
                    "keybindings": command.keybindings.iter().map(|key| json!({
                        "id": key.id, "key": key.key,
                        "effective_key": overrides.get(&key.id).unwrap_or(&key.key),
                        "enabled": !overrides.get(&key.id).is_some_and(String::is_empty),
                        "scope": match key.scope {
                            ShortcutScope::Application => "application",
                            ShortcutScope::System => "system",
                        },
                    })).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>()
        }))
        .unwrap_or_else(|error| error_json(&error.to_string()))
    }

    #[tool(
        description = "Invoke a registered FileAtlas UI command through the same Cordis event used by its UI contribution. Mutating commands require Settings → MCP → AI control to be enabled; commands requiring explicit confirmation also require confirm=true."
    )]
    async fn command_run(&self, Parameters(request): Parameters<CommandRunRequest>) -> String {
        let commands = match self.command_catalog.commands() {
            Ok(commands) => commands,
            Err(error) => return error_json(&error.to_string()),
        };
        let Some(command) = commands
            .iter()
            .find(|command| command.id == request.command_id)
        else {
            return error_json("unknown command_id; call command_list for registered commands");
        };
        if let Some(reason) = self.command_rejection(command, request.confirm) {
            return error_json(&reason);
        }
        if let Err(error) = self
            .context
            .emit::<UiCommandInvoked>(cordis::Routing::Unscoped, request.command_id.clone())
            .await
        {
            return error_json(&format!("command dispatch failed: {error}"));
        }
        json!({"queued":true,"command_id":request.command_id}).to_string()
    }
}

impl FileAtlasMcpServer {
    fn command_availability(&self, needs: CommandNeeds) -> (bool, Option<String>) {
        match needs {
            CommandNeeds::None => (true, None),
            CommandNeeds::MutationOptIn if !self.mutations_enabled() => (
                false,
                Some("启用常规控制或完全控制后才能执行此命令。".into()),
            ),
            CommandNeeds::UndoAvailable if !self.mutations_enabled() => (
                false,
                Some("启用常规控制或完全控制后才能执行此命令。".into()),
            ),
            CommandNeeds::RedoAvailable if !self.mutations_enabled() => (
                false,
                Some("启用常规控制或完全控制后才能执行此命令。".into()),
            ),
            CommandNeeds::UndoAvailable if self.history.undo_description().is_none() => {
                (false, Some("there is no action to undo".into()))
            }
            CommandNeeds::RedoAvailable if self.history.redo_description().is_none() => {
                (false, Some("there is no action to redo".into()))
            }
            CommandNeeds::ConfirmedMutation if self.full_control_guard().is_err() => {
                (false, Some("此高影响命令需要设置为完全控制。".into()))
            }
            _ => (true, None),
        }
    }

    fn command_rejection(
        &self,
        command: &RegisteredCommand,
        confirm: Option<bool>,
    ) -> Option<String> {
        let (enabled, reason) = self.command_availability(command.needs);
        if !enabled {
            return reason;
        }
        match command.needs {
            CommandNeeds::MutationOptIn
            | CommandNeeds::UndoAvailable
            | CommandNeeds::RedoAvailable => self.mutation_guard().err(),
            CommandNeeds::ConfirmedMutation => self.full_control_guard().err(),
            CommandNeeds::None => None,
        }
        .or_else(|| {
            (command.needs == CommandNeeds::ConfirmedMutation && confirm != Some(true))
                .then(|| "this command requires confirm=true".into())
        })
    }
}

fn needs_name(needs: CommandNeeds) -> &'static str {
    match needs {
        CommandNeeds::None => "none",
        CommandNeeds::MutationOptIn => "mutation_opt_in",
        CommandNeeds::UndoAvailable => "undo_available_and_mutation_opt_in",
        CommandNeeds::RedoAvailable => "redo_available_and_mutation_opt_in",
        CommandNeeds::ConfirmedMutation => "confirmed_mutation",
    }
}
