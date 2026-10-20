//! Material produces native GPUI ThemeSet JSON; it never mutates the GUI Theme.
use super::Preferences;
use fileatlas_ui_bridge::theme;
use gpui_kit::component::{ThemeColor, ThemeSet};
use material_colors::{
    color::Argb,
    dynamic_color::{DynamicScheme, Variant},
    scheme::Scheme,
};
use serde_json::{Value, json};

pub fn generate(p: &Preferences) -> Result<String, String> {
    if p.schema_version != 1 {
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
        for key in ["button.foreground","button.secondary.foreground","secondary.foreground","tab.foreground"] {put(key,scheme.on_surface);}
        put("muted.foreground",scheme.on_surface_variant);
        for key in ["button.hover.background","button.secondary.hover.background","secondary.hover.background","list.hover.background","table.hover.background"] {put(key,scheme.surface_container_high);}
        for key in ["button.active.background","button.secondary.active.background","secondary.active.background"] {put(key,scheme.surface_container_highest);}
        for key in ["primary.background","button.primary.background","primary.hover.background","button.primary.hover.background","primary.active.background","button.primary.active.background","sidebar.primary.background","progress.bar.background","slider.thumb.background"] {put(key,scheme.primary);}
        for key in ["primary.foreground","button.primary.foreground","sidebar.primary.foreground"] {put(key,scheme.on_primary);}
        for key in ["accent.background","selection.background","list.active.background","table.active.background","sidebar.accent.background","tab.active.background"] {put(key,scheme.primary_container);}
        for key in ["accent.foreground","sidebar.accent.foreground","tab.active.foreground"] {put(key,scheme.on_primary_container);}
        for key in ["link","link.hover","link.active","ring","drag.border","list.active.border","table.active.border"] {put(key,scheme.primary);}
        for key in ["border","input.border","sidebar.border","window.border","title_bar.border","status_bar.border","scrollbar.thumb.background","chart.grid"] {put(key,scheme.outline_variant);}
        put("scrollbar.thumb.hover.background",scheme.outline);
        for (name,s) in [("danger",&scheme),("warning",&warning),("success",&success),("info",&info)] {
            let (fill,text)=if name=="danger"{(s.error,s.on_error)}else{(s.primary,s.on_primary)};
            for role in [name.to_string(),format!("button.{name}")] {
                for suffix in ["background","hover.background","active.background"] {put(&format!("{role}.{suffix}"),fill);}
                put(&format!("{role}.foreground"),text);
            }
        }
        for (key,color) in [("chart.1",scheme.primary),("chart.2",success.primary),("chart.3",warning.primary),("chart.4",scheme.tertiary),("chart.5",scheme.secondary),("chart.bullish",success.primary),("chart.bearish",scheme.error)] {put(key,color);}
        // Overlay is the Material scrim with opacity, declared in JSON rather than painted by plugins.
        colors.insert("overlay".into(),json!(format!("{}80",scheme.scrim.to_hex_with_pound())));
        json!({"name":format!("Material {} {} {} {}",p.seed,p.variant,p.contrast,if dark{"Dark"}else{"Light"}),
            "mode":if dark{"dark"}else{"light"},"colors":colors})
    });
    let value: Value = json!({"name":"Material generated","author":"FileAtlas / Material Color Utilities",
        "url":"https://github.com/Aiving/material-colors","themes":themes});
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    let _: ThemeSet = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(text)
}
