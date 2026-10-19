//! Defers OLE drag start until the GPUI input callback has returned.
//!
//! `DoDragDrop` runs a modal message loop. Starting it inside GPUI's window
//! procedure lets that procedure re-enter while its callback state is active.
//! A thread-local message-only window queues the drag and starts it from a
//! separate window procedure after the originating input message has unwound.

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use std::cell::Cell;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, PostMessageW, RegisterClassW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WNDCLASSW,
};
use windows::core::PCWSTR;

const DRAG_MESSAGE: u32 = WM_APP + 0x4d1;
const CLASS_NAME: &[u16] = &[
    80, 104, 111, 84, 97, 103, 68, 114, 97, 103, 68, 105, 115, 112, 97, 116, 99, 104, 0,
];

thread_local! {
    static DRAG_QUEUED_OR_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

struct DragRequest {
    source_window: RawWindowHandle,
    paths: Vec<PathBuf>,
    status: Arc<AtomicU8>,
}

struct SourceWindow(RawWindowHandle);

impl HasWindowHandle for SourceWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // The GPUI window remains alive while the OLE operation owns this request.
        unsafe { Ok(WindowHandle::borrow_raw(self.0)) }
    }
}

pub(super) fn queue_file_drag<W: HasWindowHandle>(
    status: &Arc<AtomicU8>,
    source: &W,
    paths: &[PathBuf],
) -> Result<(), crate::DragDropError> {
    let source_window = source
        .window_handle()
        .map_err(|error| crate::DragDropError::Platform(error.to_string()))?
        .as_raw();
    DRAG_QUEUED_OR_ACTIVE.with(|active| {
        if active.replace(true) {
            return Err(crate::DragDropError::Platform(
                "another file drag is already queued or active".to_owned(),
            ));
        }

        let dispatch_window = match unsafe { create_dispatch_window() } {
            Ok(window) => window,
            Err(error) => {
                active.set(false);
                return Err(crate::DragDropError::Platform(error.to_string()));
            }
        };
        let request = Box::new(DragRequest {
            source_window,
            paths: paths.to_vec(),
            status: Arc::clone(status),
        });
        let request_ptr = Box::into_raw(request);
        if unsafe {
            PostMessageW(
                Some(dispatch_window),
                DRAG_MESSAGE,
                WPARAM(0),
                LPARAM(request_ptr as isize),
            )
        }
        .is_err()
        {
            unsafe { drop(Box::from_raw(request_ptr)) };
            unsafe { DestroyWindow(dispatch_window) }
                .map_err(|error| crate::DragDropError::Platform(error.to_string()))?;
            active.set(false);
            return Err(crate::DragDropError::Platform(
                "Windows could not queue the native drag request".to_owned(),
            ));
        }
        status.store(crate::DragDropService::QUEUED, Ordering::Release);
        Ok(())
    })
}

unsafe fn create_dispatch_window() -> windows::core::Result<HWND> {
    let module = unsafe { GetModuleHandleW(PCWSTR::null())? };
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        lpfnWndProc: Some(dispatch_window_proc),
        hInstance: instance,
        lpszClassName: PCWSTR(CLASS_NAME.as_ptr()),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        // Another PhoTag window on this UI thread may already have registered it.
        let error = unsafe { windows::Win32::Foundation::GetLastError() };
        if error.0 != 1410 {
            return Err(windows::core::Error::from_win32());
        }
    }

    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(CLASS_NAME.as_ptr()),
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )
    }
}

unsafe extern "system" fn dispatch_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message != DRAG_MESSAGE {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }

    let request = unsafe { Box::from_raw(lparam.0 as *mut DragRequest) };
    request
        .status
        .store(crate::DragDropService::ACTIVE, Ordering::Release);
    let status = Arc::clone(&request.status);
    let result = catch_unwind(AssertUnwindSafe(|| run_file_drag(*request)));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            status.store(crate::DragDropService::FAILED, Ordering::Release);
            tracing::warn!(%error, "native file drag failed");
        }
        Err(_) => {
            status.store(crate::DragDropService::FAILED, Ordering::Release);
            tracing::error!("native file drag panicked");
        }
    }
    DRAG_QUEUED_OR_ACTIVE.with(|active| active.set(false));
    unsafe { DestroyWindow(hwnd) }.unwrap_or_else(
        |error| tracing::warn!(%error, "could not destroy native drag dispatch window"),
    );
    LRESULT(0)
}

