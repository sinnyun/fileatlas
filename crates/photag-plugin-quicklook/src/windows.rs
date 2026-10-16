//! Read-only observation limited to visible windows belonging to QuickLook.exe.
use std::{collections::HashSet, fs::OpenOptions, io::Write, path::Path};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, HANDLE, HLOCAL, HWND, INVALID_HANDLE_VALUE, LPARAM, LocalFree,
        RECT, SetLastError,
    },
    Security::{
        Authorization::ConvertSidToStringSidW, GetSidSubAuthority, GetSidSubAuthorityCount,
        GetTokenInformation, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TOKEN_USER, TokenIntegrityLevel,
        TokenUser,
    },
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    },
    System::Pipes::WaitNamedPipeW,
    System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    },
};

#[derive(Clone, serde::Serialize)]
pub struct ViewerWindow {
    pub handle: usize,
    pub process_id: u32,
    pub title: String,
    pub width: i32,
    pub height: i32,
}
struct Observation {
    processes: HashSet<u32>,
    windows: Vec<ViewerWindow>,
}

pub fn visible_windows() -> Vec<ViewerWindow> {
    let mut state = Observation {
        processes: HashSet::new(),
        windows: Vec::new(),
    };
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return state.windows;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry);
        while more != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|v| *v == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length])
                .eq_ignore_ascii_case("QuickLook.exe")
            {
                state.processes.insert(entry.th32ProcessID);
            }
            more = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        EnumWindows(Some(observe), &mut state as *mut Observation as LPARAM);
    }
    state.windows
}

/// Returns a QuickLook process running above the current process's Windows
/// integrity level. A medium-integrity PhoTag process cannot reliably control
/// an elevated QuickLook instance through QuickLook's single-instance pipe.
pub fn higher_integrity_quicklook_process() -> Option<u32> {
    let current = process_integrity(std::process::id())?;
    quicklook_process_ids()
        .into_iter()
        .find(|pid| process_integrity(*pid).is_some_and(|level| level > current))
}

/// QuickLook's live instance listens on `QuickLook.App.Pipe.<user SID>`. Writing one
/// Toggle line there takes under a millisecond, while starting a second
/// `QuickLook.exe` instance to forward the same request costs about a second.
/// Win32 pipe APIs require the computer component, so this is always the full
/// `\\.\pipe\...` form used by both `WaitNamedPipeW` and the client end open.
pub fn toggle_pipe_path() -> Option<String> {
    Some(format!(
        r"\\.\pipe\QuickLook.App.Pipe.{}",
        current_user_sid()?
    ))
}

#[derive(Debug)]
pub enum PipeError {
    /// No listening instance: QuickLook is not running, or its single instance is
    /// still owned by another request.
    Unavailable,
    /// The pipe owner rejects this process, typically a higher integrity level.
    Denied,
    Other(String),
}

pub fn send_toggle(pipe_path: &str, path: &Path) -> Result<(), PipeError> {
    // QuickLook's request line must be complete before the pipe is opened: its
    // reader loop dies on a connection that carries no line.
    let message = toggle_message(path);
    // QuickLook serves exactly one pipe instance, so a busy name or one taken
    // between the wait and the open is a race to retry, not an unreachable
    // instance. A name that does not exist fails the wait immediately.
    let mut last = PipeError::Unavailable;
    for attempt in 0..3 {
        match wait_for_instance(pipe_path).and_then(|()| open_write(pipe_path)) {
            Ok(mut pipe) => {
                return pipe
                    .write_all(message.as_bytes())
                    .and_then(|()| pipe.flush())
                    .map_err(|error| PipeError::Other(error.to_string()));
            }
            Err(error @ (PipeError::Denied | PipeError::Other(_))) => return Err(error),
            Err(error) if attempt == 2 => return Err(error),
            Err(error) => last = error,
        }
    }
    Err(last)
}

/// QuickLook creates exactly one server instance, so opening while it is busy
/// would block until the previous request finishes.
fn wait_for_instance(pipe_path: &str) -> Result<(), PipeError> {
    let wide = wide(pipe_path);
    unsafe {
        SetLastError(0);
        if WaitNamedPipeW(wide.as_ptr(), 250) != 0 {
            return Ok(());
        }
        Err(match GetLastError() {
            5 => PipeError::Denied,
            2 | 53 | 121 | 161 | 231 => PipeError::Unavailable,
            code => PipeError::Other(format!("WaitNamedPipeW failed with {code}")),
        })
    }
}

fn open_write(pipe_path: &str) -> Result<std::fs::File, PipeError> {
    OpenOptions::new()
        .write(true)
        .open(pipe_path)
        .map_err(|error| match error.raw_os_error() {
            Some(5) => PipeError::Denied,
            Some(2 | 53 | 121 | 161 | 231) => PipeError::Unavailable,
            _ => PipeError::Other(error.to_string()),
        })
}

