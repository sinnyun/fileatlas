//! Windows drive enumeration for the sidebar drives section.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriveInfo {
    pub root: String,
    pub letter: char,
    pub label: String,
    pub kind: DriveKind,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub usage_error: Option<String>,
}

impl DriveInfo {
    pub fn display_name(&self) -> String {
        if self.kind == DriveKind::Network || self.label.is_empty() {
            format!("{}:", self.letter)
        } else {
            format!("{}: {}", self.letter, self.label)
        }
    }
}

pub(crate) fn drive_root_index(root: &str) -> Option<u32> {
    let bytes = root.as_bytes();
    (bytes.len() == 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\'))
    .then(|| u32::from(bytes[0].to_ascii_uppercase() - b'A'))
}
pub(crate) fn drive_root_present(root: &str) -> bool {
    let Some(index) = drive_root_index(root) else {
        return false;
    };
    #[cfg(target_os = "windows")]
    {
        unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() & (1 << index) != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = index;
        false
    }
}

/// Enumerate fixed, removable and (when enabled) network drives with usage.
pub fn enumerate_drives(include_network: bool) -> Result<Vec<DriveInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        enumerate_windows(include_network)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = include_network;
        Ok(Vec::new())
    }
}

#[cfg(target_os = "windows")]
fn utf16_to_string(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(target_os = "windows")]
fn wide_root(letter: char) -> Vec<u16> {
    format!("{}:\\\0", letter).encode_utf16().collect()
}

#[cfg(target_os = "windows")]
fn enumerate_windows(include_network: bool) -> Result<Vec<DriveInfo>, String> {
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };

    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut drives = Vec::new();
    for index in 0..26u32 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let letter = (b'A' + index as u8) as char;
        let root = wide_root(letter);
        let drive_type = unsafe { GetDriveTypeW(root.as_ptr()) };
        let kind = match drive_type {
            3 => DriveKind::Fixed,
            2 => DriveKind::Removable,
            4 => DriveKind::Network,
            _ => continue,
        };
        if kind == DriveKind::Network && !include_network {
            continue;
        }
        let mut name_buffer = [0u16; 128];
        let label = if kind == DriveKind::Network {
            0
        } else {
            unsafe {
                GetVolumeInformationW(
                    root.as_ptr(),
                    name_buffer.as_mut_ptr(),
                    name_buffer.len() as u32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            }
        };
        let label = if label == 0 {
            String::new()
        } else {
            utf16_to_string(&name_buffer)
        };
        let mut free_to_caller = 0u64;
        let mut total = 0u64;
        let mut free = 0u64;
        let usage_ok = unsafe {
            GetDiskFreeSpaceExW(root.as_ptr(), &mut free_to_caller, &mut total, &mut free)
        };
        let usage_error = (usage_ok == 0).then(|| std::io::Error::last_os_error().to_string());
        let (total_bytes, used_bytes) = if usage_ok == 0 {
            (0, 0)
        } else {
            (total, total.saturating_sub(free))
        };
        drives.push(DriveInfo {
            root: format!("{}:\\", letter),
            letter,
            label,
            kind,
            total_bytes,
            used_bytes,
            usage_error,
        });
    }
    Ok(drives)
}
