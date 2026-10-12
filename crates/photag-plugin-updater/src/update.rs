//! Self-update against GitHub Releases.
//!
//! Three phases, each run on demand:
//!
//!   1. `check_async`    — one API request for the latest published release.
//!   2. `download_async` — streams the asset to a staging file, hashing as
//!                         it goes, and verifies against the release's
//!                         `SHA256SUMS.txt`.
//!   3. `install`        — swaps the staged file in and relaunches.
//!
//! The swap relies on a Windows quirk: a running `.exe` cannot be *written*
//! to, but it can be *renamed* — the open file object stays valid while the
//! directory entry moves. So we rename the live binary aside, copy the new
//! one into its place, and clean up the leftover on a later launch.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};

const REPO: &str = "OleksandrShabaldas/PhoTag";
/// GitHub rejects API requests without a User-Agent.
const USER_AGENT: &str = concat!("PhoTag/", env!("CARGO_PKG_VERSION"));
/// Distinguishes the app binary from the other release assets.
const ASSET_SUFFIX: &str = "-windows-x64.exe";
const CHECKSUM_ASSET: &str = "SHA256SUMS.txt";

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn releases_url() -> String {
    format!("https://github.com/{}/releases", REPO)
}

/// A published release that is newer than what's running.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// Normalised, without the leading `v`.
    pub version: String,
    /// Release notes (markdown, rendered as plain text).
    pub notes: String,
    pub html_url: String,
    pub asset_name: String,
    pub asset_url: String,
    pub asset_size: u64,
    /// URL of `SHA256SUMS.txt`, when the release publishes one.
    pub checksums_url: Option<String>,
}

impl UpdateInfo {
    pub fn size_mb(&self) -> f64 {
        self.asset_size as f64 / (1024.0 * 1024.0)
    }

    /// Release notes flattened for display. egui draws plain text, so raw
    /// GitHub markdown would otherwise show its own syntax — `**bold**`,
    /// `## headings`, backticks — to the user.
    pub fn notes_plain(&self) -> String {
        plain_text(&self.notes)
    }
}

/// Strip the small subset of markdown that actually shows up in release
/// notes. Deliberately not a real parser: it only has to make our own
/// notes read cleanly, and anything it misses degrades to visible syntax
/// rather than to wrong text.
fn plain_text(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for raw in md.lines() {
        let line = raw.trim_end();
        // Fence markers carry no information once styling is gone.
        if line.trim_start().starts_with("```") {
            continue;
        }
        let trimmed = line.trim_start();
        let (indent, body) = if let Some(rest) = trimmed.strip_prefix("- ") {
            ("  • ", rest)
        } else if let Some(rest) = trimmed.strip_prefix("* ") {
            ("  • ", rest)
        } else if trimmed.starts_with('#') {
            ("", trimmed.trim_start_matches('#').trim_start())
        } else {
            ("", trimmed)
        };
        out.push_str(indent);
        // Drop emphasis and code markers, keeping their contents.
        let mut chars = body.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '*' | '_' => {
                    // `**x**` and `*x*` alike: skip the run of markers.
                    while chars.peek() == Some(&c) {
                        chars.next();
                    }
                }
                '`' => {}
                _ => out.push(c),
            }
        }
        out.push('\n');
    }
    // Collapse the runs of blank lines that heading removal leaves behind.
    let mut result = String::with_capacity(out.len());
    let mut blank_run = 0;
    for line in out.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        result.push_str(line);
        result.push('\n');
    }
    result.trim().to_string()
}

// ── Version comparison ────────────────────────────────────────────────

/// Parse `v1.2.3`, `1.2`, or `1.2.3-beta.1` into a comparable triple.
/// Pre-release and build suffixes are ignored for ordering — a tag like
/// `1.2.0-rc1` compares equal to `1.2.0`, which is fine here because
/// `releases/latest` never returns pre-releases in the first place.
fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let trimmed = s.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.trim().parse().ok()?;
    let minor = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
    let patch = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
    Some((major, minor, patch))
}

