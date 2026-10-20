# FileAtlas reference-layout concept v5

2026-10-13. Tool: built-in image_gen. User supplied two external-app screenshots as spatial references and explicitly asked to use current FileAtlas functionality.

## Scope

Static visual proposal, not an implemented UI. No application code, settings, database or fixture changes. Sample nature photos, tag names, file metadata and Chinese labels are demonstration content; proposed Chinese copy does not establish application localization. No compilation or runtime verification was performed.

## Reference adaptation and feature mapping

Borrow spatial organization: narrow sidebar, slim header/search/thumbnail control, compact filter dropdown row, broad file_manager and narrow collapsible inspector. Do not infer fixed GPUI values from reference pixels.

| Region | Existing FileAtlas capability | Proposed presentation |
|---|---|---|
| Sidebar | Registered folders, lazy hierarchy, tag hierarchy/search/actions and settings | Compact tree sections; tag search/mode on demand |
| Header | Library context, search/scopes, continuous thumbnail80–300, tools | Slim breadcrumb, slider, search and menu |
| Filter row | Media type, include/exclude tags/extensions, inclusive date bounds, rating0–5, sort/month groups | Compact labeled dropdown entries |
| FileManager | Square responsive tiles, image/video/audio, grouping, selection and cached thumbnails | Full central surface; slim filename/dimensions below tiles |
| Inspector | Selected details, metadata/EXIF, rating, tags, filesystem fields | Core details and tags expanded; EXIF/path collapsed |
| Context/Tools | Preview, AI tags, open/reveal, copy/move/delete, bulk actions, statistics, metadata export | Secondary menus and contextual footer |

Exclude reference-app features lacking an established FileAtlas mapping: smart folders, recent/random views, community, separate recycle-bin browser, color/shape/size filters, description editing, visual search, folder export, password/security settings and reference-specific folder-cover grid. FileAtlas's confirmed recycle-bin deletion remains in file actions; this does not imply a recycle-bin browsing view.

## Source review and deliberate differences

Current docs/source-feature-audit.md was re-read for folder/tag navigation, file_manager/filter, selected metadata, rating, menu and preview capability/status. Previous passes reviewed docs/reference-ui/source-layout-spec.md and pinned theme.rs:184-209, main.rs:1020-1097, gallery.rs:1383-1429 under vendor/photag-source/src/.

Source has two command rows, source-sized square tiles with radius7/gap12 and metadata overlays inside the image; this proposal explicitly changes captions to below tiles and removes the outer elevated FileManager card to match the user's supplied layout. Sidebar uses both sections rather than source pivot tabs. Source theme values/positions are reference rules, not silently rewritten by this concept. Actual implementation requires deliberate updated geometry, layout ownership and audit/validation records; generated pixels are not specifications.

Ownership: Shell composition only; FileManager filters/content/selected details; Navigation tree controls; Settings settings; UI Bridge shared tokens and lifecycle-owned registrations. Preserve Cordis contracts. Native/interactive behavior remains subject to validation.

## Generated-image review

Output: fileatlas-reference-layout-v5-2026-10-13.png. Nine items, compact toolbar, trees, caption metadata and collapsible inspector sections are represented. Image generation produced landscape slots rather than source square targets, visibly marks both All files and the folder as selected, adds a breadcrumb home icon, and contains minor Chinese text imperfections. These are concept artifacts: actual navigation must have one consistent selection; an unsupported home command must be omitted; use defined square geometry unless deliberately changing it and verified Chinese copy. Sample durations and image metadata do not represent user-library data. Hidden popovers, tool menus and real collapse behavior remain unimplemented.

## Final generation prompt

undefined
