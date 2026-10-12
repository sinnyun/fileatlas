# UI bridge ports / UI 托槽数据对接

Status: lifecycle-owned metadata and GPUI view-factory catalogs exist; the shell currently mounts only a limited subset of slot regions. This is not complete UI parity.

## Lifecycle and input/output

- Cordis service: `photag/ui-bridge`, published by the shell before feature plugins start.
- `register_for_plugin(context, owner, contributions)` registers Layout/Dock/Panel/Slot/Widget/Command/MenuItem/Keybinding metadata; duplicate/empty IDs reject the batch atomically. Cordis disposal unregisters all entries for that owner.
- `register_views_for_plugin` registers executable GPUI view factories under slot IDs; returned `OwnedUiView` instances are mounted by the shell. View catalog cleanup follows owner lifecycle.
- `UiCommandInvoked` carries the command ID through Cordis to plugin handlers.

## Integration rules / 调试

Use globally unique stable contribution IDs and stable slot names; menu/keybinding items refer to command IDs. Register only after bridge publication and do not cache a view instance across plugin generations. If a view is missing, check owner name, slot name, duplicate registration, bridge startup ordering, and catalog cleanup.
