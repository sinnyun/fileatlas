# FileAtlas Notes plugin

Status: Markdown-first note management and trash UI are implemented; same-process MCP evidence and remaining editor gaps are recorded in `docs/validation-ui/2026-10-27-notes/README.md`.

- Cordis identity: `fileatlas.notes`; service: `fileatlas/notes`.
- Storage: canonical Markdown files under `%LOCALAPPDATA%\\FileAtlas\\notes`;
  each note is a `.md` file inside a recursively nested folder tree. Inline
  JSON frontmatter (valid YAML subset) carries stable ID, category, status and
  tags. Images are copied to `assets/<note-id>/` and referenced relatively.
- UI Bridge owns the Notes navigation contribution and the single Notes panel.
  Cordis removes both contributions and service when the plugin stops.
- MCP tools are registered through the host `McpToolRegistry` and removed with
  the plugin. All file writes remain scoped to the configured notes root.
- Reads/writes are synchronous service calls; the sidebar uses the shared
  `ReadModel` and filesystem watcher revision to keep directory enumeration off
  render. Watch events are coarse invalidation signals, not durable change logs.
- Wiki links use `[[note title]]`; backlinks are derived from note contents.
- Note metadata uses `id`, `category`, `status` (`draft`, `in_progress`,
  `completed`, `archived`) and case-insensitively deduplicated tags. Missing
  or legacy-empty status values are read as `draft`. Search
  matches titles, tags and Markdown body text. Heading outline is available.
- `notes.move` moves notes between nested folders while retaining stable IDs.
  `notes.create_folder`, `notes.rename_folder`, `notes.move_folder` and
  `notes.trash_folder` manage arbitrarily nested folders. `notes.trash` and
  `notes.restore` move Markdown files through `.trash/`; image assets are stored
  below `assets/<note-id>/`. The panel includes a trash browser, note/folder
  restore and confirmed permanent deletion; permanent deletion also removes
  image assets owned by deleted notes. Folder operations are available in the
  sidebar. New notes inherit the currently selected folder.
- `notes.insert_image` copies a local image and returns a relative Markdown
  image reference. Preview embeds root-contained local image files as data URIs
  for GPUI Kit Markdown rendering. External images and absolute paths are not
  embedded.
- The panel provides create, edit, save, single preview and a live split
  editor/preview, metadata, backlinks, image insertion, cursor/selection-aware
  heading, bold, italic, code, link, list and task-list formatting with task
  completion toggling, plus find/replace-all, note move/trash and trash
  restore/delete. Split preview rendering runs as a coalesced background task;
  stale results are discarded when the note or draft changes. The sidebar
  provides nested folder create/rename/move/trash and note search.
- MCP names: `notes.list`, `notes.search`, `notes.read`, `notes.create`,
`notes.ui_state`,
  `notes.update`, `notes.rename`, `notes.move`, `notes.create_folder`, `notes.rename_folder`,
  `notes.move_folder`, `notes.trash_folder`, `notes.trash`, `notes.restore`,
  `notes.trash_list`, `notes.restore_folder`, `notes.delete_trash`,
  `notes.insert_image`, and `notes.outline`. Trash tools require Full AI
  control plus per-call `confirm=true`. Trash paths returned by MCP are
  root-relative and accepted by `notes.restore`.
- `notes.ui_state` is a read-only same-process diagnostic for the mounted panel.
  It reports selected path, edit/preview/split mode, unsaved state, text lengths
  and bounded 512-character editor/preview samples. Closing the panel clears it.
- Known limits: wiki links resolve by exact title and duplicate titles are
  ambiguous; Markdown references are not a full CommonMark AST and rename only
  rewrites exact `[[old title]]` tokens. The adapted Aster scanner only colors
  syntax spans; editing is a GPUI Kit Textarea, not Aster's rope editor.
- Failure inspection: `NotesError`, plugin startup logs, MCP tool results and
  the profile's `stderr.log`. The watcher invalidates on file changes but does
  not provide conflict resolution for simultaneous external edits.
- Aster (kumarUjjawal/aster) is the editor behavior reference. The project
  README declares MIT. Any copied source must retain upstream notices and the
  exact pinned revision; compatibility work is documented in `UPSTREAM.md`.
- External file changes, parser recovery, and live UI behavior are tracked in
  `docs/validation-ui/` after same-process MCP verification.
