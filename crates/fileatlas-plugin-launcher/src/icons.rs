//! Native shell icons. Extraction and disk decoding run only on the shared
//! ReadModel background worker; successful RGBA images persist between runs.
use gpui::RenderImage;
use std::{
    collections::HashMap,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};
use windows::{
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits, GetObjectW, HBITMAP, HGDIOBJ,
            SelectObject,
        },
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize, IBindCtx},
        UI::Shell::{
            ExtractIconExW, IShellItemImageFactory, SHCreateItemFromParsingName,
            SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
        },
        UI::WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx, HICON},
    },
    core::PCWSTR,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum IconSource {
    Disk,
    Shell,
    Failed,
}

#[derive(Clone, Default)]
pub(super) struct IconSet {
    pub images: HashMap<PathBuf, Option<Arc<RenderImage>>>,
    pub sources: HashMap<PathBuf, IconSource>,
}

impl IconSet {
    pub fn count(&self, source: IconSource) -> usize {
        self.sources
            .values()
            .filter(|value| **value == source)
            .count()
    }
}

pub(super) fn load(
    paths: Vec<PathBuf>,
    mut cached: IconSet,
    cache_dir: &Path,
) -> Result<IconSet, String> {
    if paths.iter().all(|path| cached.images.contains_key(path)) {
        return Ok(cached);
    }
    // Each ReadModel worker owns its COM apartment; no COM interfaces cross threads.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }.map_err(|e| e.to_string())?;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _apartment = Apartment;
    for path in paths {
        if cached.images.contains_key(&path) {
            continue;
        }
        if let Some(image) = read_persistent(&path, cache_dir) {
            cached.images.insert(path.clone(), Some(image));
            cached.sources.insert(path, IconSource::Disk);
            continue;
        }
        match extract(&path) {
            Ok(pixels) => {
                write_persistent(&path, cache_dir, &pixels);
                cached.images.insert(
                    path.clone(),
                    Some(Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))),
                );
                cached.sources.insert(path, IconSource::Shell);
            }
            Err(error) => {
                tracing::debug!(path = %path.display(), %error, "program icon extraction failed");
                cached.images.insert(path.clone(), None);
                cached.sources.insert(path, IconSource::Failed);
            }
        }
    }
    Ok(cached)
}

fn persistent_key(path: &Path) -> String {
    let normalized = normalize_shell_path(path)
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase();
    let metadata = fs::metadata(path).ok();
    let length = metadata.as_ref().map(fs::Metadata::len).unwrap_or(0);
    let modified = metadata
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let identity = format!("{normalized}\0{length}\0{modified}");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    identity.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn cache_paths(path: &Path, cache_dir: &Path) -> (PathBuf, PathBuf, String) {
    let key = persistent_key(path);
    (
        cache_dir.join(format!("{key}.png")),
        cache_dir.join(format!("{key}.meta")),
        key,
    )
}

fn read_persistent(path: &Path, cache_dir: &Path) -> Option<Arc<RenderImage>> {
    let (png, meta, key) = cache_paths(path, cache_dir);
    if fs::read_to_string(meta).ok()?.trim() != key {
        return None;
    }
    let pixels = image::open(png).ok()?.into_rgba8();
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])))
}

fn write_persistent(path: &Path, cache_dir: &Path, pixels: &image::RgbaImage) {
    let (png, meta, key) = cache_paths(path, cache_dir);
    if fs::create_dir_all(cache_dir).is_err() {
        return;
    }
    if pixels.save(&png).is_ok() {
        let _ = fs::write(meta, key);
    }
}

fn normalize_shell_path(path: &Path) -> PathBuf {
    let value = path.to_string_lossy().replace('/', "\\");
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", unc))
    } else if let Some(plain) = value.strip_prefix(r"\\?\") {
        PathBuf::from(plain)
    } else {
        path.to_path_buf()
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

fn extract(path: &Path) -> windows::core::Result<image::RgbaImage> {
    use std::os::windows::ffi::OsStrExt;
    // canonicalize() yields the Win32 extended prefix (\\?\). The Shell
    // image factory does not resolve that prefix consistently for .exe/.lnk.
    let shell_path = normalize_shell_path(path);
    let wide = shell_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let shell_image = (|| {
        let factory: IShellItemImageFactory =
            unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None::<&IBindCtx>)? };
        let bitmap = Bitmap(unsafe {
            factory.GetImage(
                SIZE { cx: 96, cy: 96 },
                SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
            )?
        });
        bitmap_pixels(bitmap)
    })();
    match shell_image {
        Ok(pixels) => Ok(pixels),
        Err(shell_error) => extract_resource_icon(&wide).or(Err(shell_error)),
    }
}

fn bitmap_pixels(bitmap: Bitmap) -> windows::core::Result<image::RgbaImage> {
    let mut info = BITMAP::default();
    if unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0.0),
            size_of::<BITMAP>() as i32,
            Some((&mut info as *mut BITMAP).cast()),
        )
    } == 0
    {
        return Err(windows::core::Error::from_win32());
    }
    let width = info.bmWidth;
    let height = info.bmHeight.abs();
    if !(1..=512).contains(&width) || !(1..=512).contains(&height) {
        return Err(windows::core::Error::from_win32());
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
    let mut pixels = vec![0_u8; width as usize * height as usize * 4];
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap.0,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut header,
            DIB_RGB_COLORS,
        )
    };
    unsafe {
        let _ = DeleteDC(dc);
    }
    if lines != height {
        return Err(windows::core::Error::from_win32());
    }
    normalize_alpha(&mut pixels);
    image::RgbaImage::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(windows::core::Error::from_win32)
}

struct NativeIcon(HICON);
impl Drop for NativeIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

fn extract_resource_icon(wide_path: &[u16]) -> windows::core::Result<image::RgbaImage> {
    let mut large = HICON::default();
    let count = unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), 0, Some(&mut large), None, 1) };
    if count == 0 || large.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    let icon = NativeIcon(large);
    const SIZE: i32 = 96;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: SIZE,
            biHeight: -SIZE,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    let mut bits = std::ptr::null_mut();
    let bitmap =
        match unsafe { CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) } {
            Ok(bitmap) => Bitmap(bitmap),
            Err(error) => {
                unsafe {
                    let _ = DeleteDC(dc);
                }
                return Err(error);
            }
        };
    let previous = unsafe { SelectObject(dc, HGDIOBJ(bitmap.0.0)) };
    let drawn = unsafe { DrawIconEx(dc, 0, 0, icon.0, SIZE, SIZE, 0, None, DI_NORMAL) };
    unsafe {
        let _ = SelectObject(dc, previous);
        let _ = DeleteDC(dc);
    }
    drawn?;
    if bits.is_null() {
        return Err(windows::core::Error::from_win32());
    }
    let mut pixels = vec![0_u8; SIZE as usize * SIZE as usize * 4];
    unsafe {
        std::ptr::copy_nonoverlapping(bits.cast::<u8>(), pixels.as_mut_ptr(), pixels.len());
    }
    normalize_alpha(&mut pixels);
    image::RgbaImage::from_raw(SIZE as u32, SIZE as u32, pixels)
        .ok_or_else(windows::core::Error::from_win32)
}

fn normalize_alpha(pixels: &mut [u8]) {
    let opaque = pixels.chunks_exact(4).all(|p| p[3] == 0);
    for pixel in pixels.chunks_exact_mut(4) {
        if opaque {
            pixel[3] = 255;
        } else if pixel[3] != 0 {
            let alpha = u32::from(pixel[3]);
            for channel in &mut pixel[..3] {
                *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
            }
        }
    }
}
