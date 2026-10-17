//! Locate an already installed client; only the search action may start it.
use super::{EverythingInstallation, EverythingSearchError};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub(super) fn start_background() -> Result<(), EverythingSearchError> {
    let installation = inspect();
    let executable = installation
        .path
        .ok_or(EverythingSearchError::NotInstalled)?;
    if installation.state != "ready" {
        return Err(EverythingSearchError::IncompatibleVersion(
            installation.version.unwrap_or_else(|| "unknown".into()),
        ));
    }
    Command::new(executable)
        .args(["-startup", "-first-instance"])
        .creation_flags(0x0800_0000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(EverythingSearchError::Launch)?;
    Ok(())
}

fn executable() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PHOTAG_EVERYTHING_EXE").map(PathBuf::from) {
        return path.is_file().then_some(path);
    }
    for root in [0x8000_0001u32, 0x8000_0002u32] {
        for view in [0x0001_0000u32, 0x0002_0000u32] {
            let app_key = r"Software\Microsoft\Windows\CurrentVersion\App Paths\Everything.exe";
            if let Some(path) = registry(root, app_key, None, view).map(PathBuf::from)
                && path.is_file()
            {
                return Some(path);
            }
            for name in ["Everything", "Everything_is1"] {
                let key = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}");
                if let Some(location) = registry(root, &key, Some("InstallLocation"), view) {
                    let path = PathBuf::from(location).join("Everything.exe");
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
    }
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(variable) {
            for folder in ["Everything", r"Programs\Everything"] {
                let path = PathBuf::from(&root).join(folder).join("Everything.exe");
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|path| path.join("Everything.exe"))
        .find(|path| path.is_file())
}

pub(super) fn inspect() -> EverythingInstallation {
    let path = executable();
    let version = path.as_deref().and_then(file_version);
    let state = if path.is_none() {
        "missing"
    } else if version.is_some_and(compatible) {
        "ready"
    } else {
        "incompatible"
    };
    EverythingInstallation {
        state,
        path,
        version: version.map(|v| format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3])),
    }
}

fn compatible(version: [u16; 4]) -> bool {
    (version[0], version[1]) >= (1, 5)
}

fn file_version(path: &std::path::Path) -> Option<[u16; 4]> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "version")]
    unsafe extern "system" {
        fn GetFileVersionInfoSizeW(path: *const u16, handle: *mut u32) -> u32;
        fn GetFileVersionInfoW(
            path: *const u16,
            handle: u32,
            size: u32,
            data: *mut std::ffi::c_void,
        ) -> i32;
        fn VerQueryValueW(
            data: *const std::ffi::c_void,
            subkey: *const u16,
            result: *mut *mut std::ffi::c_void,
            length: *mut u32,
        ) -> i32;
    }
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        let size = GetFileVersionInfoSizeW(path.as_ptr(), std::ptr::null_mut());
        if size == 0 {
            return None;
        }
        let mut buffer = vec![0u8; size as usize];
        if GetFileVersionInfoW(path.as_ptr(), 0, size, buffer.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let mut value = std::ptr::null_mut();
        let mut length = 0;
        if VerQueryValueW(
            buffer.as_ptr().cast(),
            [92u16, 0].as_ptr(),
            &mut value,
            &mut length,
        ) == 0
            || value.is_null()
            || length < 52
        {
            return None;
        }
        let fields = value.cast::<[u32; 13]>().read_unaligned();
        if fields[0] != 0xfeef04bd {
            return None;
        }
        Some([
            (fields[2] >> 16) as u16,
            fields[2] as u16,
            (fields[3] >> 16) as u16,
            fields[3] as u16,
        ])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sdk3_requires_everything_15() {
        assert!(!super::compatible([1, 4, 1, 1028]));
        assert!(super::compatible([1, 5, 0, 1423]));
    }
}

fn registry(root: u32, key: &str, value: Option<&str>, view: u32) -> Option<String> {
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            root: isize,
            key: *const u16,
            value: *const u16,
            flags: u32,
            kind: *mut u32,
            data: *mut std::ffi::c_void,
            size: *mut u32,
        ) -> i32;
    }
    let key = key.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let value = value.map(|name| name.encode_utf16().chain(Some(0)).collect::<Vec<_>>());
    let mut buffer = vec![0u16; 32_768];
    let mut size = (buffer.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            root as i32 as isize,
            key.as_ptr(),
            value
                .as_ref()
                .map_or(std::ptr::null(), |name| name.as_ptr()),
            0x0000_0002 | view,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    (result == 0).then(|| {
        String::from_utf16_lossy(&buffer[..size as usize / 2])
            .trim_end_matches('\0')
            .trim_matches('"')
            .to_owned()
    })
}
