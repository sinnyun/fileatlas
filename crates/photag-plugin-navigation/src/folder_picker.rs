//! Single in-flight native picker and same-process cancellation/inspection.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub(crate) struct FolderPickerState {
    pending: AtomicBool,
    title: Mutex<String>,
}

impl FolderPickerState {
    pub fn pending(&self) -> bool {
        self.pending.load(Ordering::Acquire)
    }

    pub fn begin(self: &Arc<Self>, title: String) -> Option<PickerLease> {
        self.pending
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        match self.title.lock() {
            Ok(mut current) => *current = title,
            Err(_) => {
                self.pending.store(false, Ordering::Release);
                return None;
            }
        }
        Some(PickerLease(self.clone()))
    }

    pub fn window(&self) -> Option<isize> {
        if !self.pending() {
            return None;
        }
        let title = self.title.lock().ok()?.clone();
        native::find(&title)
    }

    pub fn cancel(&self) -> Result<(), String> {
        let handle = self.window().ok_or("No native folder picker is open")?;
        native::close(handle)
    }
}

pub(crate) struct PickerLease(Arc<FolderPickerState>);
impl Drop for PickerLease {
    fn drop(&mut self) {
        // A dropped GPUI task must also dismiss its owned native window.
        if let Some(handle) = self.0.window() {
            let _ = native::close(handle);
        }
        self.0.pending.store(false, Ordering::Release);
    }
}

#[cfg(windows)]
mod native {
    type Hwnd = *mut std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(callback: unsafe extern "system" fn(Hwnd, isize) -> i32, data: isize)
        -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, process: *mut u32) -> u32;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, count: i32) -> i32;
        fn GetClassNameW(hwnd: Hwnd, text: *mut u16, count: i32) -> i32;
        fn PostMessageW(hwnd: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
    }
    struct Search<'a> {
        title: &'a str,
        found: Option<isize>,
    }
    unsafe extern "system" fn inspect(hwnd: Hwnd, data: isize) -> i32 {
        // EnumWindows invokes this synchronously; Search stays live for the call.
        let search = unsafe { &mut *(data as *mut Search<'_>) };
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
        if process != std::process::id() {
            return 1;
        }
        let mut class = [0u16; 64];
        let count = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
        if count <= 0 || String::from_utf16_lossy(&class[..count as usize]) != "#32770" {
            return 1;
        }
        let mut title = [0u16; 512];
        let count = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
        if count > 0 && String::from_utf16_lossy(&title[..count as usize]) == search.title {
            search.found = Some(hwnd as isize);
            return 0;
        }
        1
    }
    pub(super) fn find(title: &str) -> Option<isize> {
        let mut search = Search { title, found: None };
        unsafe { EnumWindows(inspect, &mut search as *mut Search<'_> as isize) };
        search.found
    }
    pub(super) fn close(handle: isize) -> Result<(), String> {
        // WM_CLOSE cancels this exact in-process dialog; no mouse/key input is generated.
        if unsafe { PostMessageW(handle as Hwnd, 0x0010, 0, 0) } == 0 {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(not(windows))]
mod native {
    pub(super) fn find(_: &str) -> Option<isize> {
        None
    }
    pub(super) fn close(_: isize) -> Result<(), String> {
        Err("Native dialog cancellation is Windows-only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_picker_at_a_time_and_cancellation_allows_reopening() {
        let state = Arc::new(FolderPickerState::default());
        let lease = state.begin("test".into()).unwrap();
        assert!(state.pending());
        assert!(state.begin("duplicate".into()).is_none());
        drop(lease);
        assert!(!state.pending());
        assert!(state.begin("reopen".into()).is_some());
    }
}
