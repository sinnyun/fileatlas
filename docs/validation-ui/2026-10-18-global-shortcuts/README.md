# Global shortcut plugin — 2026-10-18

## Implementation

- `photag-plugin-shortcuts` provides `photag/shortcuts` and registers lifecycle-owned
  `System` scope keybindings through `UiCatalog` using `global-hotkey` 0.8.0
  (MIT OR Apache-2.0).
- Settings overrides replace plugin defaults; an empty override disables a
  binding. Capture pauses system registrations while Settings records a new key.
- Default bindings are Ctrl+F for the Program Launchpad and Ctrl+Shift+V for the
  Clipboard Workbench. Pressed IDs dispatch `UiCommandInvoked`; Windows restores
  and foregrounds the PhoTag window before command dispatch.
- Cordis stop unregisters active bindings and stops the forwarding task.

## Verification

- `cargo +1.97.1 fmt --all -- --check` — passed.
- `cargo +1.97.1 check --locked -p photag-shell -j 2` — passed.
- `cargo +1.97.1 build --locked -p photag-shell -j 2` — passed (existing LNK4098 warning).
- Same-process MCP `command_list` confirmed the clipboard open command advertises
  Ctrl+Shift+V at system scope; see
  [clipboard workbench validation](../2026-10-18-clipboard-workbench/README.md).
- The host does not expose cross-application system-key injection through PhoTag
  MCP. Actual Ctrl+F/Ctrl+Shift+V delivery, another application's foreground
  restoration, and OS registration conflict behavior remain unverified.

## Limits

Bindings can be changed in Settings and may conflict with another registered
application shortcut. Failed OS registration is logged; the sidebar button and
command remain available. Windows runtime delivery needs direct user exercise.
