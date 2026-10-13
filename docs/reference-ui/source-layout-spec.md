# PhoTag source UI layout and styling specification

**Purpose:** source-code specification for restoring the migrated UI. Every
layout value below is read from the pinned source implementation; screenshots
are only a later rendered-result check and are not a source of dimensions.

**Source:** PhoTag v0.2.0 at audit baseline
`36cee242a78c0a8cd9ac6b0d63b64265aa0add90`, mirrored in
`vendor/photag-source/`. Values are egui logical points unless the source
formula says otherwise. A setting restored from a valid session can override
the listed initial value. When translating to GPUI, preserve the source value,
ordering, formula, and state rule; do not introduce a new value to make a
screenshot look similar. If egui derives a size from available space or its
theme defaults, retain/document that rule rather than inventing a fixed size.

## Global source tokens

The original centralized palette and geometry live in
`vendor/photag-source/src/theme.rs`.

| Token | Source value | Source location |
|---|---:|---|
| Base/panel/elevated/modal/faint surfaces | RGB 25 / 37 / 46 / 43 / 31 grayscale | `theme.rs:15-25` |
| Primary/secondary/muted text | RGB 235 / 160 / 112 grayscale | `theme.rs:29-39` |
| Danger/warning/star | RGB (232,96,96) / (232,168,96) / (240,200,80) | `theme.rs:166-169` |
| Accent default | Slate Blue RGB (90,127,160); six more named presets; can follow system accent | `theme.rs:53-64`, `theme.rs:94-128` |
| Corner radius | SM 4, MD 6, LG 8, XL 10, pill 999 | `theme.rs:184-192` |
| Thumbnail radius | 7 | `theme.rs:194-197` |
| Font sizes | tiny 10.5, small 12, body 13.5, button 13, heading 17, title 19 | `theme.rs:199-209` |
| Item spacing | x 8, y 6 | `theme.rs:505-512` |
| Button padding | x 12, y 5 | `theme.rs:505-512` |
| Menu/window margin | menu x10/y8; window 14 | `theme.rs:505-512` |
| Scrollbar | 7 wide, floating | `theme.rs:505-512` |
| Settings control appearance | 5px widget corners; no outline; ghost/hover/pressed fills and checkbox grays are explicit | `theme.rs:408-503` |
| Backdrop | Base/modal surfaces change alpha when native backdrop is active | `theme.rs:415-426` |

Do not hard-code a replacement palette independently in each plugin. Source
colors are premultiplied in places; preserve the source compositing behavior,
not only the nominal RGB values.

## Main window framework

Source structure is three horizontal bands around the main two-column body:

```text
Top command bar (search + filters + conditional active-filter pills)
├── Left folder/tag sidebar (160–500 wide; initial 220)
└── One elevated central Gallery card
Bottom status bar (scan progress | selection actions | idle count)
```

The window starts at **1280×800**, minimum **800×500**, titled `PhoTag`
(`vendor/photag-source/src/main.rs:73-82`). `AppState::new` starts with
sidebar width **220** and thumbnail size **150** (`src/app.rs:512`). Session
restore accepts sidebar widths **160–500** and thumbnail sizes **80–300**
(`src/main.rs:51-76`).

### Top command bar

The top-level panel is frameless, has no separator, and uses left/right margins
16, top 12, bottom 8 (`src/main.rs:1007-1018`). Its content order is:

1. Row 1: `PhoTag` wordmark (semibold 15), 4px gap, centered search field,
   right-side thumbnail-size slider and icon, Tools, Settings. The right cluster
   reserves its width first; search fills the remainder but is clamped to
   220–540 wide (`src/main.rs:1020-1080`). Search height is **32** and pill
   radius **16** (`src/ui/filterbar.rs:15-29`). Search icon x+14, text x+34,
   text vertical inset 4, trailing reserve 12 empty / 30 populated; inner text
   width is at least 40 (`filterbar.rs:27-42`).
