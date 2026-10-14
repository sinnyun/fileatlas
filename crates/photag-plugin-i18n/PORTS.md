# Localization plugin ports

Cordis plugin `photag.i18n` publishes `I18nService` as `photag/i18n`.
It requires `UiBridgeService` before startup; Shell starts it before feature
views and stops it after consumers. No independent plugin manager is added.

## Public contract

- `I18nConfig { path: Option<PathBuf> }` selects the preference file. Default:
  `%LOCALAPPDATA%/PhoTag/language.json`. The isolated dev-test profile uses
  `<PHOTAG_DEV_TEST_DIR>/language.json`; source settings/session/schema are intact.
- `I18nService::open(path) -> Result<Self, I18nError>` parses embedded Fluent
  resources and loads the preference. Missing/invalid preferences use English.
- `language() -> Language` returns `English` (`en`) or `Chinese` (`zh-CN`).
- `set_language(Language) -> Result<(), I18nError>` persists first, then changes
  the visible language and publishes a UI translation revision. A save failure
  leaves the previous language active. Unsupported string codes are rejected by
  `Language::parse`.
- `Translator::translate(source, &[(&str, String)]) -> String` formats named
  arguments using `fluent-bundle` 0.16. English is the fallback. Unknown text
  remains unchanged. `keys.json` maps English migration labels to stable Fluent
  IDs; translations live in `locales/en.ftl` and `locales/zh-CN.ftl`.

## UI ownership and updates

The plugin owns `i18n/language-selector` in `settings.language`. Settings mounts
that contribution inside Appearance. Shell does not render language controls.
The public UI Bridge `i18n::tr` / `tr_args` boundary holds a weak translator;
Cordis disposal removes both the binding and owned view. The shell observes
translation revisions and refreshes GPUI windows. Feature inputs update their
placeholders without resetting typed values, filters, or selection.

There is no Cordis event for language changes; consumers observe the public
revision. Library filenames, paths, tag names, keys and stored source enums
remain data, not translation keys. System errors, release notes supplied by
GitHub, native OS dialogs, and development diagnostics may remain in their
original language. Some generated status/error text still needs catalog coverage.

## Validation and failures

Four tests cover every embedded message/argument set, default/persistence,
failed saves, and Cordis service/view/binding cleanup. MCP validation exercised
both languages, six Settings tabs and real Gallery/Preview screenshots; the
language selector's mouse click itself has not been tested in the MCP-only
pass. See `docs/validation-report.md` for evidence and visual limitations.
Catalog parse errors fail plugin startup. Preference-save failures appear in
the language panel and return `I18nError`; MCP reports them through `control_ui`.

## Runtime diagnostics (2026-10-14)

Runtime diagnostics: plugin startup and existing error/diagnostic output now emit tracing records captured by photag.debug; instrumented public operations report wall-clock span duration and errors. Domain service contracts, event payloads and source operation ordering are unchanged. See photag-plugin-debug/PORTS.md for capture scope and limits.
