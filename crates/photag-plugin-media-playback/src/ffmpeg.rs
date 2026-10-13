//! FFmpeg discovery and FFprobe media metadata.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) struct Info {
    pub has_video: bool,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    pub rotation_quarters: u8,
}

pub(crate) fn probe(path: &Path) -> Result<Info, super::PlaybackError> {
    let output = command("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,width,height,r_frame_rate:stream_tags=rotate:stream_side_data=rotation",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
            &path.to_string_lossy(),
        ])
        .output()
        .map_err(|_| super::PlaybackError::ProbeFailed(path.to_path_buf()))?;
    if !output.status.success() {
        return Err(super::PlaybackError::ProbeFailed(path.to_path_buf()));
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| super::PlaybackError::ProbeFailed(path.to_path_buf()))?;
    parse_info(&json, path)
}

fn parse_info(json: &serde_json::Value, path: &Path) -> Result<Info, super::PlaybackError> {
    let streams = json["streams"]
        .as_array()
        .ok_or_else(|| super::PlaybackError::ProbeFailed(path.to_path_buf()))?;
    let video = streams
        .iter()
        .find(|stream| stream["codec_type"] == "video");
    let has_audio = streams.iter().any(|stream| stream["codec_type"] == "audio");
    if video.is_none() && !has_audio {
        return Err(super::PlaybackError::ProbeFailed(path.to_path_buf()));
    }
    let stream = video;
    let raw_width = stream
        .and_then(|stream| stream["width"].as_u64())
        .unwrap_or(0) as u32;
    let raw_height = stream
        .and_then(|stream| stream["height"].as_u64())
        .unwrap_or(0) as u32;
    let rotation = stream
        .and_then(|stream| {
            stream
                .get("tags")
                .and_then(|value| value.get("rotate"))
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<i64>().ok())
        })
        .or_else(|| {
            stream?
                .get("side_data_list")
                .and_then(|value| value.as_array())
                .and_then(|items| {
                    items
                        .iter()
                        .find_map(|item| item.get("rotation").and_then(|value| value.as_i64()))
                })
        })
        .unwrap_or(0);
    let rotation_quarters = ((rotation.rem_euclid(360) / 90) % 4) as u8;
    let (width, height) = if rotation_quarters % 2 == 1 {
        (raw_height, raw_width)
    } else {
        (raw_width, raw_height)
    };
    let fps = stream
        .and_then(|stream| stream["r_frame_rate"].as_str())
        .and_then(|rate| {
            if let Some((numerator, denominator)) = rate.split_once('/') {
                Some(numerator.parse::<f64>().ok()? / denominator.parse::<f64>().ok()?.max(1.0))
            } else {
                rate.parse().ok()
            }
        })
        .unwrap_or(30.0)
        .clamp(1.0, 120.0);
    let duration = json["format"]["duration"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
        .max(0.0);
    Ok(Info {
        has_video: stream.is_some(),
        width,
        height,
        fps,
        duration,
        rotation_quarters,
    })
}

pub(crate) fn command(program: &str) -> Command {
    let binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|dir| {
            [
                dir.join(format!("{program}.exe")),
                dir.join("ffmpeg")
                    .join("bin")
                    .join(format!("{program}.exe")),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(program));
    let mut command = Command::new(binary);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_audio_only_duration_without_inventing_video_dimensions() {
        let json = serde_json::json!({
            "streams": [{"codec_type": "audio"}],
            "format": {"duration": "207.25"}
        });
        let info = parse_info(&json, Path::new("ambient_nature.wav")).unwrap();

        assert!(!info.has_video);
        assert_eq!((info.width, info.height), (0, 0));
        assert_eq!(info.duration, 207.25);
    }

    #[test]
    fn rejects_metadata_without_audio_or_video_streams() {
        let json = serde_json::json!({
            "streams": [{"codec_type": "subtitle"}],
            "format": {"duration": "12.0"}
        });

        assert!(matches!(
            parse_info(&json, Path::new("captions.srt")),
            Err(super::super::PlaybackError::ProbeFailed(_))
        ));
    }
}
