# Notes plugin same-process validation — 2026-10-27

## Environment and commands

- Windows host; Rust `1.97.1-x86_64-pc-windows-msvc`.
- Isolated profile: `.local/validation/profiles/notes-2026-10-28-completion2`.
- Build: `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`.
- Tests: `cargo +1.97.1 test --locked -p fileatlas-plugin-notes -j 2`.
- Runtime: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --plugins notes --profile .local/validation/profiles/notes-2026-10-28-completion2 --access-level full --scenario scripts/validate-notes-mcp.cjs`.
- MCP launched `fileatlas-shell --mcp` as an isolated child and controlled its own GPUI window. High-impact trash operations used Full access and `confirm=true`; image input came from `test-data/fileatlas-fixtures`.
- Screenshots and report stay local under the ignored profile: `screenshots/notes-panel.png`, `screenshots/notes-preview.png`, `screenshots/notes-navigation.png`, `screenshots/notes-trash-panel.png`, `screenshots/notes-narrow-window.png`, and `report.json`.

## Results

- Passed: nested folder creation; note create/list/read/search/move/rename; category, status and tags; `[[wiki links]]`, backlinks and rename propagation; local image insertion and Markdown preview; heading outline; note/folder trash, listing, restore and permanent deletion.
- Passed: trash deletion removed the deleted note's image asset directory. Unit coverage also verifies restore of a whole nested folder and its note contents.
- Passed: Notes navigation, sidebar search and folder controls, panel preview (including save), and the populated trash page with restore and permanent-delete actions were exercised or captured in the same-process MCP window. The preview capture showed the new note status as `draft` and reported it saved. Formatting/find-replace controls were visible in the editor screenshot. Unit tests cover inline selection wrapping, line-prefix toggles, task completion toggles, replacement counts, nested directory restore, and resource cleanup.
- The panel is rebuilt when navigation selects another note or opens trash, preventing a stale single-instance view. The panel also has in-place trash and return-to-notes controls.
- The concept reference is [the Notes concept and checklist](../../reference-ui/concepts/fileatlas-notes-2026-10-27.md), with its image in the same directory. The panel retains the three-column note list/editor/properties structure. A 960×720 same-process capture showed the populated trash row and its restore/delete actions within the resized panel. The formatting toolbar was checked at the normal window size. Screenshots include temporary geometry and typography audit overlays, so they verify structure and controls but do not establish final color or typography acceptance.

## Remaining limits

- Editing uses GPUI Kit's rope-backed Textarea and its UTF-8 selection/editing API. Toolbar formatting uses the live selection and retains undo history. The adapted pinned Aster code supplies syntax span scanning only; its application editor targets incompatible GPUI versions. Live preview alongside editing remains unimplemented.
- The Markdown model is not a full CommonMark AST. Wiki links resolve by exact title and duplicate titles are ambiguous. Rename rewrites exact `[[old title]]` tokens.
- The 960×720 capture checked the trash view; narrow-window editor controls remain unverified. Large-vault performance, external concurrent edits, and interoperability with external Markdown editors remain unverified. The image preview scenario covered a PNG fixture; other image formats remain unverified.
- The Aster README declares MIT but its pinned tree has no LICENSE file. Provenance and this limitation are documented in `crates/fileatlas-plugin-notes/UPSTREAM.md`.

## 2026-10-28 live split preview follow-up

- Build and test: `cargo +1.97.1 fmt --all -- --check`; `cargo +1.97.1 test --locked -p fileatlas-plugin-notes -j 2` (7/7); `cargo +1.97.1 build --locked -p fileatlas-shell -j 2` (passed; existing MSVC `LNK4098` default-library warning).
- Governance: `node scripts/check-ui-refresh.cjs`, `node scripts/check-architecture.cjs`, `node scripts/check-feature-ledger.cjs`, and `node scripts/check-ui-theme.cjs` all passed.
- Runtime: `node scripts/fileatlas-mcp-debug.cjs --exe target/debug/fileatlas-shell.exe --profile .local/validation/profiles/notes-live-preview-2026-10-28-rerun-full --plugins core --access-level full --scenario scripts/validate-notes-mcp.cjs`. The MCP server and GPUI window ran in the same isolated child process.
- Passed: pointer focus of the editor, same-process text input, dirty state, split mode, and unsaved text arriving in the preview; the full scenario also passed note metadata, wiki-link rename/backlinks, image insertion, navigation, trash/restore/permanent deletion, and 960×720 resize flows. Screenshots and `report.json` are local under `.local/validation/profiles/notes-live-preview-2026-10-28-rerun-full/`.
- The preview refresh is coalesced in a background task and drops stale results by generation. This live run verifies one PNG fixture and one normal-size editing layout; other image formats, large vault performance, external concurrent edits, and small-window editing ergonomics remain unverified. Screenshots carry the temporary audit overlays and do not establish final typography or color acceptance.