2. Row 2: 6px vertical gap, then a wrapped horizontal filter row. Chip height
   **26**; row spacing **6×6**. Exact control order is Sort, optional Months
   (only for Newest/Oldest sorting), media-type segmented control, Extensions,
   From/To dates, rating range and conditional Clear (`main.rs:1083-1095`,
   `filterbar.rs:16-18,266-275`). The row wraps when space is insufficient.
3. Row 3: active filter pills appear only when one or more applicable filters
   are active; after them is 6px bottom spacing. It is not a permanently
   reserved blank row (`main.rs:1095-1097`).

The search-scope popup is hidden when query is empty; otherwise visible while
the field has focus or pointer is over the popup. Popup anchor is field left+8,
below by 6; minimum width 150; inner padding x12/y10; heading then 2px gap and
Filenames/Paths/Tags checkboxes (`filterbar.rs:95-155`).

The thumbnail slider is continuous over **80–300**, width **116**, height
**22**, inner track ends 8 from either side, thumb radii 6.5/5/3.2 with dark
halo, neutral fill and accent center (`main.rs:1055-1059`,
`main.rs:1055-1059`, `filterbar.rs:721-771`). Do not replace it with discrete +/-20 behavior.

Filter component details (`filterbar.rs`):

| Component | Source dimensions and behavior |
|---|---|
| Filter chip | Height 26; measured label width +24; radius13; font12.5; accent-soft when active, otherwise transparent until a white-alpha hover animation (0.08s), `158-190` |
| Segmented media type | Height26; each segment width is measured label width +26; total is widths +6; track/segment inset3; inner segment height20, `206-255` |
| Sort | Combo box height300; width is not fixed, `277-297` |
| Extensions popup | Minimum width220; Select/Exclude header, modifier hint, separator, extension entries, optional Clear all; height is content-determined, `336-410` |
| Date controls | Inactive date is a chip; activation defaults to today; active uses `%Y-%m-%d` DatePickerButton with calendar/highlight-weekends and adjacent clear; no source fixed width/height, `773-821` |
| Rating range | Hit target150×22; track 3px; track inset9; thumbs radius6.5, dark halo +1.5 and accent center3.2; range0..5; click selects nearest thumb; dragged thumbs do not cross, `600-714` |
| Active-filter pills | Entire row omitted when empty; gap above4; horizontal gap5; order Search, Type, From, To, included extensions, excluded extensions; height22, padding left10/right8, close gap6/target10, label width +34; each region can remove that filter, `456-587` |

The filter row uses `horizontal_wrapped` with spacing 6×6. Its exact control
order is Sort; conditional Months; media segmented control; Extensions;
From/To; rating; conditional Clear (`filterbar.rs:263-453`). Months appears
only for Newest/Oldest sort (`filterbar.rs:299-313`).

### Sidebar and resizer

The left SidePanel is below the top bar and above the status bar, has no
separator line, and uses frame inner margins left 10/right 6/top 2/bottom 8.
Width is exact session state, default 220, clamped 160–500
(`main.rs:1282-1296`, `app.rs:512`). The resizer hit area is **8** wide,
centered on the panel edge; it only spans the sidebar's vertical bounds. Its
visible hairline is inset top 72 and bottom 14 and is painted only while
hovered/dragged (`main.rs:1299-1346`).

