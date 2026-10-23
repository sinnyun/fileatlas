# Project tree plugin

Status: initial feature implementation. This feature is named **Project tree** in the UI. It provides a lifecycle-owned Navigation sidebar page for project roots and their recursive folder trees. It is a directory navigation aid, not the separate project-management panel feature.

## Cordis and UI

- Plugin identity: `fileatlas.project-tree`; service: `fileatlas/project-tree` (`ProjectTreeService`).
- Requires `fileatlas/navigation` and `fileatlas/ui-bridge`.
- Registers its executable view in the `navigation.sidebar-project-tree` UI slot. Navigation displays the Project tree rail tab only while this lifecycle-owned contribution exists; Cordis removes the view on plugin stop.
- The Project tree page plus button adds the directory currently shown by Navigation's address bar; it does not ask the user to type a path. Clicking a project directory dispatches `NavigationUiRequest::SelectPathPreservingPage`, so the active browser column navigates while the Project tree page stays open.

## Data and behavior

- Stores a version 1 JSON document at `%LOCALAPPDATA%\FileAtlas\project-tree\project-tree.json`; isolated dev-test profiles use `<profile>\project-tree\project-tree.json`.
- The document contains only project root paths and display names in its `roots` field, so JSON is appropriate for its small, whole-document shape. Reads the old `%LOCALAPPDATA%\FileAtlas\projects\projects.json` path and legacy `projects` field, then writes the converted document at the new path. The old file is retained. Writes use a same-directory temporary file and `.bak` recovery copy. Unknown versions stop startup with an explicit error.
- Adding a path validates and canonicalizes an existing directory. Duplicate paths are ignored using case-insensitive Windows path identity. Removing a project only changes this list; it does not delete or move files.
- Opening the sidebar page recursively enumerates directory children on the shared `ReadModel` worker. The root containing the current browser directory (or the first root) starts expanded; only one project's subtree is shown at a time. Selecting another root navigates there and switches the expanded tree; its chevron independently collapses or expands it. Adding a project selects and expands that root immediately. Child rows use dotted ancestor rails and branch marks to show parent/child relationships. Directory symlinks are not traversed to avoid loops.
- Each root is scanned independently (`scan_project_tree` returns `ProjectScan`, never a batch error). An unreadable or deleted root records one `UnavailableRoot` for itself, marked with the short 不可用/Unavailable label on its row and a localized note with the plain display path when expanded; the other roots still render their full subtrees. `ErrorKind::NotFound` selects the "no longer exists" wording; any other OS error selects "cannot be read", and its raw message goes to `tracing::warn!` only. A child directory that fails to read is skipped with a warning instead of truncating the whole root. The scan never rewrites the JSON list.
- Windows paths are compared case-insensitively after slash normalization and removal of the extended `\\?\` prefix, so canonical project roots match ordinary Navigation paths. Roots keep the canonical `\\?\` form on disk for long-path IO; views show them through `fileatlas_ui_bridge::display_path`.

## Limits and diagnostics

- Shell opens the shared database service in its filesystem-browsing mode. Navigation accepts any existing directory in that mode, and `DirectoryPathSelected` routes it to the currently active workspace column. Project roots do not need to be registered first.
- Very large trees are fully enumerated and retained in memory; there is no paging, and only one tree is rendered expanded at a time.
- Same-process MCP validation on 2026-10-19 covered page activation, navigating to a profile-local fixture and adding it with the page plus button, expansion through `docs/notes` and `src/deep`, and navigation to `src/deep` while keeping Project tree active. See `docs/validation-ui/2026-10-19-project-folders/README.md` for the exact command, screenshot paths, and remaining UI coverage.
- Same-process MCP validation on 2026-10-20 covered adding two roots with automatic expansion, switching between roots, collapse, and dotted hierarchy rails. See `docs/validation-ui/2026-10-20-project-tree-switch/README.md`.
- Same-process MCP validation on 2026-10-23 covered a seeded list with one live and one deleted root: the live subtree stayed rendered, the dead root showed only the unavailable marker plus its plain-path note, and the JSON list was unchanged. See `docs/validation-ui/2026-10-23-project-tree-missing-root/README.md`.
- Inspect `fileatlas/project-tree` startup errors and Navigation request rejection logs when folder selection fails. Per-root scan failures are logged by the view worker as `project root scan failed` / `project subfolder skipped`.
