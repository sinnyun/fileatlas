# Projects plugin

Status: initial feature implementation. It provides a lifecycle-owned Navigation sidebar page for project roots and their recursive folder trees.

## Cordis and UI

- Plugin identity: `fileatlas.projects`; service: `fileatlas/projects` (`ProjectsService`).
- Requires `fileatlas/navigation` and `fileatlas/ui-bridge`.
- Registers its executable view in the `navigation.sidebar-projects` UI slot. Navigation displays the Projects rail tab only while this lifecycle-owned contribution exists; Cordis removes the view on plugin stop.
- The Projects page plus button adds the directory currently shown by Navigation's address bar; it does not ask the user to type a path. Clicking a project directory dispatches `NavigationUiRequest::SelectPathPreservingPage`, so the active browser column navigates while the Projects page stays open.

## Data and behavior

- Stores a version 1 JSON document at `%LOCALAPPDATA%\FileAtlas\projects\projects.json`; isolated dev-test profiles use `<profile>\projects\projects.json`.
- The document contains only project root paths and display names, so JSON is appropriate for its small, whole-document shape. Writes use a same-directory temporary file and `.bak` recovery copy. Unknown versions stop startup with an explicit error; there is no schema migration yet.
- Adding a path validates and canonicalizes an existing directory. Duplicate paths are ignored using case-insensitive Windows path identity. Removing a project only changes this list; it does not delete or move files.
- Opening the sidebar page recursively enumerates directory children on the shared `ReadModel` worker. The root containing the current browser directory (or the first root) starts expanded; only one project's subtree is shown at a time. Selecting another root navigates there and switches the expanded tree; its chevron independently collapses or expands it. Adding a project selects and expands that root immediately. Child rows use dotted ancestor rails and branch marks to show parent/child relationships. Directory symlinks are not traversed to avoid loops. A failed directory read is surfaced as a tree error, while project roots remain listed so they can be removed.
- Windows paths are compared case-insensitively after slash normalization and removal of the extended `\\?\` prefix, so canonical project roots match ordinary Navigation paths.

## Limits and diagnostics

- Shell opens the shared database service in its filesystem-browsing mode. Navigation accepts any existing directory in that mode, and `DirectoryPathSelected` routes it to the currently active workspace column. Project roots do not need to be registered first.
- Very large trees are fully enumerated and retained in memory; there is no paging, and only one tree is rendered expanded at a time.
- Same-process MCP validation on 2026-10-19 covered page activation, navigating to a profile-local fixture and adding it with the page plus button, expansion through `docs/notes` and `src/deep`, and navigation to `src/deep` while keeping Projects active. See `docs/validation-ui/2026-10-19-project-folders/README.md` for the exact command, screenshot paths, and remaining UI coverage.
- Same-process MCP validation on 2026-10-20 covered adding two roots with automatic expansion, switching between roots, collapse, and dotted hierarchy rails. See `docs/validation-ui/2026-10-20-project-tree-switch/README.md`.
- Inspect `fileatlas/projects` startup errors and Navigation request rejection logs when folder selection fails.