See [Sidebar](#sidebar) below for row, tree, and tab details.

### Gallery card

The Gallery is the **only elevated card**; chrome around it is directly on base
material with no fill or border. The card uses panel fill, radius **10**, no
stroke, inner margins left 14/right 10/top 12/bottom 10, outer margins
left 0/right 12/top 2/bottom 12 (`main.rs:1007-1014,1351-1367`; radius in
`theme.rs:191`). Do not give every subregion a separate raised card.

The source gallery takes `ui.available_size()`, not the window size. With
thumbnail size `t` and fixed gap **12**, columns are
`floor((available_width + 12) / (t + 12))`, minimum 1; each image hit target
is **t×t**, each row advances `t+12` (`ui/gallery.rs:990-1005,1383-1387`).
Do not derive fixed columns from a screenshot. The month header appears only
when date dividers are enabled and sort is Newest/Oldest; header height **40**,
14px before later groups; clicking the header collapses that month
(`gallery.rs:693-707,746-766`). Flat sorts have no month header.

Gallery cards are square image surfaces, not separate thumbnail-plus-metadata
cards. Filename, rating, media badges, video/audio glyphs, tags and other card
details are painted over the same image rect (`gallery.rs:1383-1387,
1406-1429,1493-1538,1563-1590`). Thumbnail corner radius is **7**;
hover scale is **1.8%** over **120ms**; selection insets image by **7**, paints
accent fill and **2px** outline. Overlay badge offsets are 7 where specified
(`gallery.rs:1406-1429,1493-1538,1563-1578`). Empty gallery state is centered
in the available card area: icon 52, headline at center+12, hint at center+36
(`gallery.rs:997-1024`).

### Status bar

Bottom panel is frameless/no separator, margins left/right **16**, top **7**,
bottom **9** (`main.rs:1099-1105`). Exactly one mode is shown, in priority
order: scan progress, selected-item count with inline actions, then idle library
count (`main.rs:1107-1277`). Progress bar is width **180**, height **6**,
radius **3** (`main.rs:1114-1118`). Selected-state action spacing is source
layout (for example 8px after selected count at `main.rs:1185`); actions
belong here and must not be invented as a full-width Gallery row.

## Sidebar

Source: `vendor/photag-source/src/ui/sidebar.rs`.

- Folders/Tags are equal-width pivot tabs, width `max(available/2,60)`, height
  **32**, no inter-tab gap. Active is semibold 13/primary with centered short
  accent underline; inactive is 13/secondary. After tabs: **6** gap
  (`sidebar.rs:181-193,228-275`).
- Section header starts after 2, label/action row inset 2; uppercase tiny
  muted label; then 4 gap. Folder add button **22×22**, accent fill, radius
  `ROUND_SM+1`, icon 10 (fallback plus 16) (`sidebar.rs:111-160,283-288`).
- Folder list is vertical `ScrollArea(auto_shrink=false)`, row spacing **1**;
  All files row is followed by **2** (`sidebar.rs:290-316`). Selectable rows
  are full available width and height **26**; text begins x+4, truncates with
  36 reserved on the right; selected fill uses `ROUND_SM`, hover uses weak
  widget fill; selected bar is 3 wide; count is tiny/muted at right-6
  (`sidebar.rs:58-109,162-179`).
- Folder/tag arrow slot is always **16×16**, even without children. Arrow icon
  font 8, muted normally and primary on hover (`sidebar.rs:195-226,327-345`).
  Subdirectory indentation is `6 + depth*15`, no connector lines
  (`sidebar.rs:381-425`).
- Folder context menu order: New subfolder, Rescan, Open in Explorer,
  separator, Remove. Subdirectory: New subfolder, Open in Explorer
  (`sidebar.rs:353-372,414-424`).
- Tags search: icon 11; input margin x6/y2; clear target 22; row width is
  available minus clear width and 4; after row 4. Mode row is Select/Exclude/
  All; conditional Clear row follows active selections; each spacing is 4
  (`sidebar.rs:458-540`). Tag tree uses vertical non-shrinking ScrollArea,
  row spacing 1, indent `depth*16`, same 16 arrow slot and 26 row height;
  search results force ancestors expanded (`sidebar.rs:542-621`).

## Preview and information surfaces

Source: `vendor/photag-source/src/ui/preview.rs` and `src/ui/info.rs`.

- Preview is a full-window foreground surface and returns immediately when no
  item is active. A previewed item increments open count once. Mouse, key or
  wheel activity reveals chrome; it begins fading after about 2 seconds and
  reaches hidden at 2.8 seconds (`preview.rs:940-1002,255-262`). Info toggle
  is drawn before AI, then Info panel; right metadata/EXIF panel is a distinct
  overlay and only applies to images (`preview.rs:993-1093`).
- Info toggle: center x+25/y+25, 30×30 hit target, circle radius14, glyph18;
  active blue RGB(60,130,240); click only when chrome alpha > .5. Close uses
  same 25px edge offset and 30px hit rect (`preview.rs:389-453`).
- AI panel: x+8/y+88, width `min(240, 30% screen)`, dark RGB(25,25,30) alpha
  220, radius8, padding10; title12/content11; confidence bar
  `max(panel_width-60,80)×16`. Shown only for current preview with suggestions
  or inference (`preview.rs:510-653`).
- Info overlay: x+12/y+50, width `min(320,32% screen)`, height `max(screen
  remaining-16,120)` with content max; RGBA(20,22,28,230), radius8, 1px border,
  padding12×10, shadow offset(0,4)/blur14, interactive foreground scroll area
  (`info.rs:18-82`). Field order: File; image/video fields; Dates; Tags; Engagement;
  image-only Metadata. Section lead gap6, blue-gray heading RGB(170,200,255)
  plus separator (`info.rs:84-190`). Preserve fields and empty/error states.
- EXIF overlay is right-aligned, width `min(280,28% screen)`, y+50,
  height `max(screen remaining-16,120)`, padding12×10, rounded8 and scrollable.
  Fields include camera/model/lens/focal/aperture/shutter/ISO/date/color space/
  software/dimensions/size/GPS (`preview.rs:1004-1093`).
- Image filmstrip is full width, height **80**, thumbs **64×64**, gap4,
  vertical inset8; active border2 accent (`preview.rs:797-935`). Floating image
  toolbar is centered near bottom, bottom offset50 plus filmstrip lift, dark
  alpha230, radius8, padding10×6, horizontal gap6; user offset/lock persists
  (`preview.rs:2370-2419`, `settings.rs:184-194`).
- Video toolbar is screen-bottom offset100, black alpha160, radius8, padding
  x10/y4, gap6; tags/clear/separator/delete order. Side navigation hit zones
  are width60 and vertical middle 50% (top25%); close remains 25px from edges
  (`preview.rs:1241-1486`). Audio stage RGB24; center information at
  `screen_center-(150,80)` (`preview.rs:1107-1240`).

## Settings

Settings is a movable, non-collapsible, non-resizable egui Window, default
width **360**, initially centered at center-180 / center-200
(`vendor/photag-source/src/main.rs:2107-2175`). Tabs are Appearance, Toolbar,
Shortcuts, Tags, AI, Updates, height **28**; source computes tab width as
360/5 despite six tabs. This is the source behavior and an identified overflow
risk; do not “correct” or copy the arithmetic without documenting the deliberate
compatibility decision. A 4px gap and separator precede content. Toolbar groups
are Zoom controls, Photo manipulation, Tags, File, View, with 8px between
groups (`main.rs:2251-2278`). Bottom: gap8, separator, gap2, Close. Escape closes
unless shortcut capture consumes it; changes save (`main.rs:2636-2650`).

Settings data defaults are in `vendor/photag-source/src/settings.rs`, not the
UI layout: Space preview key; system accent fallback Slate Blue; date dividers
true; search scopes filenames/paths/tags true; contains matching and persistent
tag search true; merge children Reparent; filmstrip false; toolbar zero offset,
unlocked; preview toolbar buttons default true (`settings.rs:129-182,
202-255,259-281`).

### Settings tab content and exact layout rules

The settings window has width **360** and content-driven height. Unless stated
otherwise, egui controls below have no source-fixed dimensions: preserve their
sequence, row/group structure, spacing calls and current egui style. The source
tab width is **72** (`360/5`) even though it renders six tabs; this apparent
overflow is a source behavior, not a reason to infer widths from a screenshot.

| Tab | Source content, order and explicit geometry |
|---|---|
| Appearance | `Accent color` heading, gap2, muted explanatory text, gap8; Windows system accent swatch **22×22**, radius4 (selected outline expands2, radius6, stroke1.5); then each `ACCENT_PRESETS` swatch **22×22**, with gap2 before each name. Selecting a swatch applies and saves the color (`main.rs:2177-2250`, `theme.rs:53-64,94-128`). |
| Toolbar | Groups in this order: Zoom controls (Zoom mode toggle, Reset button, Zoom presets dropdown), Photo manipulation (Rotation, Mirror), Tags (Tags dropdown), File (Delete/recycle bin, Copy to…, Move to…, Show in Explorer, Open in default app), View (Filmstrip toggle). Each bold heading has gap2; groups have gap8. Controls follow source order and are checkboxes without explicit dimensions. All toolbar flags default true (`main.rs:2251-2278`, `settings.rs:202-216,240-255`). |
| Shortcuts | Tag sidebar: heading/gap2; modifier prompt on its own row; **80-wide** None/Ctrl/Shift/Alt ComboBox beside toggle explanation; gap4 and muted two-line help. Gallery: group gap12; heading/gap2; Open selected media key-capture row; gap2 and muted help. Ratings: group gap12; heading/gap2 and muted fixed explanation. Tag hotkeys F1–F9: group gap12; heading/gap2, muted help, gap4; nine rows with **200-wide** ComboBoxes (None plus recursively flattened tag tree/path). Key capture shows current key; clicking enters capture with Press a key… and Cancel; Escape cancels, another key saves. Defaults Ctrl, Space and no tag bindings (`main.rs:2280-2389,682-710`, `settings.rs:21-50,133-138,163-165,223-224,231`). |
| Tags | Tag tree search: heading/gap2; Match mode with Contains and Starts with choices; gap2; Keep search query when switching tabs checkbox. Merge section gap12; heading/gap2, muted child-handling explanation, gap2; Re-parent under target and Delete choices. Defaults are Contains, keep query, and Re-parent (`main.rs:2392-2443`, `settings.rs:57-62,64-110,151-162,228-230`). |
| AI | Auto-analyze heading/gap2 and checkbox; hover gives explanation. Mandatory tag groups section gap12; heading/gap2, muted explanation, gap8. Each group: `Group:` beside single-line name editor; gap2; each selected tag as icon/name and × removal; `Add:` beside **180-wide** selector; Remove group on following row; gap4 between groups; then + Add group. The selector omits tags already in the group. Empty group collection is the default. `AiSettings::default()` yields auto-analyze false; serde missing-field default is true, a meaningful distinction to preserve (`main.rs:2445-2542`, `settings.rs:12-19,234`). |
| Updates | Version row then gap6; startup-check checkbox and muted explanatory row; gap10, separator, gap8. Status order is checking, downloading, staged, update available, then up-to-date only without an error; error appears separately after gap2. Gap10 then one button row: Check now, conditional Show update…, Releases on GitHub. Gap10 then muted safety note. Startup-check preference defaults true; transient status/error/version is runtime state (`main.rs:2545-2632`, `settings.rs:54,177-180,235`). |

All tabs share a footer: gap8, separator, gap2, Close. Escape closes Settings except
while the shortcut capture handler consumes it. Changed settings save to
`%LOCALAPPDATA%\\PhoTag\\settings.json`; save errors are ignored in source
(`main.rs:2636-2650`, `settings.rs:284-305`). Content height and wrapping of
long text are determined by egui; do not assign invented fixed heights.

## Dialog and popup inventory

Unless a width/fixed-size is called out, the source dialog sizes itself to its
contents. `default_pos(centered_default_pos(...))` provides only the initial
placement; it does not fix the actual dialog dimensions. Source coordinates
are egui points (`main.rs:152-161`).

| Surface | Source constraints and order |
|---|---|
| Bulk actions | Non-resizable; default width440 and center-placement baseline440×480 (`main.rs:175-181`). Action (Delete/Copy/Move), gap8, Source options, gap8, AND filters in two-column grid (column gap8/row gap4), separator and action footer; name input240, extension80. Current Selection hides filters; execution disabled for zero matches; Cancel/Escape/window close cancel (`main.rs:182-329`). |
| Update available | Non-resizable; width440, placement baseline440×340 (`main.rs:802-808`). Version/size, current version, notes (max height180, scroll), error/progress, action buttons. Download state cannot be dismissed (`main.rs:820-903`). |
| Create tag | Non-resizable/non-collapsible; placement baseline280×100 but content-sized, not a fixed width. Name row then Create/Cancel. Enter/Create accept only trimmed nonempty name; Escape/close cancels (`main.rs:1481-1531`). |
| Edit tag | Non-resizable; width360, placement baseline360×320; content may extend. Name, color, icon (70 input/max8 chars), clear/picker/help, 8-column preset grid (28×28 cells, 16 icon, spacing4×4), thumbnail display, association tree (max height120), conditional child propagation, separator, Save/Cancel (`main.rs:1544-1689`). |
| Statistics | Resizable; width480, placement baseline480×500; content-determined height. Overview, rating distribution (bar base200×16), top tags (180×14), photos/month (180×14), gap8/separator/Close (`main.rs:1697-1823`). |
| Merge-tag picker | Non-resizable; width360, placement baseline360×420. Search text width280, gap4, cascade explanation, gap4/separator, candidate tree max300 and scrollable, gap4/separator/Cancel. Choosing an item merges immediately (no confirmation) (`main.rs:1842-1894`). |
| New folder | Non-resizable; content-sized, placement baseline300×120. Path, gap4, name input width260, gap6, Create/Cancel; Enter/Create submits, Escape/close cancels (`main.rs:1909-1947`). |
| Bulk delete confirmation | Non-resizable; content-sized, placement baseline320×130. Prompt, gap6, recycle-bin note, gap8, Delete-count/Cancel. Enter confirms; Escape/close cancels (`main.rs:1949-1981`). |
| Delete-tag confirmation | Non-resizable; content-sized, placement baseline380×160. Prompt lines separated by4, gap8, Remove with parents/Keep parents/Cancel; Escape/close cancels (`main.rs:1983-2022`). |
| Single-file delete confirmation | Non-resizable; content-sized, placement baseline320×130. Filename/recycle-bin prompt, gap6, restore note, gap8, Delete/Cancel; Enter confirms (`main.rs:2024-2068`). |
| Export metadata | Not an app modal. Tools invokes the OS save dialog with title `Export metadata backup`, JSON filter and default filename `photag_backup.json`; cancel does nothing (`app.rs:1535-1549`). Native dimensions/placement are OS-controlled, not specified by PhoTag. |
| Folder/file copy/move/pick | Uses native `rfd::FileDialog`; source doesn't define its size or position (`app.rs:1173-1180,1672-1690,1776-1810`). |

Settings is fixed width360 and height auto (`main.rs:2111-2134`). Its source
computes five tab widths but displays six tabs; this is a recorded source
overflow risk, not a value to silently normalize.

## Source component → migrated owner map

| Source component | Migrated owner |
|---|---|
| Window composition, command bar/status bar, sidebar/gallery placement | `photag-shell` |
| Search/filter controls, gallery card grid, selection/actions/statistics | `photag-plugin-gallery` |
| Folder/tag tree, section tabs, folder and tag commands | `photag-plugin-navigation` |
| Image/audio/video preview, floating toolbar, filmstrip | `photag-plugin-preview` + `photag-plugin-media-playback` |
| Selected details and preview Info/EXIF | Gallery and Preview feature crates; source fields above define both |
| Appearance/Toolbar/Shortcuts/Tags/AI/Updates settings | `photag-plugin-settings` |
| Source dialogs and modal confirmations | The owning feature plugin; shell only composes modal surfaces |

## Required UI change workflow

1. Read this spec and inspect the exact source render function, helper, caller,
   state/default, and relevant theme token in `vendor/photag-source/`.
2. Record source path and line references, exact values/formulas, and target
   plugin in the change notes. If this spec is incomplete or source behavior is
   ambiguous, complete the source audit first.
3. Translate source ordering, constraints, values, state predicates, overlays,
   and visible controls into the owning GPUI feature plugin. Keep lifecycle and
   public plugin boundaries; do not move product controls into the shell merely
   to make them easier to lay out.
4. Build and exercise the actual local app/MCP path. Compare screenshots only
   after implementation to catch rendering differences; never use them to
   invent missing dimensions or positions.
5. Update the source feature map and validation report. Explicitly list visible
   source controls that remain absent or intentionally differ.
