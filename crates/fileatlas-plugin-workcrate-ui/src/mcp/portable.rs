use super::operations::{required, tool};
use super::*;

pub(super) fn tools(
    projects: &Arc<WorkCrateProjectsService>,
    boards: &Arc<WorkCrateBoardsService>,
    state: &Arc<WorkCrateUiState>,
) -> Vec<McpToolDefinition> {
    let p = projects.clone();
    let b = boards.clone();
    let ui = state.clone();
    vec![tool(
        "project_folder",
        "Relocate one project's dataset into its native document folder or reopen a moved portable folder",
        json!({"action":{"enum":["relocate","open","inspect"]},"project_id":{"type":"string"},"folder":{"type":"string"}}),
        vec!["action"],
        false,
        move |v| match required(&v, "action")? {
            "relocate" => {
                let id = required(&v, "project_id")?;
                let folder = b.relocate_project(id).map_err(|e| e.to_string())?;
                ui.request_catalog_refresh();
                Ok(json!({"folder":folder,"project_id":id}))
            }
            "open" => {
                let projects = b
                    .open_project_folder(std::path::Path::new(required(&v, "folder")?))
                    .map_err(|e| e.to_string())?;
                ui.request_catalog_refresh();
                Ok(json!({"projects":projects}))
            }
            "inspect" => {
                let id = required(&v, "project_id")?;
                Ok(
                    json!({"folder":p.portable_folder(id).map_err(|e|e.to_string())?,"data_directory":p.project_directory(id).map_err(|e|e.to_string())?,"document":p.document_path(id).map_err(|e|e.to_string())?,"references":p.references(id).map_err(|e|e.to_string())?}),
                )
            }
            _ => Err("unsupported action".into()),
        },
    )]
}
