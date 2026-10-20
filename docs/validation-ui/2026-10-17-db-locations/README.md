# Program database locations in the inspector — 2026-10-17

The Program Information panel now replaces “Current browsing location” with
two real storage paths:

- Application database: the path reported by the active `DatabaseService`.
- Lore version storage: the active `VersionsService` root, containing Lore
  worktrees, shared stores, and historical preview copies. Version metadata
  records themselves are stored in the application database.

## Checks

- `rustup toolchain list`: confirmed installed `1.97.1-x86_64-pc-windows-msvc`.
- `cargo +1.97.1 fmt --all -- --check`: passed.
- `cargo +1.97.1 check --locked -p fileatlas-shell -j 2`: passed.
- `cargo +1.97.1 build --locked -p fileatlas-shell -j 2`: Rust compilation and
  linking completed, but Cargo could not replace `target/debug/fileatlas-shell.exe`
  because Windows returned access denied. Process 7144 had that executable
  open. It was left running.
- Same-process FileAtlas MCP visual verification remains pending because the
  current executable could not be replaced; the open process was not used as a
  substitute for the required MCP-launched child.

No screenshot comparison was made. Long paths and the final wrapped rendering
in the live panel remain to be checked after a fresh executable can be built.
