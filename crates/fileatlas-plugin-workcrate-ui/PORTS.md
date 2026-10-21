# WorkCrate UI plugin ports

## Cordis integration

Plugin `fileatlas.workcrate-ui` retains command `workcrate/open`, panel
`workcrate/overview`, sidebar launcher and `workcrate/settings` WorkCrate page.
Dependencies are Projects, Boards, Export, Thumbnails, FileOperations, Settings
and UiBridge. Cordis owns registrations and listener lifetime. Projects owns
project/reference/preset storage; Boards owns official application configuration,
native document binding and preview cache. UI owns no parallel document format.

## Native workflow

Projects show title/cover cards, search, name/creation sorting and archived filter.
Creation first asks for the user-owned native document folder, then creates the
internal project record and official `.vectorcraft` document with an initial
artboard. If native creation fails, UI removes the incomplete project record and
leaves the user folder untouched. Duplicate also asks for a destination folder,
then copies the native document and internal project metadata before refreshing
the duplicate's board/preview cache. Rename, cover, archive/restore and confirmed
deletion use the Projects service. Project listing decodes explicit covers only
and never opens native documents.

Project and artboard browsing reads the local project registry, cached artboard
catalogue and cached preview files. Opening a project or page never starts
VectorCraft, its CLI or a port connection. Missing local previews show a sync
prompt. The explicit “同步工程与预览” action calls `refresh_project` in a
background worker and reloads the resulting local cache. Associate an existing
`.vectorcraft` file or create an A3 landscape artboard. Existing artboard
dimensions remain unchanged. Multi-select, all/clear/exit, ordered image export
and all-artboard PDF use Export. Native file pickers choose existing documents,
placement inputs and export destinations.

Third level follows `old/src/components/AssemblyStudioView.tsx`: left page rail,
center passive native PNG preview, right material information. The October20
projects/pages/editor concept images were also inspected. GPUI Kit/theme roles
own controls and colors. Complete editing is handed to official VectorCraft via
`open_editor`; WorkCrate offers explicit project sync and arbitrary supported
design-file placement. There are no synthetic transform, crop or text editors.

Actual width/height are read-only. Title, size annotation, project override with
inherit/reset, material, size, quantity and extras use native PageInfo/header
updates. Stable input entities hold drafts; an explicit save commits. Existing
field-preset settings and native dropdown menus remain available.

The WorkCrate settings page contains the VectorCraft executable path and local
control port alongside existing field presets. “保存程序设置” persists through
`Boards.configure`; it does not connect to a port or launch either program.
Background CLI actions use the configured executable and same-directory CLI
discovery. Only “在 VectorCraft 中编辑” opens the official GUI.

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
project actions, material fields, `field-edit-input`, `field-edit-save`,
`settings-vectorcraft-executable`, `settings-vectorcraft-port`,
`settings-vectorcraft-save` and `settings-vectorcraft-browse`.

## MCP

Tools are lifecycle-owned by the global McpToolRegistry. Standard control tools:
`navigate`, `manage_project`, `create_page`, `update_page`,
`place_image` (now arbitrary official supported design file), `field_presets`,
`configure_vectorcraft`, `refresh_project`, `attach_document`, `open_editor`.
Project create and duplicate actions require a user-chosen native document folder.
Configuration only persists the executable/CLI/port values; browsing does not
connect or start VectorCraft. `navigate` and `state` read cached local project and
board data. Only explicit `refresh_project` synchronizes a saved native document
and its previews. Full/per-call confirm tools include `delete_project`, `export_pdf`,
`export_pages`. Save canvas edits in the official editor, then use
`refresh_project` to synchronize the local catalogue and previews. See src/mcp.rs and src/mcp/{operations,editing}.rs
for exact JSON schemas. Native service calls run through blocking workers.
Obsolete synthetic text/typography/link/replace tools are removed, and no
unrestricted raw official commands are exposed.

## Limits and verification

The official CLI must be available for native creation, synchronization, edits and
exports; only `open_editor` launches the GUI. Cached projects and previews remain
browsable while the official programs are unavailable. Multi-project PDF is
limited by official export and refused by Export. External window editing requires
explicit sync; there is no embedded second editor. Native dialogs and layout
acceptance require same-process FileAtlas MCP validation; compilation is not
runtime evidence. Root validation owns those screenshots and flow results.
