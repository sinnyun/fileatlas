# UI bridge ports / UI 托槽数据对接

## Unified workbench theme (2026-10-18)

`theme` owns clipboard-v2-derived navy semantic paints, fonts, geometry and the
runtime accent; `theme::apply_components(&mut App)` and
`apply_accent_components(&mut App)` synchronize every Kit state, including active
buttons/links/sidebar on accent changes. Default is `Workbench Blue`; saved
`System`/legacy presets remain valid. No schema/path migration is involved.
`components::WorkbenchStyle` applies sections, selectable cards and navigation to
any `Styled + InteractiveElement`, preserving IDs/actions. `section()`,
`action(id, label, primary)`, `icon_button(id, IconName)` and `section_heading(label)`
are stateless builders. They register no services/events and perform no IO.
`text_input(&Entity<InputState>) -> Input`, `search_input(&Entity<InputState>) -> Input`
and `textarea(&Entity<TextareaState>) -> Textarea` preserve native editing/focus,
with common inset backgrounds. Kit `Theme.colors.input` is the border token;
all plugin inputs now use these builders. Theme mapping also covers table/list
headers and alternate rows, accordion, semantic status buttons and chart colors.
Clipboard, Launcher, Gallery inspector/statistics/bulk headings and Settings consume
these builders; other Kit controls inherit the single theme. `panels` retains
floating lifecycle/geometry ownership. Reference/development contract:
[`docs/ui-theme.md`](../../docs/ui-theme.md). Diagnose theme drift with
`node scripts/check-ui-theme.cjs` and same-process MCP screenshots; token ownership
does not prove every runtime state visually verified. The source-palette notes
below describe historical compatibility APIs; the current palette is workbench navy.

Status: lifecycle-owned metadata and GPUI view-factory catalogs exist; the shell mounts sidebar, content, auxiliary, and floating modal slots. This bridge provides view placement and surface visibility state; exact native windowing and visual parity remain host concerns.

The bridge also exposes shared PhoTag design tokens and accent state from `theme`, plus Windows appearance primitives from `winfx`. These are contracts for consumers; they do not automatically restyle mounted GPUI views. The shell applies `winfx::apply_backdrop` to its native HWND and passes `Backdrop::is_translucent()` to `theme::set_backdrop_active`; Shell, Navigation, Playback and Gallery consume backdrop-aware base/card fills, while Settings, Gallery dialogs and Preview consume modal fills.

## Lifecycle and input/output

- Cordis service: `photag/ui-bridge`, published by the shell before feature plugins start.
- `UiSurfaceState` shares open/closed, shortcut-capture, and one-shot Escape suppression state of floating surfaces between the shell host and their owning plugin. Surface owners update their own view and register the corresponding command; the host mounts `workspace.modal` views above content. State remains local to this application runtime and is recreated with the bridge.
- `register_for_plugin(context, owner, contributions)` registers Layout/Dock/Panel/Slot/Widget/Command/MenuItem/Keybinding metadata; duplicate/empty IDs reject the batch atomically. Cordis disposal unregisters all entries for that owner.
- `register_views_for_plugin` registers executable GPUI view factories under slot IDs; returned `OwnedUiView` instances are mounted by the shell. View catalog cleanup follows owner lifecycle.
- Shell mounted slots reconcile by contribution ID plus registration generation. Navigation does the same for its nested launcher/add-on slots; Settings reconciles `settings.language` in its panel. Hosts that cache constructed `AnyView`s must drop them when the catalog entry disappears or its generation changes.
- `UiCommandInvoked` carries the command ID through Cordis to plugin handlers.
- `ContributionKind::Command { label, needs }` is the lifecycle-owned command
  registry entry. `CommandNeeds` declares invocation policy; `UiCatalog::commands()`
  joins command entries with their owned menu items and keybindings. Entries are
  removed with their plugin owner. The registry is metadata and dispatch contract;
  feature handlers still validate action-time state and own execution.
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

## Keyed background read models (2026-10-15)

`read_model::ReadModel<K,V>` is a shared presentation utility, not a service or
plugin manager. `update(key, FnOnce() -> Result<V,String>) -> bool` belongs in
an action/model poller; `value() -> Option<&Result<V,String>>` is the render
boundary. It admits one worker at a time, coalesces edits to the latest key,
ignores stale completions, caches successes and failures, and returns false
for unchanged/empty-channel polls. Worker disconnect/start failure becomes a
cached error. A view dropping the receiver cannot be invalidated by that read;
an already-running read can finish but must not mutate application state.
The owning plugin/view still owns its tasks and invalidation identity. Settings
uses Navigation's tag revision; Gallery dialogs use index revision plus inputs.
No persistence, Cordis registrations or DB access is owned by this utility.

## Unified refresh contract (2026-10-15)

2026-10-16 readiness extension: `refresh::Wake: Clone + Default` exposes
`trigger()` and async `wait()`, backed by one coalesced Notify permit, without
payloads or an idle timer. `refresh::listen(cx, wake, FnMut(&mut V,
&mut Context<V>) -> bool) -> Task<()>` samples through a weak entity and only
notifies for a real change. Store the returned task in the view; dropping it
cancels the listener. One consumer owns each signal; this is not broadcast.
`ReadModel::set_wake(Wake)` opts into completion/disconnect hints. The result
sender disconnects before a panic hint is delivered. Existing poll identity,
stale-result and cached-failure rules stay authoritative. Navigation/Gallery
retain their 100/200ms fallback samplers. See navigation-latency validation.

`refresh::poll<V>(Context<V>, Duration, FnMut(&mut V, &mut Context<V>) -> PollResult)` owns weak-entity timers and one notify per changed sample. `PollResult::until(changed, active)` delivers a terminal change before stopping. `ChangeGate<T>::new/changed/current` centralizes passive signature equality.

`ReadModel::request` declares a demand without receiving a result from render; `poll` delivers it, `update` combines both for model pollers. `key/is_pending/value` expose state. One running worker plus one latest queued closure per model; stale results cannot overwrite current keys, A→B→A reuses A, worker disconnects cache an error. Neither utility emits logs. Callers own semantic invalidation keys; no schema/service/plugin lifecycle change. See docs/ui-refresh-contract.md and scripts/check-ui-refresh.cjs.

### Floating-panel text input (2026-10-17)

The panel root contains shortcut/navigation key events and permits character events to continue to the focused input handler. This keeps the same-process GPUI text-input path usable inside hosted feature panels. Escape still closes the current panel. The launcher validation uses real focused search/name inputs through `control_ui input_text`, with no clipboard or OS input injection.
# Shared sidebar regions (2026-10-18)

`components::sidebar_section(Div) -> Div` applies backdrop surface, theme region
separation, compact padding/corners and nonshrinking frame geometry.
`sidebar_header(label, toggle: Button, menu: impl IntoElement, trailing: Vec<Button>)`
returns a shared chevron/title/menu/action row; dropdown wrappers retain their own
native behavior. Navigation uses these builders for drives, quick access and
every favorite group. The builders do not register resources or own product data.
