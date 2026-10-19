//! Compatibility role names mapped to the native GPUI Theme. No palette here.
use super::*;
pub fn surface_base() -> u32 {
    packed(native_colors().background)
}
pub fn surface_panel() -> u32 {
    packed(native_colors().background)
}
pub fn surface_card() -> u32 {
    packed(native_colors().group_box)
}
pub fn surface_elevated() -> u32 {
    packed(native_colors().secondary)
}
pub fn surface_modal() -> u32 {
    packed(native_colors().popover)
}
pub fn surface_faint() -> u32 {
    packed(native_colors().muted)
}
pub fn surface_audio() -> u32 {
    packed(native_colors().background)
}
pub fn surface_extreme() -> u32 {
    packed(native_colors().muted)
}
pub fn surface_rail() -> u32 {
    packed(native_colors().sidebar)
}
pub fn surface_sidebar() -> u32 {
    packed(native_colors().sidebar)
}
pub fn surface_region() -> u32 {
    packed(native_colors().group_box)
}
pub fn surface_group_header() -> u32 {
    packed(native_colors().table_head)
}
pub fn surface_group_header_hover() -> u32 {
    packed(native_colors().table_hover)
}
pub fn text_primary() -> u32 {
    packed(native_colors().foreground)
}
pub fn text_secondary() -> u32 {
    packed(native_colors().description_list_label_foreground)
}
pub fn text_muted() -> u32 {
    packed(native_colors().muted_foreground)
}
pub fn separator() -> u32 {
    packed(native_colors().border)
}
pub fn control_box() -> u32 {
    packed(native_colors().button_active)
}
pub fn control_box_hover() -> u32 {
    packed(native_colors().button_hover)
}
pub fn control_box_pressed() -> u32 {
    packed(native_colors().button_active)
}
pub fn danger() -> u32 {
    packed(native_colors().danger)
}
pub fn danger_hover() -> u32 {
    packed(native_colors().danger_hover)
}
pub fn warning() -> u32 {
    packed(native_colors().warning)
}
pub fn success() -> u32 {
    packed(native_colors().success)
}
pub fn star() -> u32 {
    packed(native_colors().warning)
}
pub fn info() -> u32 {
    packed(native_colors().info)
}
pub fn category() -> u32 {
    packed(native_colors().chart_4)
}
pub fn branch_alternate() -> u32 {
    packed(native_colors().chart_4)
}
pub fn data() -> u32 {
    packed(native_colors().chart_5)
}
pub fn accent() -> u32 {
    packed(native_colors().primary)
}
pub fn accent_hover() -> u32 {
    packed(native_colors().primary_hover)
}
pub fn selection_color() -> u32 {
    packed(native_colors().accent)
}
pub fn selection_text() -> u32 {
    packed(native_colors().accent_foreground)
}
pub fn text_on_accent() -> u32 {
    packed(native_colors().primary_foreground)
}
pub fn text_on_danger() -> u32 {
    packed(native_colors().danger_foreground)
}
pub fn text_on_warning() -> u32 {
    packed(native_colors().warning_foreground)
}
pub fn text_on_success() -> u32 {
    packed(native_colors().success_foreground)
}
pub fn accent_text() -> u32 {
    packed(native_colors().link)
}
pub fn media_badge_background() -> u32 {
    packed(native_colors().popover)
}
pub fn text_on_media_badge() -> u32 {
    packed(native_colors().popover_foreground)
}
pub fn star_on_media_badge() -> u32 {
    packed(native_colors().popover_foreground)
}
pub fn separator_light() -> u32 {
    packed_rgba(native_colors().border)
}
pub fn fill_subtle() -> u32 {
    packed_rgba(native_colors().muted)
}
pub fn fill_hover() -> u32 {
    packed_rgba(native_colors().list_hover)
}
pub fn fill_active() -> u32 {
    packed_rgba(native_colors().button_active)
}
pub fn region_edge() -> u32 {
    packed_rgba(native_colors().border)
}
pub fn scrim() -> u32 {
    packed_rgba(native_colors().overlay)
}
pub fn border_strong() -> u32 {
    packed_rgba(native_colors().list_active_border)
}
pub fn media_scrim() -> u32 {
    packed_rgba(native_colors().popover)
}
pub fn media_scrim_strong() -> u32 {
    packed_rgba(native_colors().popover)
}
pub fn tooltip_surface() -> u32 {
    packed_rgba(native_colors().popover)
}
pub fn shadow_overlay() -> u32 {
    packed_rgba(native_colors().overlay)
}
pub fn track_shadow() -> u32 {
    packed_rgba(native_colors().border)
}
pub fn track_highlight() -> u32 {
    packed_rgba(native_colors().foreground)
}
pub fn track_active() -> u32 {
    packed_rgba(native_colors().slider_bar)
}
pub fn danger_text() -> u32 {
    readable_foreground(danger())
}
pub fn warning_text() -> u32 {
    readable_foreground(warning())
}
pub fn success_text() -> u32 {
    readable_foreground(success())
}
pub fn star_text() -> u32 {
    readable_foreground(star())
}
pub fn info_text() -> u32 {
    readable_foreground(info())
}
pub fn category_text() -> u32 {
    readable_foreground(category())
}
pub fn branch_alternate_text() -> u32 {
    readable_foreground(branch_alternate())
}
pub fn data_text() -> u32 {
    readable_foreground(data())
}
pub fn fill_transparent_overlay() -> u32 {
    packed(native_colors().foreground) << 8
}
pub fn hairline() -> PremultipliedColor {
    premultiplied(native_colors().border)
}
pub fn border() -> PremultipliedColor {
    premultiplied(native_colors().border)
}
fn premultiplied(color: gpui_kit::Hsla) -> PremultipliedColor {
    let [red, green, blue] = channels(packed(color));
    PremultipliedColor {
        red,
        green,
        blue,
        alpha: 255,
    }
}
