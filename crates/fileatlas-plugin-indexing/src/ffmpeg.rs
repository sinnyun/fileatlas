//! FFmpeg utilities shared by the migrated indexing and media plugins.

use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn find_binary(name: &str) -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Check right next to the exe
            let candidate = exe_dir.join(format!("{}.exe", name));
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
            // Check in ffmpeg/bin subfolder
            let candidate = exe_dir
                .join("ffmpeg")
                .join("bin")
                .join(format!("{}.exe", name));
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    // Fallback: rely on system PATH
    name.to_string()
}

fn ffmpeg_cmd(program: &str) -> Command {
    let binary = find_binary(program);
    let mut cmd = Command::new(binary);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Check if ffmpeg is available (bundled or system PATH).
pub fn is_ffmpeg_available() -> bool {
    ffmpeg_cmd("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Extract a thumbnail frame from a video as JPEG.
///
/// Performance notes:
///   - `-ss 1 -i` (seek BEFORE input): ffmpeg uses the container index to jump
///     to ~1s in, decoding only one keyframe instead of demuxing from frame 0.
///     This is 5-20x faster for long videos. Reading 1s in also avoids the
///     all-black opening frame common in many videos.
///   - `-ss 1` is best-effort; for very short videos ffmpeg falls back to
///     extracting whatever frame it can.
pub fn extract_video_thumbnail(video_path: &str, output_path: &str) -> bool {
    let ok = ffmpeg_cmd("ffmpeg")
        .args([
            "-ss",
            "1",
            "-i",
            video_path,
            "-vframes",
            "1",
            "-vf",
            "scale=300:-2",
            "-q:v",
            "3",
            "-y",
            output_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // For videos shorter than 1 second, the seek can fail. Retry from frame 0.
    if !ok || !std::path::Path::new(output_path).exists() {
        return ffmpeg_cmd("ffmpeg")
            .args([
                "-i",
                video_path,
                "-vframes",
                "1",
                "-vf",
                "scale=300:-2",
                "-q:v",
                "3",
                "-y",
                output_path,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runtime smoke check for installations that have FFmpeg and its testsrc
    /// lavfi filter available. Kept ignored so normal unit tests need no tool.
    #[test]
    #[ignore = "requires a local FFmpeg executable with lavfi testsrc"]
    fn extracts_a_thumbnail_from_a_valid_generated_video() {
        assert!(is_ffmpeg_available(), "FFmpeg must be available on PATH");
        let root = std::env::temp_dir().join(format!(
            "fileatlas-ffmpeg-smoke-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let video = root.join("sample.mp4");
        let thumbnail = root.join("sample.jpg");
        let generated = ffmpeg_cmd("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=96x64:rate=5",
                "-t",
                "1.4",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
                "-threads",
                "1",
                "-y",
            ])
            .arg(&video)
            .output()
            .unwrap();
        assert!(
            generated.status.success(),
            "FFmpeg sample generation failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        assert!(extract_video_thumbnail(
            video.to_string_lossy().as_ref(),
            thumbnail.to_string_lossy().as_ref()
        ));
        assert!(std::fs::metadata(&thumbnail).unwrap().len() > 100);
        let _ = std::fs::remove_dir_all(root);
    }
}
