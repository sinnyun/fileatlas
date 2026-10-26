# Notes plugin same-process validation — 2026-10-27

## Environment and commands

- Windows host; Rust `1.97.1-x86_64-pc-windows-msvc`.
- Isolated profile: `.local/validation/profiles/notes-2026-10-27-final5`.
- Executable: workspace `target/debug/fileatlas-shell.exe`, built with
  `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`.
- Runtime: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --plugins notes --profile .local/validation/profiles/notes-2026-10-27-final5 --access-level full --scenario scripts/validate-notes-mcp.cjs`.
- MCP spawned `fileatlas-shell --mcp` as an isolated child and exercised its
  own GPUI window. High-impact trash used `confirm=true` in this Full-only
  profile. Sample image came from `test-data/fileatlas-fixtures`.
- Screenshot and JSON report remain local-only under the ignored profile:
  `screenshots/notes-panel.png`, `screenshots/notes-preview.png`,
  `screenshots/notes-navigation.png`, and `report.json`. The profile disabled
  UI geometry/typography audit overlays for visual inspection.

## Results

- Passed: nested folder creation; note creation/list/read/search and movement;
  category,
  status and tag update; title-based `[[wiki links]]` and backlinks; rename
  propagation; local image copy and Markdown preview; outline extraction;
  recoverable trash and restore.
- Passed: `navigation_tab` Notes entry via same-process pointer input; the
  sidebar listed the nested folder and its notes. The floating Notes panel
  displayed list, editor/preview, properties and backlink sections. The local
  PNG appeared in Markdown preview and stayed resolvable after its note moved
  into the nested folder.
- A UI layout defect found during the first capture was corrected: the panel
  used horizontal dividers inside a horizontal row, consuming its width. Both
  separators now use the UI Bridge vertical divider.
- A tool contract defect found during the first restore was corrected: `notes.trash`
  now returns the root-relative path accepted by `notes.restore`.

## Visual comparison and remaining scope

The active reference is [the Notes concept and checklist](../../reference-ui/concepts/fileatlas-notes-concept-2026-10-27.md),
with the concept image in the same directory. The verified panel follows its
three-column structure. The sidebar currently shows nested directory labels
and note rows but has no search/filter field, folder action menu, expansion
controls or trash view. The panel has no note move/trash controls, and it uses
a plain Markdown textarea rather than a formatting toolbar or split preview.
These are known product gaps; this is not a claim of complete parity or of a
complete general-purpose note application. Narrow-window behavior, external
concurrent edits, very large vault performance and non-PNG/JPEG image preview
remain unverified.
