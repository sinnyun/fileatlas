//! Windows title bar, system accent and Mica/Acrylic integration.
//!
//! Behavior follows `vendor/photag-source/src/winfx.rs`. Native effects are
//! opt-in at the shell after GPUI creates its HWND; non-Windows builds return
//! `Backdrop::None` and do not load Windows libraries.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    None,
    Mica,
    Acrylic,
}

impl Backdrop {
    pub fn is_translucent(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;

    use super::Backdrop;

    type Hkey = isize;
    type Hwnd = isize;
    const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as i32 as isize;
    const RRF_RT_ANY: u32 = 0xffff;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            hkey: Hkey,
            sub: *const u16,
            value: *const u16,
            flags: u32,
            ty: *mut u32,
            data: *mut c_void,
            size: *mut u32,
        ) -> i32;
    }
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: Hwnd,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> *const c_void;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    fn registry_value(key: &str, name: &str) -> Option<Vec<u8>> {
        let key = wide(key);
        let name = wide(name);
        let mut size = 0;
        unsafe {
            if RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_ANY,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            ) != 0
                || size == 0
            {
                return None;
            }
            let mut bytes = vec![0; size as usize];
            if RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_ANY,
                std::ptr::null_mut(),
                bytes.as_mut_ptr().cast(),
                &mut size,
            ) != 0
            {
                return None;
            }
            bytes.truncate(size as usize);
            Some(bytes)
        }
    }

    fn visible_accent(r: u8, g: u8, b: u8) -> u32 {
        let max = r.max(g).max(b) as u32;
        if max == 0 {
            return 0x969696;
        }
        if max >= 150 {
            return ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        }
        let lift = |channel: u8| ((channel as u32 * 150) / max).min(255);
        (lift(r) << 16) | (lift(g) << 8) | lift(b)
    }

    pub fn system_accent() -> Option<u32> {
        if let Some(palette) = registry_value(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent",
            "AccentPalette",
        ) {
            if palette.len() >= 8 {
                return Some(visible_accent(palette[4], palette[5], palette[6]));
            }
        }
        registry_value(r"Software\Microsoft\Windows\DWM", "AccentColor")
            .filter(|bytes| bytes.len() >= 3)
            .map(|bytes| visible_accent(bytes[0], bytes[1], bytes[2]))
    }

    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform_id: u32,
        csd: [u16; 128],
    }

    fn build_number() -> u32 {
        type RtlGetVersion = unsafe extern "system" fn(*mut OsVersionInfoW) -> i32;
        unsafe {
            let module = GetModuleHandleW(wide("ntdll.dll").as_ptr());
            if module == 0 {
                return 0;
            }
            let address = GetProcAddress(module, b"RtlGetVersion\0".as_ptr());
            if address.is_null() {
                return 0;
            }
            let get_version: RtlGetVersion = std::mem::transmute(address);
            let mut version: OsVersionInfoW = std::mem::zeroed();
            version.size = std::mem::size_of::<OsVersionInfoW>() as u32;
            if get_version(&mut version) == 0 {
                version.build
            } else {
                0
            }
        }
    }

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;

    fn set_dark_title_bar(hwnd: Hwnd) {
        let enabled: i32 = 1;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&enabled as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );
        }
    }

    fn try_mica(hwnd: Hwnd) -> bool {
        let main_window: i32 = 2;
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&main_window as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            ) == 0
        }
    }

    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }
    #[repr(C)]
    struct CompositionAttributeData {
        attribute: u32,
        data: *mut c_void,
        size: usize,
    }

    fn try_acrylic(hwnd: Hwnd) -> bool {
        type SetCompositionAttribute =
            unsafe extern "system" fn(Hwnd, *mut CompositionAttributeData) -> i32;
        unsafe {
            let user32 = GetModuleHandleW(wide("user32.dll").as_ptr());
            if user32 == 0 {
                return false;
            }
            let address = GetProcAddress(user32, b"SetWindowCompositionAttribute\0".as_ptr());
            if address.is_null() {
                return false;
            }
            let set_attribute: SetCompositionAttribute = std::mem::transmute(address);
            let mut policy = AccentPolicy {
                state: 4,
                flags: 0,
                gradient_color: 0x3318_1818,
                animation_id: 0,
            };
            let mut data = CompositionAttributeData {
                attribute: 19,
                data: (&mut policy as *mut AccentPolicy).cast(),
                size: std::mem::size_of::<AccentPolicy>(),
            };
            set_attribute(hwnd, &mut data) != 0
        }
    }

    pub fn apply_backdrop(hwnd: isize) -> Backdrop {
        if hwnd == 0 {
            return Backdrop::None;
        }
        set_dark_title_bar(hwnd);
        if build_number() >= 22_000 && try_mica(hwnd) {
            return Backdrop::Mica;
        }
        if try_acrylic(hwnd) {
            return Backdrop::Acrylic;
        }
        Backdrop::None
    }
}

#[cfg(target_os = "windows")]
pub fn system_accent() -> Option<u32> {
    imp::system_accent().map(crate::theme::normalize_accent)
}

#[cfg(not(target_os = "windows"))]
pub fn system_accent() -> Option<u32> {
    None
}

#[cfg(target_os = "windows")]
pub fn apply_backdrop(hwnd: isize) -> Backdrop {
    imp::apply_backdrop(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn apply_backdrop(_hwnd: isize) -> Backdrop {
    Backdrop::None
}

/// Reports whether source-compatible Windows font files are installed.
/// GPUI resolves family names through the OS font database; it owns actual
/// font selection and glyph fallback during text shaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontAvailability {
    pub ui: bool,
    pub semibold: bool,
    pub symbols: bool,
    pub emoji: bool,
    pub mdl2_icons: bool,
}

#[cfg(target_os = "windows")]
pub fn font_availability() -> FontAvailability {
    let exists = |name: &str| {
        std::path::Path::new(r"C:\Windows\Fonts")
            .join(name)
            .is_file()
    };
    FontAvailability {
        ui: exists("segoeui.ttf") || exists("SegUIVar.ttf"),
        semibold: exists("seguisb.ttf"),
        symbols: exists("seguisym.ttf"),
        emoji: exists("seguiemj.ttf"),
        mdl2_icons: exists("segmdl2.ttf"),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn font_availability() -> FontAvailability {
    FontAvailability {
        ui: false,
        semibold: false,
        symbols: false,
        emoji: false,
        mdl2_icons: false,
    }
}
