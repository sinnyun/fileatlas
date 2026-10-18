# System shortcuts plugin / 系统快捷键插件

## Cordis identity

- Plugin: `photag.shortcuts`; service: `photag/shortcuts` (`ShortcutService`).
- Requires `photag/ui-bridge`; Cordis cleanup unregisters global shortcuts and
  stops the command forwarding task.
- Shell starts it after the launcher in the normal `core` composition. Selecting
  `PHOTAG_DEV_PLUGINS=shortcuts` also enables the launcher dependency.

## Shortcut behavior

- Uses the open-source `global-hotkey` crate (Tauri team, MIT OR Apache-2.0).
- Every lifecycle-owned `Keybinding` contribution with `System` scope is read
  from `UiCatalog.commands()`. Settings overrides take priority; missing values
  use plugin defaults and empty values disable bindings. The launcher
  contributes Ctrl+F and the clipboard workbench contributes Ctrl+Shift+V.
- A pressed binding emits its registered `UiCommandInvoked` command ID. On
  Windows the host restores and foregrounds PhoTag before dispatch.
- `ShortcutService::start()` and `poll()` must run on the GPUI window/event-loop
  thread, as required by `global-hotkey` on Windows. The plugin itself owns the
  registration and stop cleanup.
- Changed settings unregister and re-register system bindings on the owning UI
  thread. Bindings suspend during Settings key capture. Registration conflicts
  and OS rejection are logged; the plugin command remains available to menus.
- The GPUI host polls `ShortcutService::poll(hwnd, overrides, capture_active)`
  through its existing shared 100 ms refresh loop.

## Limits and diagnostics

- Windows is the product target. The library also documents macOS and Linux X11;
  Wayland is unsupported by the dependency.
- System parsing supports letters, digits, F1–F12, navigation keys, Space,
  Enter, Tab, Escape, Backspace and Delete. Settings prevents duplicate
  bindings within one scope and requires a modifier for system-wide bindings.
- Each registered system shortcut is a system-wide reservation while PhoTag is
  running. If another app has already registered one, inspect shortcut logs.
- Live global-key delivery and foreground restoration require Windows runtime
  verification. Same-process PhoTag MCP currently has no API for pressing a
  system-wide key while another application is foregrounded.

## Corrected foreground routing — 2026-10-18

ShortcutService::poll retains its HWND argument for compatibility but no longer restores
that HWND unconditionally. It dispatches the registered command. The tool command chooses
`PanelCatalog::open_presented`; the host activates the resulting native window or restores
the main window for embedded mode. Direct screenshot capture does not restore PhoTag.
Actual OS global-key delivery remains unverified; MCP invokes the same command handlers.
