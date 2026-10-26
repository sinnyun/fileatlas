# FileAtlas Canvas — tldraw layout pass (2026-10-26)

This pass follows the actual blank editor opened at [tldraw.com](https://www.tldraw.com/)
and its official [UI component layout](https://tldraw.dev/sdk-features/ui-components).
The app presents an edge-to-edge infinite canvas, a compact document/menu area at top left,
contextual style controls at top right, a floating icon toolbar at bottom center, and zoom
navigation at bottom left. Undo/redo sits just above the drawing tools. FileAtlas keeps its
own board/resource browser and layer inspector as slim side columns around that canvas.

![FileAtlas Canvas layout concept](fileatlas-canvas-tldraw-layout-2026-10-26.png)

## Implementation checklist

- The single managed FileAtlas panel uses most of the host viewport while retaining resize,
  move, minimize/maximize and close behavior. The canvas gets the largest center column.
- Top bar: board name and compact icon actions for new, rename, import, export, remove and
  refresh. Tool names are available from button tooltips.
- Left rail: a narrow scrollable board list followed by the external-resource list. Selecting a
  board activates it; selecting a resource opens it through its provider/QuickLook path.
- Center: the embedded upstream canvas remains the only document renderer. Main tools use one
  compact floating icon toolbar centered near the bottom; undo and redo float directly above.
  Color, font and template controls sit at the canvas upper-right and continue to open the
  existing engine controls. Zoom/navigation remains upstream-owned at the lower-left.
- Right: a narrow layer/selection inspector remains inside the FileAtlas host panel. Layer
  order, selection, geometry readout and reorder actions retain their existing semantics.
- Theme colors, button states, borders, radii and shadows all come from GPUI Kit Theme and UI
  Bridge components. The canvas dot grid and user artwork remain document rendering.
- Default state: the active board and drawing tools are visible with a broad canvas. Selected
  tool uses the shared accent selection state; menus and pickers appear only when opened.
  No selection shows the layer list and “未选择图层”. Save status and errors remain at the foot.
- Small windows keep the columns scrollable, keep the bottom tools in the canvas viewport where
  possible, and allow the host panel's existing resize/minimum-size behavior.

## Behavior boundaries

The drawing toolbar still delegates every drawing operation to the copied MIT canvas engine.
FileAtlas owns document actions, board/resource navigation, extension commands and layer
inspection. This layout does not claim that a video card has an embedded player, or that the
inspector can edit every engine style.

The prior `fileatlas-canvas-workspace-2026-10-26.png` concept described the first three-column
prototype. This tldraw-informed reference supersedes its top-toolbar placement and panel sizing;
its left-resource/center-canvas/right-inspector concept remains useful.
