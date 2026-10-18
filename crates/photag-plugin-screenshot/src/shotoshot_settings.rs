//! Narrow bridge to Shotoshot's documented on-disk settings format.
//! Only the output-folder field is changed; PhoTag does not embed Shotoshot.

#[cfg(windows)]
pub fn set_save_folder(folder: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0},
            Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
            System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject},
        },
        core::w,
    };

    struct SettingsLock(HANDLE);
    impl Drop for SettingsLock {
        fn drop(&mut self) {
            unsafe {
                let _ = ReleaseMutex(self.0);
                let _ = CloseHandle(self.0);
            }
        }
    }

    let lock = unsafe { CreateMutexW(None, false, w!("Local\\Shotoshot.SettingsWrite")) }
        .map_err(|error| format!("无法锁定 Shotoshot 设置：{error}"))?;
    let guard = match unsafe { WaitForSingleObject(lock, 2_000) } {
        WAIT_OBJECT_0 | WAIT_ABANDONED => SettingsLock(lock),
        _ => {
            unsafe {
                let _ = CloseHandle(lock);
            }
            return Err("Shotoshot 设置正在使用，请稍后重试".into());
        }
    };

    let settings_path = dirs::data_local_dir()
        .ok_or_else(|| "无法确定 Shotoshot 设置目录".to_owned())?
        .join("Shotoshot")
        .join("settings.txt");
    let original = std::fs::read_to_string(&settings_path)
        .map_err(|error| format!("无法读取 Shotoshot 设置（请先启动一次 Shotoshot）：{error}"))?;
    let mut lines: Vec<String> = original.lines().map(str::to_owned).collect();
    let version = lines
        .first()
        .and_then(|header| {
            header
                .strip_prefix("shotoshot-settings-")
                .or_else(|| header.strip_prefix("shotshot-settings-"))
        })
        .and_then(|version| version.parse::<u8>().ok())
        .filter(|version| (1..=15).contains(version))
        .ok_or_else(|| "Shotoshot 设置格式不受支持，未做更改".to_owned())?;
    let expected_fields = match version {
        1 => 5,
        2 => 6,
        3 => 9,
        4 => 10,
        5 => 13,
        6 => 15,
        7 => 17,
        8 => 18,
        9 => 20,
        10 | 11 => 22,
        12 => 24,
        13 => 25,
        14 => 27,
        _ => 28,
    };
    if lines.len() != expected_fields {
        return Err("Shotoshot 设置格式不完整，未做更改".into());
    }
    let encoded_folder = folder
        .as_os_str()
        .encode_wide()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",");
    lines[4] = encoded_folder;
    let mut updated = lines.join("\n");
    updated.push('\n');

    let temp_path = settings_path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp_path, updated)
        .map_err(|error| format!("无法写入临时 Shotoshot 设置：{error}"))?;
    let wide = |path: &std::path::Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    unsafe {
        MoveFileExW(
            windows::core::PCWSTR(wide(&temp_path).as_ptr()),
            windows::core::PCWSTR(wide(&settings_path).as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        format!("无法更新 Shotoshot 保存目录：{error}")
    })?;
    drop(guard);
    Ok(())
}

#[cfg(not(windows))]
pub fn set_save_folder(_folder: &std::path::Path) -> Result<(), String> {
    Ok(())
}
