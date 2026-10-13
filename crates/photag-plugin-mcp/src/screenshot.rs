//! Native capture of the PhoTag window passed by the Shell.

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
