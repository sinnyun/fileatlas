# MCP-only validation captures (2026-10-13)

Captured from `photag-shell --mcp` by `scripts/validate-ui-mcp.cjs` on the real
Windows GPUI process. Isolated dev-test profile,9 generated media,1280×800
logical viewport; PrintWindow reports1942×1256 physical pixels with frame/DPI.
No computer-control or pointer input was used. These are migration captures,
not reference screenshots or proof of complete visual parity.

The seven PNGs retained here cover English/Chinese Gallery, English/Chinese
Appearance (including the new language selector), Chinese Preview Info,
Statistics and Bulk. All23 captured states are listed in report.json and remain
locally under target/mcp-ui-screenshots; the remaining Settings tabs and selection
captures are not duplicated here. Source references and remaining differences
are in ../../validation-report.md (the workspace docs/validation-report.md).

The report includes service snapshots and real idle-chrome opacity samples.
Hover activation, tooltip contents, pointer-driven transitions and native dialogs
are unverified; static PNGs must not be described as animation verification.
