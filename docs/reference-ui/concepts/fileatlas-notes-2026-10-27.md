# FileAtlas Notes concept and implementation checklist

Concept image: [fileatlas-notes-concept-2026-10-27.png](fileatlas-notes-concept-2026-10-27.png).
Generated 2026-10-27 to guide the initial Notes plugin implementation. Adapt the
surface colors through the active GPUI Kit theme; the image's light palette does
not override the user's current theme.

## Layout checklist

- Keep the global 48 px navigation rail. Notes is a sibling of Folders, Tags,
  Project tree and Add-ons. The Notes page body owns the nested note tree and
  opens the floating Notes panel when a note is selected.
- The floating panel separates a narrow note list, a flexible document editor,
  and a properties area for category, status, tags and backlinks. The title and
  saved state sit above the editor; editor actions stay grouped with the title.
- Folder rows can nest without a fixed depth limit. Indent by hierarchy, and
  retain a distinct selected note surface. Long trees and note bodies scroll
  inside their own regions.
- The editor supports Markdown source plus formatted preview, links and local
  images. Edit and preview can appear separately or side by side; split preview
  follows unsaved editor changes. The reference's toolbar is directional: all
  actions must be real, and unavailable behaviors must not be represented as
  working controls.
- Empty notes, no selected note, save error, long title, unsaved edits and a
  narrow floating panel must remain legible and usable. Theme roles come only
  from UI Bridge/GPUI Kit.

## Data defaults

- Nested folders are the primary classification hierarchy; frontmatter category
  and status are additional queryable markers, with tags for cross-folder views.
- A note is a `.md` file with a stable frontmatter ID. `[[title]]` links resolve
  by note title and rename updates exact matching links. Backlinks are derived
  from Markdown content.
- Images are copied under `assets/<note-id>/` and linked using a relative path.
