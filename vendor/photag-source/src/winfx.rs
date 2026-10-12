//! Windows desktop integration for the Fluent / Windows 11 look:
//!   • read the user's system accent color from the registry,
//!   • apply a dark title bar (Win10 1903+ / Win11),
//!   • apply the Mica backdrop on Windows 11 (build ≥ 22000), falling back
//!     to an Acrylic blur-behind on Windows 10.
//!
//! Every entry point degrades gracefully: off Windows, or when the OS
//! doesn't support a given effect, it simply returns `None` / `Backdrop::None`
//! and changes nothing. The DWM / composition calls are made through small
//! hand-written FFI bindings (and `GetProcAddress` for the undocumented
//! exports) so we don't pull in a heavyweight Win32 crate just for these.

#![allow(dead_code)]

use egui::Color32;

/// Which translucent window material (if any) we managed to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    /// Opaque window — no translucency.
    None,
    /// Windows 11 Mica (`DWMWA_SYSTEMBACKDROP_TYPE` = main window).
    Mica,
    /// Windows 10 Acrylic blur-behind (`SetWindowCompositionAttribute`).
    Acrylic,
}

impl Backdrop {
    /// True when a translucent material is active and the window therefore
    /// wants a transparent clear color + slightly translucent panel fills.
    pub fn is_translucent(self) -> bool {
        !matches!(self, Backdrop::None)
    }
}

