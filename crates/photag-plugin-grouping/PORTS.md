# Grouping plugin ports

Cordis plugin `photag.grouping` publishes `photag/grouping` (`GroupingService`). Shell starts it before Gallery and stops it after Gallery. No parallel lifecycle manager, dynamic loading, database schema or new user data path. The mode metadata contribution `grouping/modes` is owned by the Cordis UI bridge; disposal removes both it and the service. Gallery supplies the existing native filter popover and Tools choice-list adapters and consumes the public modes/arrangement/rows contract.

## Inputs and outputs

- `GroupingMode::{None, Month, FileType, Modified, Created, Folder, Subfolder}`; `key/parse/label/ALL/is_date` expose stable strings, menu choices and date semantics. Unknown saved mode falls back to the old setting.
- `GroupingService::arrange(&self, &mut Vec<MediaSkeleton>, &[MediaGroupingFields], GroupingMode, SortOrder) -> Vec<Option<String>>` returns group labels aligned with the reordered skeletons. Stable grouping retains the query order inside each group. Date groups order newest first, or oldest first for OldestFirst; unavailable dates remain last. Subfolder puts direct current-directory groups first, then descendant paths alphabetically; other groups order alphabetically. None retains query order.
- Month uses the original `COALESCE(date_taken, modified_at)` timestamp. Modified uses the indexed `modified_at`; Created reads filesystem `metadata.created()` and never substitutes capture/modification time. All date modes use local calendar month `YYYY-MM`. Missing/offline/unavailable creation timestamps become Unknown date. Creation metadata is read on grouping/reload, never during each card frame; large/offline filesystems can delay this synchronous operation and are not performance-certified.
- FileType groups case-insensitive uppercase extensions; no extension has a dedicated group. This does not imply preview/decoder support.
- Folder groups by registered database root path; Subfolder groups by the full immediate parent path, including files directly in the root. Full paths disambiguate repeated directory names; headers ellipsize long text.
- `GroupingService::rows(labels, columns, collapsed) -> Vec<GroupRow>` calculates virtual sections, counts, column boundaries and collapsed ranges. GroupRow retains legacy month-named rendering fields for Gallery compatibility but carries any grouping mode.
- `restore_collapsed(keys, mode)` maps legacy English month keys to YYYY-MM for old sessions. Gallery uses existing session storage with additive optional `grouping_mode`; old `group_by_month`, settings `show_date_dividers` and collapse fields remain readable. Switching modes clears the previous mode's collapse set; current mode/collapses restore on restart.

## Consumers, events and failures

Gallery obtains compact filtered inputs through `DatabaseService` / `queries::get_media_grouping_fields`, passes them to this service, and fetches rich records by arranged IDs so the 200-row cache, selection and Preview stay aligned. Existing MediaIndexChanged refreshes the snapshot; this plugin adds no events and writes no files. Gallery reports database/dependency errors through its existing error path. Unknown creation dates and extensionless files are explicit sections, not errors.

MCP uses GalleryUiRequest::Grouping/ToggleGroup and the original MonthGrouping compatibility action. get_ui_state publishes mode, ordered [label,count] groups, collapsed_groups and window counts from the live Gallery. Mutations require the existing MCP guard. Lifecycle and grouping/window/filter tests plus isolated Windows MCP evidence are in docs/validation-ui/2026-10-14-grouping/README.md.

## Current-directory priority (2026-10-14)

`arrange_in_scope(&self, &mut Vec<MediaSkeleton>, &[MediaGroupingFields], GroupingMode, SortOrder, Option<&str>) -> Vec<Option<String>>` accepts the active directory. `arrange` delegates with None for compatibility. Subfolder ranks the active directory's direct file group first; without a scope, direct groups of all registered roots precede descendant groups. Missing direct files do not produce an empty group. Group keys/labels stay actual parent paths; lexical Windows path normalization is only for rank comparison (no filesystem IO). Within each group, stable query order is retained. Gallery passes MediaQuery.path_prefix. See docs/validation-ui/2026-10-14-root-first for Windows evidence and limits.

Gallery's None consumer now supplies a native immediate-directory browser projection with folder cards; this does not change GroupingService::arrange/rows ordering contracts or source recursive database APIs. Grouped modes still arrange recursive matching files. See Gallery PORTS and docs/validation-ui/2026-10-14-folder-browser.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.
