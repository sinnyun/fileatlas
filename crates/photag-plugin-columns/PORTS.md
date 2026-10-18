# Main Window Columns plugin ports

## Identity and lifecycle

- Cordis plugin: `photag.columns`; service: `photag/columns` (`ColumnLayoutService`).
- Requires `WorkspaceService`; Shell starts it after `photag.workspaces` and before Gallery.
- Cordis owns service publication and cleanup. State changes trigger the shared `photag_ui_bridge::refresh::Wake`.

## Input / output

- `snapshot()` returns current `ColumnState { layout, active_index, revision }`.
- `set_layout(Single|Dual|Quad)` changes layout and clamps active index.
- `activate(index)` selects an existing column and rejects out-of-range indices.
- Startup layout is read from the selected program workspace.
- Lifecycle-owned view contribution: `columns/toolbar` in slot `command.columns`.
  The toolbar offers explicit Single/Dual/Quad layout actions and, for multi-
  column layouts, a numbered active-column selector. It calls this service and
  refreshes from its `Wake`; removing `photag.columns` unregisters the view.

## Current limits

- The Shell mounts the contribution in the existing top control row and lays out
  Gallery panes as one, two side-by-side, or four in a 2×2 grid. Shared Gallery
  filter/search/view operations mount from the active pane; each pane retains its
  own address, navigation history, content, selection, scroll, and query state.
- `photag.gallery` routes folder/tag/path selection events to the active pane.
  Each pane has a minimum width and can scroll horizontally when the window is
  narrower than the combined pane widths.
- Same-process PhoTag MCP `get_ui_state.program_workspace` reports current layout
  and active pane. `control_ui` supports `columns_layout` and `activate_column`.
- Windows pointer behavior, very small-window scrolling, and large directory
  content remain subject to further platform-specific validation.
