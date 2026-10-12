# Shell composition ports / Shell 对接

`photag-shell` is the host composition root. It publishes `UiBridgeService`, starts Cordis plugins in dependency order, mounts GPUI view factories into host regions, and dispatches command IDs as Cordis events.

## Current runtime dependency order

Database → Preview → Gallery; Database → FileOperations → DragDrop → Gallery; Database → Thumbnails → Indexing → Watcher; Navigation depends on Database/Indexing and optionally Watcher; Tagging depends on Database; History starts after Database, FileOperations, and Tagging. MediaPlayback is independent. Preserve service availability before dependent plugin startup. Preview is started with Gallery and its UI opens selected images from the gallery; media playback and drag-drop services are registered but their Gallery gestures are not yet connected.

## Boundaries

The shell must not absorb scanning, SQL, filtering, tagging, or file operations. Stop plugin fibers before dropping shared runtime resources so Cordis cleanup can remove published services and UI contributions. Current shell and view host remain partial; see `docs/source-feature-audit.md` for unmigrated UI and behavior.
