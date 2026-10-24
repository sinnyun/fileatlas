//! Material produces native GPUI ThemeSet JSON; it never mutates the GUI Theme.
use super::{PALETTE_ROLES, Preferences};
use fileatlas_ui_bridge::theme;
use gpui_kit::component::{ThemeColor, ThemeSet};
use material_colors::{
    color::Argb,
    dynamic_color::{DynamicScheme, Variant},
    scheme::Scheme,
};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static CONFIG_GENERATION: AtomicU64 = AtomicU64::new(0);

const BACKGROUND_SURFACE_ROLES: &[&str] = &[
    "background",
    "list.background",
    "table.background",
    "sidebar.background",
    "title_bar.background",
    "status_bar.background",
    "group_box.background",
    "accordion.background",
    "list.head.background",
    "table.head.background",
    "table.foot.background",
    "tab_bar.background",
    "tab_bar.segmented.background",
    "popover.background",
    "muted.background",
    "list.even.background",
    "table.even.background",
    "slider.background",
    "skeleton.background",
    "button.background",
    "tab.background",
    "switch.background",
    "description_list.label.background",
    "scrollbar.background",
];
const FOREGROUND_ROLES: &[&str] = &[
    "foreground",
    "button.foreground",
    "popover.foreground",
    "sidebar.foreground",
    "group_box.foreground",
    "group_box.title.foreground",
    "table.head.foreground",
    "table.foot.foreground",
    "description_list.label.foreground",
    "tab.foreground",
];
const PRIMARY_ROLES: &[&str] = &[
    "primary.background",
    "primary.hover.background",
    "primary.active.background",
    "button.primary.background",
    "button.primary.hover.background",
    "button.primary.active.background",
    "sidebar.primary.background",
    "progress.bar.background",
    "slider.thumb.background",
    "link",
    "link.hover",
    "link.active",
    "ring",
    "drag.border",
    "base.blue",
    "chart.1",
];
const ACCENT_ROLES: &[&str] = &[
    "accent.background",
    "button.hover.background",
    "button.secondary.hover.background",
    "secondary.hover.background",
    // Keep native Ghost button hover aligned with shared list/card hover surfaces.
    "list.hover.background",
    "table.hover.background",
    "selection.background",
    "button.active.background",
    "button.secondary.active.background",
    "secondary.active.background",
    "list.active.background",
    "table.active.background",
    "sidebar.accent.background",
    "tab.active.background",
    "drop_target.background",
];
const SECONDARY_ROLES: &[&str] = &[
    "secondary.background",
    "button.secondary.background",
    "chart.4",
];

