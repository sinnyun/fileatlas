# WorkCrate concept prompt set

Tool: built-in `image_gen.imagegen` via the imagegen skill; three independent
`ui-mockup` images, opaque backgrounds. Generated files were copied into this
directory and inspected before the three-level UI implementation.

## Project management

Generate a single high-fidelity 16:10 widescreen desktop application screenshot.
This is level 1 of 3: projects → pages → editor. Native Windows design-tool
aesthetic, light neutral surfaces, subtle cool gray dividers, restrained blue
selection accent, Segoe UI-like typography, generous clean white space. Thin
native window chrome, no OS desktop around it. Main toolbar shows WorkCrate /
项目管理 at left, compact 搜索项目 input, sort menu 最近修改, 已归档 filter, and
prominent 新建项目 at right. Main area is a responsive 3-column grid showing
three project cards: 1024 工程师文化节; 2026 新品节项目; 2022 新品开放项目.
Each card shows ONLY a large project title above a main cover photo/design
artwork. NO page numbers, physical dimensions, pixel sizes, quantities, material
names, page metadata strips, or counts. Covers are realistic colorful blue-orange
exhibition artwork. Very subtle card separation; avoid complete heavy outlines.
Each card has a tiny ellipsis menu beside its title. Show the second card's
contextual menu open and anchored to ellipsis: 打开项目, 重命名, 更换主图,
复制项目, 归档, 删除项目. Its cover stays visible. The first card has subtle
hover state, second selected. Lower area remains open with restrained '+ 新建项目'.
Avoid dashboard statistics, skeleton placeholders, thick colored borders,
red/green diagram boxes, project detailed information, side inspectors, page
thumbnails, watermarks and tiny illegible copy. Complete screen, not collage.

Output: `workcrate-projects-concept-2026-10-20.png`.

## Page management

Generate level 2, PAGE MANAGEMENT, in a consistent FileAtlas WorkCrate three-screen
desktop concept set. Single complete widescreen 16:10 screenshot, light neutral
theme, restrained blue accent, crisp Chinese typography, subtle dividers, compact
controls and thin Windows titlebar. Top breadcrumb/action bar: back arrow 项目管理 /
1024 工程师文化节, prominent project name, small settings ellipsis, search pages,
新增页面 at right. Main area: wide responsive three-column grid, exactly six
page previews in two rows, evenly spaced. Each has a slim information line ABOVE
the page: editable filename at left (主题背景.jpg, 人形展板.jpg, 游戏区.jpg etc.),
420 × 297 mm and actual source pixels such as 6000 × 4000 px at right. Page is A3
LANDSCAPE, aspect 1.414:1. Inside each page: compact pale gray content header with
ordinal 01, inherited project name at left and material/size/quantity at right;
below, freely positioned realistic colorful exhibition graphics, including a
portrait image and a second rotated image on some pages. Outside page information
is excluded from export; inside content header belongs to exported page. Subtle
white edge/shadow, no thick red/green/black sketch rectangles. Selection checkbox
outside each page at title left; 01 and 03 selected with quiet blue cue. Opening
separate from selection. Fixed bottom: 已选 2 页, PNG dropdown,
导出选中页为图片, primary 导出全部为 PDF. Default A3 420 × 297 mm regardless of
source-photo size. No project grid or inspector on this level, no editing handles.
Avoid heavy frames, giant duplicate buttons, dashboards, charts, fake warnings,
watermarks and gibberish. Actual artwork rather than blank placeholder rectangles.

Output: `workcrate-pages-concept-2026-10-20.png`.

## Page editor

Generate level 3 PAGE EDITOR for FileAtlas WorkCrate. One complete wide 16:10
desktop screenshot, native Windows design application, light neutral surfaces,
subtle cool gray separators, restrained blue accent, readable Chinese typography,
compact real controls. Breadcrumb: 项目管理 / 1024 工程师文化节 / 页面编辑.
Command row: selection tool, zoom 65%, fit, 添加图片, PNG导出, PDF导出. Three
columns below: narrow fixed LEFT PAGE RAIL about 180 px, heading 页面, three actual
page thumbnails with page number/filename above each, first selected in blue,
add-page action below. CENTER broad light-gray canvas workspace about 900 px in
a 1600 px window. Title above page: 主题背景.jpg left; A3 · 420 × 297 mm and
6000 × 4000 px right. Large white A3 LANDSCAPE artboard aspect 1.414:1 with subtle
shadow. Exportable content stays inside, including pale content header: 01 |
1024 工程师文化节 left, 人物立牌 · 80 × 180 cm · 20 套 right. Two freely positioned
images: portrait blue engineering-festival graphic and overlapping rotated
landscape graphic. One selected image with small resize handles and rotation
control. RIGHT INSPECTOR about 310 px, scrollable: 页面信息 (editable filename,
physical 420 × 297 mm, actual pixels); 页面内容 (project name with 继承全局 badge,
全局设置 and 局部修改; 物料名称, 尺寸, 数量, 添加字段); 页面图片 (two compact entries,
actual thumbnail, JPG filename, linked PSB filename, source pixels, 更新/替换/编辑).
Content fields update the exported header. Source filename/info above page are
not artwork. Use whitespace/dividers rather than full box frames. Avoid catalog
grids, red/green sketch borders, fake errors, oversized blank boxes, charts,
secondary windows, watermarks and gibberish. Show selected, focused and inherited
states. A3 page/output remains default regardless of source pixel dimensions.

Output: `workcrate-editor-concept-2026-10-20.png`.

The user sketches remain the source for information hierarchy. Real data and
the host's native theme replace concept artwork/paint. Source pixels are factual
metadata. Implementation differences and validation are recorded separately.
