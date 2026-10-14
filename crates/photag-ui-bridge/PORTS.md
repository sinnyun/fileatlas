# UI bridge ports / UI 托槽数据对接

Status: lifecycle-owned metadata and GPUI view-factory catalogs exist; the shell mounts sidebar, content, auxiliary, and floating modal slots. This bridge provides view placement and surface visibility state; exact native windowing and visual parity remain host concerns.

The bridge also exposes shared PhoTag design tokens and accent state from `theme`, plus Windows appearance primitives from `winfx`. These are contracts for consumers; they do not automatically restyle mounted GPUI views. The shell applies `winfx::apply_backdrop` to its native HWND and passes `Backdrop::is_translucent()` to `theme::set_backdrop_active`; Shell, Navigation, Playback and Gallery consume backdrop-aware base/card fills, while Settings, Gallery dialogs and Preview consume modal fills.

## Lifecycle and input/output

- Cordis service: `photag/ui-bridge`, published by the shell before feature plugins start.
- `UiSurfaceState` shares open/closed, shortcut-capture, and one-shot Escape suppression state of floating surfaces between the shell host and their owning plugin. Surface owners update their own view and register the corresponding command; the host mounts `workspace.modal` views above content. State remains local to this application runtime and is recreated with the bridge.
- `register_for_plugin(context, owner, contributions)` registers Layout/Dock/Panel/Slot/Widget/Command/MenuItem/Keybinding metadata; duplicate/empty IDs reject the batch atomically. Cordis disposal unregisters all entries for that owner.
- `register_views_for_plugin` registers executable GPUI view factories under slot IDs; returned `OwnedUiView` instances are mounted by the shell. View catalog cleanup follows owner lifecycle.
- `UiCommandInvoked` carries the command ID through Cordis to plugin handlers.
- `theme` exports source palette values (`0xRRGGBB`), source premultiplied control/border colors with explicit alpha, dimensions, text sizes, Segoe family names, MDL2 glyphs, seven accent presets, and the atomic current accent. `PremultipliedColor::to_rgba()` converts source colors to GPUI's straight-alpha `0xRRGGBBAA` format; `base_rgba()`, `modal_rgba()`, and `panel_rgba()` apply the source's Mica/Acrylic alphas (0xDE base, 0xF4 modal, 0xE8 card). Shell/base, Settings and Preview modals, Gallery dialogs/cards, Navigation and Playback surfaces consume these helpers. Settings consumers call `set_accent_name` after restoring or changing the saved preset; GPUI views use `accent()` and shared token constants while rendering. The token catalog does not install fonts or prove that GPUI Kit's named text sizes resolve to the source font faces.
- `winfx::system_accent()` reads the Windows AccentPalette/DWM registry values and lifts colors by scaling all channels until the brightest reaches 150, matching PhoTag's `ensure_min_brightness`; black maps to neutral gray. `theme::normalize_accent()` preserves the same rule for callers supplying a color. `winfx::font_availability()` reports source font-file availability; GPUI still resolves font family names and fallback while shaping text.
- `winfx::apply_backdrop(hwnd)` applies the dark title bar and returns Mica (Windows 11) or Acrylic (Windows 10) when supported. It is a shell startup operation after native window creation, not a plugin UI contribution. Non-Windows calls safely return no accent/backdrop.

## Integration rules / 调试

Use globally unique stable contribution IDs and stable slot names; menu/keybinding items refer to command IDs. Register only after bridge publication and do not cache a view instance across plugin generations. If a view is missing, check owner name, slot name, duplicate registration, bridge startup ordering, and catalog cleanup.

Appearance debugging: verify the settings consumer called `set_accent_name`, confirm the installed system family names with `font_availability`, and inspect `Backdrop` returned for the real HWND. Registry probing, font resolution, title-bar material and transparency still require Windows runtime validation. The shared bridge does not yet provide per-plugin theme cleanup because tokens and the accent are application-wide shell state.

## Translation and surface-value contracts (2026-10-13)

`i18n::Translator` is the public rendering boundary; the i18n plugin supplies a
weak implementation. `tr(source)` and `tr_args(source, named_strings)` use it,
falling back to source text when absent. install/changed/revision support plugin
cleanup and GPUI refresh without retaining the provider. UiSurfaceState value /
set_value carry Settings' tab request alongside existing open/closed state.
`theme::apply_components(App)` applies source ghost-control surfaces and
Segoe typography to GPUI Kit controls, avoiding the earlier light/gray defaults.

`theme::apply_accent_components(&mut gpui_kit::App)` is the GPUI-thread runtime
accent synchronization entry point. Call after `set_accent_name` in Settings;
it updates only primary/accent/selection using the existing solid/64-alpha rules.
`Theme::update` reconciles component tokens and Base projections and refreshes
windows; the runtime helper does not reset typography, geometry or other colors.

## Floating panel contract (2026-10-14)

`panels::mount(AnyView, PanelOptions, Arc<UiSurfaceState>, visible, close, &mut App) -> AnyView` supplies a shared draggable/resizable frame with maximize/restore/close. Options carry stable instance ID, translated title, independent default/minimum logical size. Actual GPUI content/client bounds are published; Preview uses `panels::content_bounds`. The host supplies one background occlusion layer and orders `UiViewContribution.with_panel_id` views by `panel.active`. A drag-only capture surface is painted above the frame so frame occlusion cannot swallow movement/up events.

`UiBridgeService.panels: Arc<PanelCatalog>` exposes register/open/close/unregister_owner. `register_panel_for_plugin(Context, owner, PanelDefinition)` binds cleanup to Cordis. Definitions contain options, Single/Multiple policy and the real GPUI factory. Single open returns/focuses its existing ID; Multiple returns a new ID and creates independent content entities when mounted. Close removes the instance contribution; owner stop removes definitions, instances, open flags and views. UiViewCatalog attaches a monotonically allocated registration generation; host mounts compare ID plus generation to avoid retaining entities across plugin restart. Service-driven existing views use mount and retain their original service semantics.

Window-internal geometry persists only for each entity lifetime; native detached windows and automatic geometry persistence are not supplied. Builtin Preview/Playback remain single-session services. See docs/floating-panels.md and docs/validation-ui/2026-10-14-panels for runtime evidence.

## Runtime diagnostics (2026-10-14)

Runtime debug integration: workspace.bottom is the shell bottom dock slot and workspace.debug-status is the footer suffix. UiSurfaceState::set_open logs only actual visibility transitions. Registration, generation and Cordis-owned cleanup semantics are unchanged.
