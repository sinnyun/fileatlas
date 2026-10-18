# PhoTag MCP functional test fixture

Dedicated same-process MCP validation assets created on 2026-10-19.

- `mcp-sample.jpg` is a copy of the checked-in tiny JPEG fixture.
- `mcp-sample.txt` is a small file used for version create/restore checks.
- The metadata export was generated during testing but is intentionally not versioned because it contains real library filenames and absolute paths.
- `ops-a` and `ops-b` were destinations for copy/rename/move/delete checks.

The fixture folder was registered, scanned and then unregistered through MCP. The copied image used for file-operation checks was sent to the Recycle Bin by the test and is no longer present. The `all` profile stores its application state in `target/dev-test`, isolated from the normal PhoTag profile. These fixture files and empty operation folders can be removed after review.
