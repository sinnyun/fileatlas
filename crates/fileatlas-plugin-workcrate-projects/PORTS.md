# WorkCrate Projects ports

Cordis plugin/service and project CRUD, reference registry, preview and field presets retain their existing public APIs. The version-1 feature-root/projects.json is a recent-project registry and locator catalogue. Legacy projects retain projects/<uuid>/project.json until explicitly migrated. New native projects keep authoritative project data beside their native file at `<document folder>/.workcrate/<project UUID>/project.json`, plus owned previews and Boards cache in that directory. Atomic writes retain existing interrupted-write json.bak recovery. This modest metadata registry intentionally uses versioned JSON, not a duplicate database service.

New `document_path(project_id) -> Result<Option<PathBuf>,WorkCrateProjectsError>` and `set_document_path(project_id,&Path) -> Result<(),...>` associate one canonical native document path. Association stores no artboard design copy. Boards owns native extension/open validation. `duplicate_project_document(id,name,directory)` copies the native file without overwriting into the explicitly selected directory, carries FileAtlas-owned project metadata/previews, and rotates the new project's association binding. The external source is unchanged. New native project creation flows should ask for a folder and call Boards `create_project_document`; the older `duplicate_project` remains for existing callers. Delete retains feature-owned data in trash and never removes external original files.

References remain original files, custom previews remain feature-owned; presets keep existing validation/storage. Portable manifests store in-folder reference paths and the native path relative to the document folder; public document_path/references still resolve absolute paths for existing callers. External references remain external; no source image is copied or rewritten, and VectorCraft retains ownership of native links. Service revision and shared refresh Wake announce changes. No new events or infrastructure; errors expose IO/JSON/validation/dependency failures. No legacy board-JSON migration is implemented because the user authorized replacement.

## 2026-10-23 adjacent project storage

`portable_folder(id) -> Result<Option<PathBuf>, WorkCrateProjectsError>` reads the memory locator;
`relocate_project_data(id) -> Result<PathBuf, ...>` copies the complete legacy dataset into an
adjacent staging directory, writes a portable version-1 manifest, atomically renames it into
`.workcrate/<id>`, then publishes the locator in projects.json. The old feature-root dataset is
retained for recovery; subsequent writes use the adjacent copy. Existing target data is never
overwritten. UUID subdirectories isolate multiple native projects in one folder.
ProjectDocument adds defaulted portable/project members, ProjectIndex adds defaulted locations;
old version-1 JSON remains readable. The portable project snapshot retains name, creation time,
archive state and cover; the existing document retains binding, references/previews and presets.
Program presets and executable/port settings remain application-wide.

`open_project_folder(&Path) -> Result<Vec<WorkCrateProject>, ...>` reads all UUID datasets in
that folder's .workcrate directory, validates identities/versions/relative paths/native files/
presets and name conflicts before publishing the registry update, preserving original IDs and
bindings. It also works with a fresh application profile. A missing relocated folder never
removes its recent-project entry or prevents startup; edits require reopening its new location.
Normal project changes update the adjacent project snapshot. Delete removes registration,
copies metadata to existing trash and leaves native files/images/adjacent data for recovery.
Duplicate creates an adjacent dataset and new binding; explicit native reassociation outside
the old folder copies metadata beside the new association. Source data/documents survive.
Directory copying skips filesystem links. Failures surface through errors, UI and MCP feedback.


## 2026-10-23 project default information

`project_field_presets(&str) -> Result<FieldPresets, WorkCrateProjectsError>` is a memory-only snapshot validated against the live project index. `save_project_field_presets(&str, FieldPresets) -> Result<(), ...>` writes an optional `field_presets` member into the existing version-1 project manifest (legacy `projects/<uuid>/project.json`, adjacent `.workcrate/<uuid>/project.json`); old manifests have no member and start with empty builtin columns. Startup hydrates the memory catalogue; duplicate preserves project presets via the copied ProjectDocument, delete retains them in existing trash. Atomic JSON publication and `.json.bak` recovery are reused; invalid/unsupported presets remain visible errors. No SQLite, binary assets or new settings framework.

`effective_field_presets(&str) -> Result<FieldPresets, ...>` returns project-first union with program defaults by stable field key, deduplicating values. `promote_project_field(&str, &str) -> Result<(), ...>` merges only the requested saved project column into `field-presets.json` under the existing write gate; unrelated global columns remain identical. Empty or unknown columns, duplicate/empty/oversized values, unsupported versions and missing projects are refused before publication. Saves/promotions increment existing revision and Wake. Global `local` label normalizes from historical 项目名称 to 物料类别 on read; stable key stays compatible. Defaults never fill or rewrite page values automatically.


## 2026-10-23 adjacent dataset folders

A project's adjacent `.workcrate/<uuid>` folder holds `project.json`, `boards-cache.json`,
`previews/` (covers and custom preview images), `native-previews/` (the one cached frame per
page, owned and reclaimed by Boards) and `cli-config/`. `copy_directory` carries the whole
folder for duplicate, migration and relocation, then drops the `canvas/` render mirror older
releases wrote beside each cached frame; the source dataset is never rewritten by that copy.
