# Projects plugin

Status: initial feature implementation. It provides a lifecycle-owned Navigation sidebar page for project roots and their recursive folder trees.

## Cordis and UI

- Plugin identity: `photag.projects`; service: `photag/projects` (`ProjectsService`).
- Requires `photag/navigation` and `photag/ui-bridge`.
- Registers its executable view in the `navigation.sidebar-projects` UI slot. Navigation displays the Projects rail tab only while this lifecycle-owned contribution exists; Cordis removes the view on plugin stop.
- The Projects page plus button adds the directory currently shown by Navigation's address bar; it does not ask the user to type a path. Clicking a project directory dispatches `NavigationUiRequest::SelectPathPreservingPage`, so the active browser column navigates while the Projects page stays open.

## Data and behavior

- Stores a version 1 JSON document at `%LOCALAPPDATA%\PhoTag\projects\projects.json`; isolated dev-test profiles use `<profile>\projects\projects.json`.
- The document contains only project root paths and display names, so JSON is appropriate for its small, whole-document shape. Writes use a same-directory temporary file and `.bak` recovery copy. Unknown versions stop startup with an explicit error; there is no schema migration yet.
- Adding a path validates and canonicalizes an existing directory. Duplicate paths are ignored using case-insensitive Windows path identity. Removing a project only changes this list; it does not delete or move files.
- Opening the sidebar page recursively enumerates directory children on the shared `ReadModel` worker and displays them all as expanded rows under each root. Directory symlinks are not traversed to avoid loops. A failed directory read is surfaced as a tree error, while project roots remain listed so they can be removed.

## Limits and diagnostics

- Shell opens the shared database service in its filesystem-browsing mode. Navigation accepts any existing directory in that mode, and `DirectoryPathSelected` routes it to the currently active workspace column. Project roots do not need to be registered first.
- Very large trees are fully enumerated and retained in memory to satisfy the expanded-tree behavior; the initial version has no paging or collapse controls.
- Same-process MCP validation on 2026-10-19 covered page activation, navigating to a profile-local fixture and adding it with the page plus button, expansion through `docs/notes` and `src/deep`, and navigation to `src/deep` while keeping Projects active. See `docs/validation-ui/2026-10-19-project-folders/README.md` for the exact command, screenshot paths, and remaining UI coverage.
- Inspect `photag/projects` startup errors and Navigation request rejection logs when folder selection fails.
