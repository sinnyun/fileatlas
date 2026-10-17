# Settings software check — 2026-10-17

## Scope

Added Settings > Software to detect the external helpers QuickLook and Everything,
show detected version/path and provide project, download, and GitHub links. This
change is informational only: it does not install, launch, or invoke either
program, and it does not connect PhoTag search to Everything. The future search
provider is identified as planned only.

The detector reads Windows uninstall and App Paths registry records and checks
standard executable locations/PATH. “Not detected” means no match in those
locations; portable or packaged installations outside them can be missed.
Everything itself is distributed from voidtools.com. The linked
`voidtools/ES` GitHub repository is the official command-line tool, not the
Everything desktop application repository.

## Verification

- Reference: `docs/reference-ui/final-v5/03-settings-tags-ai-updates.png` and
  `docs/reference-ui/concepts/photag-v5-companion-interfaces-2026-10-13.md`.
  The new category follows the existing vertical Settings navigation and
  graphite/slate-blue visual language; it adds one row without changing the
  accepted layout.
- Toolchain: Rust 1.97.1 MSVC, Windows, isolated profile
  `target/mcp-software-check-20261017`.
- Static checks: `cargo +1.97.1 fmt --all -- --check`; `cargo +1.97.1 check
  --locked -p photag-shell -j 2`; `cargo +1.97.1 build --locked -p photag-shell
  -j 2`; `cargo +1.97.1 test --locked -p photag-plugin-settings
  -p photag-plugin-i18n -p photag-plugin-mcp --lib -j 2`; UI refresh,
  architecture, feature-ledger checks, and `git diff --check`.
- Same-process MCP command:
  `node scripts/photag-mcp-debug.cjs --exe target/debug/photag-shell.exe --profile target/mcp-software-check-20261017 --allow-mutations --scenario scripts/validate-software-check.cjs`.
- Result: page reached ready state and detected QuickLook 4.5.0 at
  `C:\Users\Administrator\AppData\Local\Programs\QuickLook\QuickLook.exe`
  and Everything 1.5.0.1423 at `C:\Program Files\Everything\Everything.exe`.
  Visible download/GitHub links were checked. The target applications were not
  launched, installed, or queried.
- Screenshot: [settings-software-check.png](../../../target/mcp-software-check-20261017/screenshots/settings-software-check.png)
  (local generated evidence; under ignored `target/`, not a repository asset).

## Remaining limits

This validates the current machine and the software-check page only. It does not
validate detection of every portable/MSIX installation layout, link navigation,
or any future Everything search integration.