pub fn generate(p: &Preferences) -> Result<String, String> {
    if p.schema_version != 3 {
        return Err("Unsupported Material preferences version".into());
    }
    let rgb = theme::parse_hex_color(&p.seed).ok_or("Seed must be #RRGGBB")?;
    if !p.contrast.is_finite() || !(0.0..=1.0).contains(&p.contrast) {
        return Err("Contrast must be between 0 and 1".into());
    }
    let variant = match p.variant.as_str() {
        "neutral" => Variant::Neutral,
        "tonal-spot" => Variant::TonalSpot,
        "monochrome" => Variant::Monochrome,
        _ => return Err("Unsupported Material variant".into()),
    };
    let seed = Argb::new(255, rgb[0], rgb[1], rgb[2]);
    let generation = CONFIG_GENERATION.fetch_add(1, Ordering::Relaxed);
    let themes=[false,true].map(|dark| {
        let scheme:Scheme=DynamicScheme::by_variant(seed,&variant,dark,Some(p.contrast)).into();
        let native=if dark {ThemeColor::dark()} else {ThemeColor::light()};
        let status=|color:gpui_kit::Hsla| {
            let [r,g,b]=theme::channels(theme::packed(color));
            Scheme::from(DynamicScheme::by_variant(Argb::new(255,r,g,b),&Variant::TonalSpot,dark,Some(p.contrast)))
        };
        let success=status(native.success);let warning=status(native.warning);let info=status(native.info);
        let mut colors=serde_json::Map::new();
        let mut put=|key:&str,color:Argb| {colors.insert(key.into(),json!(color.to_hex_with_pound()));};
        for key in ["background","list.background","table.background","sidebar.background","title_bar.background","status_bar.background"] {put(key,scheme.surface);}
        for key in ["foreground","caret","popover.foreground","sidebar.foreground","group_box.foreground","table.head.foreground","table.foot.foreground","description_list.label.foreground"] {put(key,scheme.on_surface);}
        for key in ["group_box.background","accordion.background","list.head.background","table.head.background","table.foot.background","tab_bar.background"] {put(key,scheme.surface_container_low);}
        for key in ["popover.background","button.background","button.secondary.background","secondary.background","tab.background"] {put(key,scheme.surface_container);}
        for key in ["muted.background","list.even.background","table.even.background","slider.background","skeleton.background","tab_bar.segmented.background"] {put(key,scheme.surface_container_high);}
        put("description_list.label.background", scheme.surface_container_high);
        put("scrollbar.background", scheme.surface_container);
        put("switch.background", scheme.surface_container_high);
        put("switch.thumb.background", scheme.primary);
        put("drop_target.background", scheme.primary_container);
        for key in ["button.foreground","tab.foreground"] {put(key,scheme.on_surface);}
        for key in ["button.secondary.foreground","secondary.foreground"] {put(key,scheme.on_secondary);}
        put("muted.foreground",scheme.on_surface_variant);
        put("group_box.title.foreground",scheme.on_surface);
        for key in ACCENT_ROLES {put(key,scheme.primary_container);}
        for key in PRIMARY_ROLES {put(key,scheme.primary);}
        for key in ["primary.foreground","button.primary.foreground","sidebar.primary.foreground"] {put(key,scheme.on_primary);}
        for key in ["accent.foreground","sidebar.accent.foreground","tab.active.foreground"] {put(key,scheme.on_primary_container);}
        for key in ["link","link.hover","link.active","ring","drag.border","list.active.border","table.active.border"] {put(key,scheme.primary);}
        put("table.row.border", scheme.outline_variant);
        for key in ["border","input.border","sidebar.border","window.border","title_bar.border","status_bar.border","scrollbar.thumb.background","chart.grid"] {put(key,scheme.outline_variant);}
        put("scrollbar.thumb.hover.background",scheme.outline);
        for (name,s) in [("danger",&scheme),("warning",&warning),("success",&success),("info",&info)] {
            let (fill,text)=if name=="danger"{(s.error,s.on_error)}else{(s.primary,s.on_primary)};
            for role in [name.to_string(),format!("button.{name}")] {
                for suffix in ["background","hover.background","active.background"] {put(&format!("{role}.{suffix}"),fill);}
                put(&format!("{role}.foreground"),text);
            }
        }
        for (key,color) in [("chart.1",scheme.primary),("chart.2",success.primary),("chart.3",warning.primary),("chart.4",scheme.secondary),("chart.5",info.primary),("chart.bullish",success.primary),("chart.bearish",scheme.error)] {put(key,color);}
        for (key,color) in [
            ("base.blue", scheme.primary), ("base.blue.light", scheme.primary_container),
            ("base.cyan", info.primary), ("base.cyan.light", info.primary_container),
            ("base.green", success.primary), ("base.green.light", success.primary_container),
            ("base.magenta", info.primary), ("base.magenta.light", info.primary_container),
            ("base.red", scheme.error), ("base.red.light", scheme.error_container),
            ("base.yellow", warning.primary), ("base.yellow.light", warning.primary_container),
        ] {put(key,color);}
        // Overlay is the Material scrim with opacity, declared in JSON rather than painted by plugins.
        colors.insert("overlay".into(),json!(format!("{}80",scheme.scrim.to_hex_with_pound())));
        json!({"name":format!("Material {} {} {} {} Generation-{generation} {}",p.theme_name,p.seed,p.variant,p.contrast,if dark{"Dark"}else{"Light"}),
            "mode":if dark{"dark"}else{"light"},"colors":colors})
    });
    let value: Value = json!({"name":"Material generated","author":"FileAtlas / Material Color Utilities",
        "url":"https://github.com/Aiving/material-colors","themes":themes});
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    let mut value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let valid_roles = serde_json::to_value(gpui_kit::component::ThemeConfigColors::default())
        .map_err(|e| e.to_string())?;
    let mut manual = std::collections::BTreeMap::new();
    for (role, color) in &p.manual_colors {
        if valid_roles.get(role).is_none() {
            return Err(format!("Unknown theme color role: {role}"));
        }
        if !PALETTE_ROLES.contains(&role.as_str()) {
            return Err(format!("Unknown semantic palette role: {role}"));
        }
        let rgb = theme::parse_hex_color(color)
            .ok_or_else(|| format!("Invalid color for {role}; expected #RRGGBB"))?;
        let canonical = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        manual.insert(role.clone(), canonical);
    }
    for (role, color) in &manual {
        set_role(&mut value, role, color)?;
    }
    let status_roles: &[&str] = &[
        "success.background",
        "success.hover.background",
        "success.active.background",
        "button.success.background",
        "button.success.hover.background",
        "button.success.active.background",
        "chart.2",
        "chart.bullish",
        "base.green",
        "base.green.light",
    ];
    let warning_roles: &[&str] = &[
        "warning.background",
        "warning.hover.background",
        "warning.active.background",
        "button.warning.background",
        "button.warning.hover.background",
        "button.warning.active.background",
        "chart.3",
        "base.yellow",
        "base.yellow.light",
    ];
    let danger_roles: &[&str] = &[
        "danger.background",
        "danger.hover.background",
        "danger.active.background",
        "button.danger.background",
        "button.danger.hover.background",
        "button.danger.active.background",
        "chart.bearish",
        "base.red",
        "base.red.light",
    ];
    for (core, roles) in [
        ("background", BACKGROUND_SURFACE_ROLES),
        ("foreground", FOREGROUND_ROLES),
        ("secondary.background", SECONDARY_ROLES),
        ("success.background", status_roles),
        ("warning.background", warning_roles),
        ("danger.background", danger_roles),
        (
            "info.background",
            &[
                "info.background",
                "info.hover.background",
                "info.active.background",
                "button.info.background",
                "button.info.hover.background",
                "button.info.active.background",
                "chart.5",
                "base.cyan",
                "base.cyan.light",
                "base.magenta",
                "base.magenta.light",
            ][..],
        ),
    ] {
        if let Some(color) = manual.get(core) {
            for role in roles.iter().copied().filter(|role| *role != core) {
                set_role(&mut value, role, color)?;
            }
        }
    }
    if let Some(background) = manual.get("background") {
        if let Some(foreground) = manual.get("foreground") {
            let background = theme::parse_hex_color(background).expect("validated background");
            let foreground = theme::parse_hex_color(foreground).expect("validated foreground");
            let background = pack(background);
            let foreground = pack(foreground);
            if theme::contrast_ratio(foreground, background) < 4.5 {
                return Err(
                    "Neutral text and surface colors must keep a 4.5:1 contrast ratio".into(),
                );
            }
        } else {
            set_contrasting_foreground(&mut value, "background", FOREGROUND_ROLES)?;
        }
    }
    for (background, foregrounds) in [
        (
            "secondary.background",
            &["secondary.foreground", "button.secondary.foreground"][..],
        ),
        (
            "success.background",
            &["success.foreground", "button.success.foreground"][..],
        ),
        (
            "warning.background",
            &["warning.foreground", "button.warning.foreground"][..],
        ),
        (
            "danger.background",
            &["danger.foreground", "button.danger.foreground"][..],
        ),
        (
            "info.background",
            &["info.foreground", "button.info.foreground"][..],
        ),
    ] {
        if manual.contains_key(background) {
            set_contrasting_foreground(&mut value, background, foregrounds)?;
        }
    }
    // User color family edits fan out only through these shared native role maps.
    for (role, color) in &manual {
        set_role(&mut value, role, color)?;
    }
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    let _: ThemeSet = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(text)
}

