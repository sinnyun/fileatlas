# Runtime debug panel validation

Implemented against the accepted `docs/reference-ui/final-v5/00-main-library.png` workspace structure with a targeted bottom status button and expandable lower drawer. This feature adds a Cordis-owned `photag-plugin-debug`; it does not alter the main navigation layout.

Environment: Windows MSVC, installed Rust 1.97.1, same-process PhoTag MCP child process, isolated profile under `target/mcp-debug-panel-final` and `target/mcp-debug-panel-menus`. The test profiles keep normal user DB/settings/session/cache isolated. Screenshots are captures of the actual GPUI window through the project MCP client. The compiled executable was `target/debug/photag-shell-debug-panel.exe`.

Validation commands:

- `cargo +1.97.1 rustc --locked -p photag-shell --bin photag-shell -j 2 -- -o F:/AI/geren/pho_cordis/target/debug/photag-shell-debug-panel.exe` — passed; build output is `target/debug-plugin-build.log`.
- `cargo +1.97.1 test --locked -p photag-plugin-debug -p photag-ui-bridge -j 2` — passed, 10 tests total; test output is `target/debug-plugin-final-tests.log`.
- `rustfmt --edition 2024 --check crates/photag-plugin-debug/src/capture.rs crates/photag-plugin-debug/src/lib.rs crates/photag-plugin-debug/src/store.rs crates/photag-plugin-debug/src/view.rs crates/photag-plugin-mcp/src/tools_debug.rs` — passed.
- `node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-debug-panel.exe --profile target/mcp-debug-panel-final --plan target/debug-panel-plan.json --allow-mutations` — 46 actions; report `target/mcp-debug-panel-final/report.json`.
- `node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell-debug-panel.exe --profile target/mcp-debug-panel-menus --plan target/debug-panel-native-menus.json --allow-mutations` — 28 actions; report `target/mcp-debug-panel-menus/report.json`.
- Latest bounded category-menu check used the same client and executable with profile `target/mcp-debug-panel-menu-check`; the screenshot `debug-category-scrolling-menu.png` verifies the scrollable category popup stays within the window.

The live run exercised the bottom button, drawer open/close, event and indexing records, scan duration, a real failed rename and its error details, row selection, category and severity filters, search with no matches, page navigation and page sizes, pause/follow while records continued arriving, minimum window sizing, and clear. The category popup was checked after the final height constraint. Evidence screenshots are stored beside this report: `debug-open.png`, `debug-indexing.png`, `debug-scan-duration.png`, `debug-error-detail.png`, `debug-page-2.png`, `debug-paused.png`, `debug-follow.png`, `debug-empty-filter.png`, `debug-minimum.png`, `debug-cleared.png`, and `debug-category-scrolling-menu.png`.

The bounded in-memory buffer retains up to 10,000 records and resets at process restart. It captures application tracing/log events, structured operation spans, panics, explicit MCP actions and worker duration fields; native OS internals and process-abort crash dumps are not observable through this panel. Worker spans do not automatically propagate parent context across every thread boundary, and a cancelled span's elapsed time is not a business-success signal. RMCP transport DEBUG/TRACE payloads are suppressed to avoid recording the panel's own log-query response; MCP application actions and rejected requests are still recorded.
