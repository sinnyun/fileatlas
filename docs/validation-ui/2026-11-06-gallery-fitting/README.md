# Gallery fitting correction — 2026-11-06

Reference: `docs/reference-ui/concepts/gallery-layouts-2026-11-05.png`, opened and
compared with actual MCP captures. Checklist: Grid contains full artwork with
neutral letterboxing; Waterfall keeps equal widths and varied heights; Justified
keeps equal heights per row and varied widths; List keeps compact contained
previews; group boundaries, filenames and dimension captions remain separate.

## Reproduction and fix

Existing GUI PID 33296 at the workspace's normal debug executable was inspected
through `http://127.0.0.1:38473/mcp`. It showed the supplied clipping on real PNGs.
MCP `file_manager.get_media {media_id:10}` returned null original dimensions for
`Shot 007.png`, while its Shell transport PNG was 768×544. After correction the
same media reports 2143×1518 (original header dimensions), not transport size.

GPUI image layout assigns intrinsic width/height when style dimensions are Auto;
absolute inset alone therefore did not bound image artwork. Explicit `size_full`
before Contain corrects this. Missing original dimensions are read on the existing
bounded viewport worker; completion updates skeleton ratios and invalidates cached
geometry on actual changes. The ungrouped browser path previously always used grid
rows, bypassing gallery geometry; Waterfall/Justified now use their geometry there.

## Runtime evidence

Only the existing instance was initially connected. Rebuild encountered its EXE
lock, so its verified process was closed normally, the same executable rebuilt,
and the normal `all` development configuration restarted. All subsequent checks
used that GUI's HTTP endpoint, not a new stdio test profile. Data remained in the
existing `target/dev-test` profile. Windows MSVC 1.97.1, 150% DPI, 3286×1831 captures.

Commands use `node scripts/fileatlas-mcp-http.cjs <tool> '<JSON>' <output>`:
`get_ui_state {}`, `screenshot_window {}`, `file_manager.get_media {"media_id":10}`,
`control_file_manager {"action":"layout","value":"grid|waterfall|justified|list"}`,
`control_file_manager {"action":"show_descendants","enabled":true}` and
`control_file_manager {"action":"grouping","value":"subfolder"}` (each layout
value is sent separately). Actual captures were opened and inspected.

Grid now shows the full annotated Shot 007 image and full desktop screenshots with
neutral letterboxing. Waterfall uses equal column widths and varied image heights;
Justified uses equal image heights within each row. The three 1600×1131 native
preview files also show complete pages. List images are contained. Both gallery
modes were inspected with grouping disabled and subfolder grouping enabled.
The app was left in Grid with subfolder grouping and descendants enabled.

Local-only artifacts in `.local/validation/profiles/gallery-fitting-20261106/`:
`current-before.png`, `grid-after.png`,
`waterfall-after.png`, `justified-after.png`, `list-after.png`,
`grouped-waterfall-after.png`, `grouped-justified-after.png`, and paired
`settled-a.json`/`settled-b.json`. No generated files
are committed. Large directory scrolling, all Shell provider formats, EXIF
orientation and multi-column resize are not established by this pass.

Build and governance: `cargo +1.97.1 build --locked -j 2 -p fileatlas-shell`
passed (existing LNK4098 warning). Theme, refresh, architecture and feature-ledger
guards passed. Focused layout regression tests and fmt results are recorded below.

`cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-file-manager gallery_layout`
passed both layout tests (shortest-column placement and full-row width/ratio
preservation; 25 unrelated tests filtered out). `cargo +1.97.1 fmt --all -- --check`
and `git diff --check` passed. Theme guard `--self-test` also passed.

Quiet-state check: two HTTP `get_ui_state` reads two seconds apart both reported
snapshot revision 11, loaded count 110, directory worker inactive, browser status
`[4,3,false,110,null]` and Justified layout. This establishes stable published
directory/snapshot state, not a CPU or filesystem I/O measurement.

## Follow-up: later groups and mislabeled external cache images

The supplied follow-up screenshot used file-type grouping, with JPG files after
the first group all falling back to square geometry. Three inspected hashed .jpg
files in `D:/桌面文件夹/数据库/cache/thumbnails` actually start with PNG magic bytes.
Extension-based `image_dimensions` therefore failed. Header-sniffed ImageReader
dimensions fixes this: MCP now reports `2a7423abd5ac3fb8.jpg` as 256×120 and the
live Justified capture shows distinct 256×120, 171×256 and 256×192 image geometry.
The regression `dimensions_follow_file_header_when_extension_is_wrong` saves PNG
bytes with a .jpg suffix, verifies 30×10 dimensions, and passed.

Shell dimensions are also returned separately as display-only fallback ratios
when originals are unavailable. Original media width/height remain unchanged by
that fallback. Later gallery chunks exposed cached List elements that did not
invoke their builders again on scroll; viewport demand now independently follows
the logical top row and following row, preserving keyed ReadModel deduplication.

Follow-up local artifacts share this run directory: `hash-jpg-fixed.png`,
`hash-jpg-later-fixed.png`, `hash-jpg-waterfall-fixed.png`, and MCP state JSON.
The focused directory finished enumerating 130451 files, of which 499 matched
the .jpg filter. Directory enumeration was allowed to finish before visual checks.

Final follow-up run: after enumeration finished, scrolling to result 250 reported
window offset 184 and 315 loaded records. The final later-chunk screenshot was
opened: previously blank cards now contain image pixels and dimension captions,
including 130×40 and 200×60 wide images beside 200×200 images. Waterfall was also
captured after switching at that position. The app is left in the focused cache
directory with JPG filtering, file-type grouping and Justified mode at the top.
`cargo +1.97.1 test --locked -j 2 -p fileatlas-plugin-file-manager` passed all 28
tests; fmt, build, refresh/architecture/feature-ledger checks and diff checks passed.
