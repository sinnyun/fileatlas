# WorkCrate UI plugin ports

## Cordis integration

Plugin `fileatlas.workcrate-ui` retains command `workcrate/open`, panel
`workcrate/overview`, sidebar launcher and `workcrate/settings` field preset page.
Dependencies are Projects, Boards, Export, Thumbnails, FileOperations, Settings
and UiBridge. Cordis owns registrations and listener lifetime. Projects owns
project/reference/preset storage; Boards owns official application configuration,
native document binding and preview cache. UI owns no parallel document format.

## Native workflow

Projects show title/cover cards, search, name/creation sorting and archived filter.
Creation, rename, cover, duplicate, archive/restore and confirmed deletion use the
Projects service. Native document duplication is performed by Projects once.
Project listing decodes explicit covers only and never opens native documents.

Selected projects show their native artboard matrix, actual point dimensions and
official previews. Associate an existing `.vectorcraft` file or create an A3
landscape artboard. Existing artboard dimensions remain unchanged. Multi-select,
all/clear/exit, ordered image export and all-artboard PDF use Export. Native file
pickers choose programs, documents, placement inputs and export destinations.

Third level follows `old/src/components/AssemblyStudioView.tsx`: left page rail,
center passive native PNG preview, right material information. The October20
projects/pages/editor concept images were also inspected. GPUI Kit/theme roles
own controls and colors. Complete editing is handed to official VectorCraft via
`open_editor`; WorkCrate offers refresh, native save and arbitrary design-file
placement. There are no synthetic transform, crop or text editors.

Actual width/height are read-only. Title, size annotation, project override with
inherit/reset, material, size, quantity and extras use native PageInfo/header
updates. Stable input entities hold drafts; an explicit save commits. Existing
field-preset settings and native dropdown menus remain available.

Program selection uses a native EXE picker, stored configuration and port input.
Connect reports errors visibly. No developer-specific executable is embedded.
Unconfigured users can manage projects and see setup instructions.

## Background work and UI inspection

Catalog/cover reads use keyed ReadModel workers, completion Wake/listen and
cached snapshots. Preview decoding uses one sequential ReadModel queue; generation
keys reject results after navigation. Jobs use background_executor and busy gating;
Boards serializes native transactions. Render projects cached snapshots only.
ChangeGate publishes `workcrate.ui` only when its serialized state changes.

Snapshot contains screen (`projects`, `pages`, `editor`), project_id, board_id,
busy, page_count, boards (id/native_id/index/title/dimensions/info/preview_ready/
error), previews_ready, preview_pending, selection, connection, executable, port,
document and message. View drop clears it. Existing tracking measures controls
under `workcrate.control.<id>`, including artboard, inspector, page previews,
project actions, material fields, `field-edit-input` and `field-edit-save`.

## MCP

Tools are lifecycle-owned by the global McpToolRegistry. Standard control tools:
`navigate`, `open_board`, `manage_project`, `create_page`, `update_page`,
`place_image` (now arbitrary official supported design file), `field_presets`,
`configure_vectorcraft`, `connect_vectorcraft`, `attach_document`, `open_editor`.
Read-only `state` reads cached UI/local project metadata; navigate/open_board
refresh the explicitly selected native catalog. Full/per-call confirm tools:
`delete_project`, `export_pdf`, `export_pages`, `save_project`. See src/mcp.rs and
src/mcp/{operations,editing}.rs for exact JSON schemas. Native service calls run
through blocking workers. Obsolete synthetic text/typography/link/replace tools
are removed, and no unrestricted raw official commands are exposed.

## Limits and verification

Official editor must be available on the configured loopback port. Native
connection/document/import/export errors remain visible in the panel. Multi-project
PDF is limited by official export and refused by Export. External window editing
requires explicit refresh; there is no embedded second editor. Native dialogs and
layout acceptance require same-process FileAtlas MCP validation; compilation is
not runtime evidence. Root validation owns those screenshots and flow results.
