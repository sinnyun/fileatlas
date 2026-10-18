# Program Workspaces plugin ports

## Identity and lifecycle

- Cordis plugin: `photag.workspaces`; service: `photag/workspaces` (`WorkspaceService`).
- Starts before `photag.columns` and Gallery. The Shell supplies an isolated JSON path for dev-test; ordinary startup uses `%LOCALAPPDATA%\\PhoTag\\workspaces\\workspaces.json`.
- Cordis owns service publication and removes it with the plugin fiber.

## Storage and data

- A versioned JSON document stores named launch layouts, default workspace ID,
  one/two/four column mode and each column's address/history, search scopes,
  media/tag/extension/date/rating filters, sort, grouping, grid/list mode,
  subfolder/thumbnail preferences, selected file paths, and scroll offset.
- It stores no media rows. Gallery queries continue through `photag-plugin-database`.
- Writes serialize to a same-directory temporary file, sync it, then atomically persist it over the target. Unknown future format versions are rejected without rewriting the file. Missing files load an in-memory single-column default and are created on the first change.
- `ProgramWorkspace::validate` checks IDs/names, exact column count, and rating bounds. `WorkspaceService::upsert`, `set_default`, `remove`, `snapshot`, and `active` are the public operations.

## Current behavior

- The JSON model, persistence service, and manager UI support create, activate,
  rename, set default, save current, and remove non-default definitions. Removing
  the active non-default workspace returns to the current default.
- Startup restores the default workspace. Activating a workspace applies every
  column definition to the independent Gallery pane instances; Gallery queries
  still run through the shared database/local browser service.
- Save current consumes `workspace.current_snapshot`, a host-published transient
  configuration snapshot. Each pane merges its own current state so edits from
  other panes are retained.
- Same-process PhoTag MCP `get_ui_state.program_workspace` reports active and
  default workspace definitions; `control_ui` can activate an existing workspace
  and open/close the manager.
- Invalid/missing addresses are retained as configuration and surface an error
  when opened; they do not alter Gallery database data.

## UI integration

- Cordis registers `workspace/manager-panel` in `workspace.modal` through
  `photag-ui-bridge`; Cordis removes the contribution when the plugin stops.
- The main toolbar opens it with `UiSurfaceState::set_open("workspace-manager", true)`.
- Selecting or creating a workspace writes its ID to `workspace.selection` and
  its layout key (`single`, `dual`, `quad`) to `workspace.layout`. The public
  `WorkspaceService::active()` returns this runtime selection; setting the
  default remains separate and controls next startup.
- Before saving a live layout, the host may serialize a current
  `ProgramWorkspace` into surface value `workspace.current_snapshot`. The Save
  current action persists that snapshot; if the host has not published one, it
  saves the current workspace definition held by the service.
- `workspace.selection` is a UI diagnostic hint; `WorkspaceService::active()` is
  the source of truth for the selected definition.