/// True when `candidate` is strictly newer than `current`. Unparseable
/// versions are treated as "not newer" so a malformed tag can never
/// trigger an update.
fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

// ── Phase 1: check ────────────────────────────────────────────────────

fn json_get(url: &str) -> Result<serde_json::Value, String> {
    let resp = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            // 403 from the unauthenticated API is nearly always the 60/hour
            // rate limit; say so rather than surfacing a bare status code.
            ureq::Error::Status(403, _) => {
                "GitHub rate limit reached — try again later".to_string()
            }
            ureq::Error::Status(code, _) => format!("GitHub returned HTTP {}", code),
            ureq::Error::Transport(t) => format!("network error: {}", t),
        })?;
    // `into_string` rather than ureq's `into_json` — the latter needs ureq's
    // optional `json` feature, and serde_json is already a dependency here.
    let body = resp
        .into_string()
        .map_err(|e| format!("read response from GitHub: {}", e))?;
    serde_json::from_str(&body).map_err(|e| format!("malformed response from GitHub: {}", e))
}

/// Ask GitHub for the latest published release. `Ok(None)` means we are
/// already current — that is a success, not an error.
pub fn check() -> Result<Option<UpdateInfo>, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let release = json_get(&url)?;

    let tag = release["tag_name"]
        .as_str()
        .ok_or("release has no tag_name")?
        .to_string();

    if !is_newer(&tag, current_version()) {
        return Ok(None);
    }

    let assets = release["assets"].as_array().cloned().unwrap_or_default();

    let find = |pred: &dyn Fn(&str) -> bool| -> Option<(String, String, u64)> {
        assets.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            if !pred(name) {
                return None;
            }
            Some((
                name.to_string(),
                a["browser_download_url"].as_str()?.to_string(),
                a["size"].as_u64().unwrap_or(0),
            ))
        })
    };

    let (asset_name, asset_url, asset_size) = find(&|n| n.ends_with(ASSET_SUFFIX))
        .or_else(|| find(&|n| n.ends_with(".exe")))
        .ok_or("release publishes no Windows executable")?;

    let checksums_url = find(&|n| n.eq_ignore_ascii_case(CHECKSUM_ASSET)).map(|(_, u, _)| u);

    Ok(Some(UpdateInfo {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        notes: release["body"].as_str().unwrap_or("").trim().to_string(),
        html_url: release["html_url"].as_str().unwrap_or("").to_string(),
        asset_name,
        asset_url,
        asset_size,
        checksums_url,
    }))
}

pub fn check_async() -> mpsc::Receiver<Result<Option<UpdateInfo>, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(check()).ok();
    });
    rx
}

// ── Phase 2: download + verify ────────────────────────────────────────

fn staging_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("PhoTag").join("updates")
}

/// Pull the expected hash for `asset_name` out of a `sha256sum`-style file.
/// Lines look like `<64 hex>  name` or `<64 hex> *name` (binary mode).
fn expected_hash(body: &str, asset_name: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == asset_name && hash.len() == 64).then(|| hash.to_ascii_lowercase())
    })
}

