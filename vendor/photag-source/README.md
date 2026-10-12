# PhoTag

**A fast, local-first photo, video and audio library for Windows — built around hierarchical tagging.**

PhoTag indexes the folders you point it at, builds thumbnails, and gets out of the way. Everything lives on your machine: no account, no cloud, no subscription, no phoning home. The index is a single SQLite file; your originals are never modified unless you explicitly ask PhoTag to write tags into them.

It's written in Rust on [`egui`](https://github.com/emilk/egui) — a single ~35 MB executable with no installer and no runtime to preinstall.

> **Platform:** Windows 10 (1903+) and Windows 11. Some features (Mica/Acrylic backdrop, drag-out to Explorer, Recycle Bin deletes) are Win32-specific by design.

---

## Contents

- [Why](#why)
- [Features](#features)
- [Install](#install)
- [Build from source](#build-from-source)
- [Quick start](#quick-start)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [How tagging works](#how-tagging-works)
- [Search and filtering](#search-and-filtering)
- [AI tag suggestions](#ai-tag-suggestions)
- [Updates](#updates)
- [Video and audio support](#video-and-audio-support)
- [Writing tags into your files](#writing-tags-into-your-files)
- [Where PhoTag stores its data](#where-photag-stores-its-data)
- [Project layout](#project-layout)
- [Performance notes](#performance-notes)
- [Troubleshooting](#troubleshooting)
- [License](#license)

---

## Why

Folder trees force one hierarchy onto files that belong in several. A photo of your dog at the beach in 2019 is *Animals > Dogs*, *Places > Beach*, and *2019* all at once — but it can only live in one directory.

PhoTag keeps your files exactly where they are on disk and layers a tag graph on top. Nothing is imported, copied, or relocated behind your back. Point it at `D:\Photos`, tag freely, and filter by any combination of tag, date, rating, type, extension, or filename.

## Features

### Library

- **Non-destructive indexing** — add any number of root folders; files stay where they are.
- **Multi-threaded scanning** with live progress, capped below your core count so the UI never freezes.
- **Filesystem watching** — files added, moved, or deleted outside PhoTag are picked up automatically.
- **Images:** JPG, JPEG, PNG, GIF, BMP, WEBP, TIFF, TIF, SVG, ICO, HEIC, HEIF, AVIF
- **Video:** MP4, MOV, AVI, MKV, WMV, FLV, WEBM, M4V, MPG, MPEG, 3GP
- **Audio:** MP3, WAV, FLAC, AAC, OGG, WMA, M4A, OPUS, APE, AIFF, AIF
- **EXIF-aware** — reads capture date and orientation, and reuses the embedded EXIF thumbnail when one is present.

### Tagging

- **Hierarchical tags** — `Animals > Birds > Owls`, nested as deep as you like.
- **Ancestor auto-apply** — tagging something *Owls* implicitly applies *Birds* and *Animals*. PhoTag tracks which assignments were manual versus automatic, so cleaning up a child tag never silently strips a tag you set by hand.
- **Tag associations** — assigning one tag can automatically pull in others (assign *Wedding*, get *People* and *Event* for free).
- **Per-tag colour and icon**, optionally cascaded to descendants, with an "leave customised children alone" option.
- **Drag to reorder and re-parent** anywhere in the tag tree.
- **Merge tags** — fold one tag into another, choosing whether its children get re-parented or deleted.
- **Rename with cascade** — renaming a parent rewrites every descendant path correctly.
- **Tag hotkeys** — bind F1–F9 to your most-used tags and toggle them on the current selection instantly.

### Browsing and culling

- **Virtualised thumbnail grid** — smooth at tens of thousands of items; thumbnail size adjustable from 80 to 300 px.
- **Month dividers** when sorted by date, collapsible per month.
- **Sort** by newest, oldest, largest, smallest, name (A–Z / Z–A), or rating (highest / lowest).
- **Star ratings 0–5**, applied to an entire multi-selection with a single keypress.
- **Preview mode** with pan, zoom (modes and presets), rotate, flip, an optional filmstrip, and a floating toolbar you can reposition, lock, or trim button by button.
- **Progressive decoding** — a fast low-resolution decode appears immediately, the full-quality image replaces it a moment later, and neighbours are pre-warmed so arrow-key navigation feels instant.
- **Info panel** (`I`) with dimensions, size, dates, path, tags, and how many times you've opened the item.

### File operations

- **Recycle Bin deletes** — single or bulk, always recoverable, never a hard unlink.
- **Copy, move, and rename** from inside the app, with the index following the file.
- **Create folders** and move items into any registered library folder.
- **Drag files out** of PhoTag into Explorer, Premiere Pro, or any drop target that accepts `CF_HDROP`.
- **Bulk action console** (`Ctrl+Shift+D`) — delete, copy, or move everything matching a filter set (date range, age in days, rating band, filename fragment, extension, size range).
- **Undo / redo** (`Ctrl+Z` / `Ctrl+Y`) across tag and rating operations.
- **Export metadata** — dump every item's tags and ratings to JSON.
- **Open in default app** or **reveal in Explorer**.

### Windows integration

- Mica backdrop on Windows 11 (build 22000+), Acrylic blur-behind on Windows 10, opaque fallback everywhere else — each step degrades gracefully.
- Dark title bar and system accent colour pickup, or choose from built-in accent presets.
- Session restore: selected folder, scroll position, expanded nodes, sidebar width, and every active filter come back exactly as you left them.
- **Built-in updater** — checks GitHub once per launch and installs new versions in place. See [Updates](#updates).

## Install

1. Download `photag.exe` from the [latest release](../../releases/latest).
2. Put it anywhere you like and run it.

There is no installer and no dependency to preinstall. On first launch Windows SmartScreen may warn about an unrecognised publisher — the binary is unsigned. Choose **More info → Run anyway**, or [build it yourself](#build-from-source).

To play video or generate video thumbnails, see [Video and audio support](#video-and-audio-support).

## Build from source

Requires a [Rust toolchain](https://rustup.rs/) (stable, MSVC target) and the Visual Studio C++ build tools. SQLite is compiled in via `rusqlite`'s bundled feature — no separate install needed.

```sh
git clone https://github.com/OleksandrShabaldas/PhoTag.git
cd PhoTag
cargo build --release
```

The binary lands at `target/release/photag.exe`. The release profile uses `opt-level = 3` with fat LTO, so a clean build takes a few minutes; incremental rebuilds are much faster.

## Quick start

1. Launch PhoTag and click **Add folder** in the sidebar. Pick a folder containing photos.
2. Scanning starts immediately — thumbnails fill in progressively while you browse.
3. Switch the sidebar to the **Tags** tab and create a tag. Nest tags by dragging one onto another.
4. Select thumbnails (click, `Ctrl`+click, `Shift`+click, or `Ctrl+A`) and assign tags from the sidebar or the context menu.
5. Click a tag to filter the gallery by it. Hold `Ctrl` (configurable) and click to *exclude* it instead.
6. Press `Space` on a thumbnail to open the full preview. Arrow keys move through the library.

## Keyboard shortcuts

### Gallery

| Key | Action |
| --- | --- |
| `Space` | Open the selected item in preview *(the key is configurable in Settings)* |
| `←` `→` `↑` `↓` | Move the selection |
| `Home` / `End` | Jump to first / last item |
| `Ctrl+A` | Select all |
| `0`–`5` | Set rating on the whole selection (`0` clears it) |
| `F1`–`F9` | Toggle a bound tag on the selection |
| `Delete` / `Backspace` | Delete the selection (to the Recycle Bin, with confirmation) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+Shift+D` | Open the bulk action console |

### Preview

| Key | Action |
| --- | --- |
| `←` `→` | Previous / next item |
| `I` | Toggle the info panel |
| `F2` | Rename the file |
| `Delete` / `Backspace` | Delete and advance to the next item |
| `Esc` | Back to the gallery |

## How tagging works

Tags form a tree, and each tag stores its own full path (`Animals>Birds>Owls`) so filtering stays a single indexed lookup rather than a recursive walk.

Two behaviours are worth understanding:

**Ancestor auto-apply.** Assigning *Owls* also records *Birds* and *Animals* against that file, flagged as automatic. This means a filter on *Animals* finds every owl photo without any recursive query. Because the flag distinguishes automatic from manual assignments, removing *Owls* later cleans up the automatic ancestors while leaving any *Animals* tag you applied deliberately untouched.

**Associations.** Separately from the hierarchy, any tag can list associated tags that get applied alongside it. Useful for cross-cutting concepts that aren't parent-child — assigning *Wedding* can bring in *People* and *Event* without forcing them into one branch.

Merging, renaming, re-parenting, and deleting all cascade correctly through both mechanisms.

## Search and filtering

The search box matches against **filenames**, **full paths**, and **tag names** — each scope independently toggleable and remembered between sessions. Filters combine:

- **Tags** — include and exclude, any depth
- **Date taken** — arbitrary from/to range
- **Rating** — minimum and maximum band
- **Media type** — images, video, audio
- **Extension** — include and exclude specific formats
- **Folder or subfolder** — including paths not registered as library roots

Everything you set survives a restart.

## AI tag suggestions

PhoTag ships an optional on-device tag suggester built on MobileNetV2 via [`tract`](https://github.com/sonos/tract). The ~14 MB ONNX model is **not bundled** — it's fetched from the ONNX Model Zoo into your local data directory the first time you enable the feature, and everything after that runs offline on your CPU. Inference happens on a background thread, so the UI stays responsive.

Beyond the network's own labels, PhoTag analyses brightness, colour warmth, and saturation to fill **mandatory tag groups** — groups you define where exactly one option should always be chosen (for example *Lighting: bright / dim / dark*). Suggestions are always proposals: nothing is written until you accept it.

Enable it, define groups, and toggle automatic analysis of previewed photos in **Settings → AI**.

## Updates

PhoTag checks this repository's releases once per launch and offers anything newer. The check is a single background request; **nothing is downloaded or installed until you say so.**

When you accept, PhoTag downloads the new build, verifies it against the release's published `SHA256SUMS.txt`, and only then swaps it in — a download that doesn't match its checksum is discarded rather than installed. The replacement happens as PhoTag exits, after your session is saved, and the app relaunches on the new version. Your previous build is kept next to the new one as `photag.old` and removed on a later launch, so a bad update is one rename away from being undone.

Manage it under **Settings → Updates**: toggle the startup check, run a check on demand, and see which version you're on.

A few things worth knowing:

- **Turning the check off** (Settings → Updates) stops all network activity related to updates. PhoTag makes no other outbound requests except the optional AI model download.
- **PhoTag must be able to write to its own folder.** Portable installs in your user profile are fine; a copy under `Program Files` will fail to self-update without elevation, and PhoTag will say so rather than half-applying anything.
- **Checksums protect against corrupted or truncated downloads, not against a compromised release.** The binaries are unsigned, so the checksum and the release itself come from the same place. If that matters for your threat model, build from source instead — the toggle is there precisely so you can turn this off.

## Video and audio support

Audio playback works out of the box. **Video playback and video thumbnails require `ffmpeg` and `ffprobe`** — these are not redistributed with PhoTag.

PhoTag looks for them in this order:

1. An `ffmpeg\bin\` folder next to `photag.exe`
2. Your system `PATH`

Get a Windows build from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) or [BtbN](https://github.com/BtbN/FFmpeg-Builds/releases), then either add it to your `PATH` or drop `ffmpeg.exe` and `ffprobe.exe` into `ffmpeg\bin\` beside the executable.

Without them, PhoTag still indexes video files and hands playback off to your system's default player; only in-app playback and generated video thumbnails are unavailable.

The player decodes video and audio in two separate ffmpeg subprocesses, keeps them in sync, honours container rotation metadata, and supports variable playback speed.

## Writing tags into your files

By default, tags live only in PhoTag's database — your originals are untouched.

When you do ask PhoTag to write metadata:

- **JPEG** — tags are written into the file as IPTC keywords
- **Everything else** — tags go to an XMP sidecar (`photo.jpg` → `photo.xmp`), leaving the original byte-identical

In both cases the file's original modification time is restored afterwards, so date-based sorting doesn't jump every time you adjust a tag.

## Where PhoTag stores its data

Everything lives under `%LOCALAPPDATA%\PhoTag\`:

| Path | Contents |
| --- | --- |
| `photag.db` | SQLite index — folders, media, tags, ratings |
| `settings.json` | Preferences, hotkeys, toolbar layout, AI groups |
| `session.json` | Last session state (selection, scroll, filters) |
| `thumbs\` | Generated thumbnail cache (JPEG, keyed by path hash) |
| `models\` | Downloaded ONNX model, if AI tagging was enabled |
| `updates\` | Staging area for a downloaded update; cleared on the next launch |

Deleting `thumbs\` is always safe — thumbnails regenerate on demand. Deleting `photag.db` resets your library, tags and all; back it up if your tagging matters to you. **No media file is ever stored inside PhoTag's data directory.**

## Project layout

```
src/
├── main.rs         Entry point, top-level frame loop, modals, settings UI
├── app.rs          AppState, pending-action dispatch, undo/redo stacks
├── db/
│   ├── mod.rs      Connection setup, schema, migrations
│   └── queries.rs  All SQL — media, tags, filters, statistics
├── ui/
│   ├── gallery.rs  Virtualised thumbnail grid, selection, texture cache
│   ├── preview.rs  Full-screen viewer, decode pipeline, floating toolbar
│   ├── sidebar.rs  Folder tree and tag tree, drag-and-drop reordering
│   ├── filterbar.rs Search box, sort, and every filter control
│   └── info.rs     Metadata panel
├── scan.rs         Multi-threaded folder scanning and indexing
├── thumbs.rs       Thumbnail generation, fast EXIF orientation parser
├── video.rs        ffmpeg-backed video/audio playback
├── ai.rs           MobileNetV2 inference and image analysis
├── update.rs       Self-update: GitHub release check, download, verify, swap
├── xmp.rs          IPTC / XMP sidecar metadata writing
├── watcher.rs      Filesystem change notifications
├── drag.rs         Win32 COM drag-out (CF_HDROP)
├── winfx.rs        Mica / Acrylic backdrop, dark title bar, system accent
├── theme.rs        Design tokens, fonts, colours
├── settings.rs     Persisted preferences
└── session.rs      Session restore
```

## Performance notes

A few deliberate choices that matter on large libraries:

- **mimalloc as the global allocator.** Windows' default heap degrades badly with tens of thousands of small live allocations — exactly what a media library of `MediaItem` structs produces. Swapping the allocator removed a ~60 ms per-frame cost on a loaded library.
- **SQLite in WAL mode** with `synchronous = NORMAL` and a 4 MB autocheckpoint, plus a separate read connection for the UI thread. The scanner writes while the gallery reads, with neither blocking the other.
- **Two-stage image decode.** A quarter-resolution decode (free on JPEG via DCT scaling, roughly 16× faster) shows instantly, then the full decode swaps in. Neighbours are pre-warmed once the current item is sharp.
- **8×8 micro-thumbnails stored as BLOBs** in the database, so the grid has something to draw the moment a row scrolls into view.
- **Scan workers capped at `cores − 1`** so decode threads can't starve the UI thread into "Not Responding".

## Troubleshooting

**Videos won't play in-app.** `ffmpeg`/`ffprobe` aren't being found — see [Video and audio support](#video-and-audio-support).

**Thumbnails are missing for some videos.** Same cause: video thumbnails are extracted with ffmpeg during scanning. Install it, then re-scan the folder.

**SmartScreen blocks the download.** The executable is unsigned. **More info → Run anyway**, or build from source.

**A folder's contents look stale.** Use **Rescan** on that folder in the sidebar. The watcher covers most changes, but a rescan is authoritative.

**The window looks flat, without translucency.** Mica requires Windows 11 build 22000+; Windows 10 gets Acrylic instead, and if neither is available PhoTag renders an opaque window. This is expected, not a bug.

**An update won't install.** PhoTag needs write access to the folder holding `photag.exe` — move it somewhere in your user profile, or download the new build manually. If the swap fails on exit, the reason is written to `%LOCALAPPDATA%\PhoTag\update-error.txt`.

**"GitHub rate limit reached".** Update checks are unauthenticated and share GitHub's 60-requests-per-hour-per-IP budget. It resets within the hour; nothing is wrong with your install.

## License

[MIT](LICENSE) © Oleksandr Shabaldas

PhoTag depends on `ffmpeg` at runtime for video features but does not bundle or redistribute it; ffmpeg is licensed separately under the LGPL or GPL depending on the build you install.
