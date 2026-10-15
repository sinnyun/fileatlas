//! Native capture of the PhoTag window passed by the Shell.

/// Ask only this process's published window for its UI Automation provider.
/// This activates the same GPUI accessibility path as a Windows UIA client.
#[cfg(target_os = "windows")]
pub(super) fn activate_accessibility(handle: isize) -> Result<bool, String> {
    use windows_sys::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            GetWindowThreadProcessId, IsWindow, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_GETOBJECT,
        },
    };
    let hwnd = handle as HWND;
    let mut process = 0;
    let mut provider = 0;
    unsafe {
        if IsWindow(hwnd) == 0 {
            return Err("PhoTag window is unavailable".into());
        }
        GetWindowThreadProcessId(hwnd, &mut process);
        if process != std::process::id() {
            return Err("Window does not belong to this PhoTag process".into());
        }
        // UIA's UiaRootObjectId is -25; no external HWND or input is accepted.
        if SendMessageTimeoutW(
            hwnd,
            WM_GETOBJECT,
            0,
            -25,
            SMTO_ABORTIFHUNG,
            2000,
            &mut provider,
        ) == 0
        {
            return Err("PhoTag accessibility activation timed out".into());
        }
    }
    Ok(provider != 0)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn activate_accessibility(_: isize) -> Result<bool, String> {
    Err("PhoTag accessibility activation is Windows-only".into())
}

#[cfg(target_os = "windows")]
pub(super) fn resize_window(handle: isize, width: u32, height: u32) -> Result<(u32, u32), String> {
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{
            GetWindowRect, IsWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
        },
    };

    if !(640..=4096).contains(&width) || !(480..=2160).contains(&height) {
        return Err("PhoTag window size must be between 640×480 and 4096×2160 pixels.".into());
    }
    let hwnd = handle as HWND;
    unsafe {
        if IsWindow(hwnd) == 0 {
            return Err("PhoTag 原生窗口句柄已失效。".into());
        }
        if SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            width as i32,
            height as i32,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        ) == 0
        {
            return Err("Windows 无法调整 PhoTag 窗口尺寸。".into());
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err("Windows 无法读取调整后的 PhoTag 窗口尺寸。".into());
        }
        return Ok((
            rect.right.saturating_sub(rect.left) as u32,
            rect.bottom.saturating_sub(rect.top) as u32,
        ));
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn resize_window(
    _handle: isize,
    _width: u32,
    _height: u32,
) -> Result<(u32, u32), String> {
    Err("PhoTag MCP window resizing is currently supported only on Windows.".into())
}

#[cfg(target_os = "windows")]
pub(super) fn capture_png(handle: isize) -> Result<(Vec<u8>, u32, u32), String> {
    use std::{mem, ptr};
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
        },
        Storage::Xps::PrintWindow,
        UI::WindowsAndMessaging::{GetWindowRect, IsIconic, IsWindow, PW_RENDERFULLCONTENT},
    };

    let hwnd = handle as HWND;
    // The caller supplies only the HWND created by this PhoTag process.
    unsafe {
        if IsWindow(hwnd) == 0 {
            return Err("PhoTag 原生窗口句柄已失效。".into());
        }
        if IsIconic(hwnd) != 0 {
            return Err("PhoTag 窗口已最小化，当前无法取得有效截图。".into());
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err("无法读取 PhoTag 窗口尺寸。".into());
        }
        let width = rect.right.saturating_sub(rect.left) as u32;
        let height = rect.bottom.saturating_sub(rect.top) as u32;
        if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
            return Err("PhoTag 窗口尺寸不在允许范围内。".into());
        }
        let byte_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= 256 * 1024 * 1024)
            .ok_or_else(|| "截图缓冲区超出 256 MiB 上限。".to_owned())?;

        let screen_dc = GetDC(ptr::null_mut());
        if screen_dc.is_null() {
            return Err("无法创建屏幕设备上下文。".into());
        }
        let memory_dc = CreateCompatibleDC(screen_dc);
        if memory_dc.is_null() {
            ReleaseDC(ptr::null_mut(), screen_dc);
            return Err("无法创建截图设备上下文。".into());
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, width as i32, height as i32);
        if bitmap.is_null() {
            DeleteDC(memory_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            return Err("无法创建 PhoTag 截图位图。".into());
        }
        let old_object = SelectObject(memory_dc, bitmap as _);
        let captured = PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) != 0;
        SelectObject(memory_dc, old_object);
        let result = (|| {
            if !captured {
                return Err("Windows 未能捕获 PhoTag 窗口。".to_owned());
            }
            let mut info = BITMAPINFO::default();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            };
            let mut pixels = vec![0_u8; byte_len];
            let rows = GetDIBits(
                screen_dc,
                bitmap,
                0,
                height,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            );
            if rows == 0 {
                return Err("Windows 未能读取 PhoTag 截图像素。".to_owned());
            }
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                pixel[3] = 255;
            }
            let image = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or_else(|| "截图像素缓冲区格式无效。".to_owned())?;
            let mut png = Vec::new();
            image::DynamicImage::ImageRgba8(image)
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .map_err(|error| format!("PNG 编码失败：{error}"))?;
            Ok((png, width, height))
        })();
        DeleteObject(bitmap as _);
        DeleteDC(memory_dc);
        ReleaseDC(ptr::null_mut(), screen_dc);
        result
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn capture_png(_handle: isize) -> Result<(Vec<u8>, u32, u32), String> {
    Err("PhoTag MCP 窗口截图目前仅支持 Windows。".into())
}
/// Read only this child's native window state; no repaint/input is generated.
#[cfg(target_os = "windows")]
pub fn window_status(handle: isize) -> serde_json::Value {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowThreadProcessId, IsHungAppWindow, IsIconic, IsWindow, IsWindowVisible,
    };
    let hwnd = handle as windows_sys::Win32::Foundation::HWND;
    let mut owner = 0;
    let valid = handle != 0 && unsafe { IsWindow(hwnd) } != 0;
    if valid {
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut owner);
        }
    }
    serde_json::json!({"valid":valid,"owned":valid && owner == std::process::id(),
        "visible":valid && unsafe { IsWindowVisible(hwnd) } != 0,
        "minimized":valid && unsafe { IsIconic(hwnd) } != 0,
        "hung":valid && unsafe { IsHungAppWindow(hwnd) } != 0})
}

#[cfg(not(target_os = "windows"))]
pub fn window_status(_: isize) -> serde_json::Value {
    serde_json::json!({"unsupported":true})
}

#[cfg(target_os = "windows")]
pub fn restore_test_window(handle: isize) -> Result<(), String> {
    let state = window_status(handle);
    if state["owned"] != true {
        return Err("PhoTag window is unavailable or belongs to another process".into());
    }
    if state["minimized"] == true {
        // Restore only this MCP child's window, without taking keyboard focus.
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                handle as windows_sys::Win32::Foundation::HWND,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE,
            );
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn restore_test_window(_: isize) -> Result<(), String> {
    Err("Window restoration is Windows-only".into())
}
