# Main Window Columns plugin ports

## Identity and lifecycle

- Cordis plugin: `fileatlas.columns`; service: `fileatlas/columns` (`ColumnLayoutService`).
- Requires `WorkspaceService`; Shell starts it after `fileatlas.workspaces` and before FileManager.
- Cordis owns service publication and cleanup. State changes trigger the shared `fileatlas_ui_bridge::refresh::Wake`.

## Input / output

- `snapshot()` returns current `ColumnState { layout, active_index, revision }`.
- `set_layout(Single|Dual|Quad)` changes layout and clamps active index.
- `activate(index)` selects an existing column and rejects out-of-range indices.
- Startup layout is read from the selected program workspace.
- Lifecycle-owned view contribution: `columns/toolbar` in slot `command.columns`.
  One compact layout icon opens a dropdown with checked Single/Dual/Quad layout
  choices and, when multiple columns are visible, checked active-column choices.
  Both operations call this service and the toolbar refreshes from its `Wake`;
  removing `fileatlas.columns` unregisters the view.
  Shell places this contribution between the active FileManager pane's view-mode
  and view-option projections, next to grid/list. It uses the shared ghost icon
  button with room for the caret; Square/Columns2/Grid2x2 represent single,
  side-by-side dual and 2×2 quad layouts respectively.

## Current limits

- The Shell mounts the contribution in the existing top control row and lays out
  FileManager panes as one, two side-by-side, or four in a 2×2 grid. Shared FileManager
  filter/search/view operations mount from the active pane; each pane retains its
  own address, navigation history, content, selection, scroll, and query state.
- `fileatlas.file_manager` routes folder/tag/path selection events to the active pane.
  Each pane has a minimum width and can scroll horizontally when the window is
  narrower than the combined pane widths.
- Same-process FileAtlas MCP `get_ui_state.program_workspace` reports current layout
  and active pane. `control_ui` supports `columns_layout` and `activate_column`.
- Windows pointer behavior, very small-window scrolling, and large directory
  content remain subject to further platform-specific validation.
