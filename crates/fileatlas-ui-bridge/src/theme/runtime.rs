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
    typography_audit: bool,
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
/// Reapply the resolved native theme when the typography review projection changes.
pub fn set_typography_audit_enabled(enabled: bool) {
    let mut request = REQUEST.write().expect("theme request lock");
    if request.typography_audit != enabled {
        request.typography_audit = enabled;
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
        typography_audit: super::typography_audit_enabled(),
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
        // Native registry owns discovery and resolution of the selected theme.
        if let Err(error) = ThemeRegistry::global_mut(cx).load_themes_from_str(json) {
            tracing::error!(%error, "could not register theme");
            return;
        }
        let set: ThemeSet = match serde_json::from_str(json) {
            Ok(set) => set,
            Err(error) => {
                tracing::error!(%error, "validated theme JSON failed to deserialize");
                return;
            }
        };
        let Some(config) = set.themes.iter().find(|t| t.mode == mode) else {
            tracing::error!(?mode, "theme has no configuration for the selected mode");
            return;
        };
        let Some(config) = ThemeRegistry::global(cx)
            .themes()
            .get(&config.name)
            .cloned()
        else {
            tracing::error!(theme = %config.name, "theme configuration was not registered");
            return;
        };
        Theme::update(cx, |theme| {
            theme.apply_config(&config);
            apply_typography_audit_theme(theme, request.typography_audit);
        });
    } else {
        let registry = ThemeRegistry::global(cx);
        let config = if mode.is_dark() {
            registry.default_dark_theme()
        } else {
            registry.default_light_theme()
        }
        .clone();
        Theme::update(cx, |theme| {
            theme.apply_config(&config);
            apply_typography_audit_theme(theme, request.typography_audit);
        });
    }
    publish(cx);
    cx.global_mut::<ThemeListener>().applied = Some(request);
    cx.refresh_windows();
}
/// TEMP audit-role projection for native GPUI Kit foregrounds that bypass
/// WorkbenchTypography through component hover/variant styles.
fn apply_typography_audit_theme(theme: &mut Theme, enabled: bool) {
    if !enabled {
        return;
    }
    theme.muted_foreground = gpui_kit::rgb(super::AUDIT_TEXT_ASSISTANT).into();
    let body = gpui_kit::rgb(super::AUDIT_TEXT_BODY).into();
    theme.foreground = body;
    // GPUI Kit tooltip/popup labels render with `popover_foreground` directly
    // instead of WorkbenchTypography; classify their 14px primary text as body.
    theme.popover_foreground = body;
    theme.button_foreground = body;
    theme.accent_foreground = body;
    theme.primary_foreground = body;
    theme.secondary_foreground = body;
    theme.button_primary_foreground = body;
    theme.button_secondary_foreground = body;
    theme.button_danger_foreground = body;
    theme.button_warning_foreground = body;
    theme.button_success_foreground = body;
    theme.button_info_foreground = body;
    theme.link = body;
    theme.link_hover = body;
    theme.link_active = body;
}
pub fn packed(color: Hsla) -> u32 {
    let rgb = color.to_rgb();
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u32;
    (byte(rgb.r) << 16) | (byte(rgb.g) << 8) | byte(rgb.b)
}
pub fn packed_rgba(color: Hsla) -> u32 {
    (packed(color) << 8) | (color.a.clamp(0., 1.) * 255.).round() as u32
}
