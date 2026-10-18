# Screenshot launcher / Shotoshot integration

## Cordis identity and dependencies

- Plugin: `photag.screenshot`; service: `photag/screenshot` (`ScreenshotService`).
- Requires `photag/settings`, `photag/ui-bridge`, and the existing shortcuts dispatcher.
- Runtime-controllable as standalone tool `photag.screenshot`; Settings can dispose/re-spawn its Cordis Fiber to stop/restore its panel, commands, and system shortcut.
- Shell enables it in `core`, `all`, MCP, or `PHOTAG_DEV_PLUGINS=screenshot` profiles.
- It registers one single-instance generic floating manager panel, a sidebar entry, an open-panel command, a capture command, and the system shortcut `Ctrl+Shift+S`. UI Bridge and Cordis remove all contributions/listeners with plugin stop.

## Upstream application boundary

- At PhoTag startup, the service checks the configured executable and the supported install locations once, caches the result, and stores a discovered path in the existing settings. Rendering and shortcut activation use that startup snapshot; they do not rescan installation locations. The user can select a different `shotoshot.exe` in the manager.
- If no executable is detected, the manager offers Shotoshot's official installation page and keeps capture disabled until the user installs it and selects its executable. Capture also reports a clear missing/invalid path message.
- Capture command sends the capture request to a resident Shotoshot window; if no resident window exists, it launches the selected executable with `--capture`. The editor and capture processes are controlled by Shotoshot.
- Shotoshot is single-instance. If it is already running, PhoTag sends its existing Windows `WM_APP+2` capture message to `Shotoshot.Window`; otherwise it launches the installed executable with `--capture`.
- No screenshot surface, selection overlay, annotation tool, or editor behavior is implemented in PhoTag.
- PhoTag does not vendor or build Shotoshot. The user installs it separately. The launcher discovers a sibling executable, `%LOCALAPPDATA%\Programs\Shotoshot\shotoshot.exe`, or `PHOTAG_SHOTOSHOT_EXE`; the manager can also select the executable directly.

## Settings and output

- PhoTag persists `screenshot_tool.executable` and `screenshot_tool.save_directory` additively through the existing `SettingsService` JSON model.
- `Settings.plugin_launch_modes["photag.screenshot"]` defaults to its manager panel. `PanelOnly` hides the PhoTag workspace behind the shared panel; `Direct` routes the manager shortcut to Shotoshot capture. The dedicated `Ctrl+Shift+S` capture command always calls Shotoshot directly.
- MCP `get_ui_state.screenshot_tool` reports the cached executable, availability, output directory, `last_capture_action`, and `last_capture_error`. The debug drawer receives explicit request/result/error tracing events.
- Default output: `%LOCALAPPDATA%\PhoTag\Screenshots`.
- The plugin creates the directory and updates only the output-folder field in Shotoshot's versioned settings file while holding the same named Windows mutex and using an atomic write-through replacement. Other Shotoshot fields are preserved. If the settings file does not exist yet, start Shotoshot once so it creates its defaults, then retry.
- Directory/path errors are returned to the manager panel. Capture process startup errors are logged by the Cordis command listener; inspect `photag.screenshot` tracing events.

## Limits

- The application window, tooltips, and menus are supplied by Shotoshot and are currently English.
- The default save directory is `%LOCALAPPDATA%\\PhoTag\\Screenshots`; PhoTag synchronizes it to Shotoshot's native save setting so the user's final PNG is stored under PhoTag's screenshot directory and can be managed there. This integration does not yet import screenshot rows into the photo-library database.
- Earlier same-process PhoTag MCP verified only that `PostMessageW` accepted a `WM_APP+2` queue submission; the user reported no editor appeared, so that was insufficient evidence of a working capture. The bridge now uses synchronous `SendMessageTimeoutW` (3-second timeout), reports delivery failures, and records request/result/error diagnostics. Rebuilt-host live capture and user-driven selection/annotation/save still require verification.
- GPL-3.0 is accepted for this personal-use workspace. The workspace's declared MIT package licenses are not changed here; distribution of a combined executable would require following GPL-3.0 and dependency notices.
