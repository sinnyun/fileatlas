# Unified TagCapsule — 2026-10-17

Visual direction: [unified-tag-capsule-2026-10-17.png](unified-tag-capsule-2026-10-17.png).
The component uses one compact rounded pill with an optional configured icon,
its label, a restrained tag-color tint, and a consistent selected outline. A tag
without a configured color uses a neutral graphite capsule; a missing icon uses
the shared tag glyph. The generated sheet informed structure and palette only;
its sample tag names, icons, and colors are not PhoTag data.

The interface retains its accepted dark charcoal panels, slate-blue selection,
readable primary text, compact spacing, and thin separators. The capsule fits
within sidebars, pickers, media thumbnails, and settings rows; parent controls
continue to own checkboxes, removal actions, counters, and click behavior.
Gallery thumbnail cards can constrain capsule width and use ellipsis; the
existing three-tag cap and `+N` overflow counter remain.

Implemented in `photag-ui-bridge::tag_capsule::TagCapsule` and used by Gallery,
Navigation, and Settings view modules. The currently designed capsule states
are default, tag-color tint, selected, and hover. Live screenshot comparison
and remaining layout gaps are recorded in
`docs/validation-ui/2026-10-17-unified-tag-capsule/README.md`.
