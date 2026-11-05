# FileManager gallery layouts concept and implementation checklist

Active concept for the FileManager gallery layout extension. It extends the
FileAtlas 0.2 multi-column reference and keeps its toolbar, global navigation,
inspector, deep-blue theme and per-pane behavior.

Concept image: [four gallery layouts](gallery-layouts-2026-11-05.png).
Baseline references: [multi-column workspaces](multi-column-workspaces-2026-10-19.png)
and [FileManager inspector](sidebar-pages-preview-2026-10-14-v2.png).

## Gallery structure

- The common toolbar remains a single horizontal row. Grid, List, layout menu,
  descendant toggle and thumbnail-size control remain together in the existing
  view-control group.
- The layout menu selects Waterfall, Justified, Grid or List. The selected mode
  is visible in the menu and is saved independently for each FileManager pane.
- Waterfall uses equal-width columns and the media aspect ratio; each next card
  is placed in the currently shortest column.
- Justified arranges images in horizontal rows of equal height. Full rows fill
  available width while preserving every image's aspect ratio; the final
  incomplete row keeps the selected row height.
- Grid keeps square card frames. The thumbnail is centered, contained and never
  cropped or stretched; unused space uses the native neutral thumbnail surface.
- List keeps one file per row and exposes configurable columns. Initial columns
  are name, type, dimensions, modified time, size and rating.
- Card controls expose Show name, Show details and Show extension. Defaults
  preserve current FileAtlas card content.
- Thumbnail size is interpreted as card width in Waterfall/Grid, target row
  height in Justified, and the existing compact preview size in List.

## State and interaction

- All modes preserve independent pane selection, keyboard focus, drag payloads,
  active filters, grouping, navigation and viewport restoration.
- Reflow after width/layout/size changes preserves the first visible file and
  its relative viewport offset instead of returning to the beginning.
- Group headers remain before their group's gallery content. Collapse and
  expand operate on group identity, not transient layout row indices.
- Only viewport-visible media requests rich metadata and system thumbnails;
  geometry may use lightweight file identity and available dimensions.
- The source image is fitted without crop or aspect distortion. Empty frame area
  uses the shared native neutral surface. Selection, hover and badges remain
  above the image as overlays.
- Window resize, pane resize, compact panes, empty/loading/error states, long
  names, missing thumbnails, drag/drop, selection and restored scroll all need
  visual/behavioral review.

## System thumbnail source

- FileManager asks the Windows Shell image factory for its thumbnail-only result
  so installed format handlers and the Windows thumbnail cache are used.
- FileAtlas does not decode originals or run FFmpeg to create FileManager card
  thumbnails. When Shell has no thumbnail, the card shows the existing
  file-type placeholder.
- The Shell bitmap is persisted losslessly as a transport cache for GPUI and
  keyed by source identity/version. This stores Shell output; it does not
  synthesize or resize a FileAtlas thumbnail.
- FileManager ignores stale pre-existing FileAtlas-generated thumbnail paths
  and asks the Shell path for visible items. Other feature-owned preview assets
  remain on their existing service contracts unless they represent ordinary
  FileManager media cards.