fn set_role(value: &mut Value, role: &str, color: &str) -> Result<(), String> {
    for entry in value["themes"]
        .as_array_mut()
        .ok_or("Invalid generated ThemeSet")?
    {
        entry["colors"][role] = json!(color);
    }
    Ok(())
}

fn set_contrasting_foreground(
    value: &mut Value,
    background_role: &str,
    foreground_roles: &[&str],
) -> Result<(), String> {
    for config in value["themes"]
        .as_array_mut()
        .ok_or("Generated ThemeSet has no themes")?
    {
        let colors = config["colors"]
            .as_object_mut()
            .ok_or("Generated ThemeConfig has no colors")?;
        let background = colors
            .get(background_role)
            .and_then(Value::as_str)
            .and_then(theme::parse_hex_color)
            .map(pack)
            .ok_or_else(|| format!("Missing generated background role: {background_role}"))?;
        let candidates = [
            "foreground",
            "primary.foreground",
            "accent.foreground",
            "popover.foreground",
            "success.foreground",
            "warning.foreground",
            "danger.foreground",
            "info.foreground",
        ];
        let mut best = candidates
            .iter()
            .filter_map(|role| {
                colors
                    .get(*role)
                    .and_then(Value::as_str)
                    .and_then(theme::parse_hex_color)
                    .map(|rgb| (pack(rgb), *role))
            })
            .max_by(|(a, _), (b, _)| {
                theme::contrast_ratio(*a, background)
                    .total_cmp(&theme::contrast_ratio(*b, background))
            })
            .map(|(color, _)| color);
        for neutral in [0x000000, 0xffffff] {
            if best.is_none_or(|current| {
                theme::contrast_ratio(neutral, background)
                    > theme::contrast_ratio(current, background)
            }) {
                best = Some(neutral);
            }
        }
        let foreground = best
            .filter(|color| theme::contrast_ratio(*color, background) >= 4.5)
            .map(|color| format!("#{color:06x}"))
            .ok_or_else(|| {
                format!("No readable foreground can be paired with {background_role}")
            })?;
        for role in foreground_roles {
            colors.insert((*role).into(), Value::String(foreground.clone()));
        }
    }
    Ok(())
}

fn pack([red, green, blue]: [u8; 3]) -> u32 {
    u32::from_be_bytes([0, red, green, blue])
}
