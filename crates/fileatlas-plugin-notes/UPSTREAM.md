# Markdown editor upstream

Reference project: [Aster](https://github.com/kumarUjjawal/aster), by
kumarUjjawal. Its README describes an MIT-licensed Rust/GPUI inline Markdown
editor with rope-backed editing, background span parsing, local image display,
find, and document outline.

The pinned reference is commit `8f1a05b873a268461c10f3a3c96265f997cc5eb8`.
Its README says MIT but the repository tree at that commit contains no `LICENSE`
file; retain this provenance and the README claim with the adapted Markdown span
scanner. The source targets `gpui = 0.2.2` and `gpui-component = 0.4.0`, while
FileAtlas uses GPUI Kit 0.7.1 and GPUI pre 0.3.8. Only the self-contained
`src/services/syntax.rs` scanner is adapted; Aster's application, UI and theme
are not copied. Editing uses GPUI Kit's pinned `TextareaState`, whose GPUI Base
editor is rope-backed and exposes selection-aware edits, cursor positioning and
undo history. That API supports the Notes formatting commands while matching
the workspace's GPUI runtime. Confirm the upstream license file before
redistributing a combined application.
