//! FFmpeg discovery and FFprobe media metadata.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) struct Info {
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
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate:stream_tags=rotate:stream_side_data=rotation",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
            &path.to_string_lossy(),
        ])
        .output()?;
    if !output.status.success() {
        return Err(super::PlaybackError::ProbeFailed(path.to_path_buf()));
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let stream = json["streams"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| super::PlaybackError::ProbeFailed(path.to_path_buf()))?;
    let raw_width = stream["width"].as_u64().unwrap_or(0) as u32;
    let raw_height = stream["height"].as_u64().unwrap_or(0) as u32;
    if raw_width == 0 || raw_height == 0 {
        return Err(super::PlaybackError::ProbeFailed(path.to_path_buf()));
    }
    let rotation = stream
        .get("tags")
        .and_then(|value| value.get("rotate"))
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| {
            stream
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
    let fps = stream["r_frame_rate"]
        .as_str()
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
