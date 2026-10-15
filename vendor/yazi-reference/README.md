# Pinned Yazi source references

Upstream: https://github.com/sxyazi/yazi
Commit: `ffb2e78cc70341946c04befa52e14a4494015ce8`.
Copyright (c) 2023 - sxyazi. MIT license: [LICENSE](LICENSE).

The three source files in this directory are unmodified review references and are not compiled as PhoTag crates. PhoTag adapts `VfsEntries::from_dir` producer/receiver-closure cancellation and Refresher batching/ticket/latest-request concepts in `crates/photag-plugin-gallery/src/live.rs`; local watcher fallback, nonrecursive subscription and stat-based patching are adapted to PhoTag's existing watcher. The adaptations replace Yazi's unbounded Tokio channels with bounded synchronous batches, local path/cache types, cooperative cancellation, Cordis events and GPUI ReadModel delivery. They do not import Yazi's TUI, VFS URL stack, scheduler or plugin manager. Existing PhoTag preview/image pipelines are reused.

Full per-source mapping and implementation differences: `docs/browser-mode-requirements-yazi.md`.