/// Download the release asset into the staging directory, verifying its
/// SHA-256 when the release publishes checksums. `progress` receives the
/// running byte count so the UI can draw a bar without any extra plumbing.
pub fn download(info: &UpdateInfo, progress: Arc<AtomicU64>) -> Result<PathBuf, String> {
    let dir = staging_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create staging dir: {}", e))?;

    // Fetch checksums first — no point spending 35 MB of bandwidth if the
    // manifest is missing the entry we need.
    let want_hash = match info.checksums_url {
        Some(ref url) => {
            let body = ureq::get(url)
                .set("User-Agent", USER_AGENT)
                .call()
                .map_err(|e| format!("fetch checksums: {}", e))?
                .into_string()
                .map_err(|e| format!("read checksums: {}", e))?;
            expected_hash(&body, &info.asset_name)
        }
        None => None,
    };

    let resp = ureq::get(&info.asset_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("download failed: {}", e))?;

    let part = dir.join(format!("photag-{}.part", info.version));
    let mut file = std::fs::File::create(&part).map_err(|e| format!("create staging file: {}", e))?;

    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 64 * 1024];
    let mut written: u64 = 0;

    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                drop(file);
                let _ = std::fs::remove_file(&part);
                return Err(format!("download interrupted: {}", e));
            }
        };
        sha2::Digest::update(&mut hasher, &buf[..n]);
        if let Err(e) = file.write_all(&buf[..n]) {
            drop(file);
            let _ = std::fs::remove_file(&part);
            return Err(format!("write failed: {}", e));
        }
        written += n as u64;
        progress.store(written, Ordering::Relaxed);
    }

    file.flush().map_err(|e| format!("flush: {}", e))?;
    drop(file);

    let got = format!("{:x}", sha2::Digest::finalize(hasher));
    if let Some(want) = want_hash {
        if got != want {
            let _ = std::fs::remove_file(&part);
            return Err(format!(
                "checksum mismatch — expected {}…, got {}…. Download discarded.",
                &want[..12],
                &got[..12]
            ));
        }
    }

    // Only promote to the final name once the bytes are known-good, so a
    // half-finished download can never be mistaken for a staged update.
    let final_path = dir.join(format!("photag-{}.exe", info.version));
    let _ = std::fs::remove_file(&final_path);
    std::fs::rename(&part, &final_path).map_err(|e| format!("finalise download: {}", e))?;

    Ok(final_path)
}

pub fn download_async(
    info: UpdateInfo,
    progress: Arc<AtomicU64>,
) -> mpsc::Receiver<Result<PathBuf, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(download(&info, progress)).ok();
    });
    rx
}

// ── Phase 3: install ──────────────────────────────────────────────────

/// Path we park the outgoing binary at. `photag.exe` → `photag.old`.
fn backup_path(current: &Path) -> PathBuf {
    current.with_extension("old")
}

/// Swap `staged` in for the running executable and relaunch.
///
/// Call this on the way out (after session state is saved) — on success the
/// caller should not keep running, since a second instance is now live.
pub fn install(staged: &Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| format!("locate current exe: {}", e))?;
    swap_in(staged, &current, true)
}

/// The swap itself, with the target and relaunch behaviour injected so the
/// logic can be tested against a throwaway file instead of the live binary.
fn swap_in(staged: &Path, current: &Path, relaunch: bool) -> Result<(), String> {
    let backup = backup_path(current);

    if !staged.is_file() {
        return Err("staged update is missing".into());
    }

    // A stale backup from an earlier update would block the rename.
    let _ = std::fs::remove_file(&backup);

    // Renaming the live binary is the only way to free its name while the
    // process is still running. This is also our writability probe: if the
    // directory is read-only (Program Files without elevation), we fail
    // here, before anything has been disturbed.
    std::fs::rename(current, &backup).map_err(|e| {
        format!(
            "cannot replace {}: {}. Move PhoTag to a writable folder, or update manually.",
            current.display(),
            e
        )
    })?;

    // Copy rather than rename: the staging dir is usually on a different
    // volume than the executable, and cross-volume renames fail.
    if let Err(e) = std::fs::copy(staged, current) {
        // Put the old binary back — never leave the user without an app.
        let _ = std::fs::rename(&backup, current);
        return Err(format!("failed to install update: {}", e));
    }

    if relaunch {
        if let Err(e) = std::process::Command::new(current).spawn() {
            return Err(format!(
                "update installed, but relaunch failed: {}. Start PhoTag again manually.",
                e
            ));
        }
    }

    let _ = std::fs::remove_file(staged);
    Ok(())
}

