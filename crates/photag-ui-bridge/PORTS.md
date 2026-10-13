# UI bridge ports / UI 托槽数据对接

Status: lifecycle-owned metadata and GPUI view-factory catalogs exist; the shell currently mounts only a limited subset of slot regions. This is not complete UI parity.

The bridge also exposes shared PhoTag design tokens and accent state from `theme`, plus Windows appearance primitives from `winfx`. These are contracts for consumers; they do not automatically restyle mounted GPUI views. The shell must apply `winfx::apply_backdrop` to its native HWND and pass `Backdrop::is_translucent()` to `theme::set_backdrop_active`; surfaces can then use `panel_alpha()` for the source's translucent gallery card fill.

## Lifecycle and input/output

- Cordis service: `photag/ui-bridge`, published by the shell before feature plugins start.
- `register_for_plugin(context, owner, contributions)` registers Layout/Dock/Panel/Slot/Widget/Command/MenuItem/Keybinding metadata; duplicate/empty IDs reject the batch atomically. Cordis disposal unregisters all entries for that owner.
- `register_views_for_plugin` registers executable GPUI view factories under slot IDs; returned `OwnedUiView` instances are mounted by the shell. View catalog cleanup follows owner lifecycle.
- `UiCommandInvoked` carries the command ID through Cordis to plugin handlers.
- `theme` exports source palette values (`0xRRGGBB`), source premultiplied control/border colors with explicit alpha, dimensions, text sizes, Segoe family names, MDL2 glyphs, seven accent presets, and the atomic current accent. `PremultipliedColor::to_rgba()` converts source colors to GPUI's straight-alpha `0xRRGGBBAA` format. Settings consumers call `set_accent_name` after restoring or changing the saved preset; GPUI views use `accent()` and the shared token constants while rendering. The token catalog does not install fonts or prove that GPUI Kit's named text sizes resolve to the source font faces.
- `winfx::system_accent()` reads the Windows AccentPalette/DWM registry values and applies source brightness normalization. `winfx::font_availability()` reports source font-file availability; GPUI still resolves font family names and fallback while shaping text.
- `winfx::apply_backdrop(hwnd)` applies the dark title bar and returns Mica (Windows 11) or Acrylic (Windows 10) when supported. It is a shell startup operation after native window creation, not a plugin UI contribution. Non-Windows calls safely return no accent/backdrop.

## Integration rules / 调试

Use globally unique stable contribution IDs and stable slot names; menu/keybinding items refer to command IDs. Register only after bridge publication and do not cache a view instance across plugin generations. If a view is missing, check owner name, slot name, duplicate registration, bridge startup ordering, and catalog cleanup.

Appearance debugging: verify the settings consumer called `set_accent_name`, confirm the installed system family names with `font_availability`, and inspect `Backdrop` returned for the real HWND. The shared bridge does not yet provide per-plugin theme cleanup because tokens and the accent are application-wide shell state.
