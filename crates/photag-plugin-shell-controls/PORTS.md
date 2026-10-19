# Shell controls plugin ports

- Cordis identity: `photag.shell-controls`; no public service or persisted data.
- Startup dependency: the host publishes `photag-ui-bridge::UiBridgeService` before this plugin starts.
- Cordis-owned views: sidebar toggle → `shell.title-left-action`; inspector, workspaces and Tools → `shell.toolbar-actions`.
- View actions update `sidebar.visible`, `inspector`, `workspace-manager` and `tools` in `UiSurfaceState`; Shell observes those values and composes visibility.
- No events, payloads, database calls or IO. Diagnose clicks by inspecting the surface values and Shell's surface observer.
- Limits: Shell retains framing, title branding, slot mounting, panel geometry and resize behavior. Generic plugin command contributions are rendered by the host from the contribution catalog.
