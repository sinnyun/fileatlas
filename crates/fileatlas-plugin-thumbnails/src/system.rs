//! Windows Shell cache-only thumbnail lookup for ordinary file cards.
use std::{
    fs,
    hash::{Hash, Hasher},
    os::windows::ffi::OsStrExt,
    path::Path,
    time::UNIX_EPOCH,
};

use windows::{
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject,
            GetDIBits, GetObjectW, HBITMAP, HGDIOBJ,
        },
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize, IBindCtx},
        UI::Shell::{
            IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
            SIIGBF_INCACHEONLY, SIIGBF_THUMBNAILONLY,
        },
    },
    core::PCWSTR,
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
struct Bitmap(HBITMAP);
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0.0));
        }
    }
}

pub(super) fn cached_thumbnail(
    source: &Path,
    directory: &Path,
    revision: u64,
) -> Option<(String, u32, u32)> {
    let metadata = fs::metadata(source).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    (
        source.to_string_lossy().to_lowercase(),
        metadata.len(),
        modified,
        revision,
    )
        .hash(&mut hash);
    let path = directory
        .join("system-shell")
        .join(format!("{:016x}.png", hash.finish()));
    if path.is_file() {
        let dimensions = image::image_dimensions(&path).ok()?;
        return Some((
            path.to_string_lossy().into_owned(),
            dimensions.0,
            dimensions.1,
        ));
    }
    let pixels = extract(source)?;
    let (width, height) = pixels.dimensions();
    fs::create_dir_all(path.parent()?).ok()?;
    pixels.save(&path).ok()?;
    Some((path.to_string_lossy().into_owned(), width, height))
}

fn extract(source: &Path) -> Option<image::RgbaImage> {
    // S_FALSE means this thread already has an apartment; only balance S_OK/S_FALSE.
    if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_err() {
        return None;
    }
    let _apartment = Apartment;
    let shell_path = normalize_shell_path(source);
    let wide = shell_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let factory: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None::<&IBindCtx>).ok()? };
    // INCACHEONLY prevents Shell from invoking a provider or generating a new thumbnail.
    let bitmap = Bitmap(unsafe {
        factory
            .GetImage(
                SIZE { cx: 512, cy: 512 },
                SIIGBF_THUMBNAILONLY | SIIGBF_INCACHEONLY | SIIGBF_BIGGERSIZEOK,
            )
            .ok()?
    });
    bitmap_pixels(bitmap)
}

fn normalize_shell_path(path: &Path) -> std::path::PathBuf {
    let value = path.to_string_lossy().replace('/', "\\");
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        std::path::PathBuf::from(format!(r"\\{}", unc))
    } else if let Some(plain) = value.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(plain)
    } else {
        path.to_path_buf()
    }
}

fn bitmap_pixels(bitmap: Bitmap) -> Option<image::RgbaImage> {
    use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    let mut info = BITMAP::default();
    if unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0.0),
            size_of::<BITMAP>() as i32,
            Some((&mut info as *mut BITMAP).cast()),
        )
    } == 0
    {
        return None;
    }
    let (width, height) = (info.bmWidth, info.bmHeight.abs());
    if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
        return None;
    }
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bytes = vec![0u8; width as usize * height as usize * 4];
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return None;
    }
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap.0,
            0,
            height as u32,
            Some(bytes.as_mut_ptr().cast()),
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    unsafe {
        let _ = DeleteDC(dc);
    }
    if lines != height {
        return None;
    }
    // GDI returns premultiplied BGRA; convert to straight RGBA.
    let opaque = bytes.chunks_exact(4).all(|pixel| pixel[3] == 0);
    for pixel in bytes.chunks_exact_mut(4) {
        let alpha = pixel[3] as u16;
        if opaque {
            pixel[3] = 255;
        } else if alpha != 0 {
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u16 * 255 / alpha).min(255)) as u8;
            }
        }
        pixel.swap(0, 2);
    }
    image::RgbaImage::from_raw(width as u32, height as u32, bytes)
}
