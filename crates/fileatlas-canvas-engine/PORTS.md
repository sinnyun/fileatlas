# FileAtlas canvas engine

## Identity and responsibilities

Workspace package `fileatlas-canvas-engine` is the in-process canvas engine used by
the `fileatlas-plugin-whiteboard` Cordis product plugin. It is not a second plugin
manager and has no Cordis lifecycle or storage. It directly contains the MIT canvas
source copy, owns scene serialization/model, viewport, GPUI input dispatch, hit
testing, drawing, selection and transforms, plus the single scene undo/redo history.

Source: crates.io package `gpui-whiteboard 0.5.1`, upstream Zorite commit
`d94ade3eb162c041b71994681356fcbcb4dcfd8e`. The workspace no longer depends on the
external crate. The copy is from that package's `src/` and `assets/` boundary only;
no GPL application layer is included. The original MIT `LICENSE` is retained.
JetBrains Mono has its bundled OFL notice in `assets/JetBrainsMono-OFL.txt`;
Lucide assets have the ISC notice in `assets/icons/LICENSE`. Keep all notices and
pinned attribution if distributing.

## FileAtlas adaptations

- `WhiteboardView::new_embedded` uses the same view/scene engine while hiding its
  floating built-in toolbar. FileAtlas owns the only product toolbar.
- Public snapshots expose element paint order and selection IDs to the host.
- Host layer selection changes engine selection; host reorder edits Scene, pushes
  the same undo stack, marks dirty and calls the same `on_change` persistence hook.
- Existing grouped tools, color/font/template pickers, text/menu interactions,
  image loading, clipboard and import/export behavior are reused instead of being
  replaced with a second renderer or interaction implementation.

## Current limits

The engine Scene has no stable hidden/locked/group hierarchy properties. Elements
are a flat paint-order vector. It has no generic external card renderer or video
decoder. Changes to this copied source must preserve MIT notices and keep the scene
JSON additive/backward-compatible. Test engine behavior with `cargo +1.97.1 test
--locked -p fileatlas-canvas-engine -j 2`; run live interaction through the FileAtlas
MCP child, never a separately started application.
