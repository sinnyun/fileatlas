# Shell composition ports / Shell 对接

`photag-shell` is the host composition root. It publishes `UiBridgeService`, starts Cordis plugins in dependency order, mounts GPUI view factories into host regions, and dispatches command IDs as Cordis events.

## Current runtime dependency order

Database → FileOperations → DragDrop → Gallery; Database → Preview → Gallery; Database → MediaPlayback → Gallery; Database → Thumbnails → Indexing → Watcher; Navigation depends on Database/Indexing and optionally Watcher; Settings optionally reads Navigation; Tagging depends on Database; History starts after Database, FileOperations, and Tagging. Shell starts Gallery after all required providers and the optional Settings/Tagging/History providers. On shutdown it stops Gallery and History before those services, then stops Navigation before Watcher, Indexing, and Thumbnails. Preserve service availability before dependent plugin startup and stop consumers before providers. Gallery opens images in Preview, routes videos to MediaPlayback, calls DragDrop from card drag gestures, and optionally consumes Settings, Tagging, and History services.

## Boundaries

The shell must not absorb scanning, SQL, filtering, tagging, or file operations. Stop plugin fibers before dropping shared runtime resources so Cordis cleanup can remove published services and UI contributions. Current shell and view host remain partial; see `docs/source-feature-audit.md` for unmigrated UI and behavior.