// ─────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::ffi::c_void;

    type Hkey = isize;
    type Hwnd = isize;
    // HKEY_CURRENT_USER is `(HKEY)(LONG)0x80000001` — the cast sign-extends
    // to 0xFFFFFFFF80000001 on 64-bit, which this expression reproduces.
    const HKEY_CURRENT_USER: Hkey = 0x80000001u32 as i32 as isize;
    const RRF_RT_ANY: u32 = 0x0000_ffff;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegGetValueW(
            hkey: Hkey,
            sub: *const u16,
            val: *const u16,
            flags: u32,
            ty: *mut u32,
            data: *mut c_void,
            cb: *mut u32,
        ) -> i32;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: Hwnd, attr: u32, val: *const c_void, cb: u32) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> *const c_void;
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Read a value under HKCU as raw bytes (any type). Returns `None` if the
    /// key/value is missing.
    fn reg_get(subkey: &str, value: &str) -> Option<Vec<u8>> {
        let sub = wide(subkey);
        let val = wide(value);
        let mut cb: u32 = 0;
        unsafe {
            let rc = RegGetValueW(
                HKEY_CURRENT_USER, sub.as_ptr(), val.as_ptr(), RRF_RT_ANY,
                std::ptr::null_mut(), std::ptr::null_mut(), &mut cb,
            );
            if rc != 0 || cb == 0 {
                return None;
            }
            let mut buf = vec![0u8; cb as usize];
            let rc = RegGetValueW(
                HKEY_CURRENT_USER, sub.as_ptr(), val.as_ptr(), RRF_RT_ANY,
                std::ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, &mut cb,
            );
            if rc != 0 {
                return None;
            }
            buf.truncate(cb as usize);
            Some(buf)
        }
    }

    /// Lift a color toward usable brightness on a dark surface without
    /// shifting its hue. Needed because a user with the neutral/"no color"
    /// Windows accent gets a dark-gray palette whose light shades are still
    /// too dim to read as a selection indicator on our dark theme.
    fn ensure_min_brightness(c: Color32) -> Color32 {
        const FLOOR: u32 = 150; // brightest channel should reach at least this
        let max = c.r().max(c.g()).max(c.b()) as u32;
        if max == 0 {
            return Color32::from_gray(FLOOR as u8);
        }
        if max >= FLOOR {
            return c;
        }
        let scale = |v: u8| ((v as u32 * FLOOR) / max).min(255) as u8;
        Color32::from_rgb(scale(c.r()), scale(c.g()), scale(c.b()))
    }

    pub fn system_accent() -> Option<Color32> {
        // Preferred source: the 8-shade `AccentPalette` (RGBA per shade).
        // On a dark theme Windows itself tints with the "Light2" shade
        // (index 1, byte offset 4); we mirror that, then guarantee a
        // minimum brightness so neutral-gray accents stay legible.
        if let Some(p) = reg_get(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent",
            "AccentPalette",
        ) {
            if p.len() >= 8 {
                return Some(ensure_min_brightness(Color32::from_rgb(p[4], p[5], p[6])));
            }
        }
        // Fallback: DWM `AccentColor` is a DWORD stored ABGR, so the
        // little-endian bytes are [R, G, B, A].
        if let Some(b) = reg_get(r"Software\Microsoft\Windows\DWM", "AccentColor") {
            if b.len() >= 3 {
                return Some(ensure_min_brightness(Color32::from_rgb(b[0], b[1], b[2])));
            }
        }
        None
    }

    // ── OS build number, for gating Mica vs. Acrylic ────────────────────
    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform_id: u32,
        csd: [u16; 128],
    }

    /// Real OS build via `RtlGetVersion` (not subject to the compatibility
    /// shims that lie to `GetVersionEx`). Loaded dynamically from ntdll.
    fn build_number() -> u32 {
        type RtlGetVersionFn = unsafe extern "system" fn(*mut OsVersionInfoW) -> i32;
        unsafe {
            let ntdll = GetModuleHandleW(wide("ntdll.dll").as_ptr());
            if ntdll == 0 {
                return 0;
            }
            let proc = GetProcAddress(ntdll, b"RtlGetVersion\0".as_ptr());
            if proc.is_null() {
                return 0;
            }
            let f: RtlGetVersionFn = std::mem::transmute(proc);
            let mut info: OsVersionInfoW = std::mem::zeroed();
            info.size = std::mem::size_of::<OsVersionInfoW>() as u32;
            if f(&mut info) == 0 {
                info.build
            } else {
                0
            }
        }
    }

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
    const DWMSBT_MAINWINDOW: i32 = 2; // Mica

    // Undocumented user32 acrylic API (Windows 10 1803+).
    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32, // 0xAABBGGRR
        animation_id: u32,
    }
    #[repr(C)]
    struct WinCompAttrData {
        attribute: u32,
        data: *mut c_void,
        size: usize,
    }
    const WCA_ACCENT_POLICY: u32 = 19;
    const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

    fn set_immersive_dark(hwnd: Hwnd) {
        unsafe {
            let on: i32 = 1;
            DwmSetWindowAttribute(
                hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE,
                &on as *const i32 as *const c_void, 4,
            );
        }
    }

    fn try_mica(hwnd: Hwnd) -> bool {
        unsafe {
            let kind: i32 = DWMSBT_MAINWINDOW;
            DwmSetWindowAttribute(
                hwnd, DWMWA_SYSTEMBACKDROP_TYPE,
                &kind as *const i32 as *const c_void, 4,
            ) == 0
        }
    }

    fn try_acrylic(hwnd: Hwnd) -> bool {
        type SetWinCompAttrFn = unsafe extern "system" fn(Hwnd, *mut WinCompAttrData) -> i32;
        unsafe {
            let user32 = GetModuleHandleW(wide("user32.dll").as_ptr());
            if user32 == 0 {
                return false;
            }
            let proc = GetProcAddress(user32, b"SetWindowCompositionAttribute\0".as_ptr());
            if proc.is_null() {
                return false;
            }
            let f: SetWinCompAttrFn = std::mem::transmute(proc);
            // A dark, low-alpha tint so the blur reads as a dim translucent
            // pane rather than washing the content out. 0xAABBGGRR.
            let mut policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: 0,
                gradient_color: 0x3318_1818,
                animation_id: 0,
            };
            let mut data = WinCompAttrData {
                attribute: WCA_ACCENT_POLICY,
                data: &mut policy as *mut _ as *mut c_void,
                size: std::mem::size_of::<AccentPolicy>(),
            };
            f(hwnd, &mut data) != 0
        }
    }

    pub fn apply_backdrop(hwnd: isize) -> Backdrop {
        if hwnd == 0 {
            return Backdrop::None;
        }
        set_immersive_dark(hwnd);
        if build_number() >= 22000 && try_mica(hwnd) {
            return Backdrop::Mica;
        }
        if try_acrylic(hwnd) {
            return Backdrop::Acrylic;
        }
        Backdrop::None
    }
}

// ── Public surface (cross-platform stubs off Windows) ────────────────────

/// The user's Windows accent color, lifted to a brightness that reads on a
/// dark surface. `None` if it can't be determined (or off Windows).
#[cfg(target_os = "windows")]
pub fn system_accent() -> Option<Color32> {
    imp::system_accent()
}

/// Apply the dark title bar + Mica (Win11) / Acrylic (Win10) backdrop to the
/// given native window handle. Returns which material was actually applied.
#[cfg(target_os = "windows")]
pub fn apply_backdrop(hwnd: isize) -> Backdrop {
    imp::apply_backdrop(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn system_accent() -> Option<Color32> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn apply_backdrop(_hwnd: isize) -> Backdrop {
    Backdrop::None
}