fn run_file_drag(request: DragRequest) -> Result<(), crate::DragDropError> {
    let Some(preview_path) = request.paths.first().cloned() else {
        return Ok(());
    };
    let preview = make_drag_preview(&preview_path);
    let source = SourceWindow(request.source_window);
    let status = Arc::clone(&request.status);
    drag::start_drag(
        &source,
        drag::DragItem::Files(request.paths),
        drag::Image::Raw(preview),
        move |result, _| {
            let result_status = match result {
                drag::DragResult::Dropped => crate::DragDropService::DROPPED,
                drag::DragResult::Cancel => crate::DragDropService::CANCELLED,
            };
            status.store(result_status, Ordering::Release);
        },
        drag::Options::default(),
    )
    .map_err(|error| crate::DragDropError::Platform(error.to_string()))
}

const PREVIEW_EDGE: u32 = 96;

fn make_drag_preview(path: &std::path::Path) -> Vec<u8> {
    let image = image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .ok()
        .and_then(|reader| reader.decode().ok())
        .map(|image| image.thumbnail(PREVIEW_EDGE, PREVIEW_EDGE).to_rgba8());
    let image = image.unwrap_or_else(|| fallback_file_icon(path));
    let mut png = Cursor::new(Vec::new());
    if DynamicImage::ImageRgba8(image)
        .write_to(&mut png, ImageFormat::Png)
        .is_ok()
    {
        png.into_inner()
    } else {
        // The generated image is always valid RGBA; keep a tiny transparent PNG
        // as a last-resort image instead of asking drag-rs to decode the file.
        Vec::new()
    }
}

fn fallback_file_icon(path: &std::path::Path) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(80, 96, Rgba([0, 0, 0, 0]));
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("FILE")
        .to_ascii_uppercase();
    let accent = extension_color(&ext);
    fill_rect(&mut image, 14, 5, 52, 84, Rgba([245, 247, 250, 255]));
    fill_rect(&mut image, 14, 5, 52, 7, accent);
    // Folded page corner.
    fill_rect(&mut image, 55, 5, 11, 11, Rgba([218, 224, 232, 255]));
    fill_rect(&mut image, 20, 27, 39, 3, Rgba([198, 205, 215, 255]));
    fill_rect(&mut image, 20, 35, 31, 3, Rgba([218, 224, 232, 255]));
    fill_rect(&mut image, 20, 43, 35, 3, Rgba([218, 224, 232, 255]));
    let label = if ext.is_empty() { "FILE" } else { ext.as_str() };
    draw_label(&mut image, label, accent);
    image
}

fn extension_color(ext: &str) -> Rgba<u8> {
    match ext {
        "JPG" | "JPEG" | "PNG" | "GIF" | "WEBP" | "BMP" | "HEIC" | "TIFF" => {
            Rgba([59, 130, 246, 255])
        }
        "MP4" | "MOV" | "AVI" | "MKV" | "WEBM" => Rgba([139, 92, 246, 255]),
        "MP3" | "WAV" | "FLAC" | "M4A" => Rgba([16, 160, 132, 255]),
        "PDF" => Rgba([224, 76, 76, 255]),
        _ => Rgba([100, 116, 139, 255]),
    }
}

fn fill_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgba<u8>) {
    for py in y..(y + height).min(image.height()) {
        for px in x..(x + width).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

fn draw_label(image: &mut RgbaImage, label: &str, color: Rgba<u8>) {
    let chars: Vec<char> = label.chars().take(4).collect();
    let scale = 2;
    let glyph_width = 3 * scale;
    let gap = scale;
    let total_width = chars.len() as u32 * (glyph_width + gap) - gap;
    let mut x = (image.width() - total_width) / 2;
    for ch in chars {
        let rows = glyph(ch);
        for (row, bits) in rows.into_iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    fill_rect(
                        image,
                        x + col * scale,
                        61 + row as u32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        x += glyph_width + gap;
    }
}

fn glyph(ch: char) -> [u8; 5] {
    match ch {
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [5, 7, 7, 7, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 7, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [6, 1, 2, 4, 7],
        '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 6, 1, 6],
        '6' => [3, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 6],
        _ => [7, 1, 2, 0, 2],
    }
}
