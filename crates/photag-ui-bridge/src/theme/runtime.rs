//! Native Theme lifecycle, JSON application, and read-only compatibility projection.
//! Only GPUI Kit resolves palettes. Worker threads may queue a request, never paint.
use gpui_kit::component::{Theme, ThemeColor, ThemeMode, ThemeRegistry, ThemeSet};
use gpui_kit::{App, Global, Hsla, Task};
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Dark,
    Light,
}
#[derive(Clone, Default, PartialEq)]
struct Request {
    light: bool,
    json: Option<String>,
}
static REQUEST: LazyLock<RwLock<Request>> = LazyLock::new(Default::default);
static WAKE: LazyLock<crate::refresh::Wake> = LazyLock::new(Default::default);
// This is a read-only copy of the framework's resolved ThemeColor, not a palette
// or configurable state. Updated after EVERY native Theme update, including external ones.
static RESOLVED: LazyLock<RwLock<(ThemeColor, ThemeMode, String)>> =
    LazyLock::new(|| RwLock::new((*ThemeColor::dark(), ThemeMode::Dark, "Default Dark".into())));
struct ThemeListener {
    _task: Task<()>,
    applied: Option<Request>,
}
impl Global for ThemeListener {}

pub fn native_colors() -> ThemeColor {
    RESOLVED.read().expect("resolved Theme lock").0
}
pub fn theme_name() -> String {
    RESOLVED.read().expect("resolved Theme lock").2.clone()
}
pub fn color_mode() -> ColorMode {
    if RESOLVED.read().expect("resolved Theme lock").1.is_dark() {
        ColorMode::Dark
    } else {
        ColorMode::Light
    }
}
pub fn set_color_mode(mode: ColorMode) {
    let light = mode == ColorMode::Light;
    let mut request = REQUEST.write().expect("theme request lock");
    if request.light != light {
        request.light = light;
        WAKE.trigger();
    }
}
/// Validate and queue a framework ThemeSet. None selects framework defaults.
pub fn request_theme(mode: &str, json: Option<String>) -> Result<(), String> {
    if let Some(text) = &json {
        if text.len() > 2 * 1024 * 1024 {
            return Err("Theme JSON exceeds 2 MiB".into());
        }
        let set: ThemeSet = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if ![ThemeMode::Light, ThemeMode::Dark].iter().all(|mode| {
            set.themes
                .iter()
                .any(|t| t.mode == *mode && !t.name.is_empty())
        }) {
            return Err("ThemeSet must contain named light and dark themes".into());
        }
    }
    let next = Request {
        light: mode == "light",
        json,
    };
    let mut current = REQUEST.write().map_err(|_| "theme request lock")?;
    if *current != next {
        *current = next;
        WAKE.trigger();
    }
    Ok(())
}
fn publish(cx: &App) {
    let theme = Theme::global(cx);
    *RESOLVED.write().expect("resolved Theme lock") =
        (theme.colors, theme.mode, theme.theme_name().to_string());
}
/// Install once for the app lifetime. No IO, timer, or independent theme manager.
pub fn apply_components(cx: &mut App) {
    if !cx.has_global::<ThemeListener>() {
        cx.observe_global::<Theme>(|cx| publish(cx)).detach();
        let task = cx.spawn(async |cx| {
            loop {
                WAKE.wait().await;
                cx.update(apply_request);
            }
        });
        cx.set_global(ThemeListener {
            _task: task,
            applied: None,
        });
    }
    apply_request(cx);
}
pub fn apply_accent_components(cx: &mut App) {
    apply_components(cx);
}
fn apply_request(cx: &mut App) {
    let request = REQUEST.read().expect("theme request lock").clone();
    if cx.global::<ThemeListener>().applied.as_ref() == Some(&request) {
        return;
    }
    let mode = if request.light {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    };
    if let Some(json) = &request.json {
        // Validation occurred on the request path. Native registry owns discovery/resolution.
        if let Err(error) = ThemeRegistry::global_mut(cx).load_themes_from_str(&json) {
            tracing::error!(%error, "could not register theme");
            return;
        }
        let set: ThemeSet = serde_json::from_str(&json).expect("validated ThemeSet");
        let config = set
            .themes
            .iter()
            .find(|t| t.mode == mode)
            .expect("validated mode");
        let config = ThemeRegistry::global(cx)
            .themes()
            .get(&config.name)
            .cloned()
            .expect("registered theme");
        Theme::update(cx, |theme| theme.apply_config(&config));
    } else {
        let registry = ThemeRegistry::global(cx);
        let config = if mode.is_dark() {
            registry.default_dark_theme()
        } else {
            registry.default_light_theme()
        }
        .clone();
        Theme::update(cx, |theme| theme.apply_config(&config));
    }
    publish(cx);
    cx.global_mut::<ThemeListener>().applied = Some(request);
    cx.refresh_windows();
}
pub fn packed(color: Hsla) -> u32 {
    let rgb = color.to_rgb();
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u32;
    (byte(rgb.r) << 16) | (byte(rgb.g) << 8) | byte(rgb.b)
}
pub fn packed_rgba(color: Hsla) -> u32 {
    (packed(color) << 8) | (color.a.clamp(0., 1.) * 255.).round() as u32
}
