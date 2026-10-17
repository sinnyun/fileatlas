# Development test plugin ports / 开发测试插件对接

Status: an opt-in GPUI panel scans generated on-disk media fixtures and injects
or removes 1,000, 10,000, or 100,000 virtual media rows for Gallery/SQLite
scale checks. It is development tooling and is not enabled by `core` or `all`.

## Lifecycle and dependencies

- Select with `PHOTAG_DEV_PLUGINS=dev-test` or `start-dev.bat dev-test`.
- Cordis plugin name: `photag.dev-test`; service: `photag/dev-test`.
- Requires `DatabaseService`, `IndexingService`, and the shell's `UiBridgeService`.
- Owns a GPUI panel in `workspace.dev-test`; Cordis removes its service and view
  contribution when the plugin stops.
- When the same process also enables the MCP plugin, the service is exposed as
  `dev_test_status`, `dev_test_scan_fixture`, `dev_test_inject_virtual`, and
  `dev_test_clear_virtual`. MCP task starts are asynchronous; poll status until
  `running=false`, then inspect Gallery with normal read/screenshot tools.
- The shell enables Gallery, Navigation, Thumbnails, Indexing, Preview, and
  their required providers for this profile.

## Panel actions and data

- **Scan sample directory** registers `test-data/photag-library` and calls the
  real recursive `IndexingService::scan_folder`; progress and indexed row count
  are shown in the panel. `PHOTAG_TEST_LIBRARY` can select another directory.
- **Inject 1K / 10K / 100K** transactionally replaces only the hidden
  `.photag-virtual-benchmark` path prefix under the fixture folder. Rows vary
  image/video/audio types, file sizes, dimensions and ratings. One quarter point
  to `.dev-support/preview.png`; virtual paths never create physical files.
- **Clear virtual rows** deletes only that reserved path prefix. The real
  scanned fixture rows and physical files remain intact.
- Operations run off the GPUI thread and emit `MediaIndexChanged(())` after
  completion so Gallery reloads the database.
- All database work uses typed `DatabaseService` operations. The benchmark
  replace/clear methods keep reserved-prefix deletion and row insertion in one
  writer transaction. Browser-mode benchmark rows use the session cache and do
  not persist rated synthetic media; replace/clear also remove reserved-prefix
  rows persisted by older runs. Progress callbacks report every 10,000 rows.

## Data isolation and debugging

When enabled, the shell uses `target/dev-test/photag.db`, `settings.json`,
`session.json`, and `thumbs/` instead of normal PhoTag user data. Override the
root with `PHOTAG_DEV_TEST_DIR` or only the DB file with `PHOTAG_DEV_TEST_DB`.
Check the panel status first; then inspect the isolated database and thumbnail
cache. The fixture assets and reproducible generator are documented in
`test-data/README.md`.

Run the repeatable checks with:

```powershell
cargo +1.97.1 test -p photag-plugin-dev-test --locked
```

Tests scan the checked-in generated formats using the production indexer and
exercise a 100,000-row replace/cleanup cycle against a temporary SQLite file.
Interactive GPUI performance and scrolling still require launching the desktop
app with the `dev-test` profile on Windows.

## Localization integration (2026-10-13)

Visible control labels consume the public UI Bridge translator supplied by
photag.i18n. Stable commands, source settings/schema values, media paths and
user-created tag data are unchanged. Missing translations fall back to English;
OS/backend errors and some generated diagnostics are not fully localized.
The real MCP-only validation coverage and remaining source layout differences
are recorded in docs/validation-report.md; screenshots do not prove pointer
interaction or animation parity.

## Floating status panels (2026-10-14)

In the existing opt-in isolated dev-test profile, definitions dev-test/status(Single, default520×300) and dev-test/status-multi(Multiple, default600×360) use the public UI Bridge PanelCatalog and render DevTestService::status/is_running. They are real development task observers, not product plugins or fake state. Every instance polls status changes every200ms; Cordis owner photag.dev-test cleans definitions/instances on stop. MCP panel_open/close exercises the same public integration contract intended for future feature plugins.

## All-file fixture expectations (2026-10-14)

The production scan regression now expects all 12 ordinary files in `test-data/photag-library`, including README.txt, the XMP sidecar and `.dev-support/preview.png`. Virtual-record generation and profile isolation remain unchanged.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.

## Shared refresh ownership (2026-10-15)

StatusPanel and finite task monitors use UI Bridge refresh::poll and ChangeGate over status/running. A running but unchanged task no longer forces redraw; terminal changes notify before the monitor stops.
