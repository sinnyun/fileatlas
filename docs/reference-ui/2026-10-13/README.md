# PhoTag v0.2.0 runtime reference screenshots

These PNGs are committed reference artifacts for the local UI migration
comparison. They were captured from the original PhoTag v0.2.0 source in
`vendor/photag-source/` on 2026-10-13, using an isolated runtime profile and
the generated 9-item fixture library. The screenshots are runtime evidence for
the UI states below; they are not product assets.

| Files | Captured state |
|---|---|
| `reference-main.png`, `reference-main-isolated.png`, `reference-printwindow.png` | Main library window / isolated initial state captures |
| `reference-file-manager-window.png`, `reference-after-open.png`, `reference-month-toggle.png`, `reference-month-toggle2.png`, `reference-space-preview.png` | Library file_manager and alternate interaction captures |
| `reference-tags.png`, `reference-tags-tab.png`, `reference-tag-input.png`, `reference-tag-created.png`, `reference-create-tag-dialog.png` | Tag navigation and tag creation states |
| `reference-selected-media.png`, `reference-preview.png` | Media selection and preview |
| `reference-settings.png`, `reference-appearance.png`, `reference-shortcuts.png`, `reference-ai.png`, `reference-updates.png` | Settings tabs |
| `reference-after-plus.png`, `reference-plus-state.png`, `reference-tools.png` | Thumbnail-size and Tools states |

Some files capture repeated or intermediate states and are retained as
captured. Pair them with `docs/validation-report.md` for isolation details,
migration screenshots, and the current comparison limits.