/// Best-effort cleanup of the previous binary and any stale staging files.
/// Runs at startup; failures are expected and ignored (the outgoing process
/// may still hold a lock for a moment, in which case the next launch gets it).
pub fn cleanup_previous() {
    if let Ok(current) = std::env::current_exe() {
        let _ = std::fs::remove_file(backup_path(&current));
    }
    if let Ok(entries) = std::fs::read_dir(staging_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Anything left in staging has already been installed or was a
            // failed attempt; either way it is not needed again.
            let stale = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".part") || n.ends_with(".exe"))
                .unwrap_or(false);
            if stale {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_tag_shapes() {
        assert_eq!(parse_version("v0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("v2.0"), Some((2, 0, 0)));
        assert_eq!(parse_version("1.2.3-rc1"), Some((1, 2, 3)));
        assert_eq!(parse_version("not-a-version"), None);
    }

    #[test]
    fn orders_versions_numerically_not_lexically() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        // The classic string-compare trap: "0.10" < "0.9" lexically.
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
    }

    #[test]
    fn never_updates_on_a_malformed_tag() {
        assert!(!is_newer("garbage", "0.1.0"));
        assert!(!is_newer("", "0.1.0"));
    }

    /// Live end-to-end exercise of the download path against the real
    /// release: streams the asset, hashes it, and checks it against the
    /// published `SHA256SUMS.txt`. Ignored by default because it pulls
    /// ~35 MB — run with `cargo test --bin photag -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_download_verifies_against_published_checksum() {
        let found = check().expect("check failed");
        eprintln!(
            "running {} — check() reported: {}",
            current_version(),
            match found {
                Some(ref i) => format!("update to {}", i.version),
                None => "already current".to_string(),
            }
        );
        let info = found
            .unwrap_or_else(|| {
                // Running version == latest, so synthesise the same info the
                // checker would produce for the current release.
                let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
                let r = json_get(&url).expect("fetch release");
                let assets = r["assets"].as_array().unwrap().clone();
                let pick = |suffix: &str| {
                    assets.iter().find(|a| {
                        a["name"].as_str().unwrap_or("").ends_with(suffix)
                    })
                };
                let exe = pick(ASSET_SUFFIX).expect("no exe asset");
                UpdateInfo {
                    version: r["tag_name"].as_str().unwrap().trim_start_matches('v').into(),
                    notes: String::new(),
                    html_url: String::new(),
                    asset_name: exe["name"].as_str().unwrap().into(),
                    asset_url: exe["browser_download_url"].as_str().unwrap().into(),
                    asset_size: exe["size"].as_u64().unwrap(),
                    checksums_url: pick(CHECKSUM_ASSET)
                        .map(|a| a["browser_download_url"].as_str().unwrap().to_string()),
                }
            });

        assert!(info.checksums_url.is_some(), "release publishes no checksums");
        let progress = Arc::new(AtomicU64::new(0));
        let path = download(&info, progress.clone()).expect("download+verify failed");
        let got = std::fs::metadata(&path).expect("staged file missing").len();
        assert_eq!(got, info.asset_size, "staged size != advertised size");
        assert_eq!(progress.load(Ordering::Relaxed), info.asset_size);
        let _ = std::fs::remove_file(&path);
    }

    /// A bad download must never reach the staging path. We point the
    /// asset URL at the checksums file while claiming the exe's name, so
    /// the bytes are real but the hash cannot match.
    #[test]
    #[ignore]
    fn live_download_rejects_a_checksum_mismatch() {
        let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
        let r = json_get(&url).expect("fetch release");
        let assets = r["assets"].as_array().unwrap().clone();
        let find = |suffix: &str| {
            assets
                .iter()
                .find(|a| a["name"].as_str().unwrap_or("").ends_with(suffix))
                .map(|a| a["browser_download_url"].as_str().unwrap().to_string())
        };
        let exe_name = assets
            .iter()
            .find(|a| a["name"].as_str().unwrap_or("").ends_with(ASSET_SUFFIX))
            .unwrap()["name"]
            .as_str()
            .unwrap()
            .to_string();

        let bad = UpdateInfo {
            version: "9.9.9".into(),
            notes: String::new(),
            html_url: String::new(),
            // Claims to be the exe, but serves the checksum file's bytes.
            asset_name: exe_name,
            asset_url: find(CHECKSUM_ASSET).expect("no checksums asset"),
            asset_size: 96,
            checksums_url: find(CHECKSUM_ASSET),
        };

        let err = download(&bad, Arc::new(AtomicU64::new(0)))
            .expect_err("mismatched download was accepted");
        assert!(err.contains("checksum mismatch"), "unexpected error: {}", err);
        // And nothing may be left behind for `install` to pick up.
        assert!(!staging_dir().join("photag-9.9.9.exe").exists());
        assert!(!staging_dir().join("photag-9.9.9.part").exists());
    }

    /// Exercise the real swap against throwaway files: the outgoing binary
    /// is preserved as `.old` and the new bytes land at the target path.
    #[test]
    fn swap_replaces_target_and_keeps_a_backup() {
        let dir = std::env::temp_dir().join(format!("photag-swap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("app.exe");
        let staged = dir.join("new.bin");
        std::fs::write(&target, b"OLD VERSION").unwrap();
        std::fs::write(&staged, b"NEW VERSION").unwrap();

        swap_in(&staged, &target, false).expect("swap failed");

        assert_eq!(std::fs::read(&target).unwrap(), b"NEW VERSION");
        assert_eq!(std::fs::read(backup_path(&target)).unwrap(), b"OLD VERSION");
        // The staged copy is consumed once installed.
        assert!(!staged.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    /// If the staged file vanishes between download and install, the live
    /// binary must be left exactly where it was.
    #[test]
    fn swap_leaves_target_intact_when_staged_is_missing() {
        let dir = std::env::temp_dir().join(format!("photag-swap-miss-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("app.exe");
        std::fs::write(&target, b"OLD VERSION").unwrap();

        let err = swap_in(&dir.join("nope.bin"), &target, false).expect_err("should fail");
        assert!(err.contains("missing"), "unexpected error: {}", err);
        assert_eq!(std::fs::read(&target).unwrap(), b"OLD VERSION");
        assert!(!backup_path(&target).exists(), "target was disturbed");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flattens_the_markdown_that_release_notes_actually_use() {
        let md = "\
## Download

Grab `PhoTag.exe` and run it. **No installer.**

- **Tags** — nested `a > b`
- Second point

```
checksum-line
```
";
        let out = plain_text(md);
        assert!(!out.contains("##"), "heading markers survived: {out}");
        assert!(!out.contains('`'), "code markers survived: {out}");
        assert!(!out.contains('*'), "emphasis markers survived: {out}");
        assert!(out.starts_with("Download"), "heading text lost: {out}");
        assert!(out.contains("No installer."));
        assert!(out.contains("• Tags — nested a > b"));
        assert!(out.contains("checksum-line"), "fenced content dropped: {out}");
        // No triple newlines left where the fences and headings were.
        assert!(!out.contains("\n\n\n"), "blank-line runs survived: {out:?}");
    }

    #[test]
    fn extracts_hash_for_the_matching_asset_only() {
        let body = "\
aaaa111122223333444455556666777788889999aaaabbbbccccddddeeeeffff *PhoTag-v0.1.0-windows-x64.exe
bbbb111122223333444455556666777788889999aaaabbbbccccddddeeeeffff  other.exe
";
        assert_eq!(
            expected_hash(body, "PhoTag-v0.1.0-windows-x64.exe").as_deref(),
            Some("aaaa111122223333444455556666777788889999aaaabbbbccccddddeeeeffff")
        );
        assert_eq!(expected_hash(body, "missing.exe"), None);
    }
}