/// QuickLook reads a single request line per connection, so CRLF framing is required.
pub fn toggle_message(path: &Path) -> String {
    format!("QuickLook.App.PipeMessages.Toggle|{}|\r\n", path.display())
}

#[derive(serde::Serialize)]
pub struct PipeProbe {
    pub path: Option<String>,
    /// A listening instance exists, so the running QuickLook can still be reached.
    pub instance_listening: bool,
    pub wait_error: u32,
}

/// Read-only liveness check. It must never open the pipe: QuickLook's reader loop
/// calls `ReadLine` once and a connection without a line throws inside that loop.
pub fn probe() -> PipeProbe {
    let Some(path) = toggle_pipe_path() else {
        return PipeProbe {
            path: None,
            instance_listening: false,
            wait_error: 0,
        };
    };
    let wide = wide(&path);
    let listening = unsafe {
        SetLastError(0);
        let ok = WaitNamedPipeW(wide.as_ptr(), 0) != 0;
        (ok, GetLastError())
    };
    PipeProbe {
        path: Some(path),
        instance_listening: listening.0,
        wait_error: listening.1,
    }
}

fn wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 || token.is_null() {
            return None;
        }
        let mut buffer = [0u8; 512];
        let mut returned = 0;
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
            &mut returned,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let user = buffer.as_ptr().cast::<TOKEN_USER>().read_unaligned();
        let mut string_sid = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut string_sid) == 0 {
            return None;
        }
        let mut length = 0;
        while *string_sid.add(length) != 0 {
            length += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(string_sid, length));
        LocalFree(string_sid as HLOCAL);
        Some(sid)
    }
}

fn quicklook_process_ids() -> Vec<u32> {
    let mut result = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return result;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry);
        while more != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|v| *v == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length])
                .eq_ignore_ascii_case("QuickLook.exe")
            {
                result.push(entry.th32ProcessID);
            }
            more = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    result
}

fn process_integrity(pid: u32) -> Option<u32> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let result = process_integrity_from_handle(process);
        CloseHandle(process);
        result
    }
}

unsafe fn process_integrity_from_handle(process: HANDLE) -> Option<u32> {
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 || token.is_null() {
            return None;
        }
        let mut buffer = [0usize; 128];
        let mut returned = 0;
        let ok = GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
            &mut returned,
        );
        let level = if ok == 0 {
            None
        } else {
            let label = buffer
                .as_ptr()
                .cast::<TOKEN_MANDATORY_LABEL>()
                .read_unaligned();
            if label.Label.Sid.is_null() {
                None
            } else {
                let count = *GetSidSubAuthorityCount(label.Label.Sid);
                (count > 0).then(|| *GetSidSubAuthority(label.Label.Sid, u32::from(count - 1)))
            }
        };
        CloseHandle(token);
        level
    }
}
unsafe extern "system" fn observe(hwnd: HWND, data: LPARAM) -> i32 {
    unsafe {
        let state = &mut *(data as *mut Observation);
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if !state.processes.contains(&process_id) || IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0
            || rect.right - rect.left < 80
            || rect.bottom - rect.top < 80
        {
            return 1;
        }
        let mut title = [0u16; 1024];
        let length = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32).max(0) as usize;
        state.windows.push(ViewerWindow {
            handle: hwnd as usize,
            process_id,
            title: String::from_utf16_lossy(&title[..length]),
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        });
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_payload_is_one_crlf_request_line_with_the_plain_path() {
        assert_eq!(
            toggle_message(Path::new(r"D:\资料 & $\video.mp4")),
            "QuickLook.App.PipeMessages.Toggle|D:\\资料 & $\\video.mp4|\r\n"
        );
    }
    #[test]
    fn pipe_path_carries_the_computer_component_and_user_sid() {
        let path = toggle_pipe_path().expect("current user SID");
        assert!(
            path.starts_with(r"\\.\pipe\QuickLook.App.Pipe.S-1-"),
            "{path}"
        );
    }
    #[test]
    fn an_absent_instance_pipe_reports_unavailable_immediately() {
        // A name nobody created fails the wait at once; it must not stall the press
        // while the caller turns it into a prompt for the user to start QuickLook.
        let path = format!(r"\\.\pipe\PhoTag.Absent.Pipe.{}", std::process::id());
        let started = std::time::Instant::now();
        assert!(matches!(
            send_toggle(&path, Path::new(r"C:\photag-test\a.png")),
            Err(PipeError::Unavailable)
        ));
        assert!(
            started.elapsed() < std::time::Duration::from_millis(200),
            "{:?}",
            started.elapsed()
        );
    }
}
