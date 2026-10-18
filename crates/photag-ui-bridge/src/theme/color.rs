/// Source colors are premultiplied because they are composited over the
/// active surface. Keep alpha explicit when adapting them to GPUI paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PremultipliedColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl PremultipliedColor {
    /// Pack this source color as the straight-alpha `0xRRGGBBAA` value that
    /// GPUI's `rgba` helper expects. Source egui tokens are already premultiplied.
    pub const fn to_rgba(self) -> u32 {
        if self.alpha == 0 {
            return 0;
        }
        ((unpremultiply(self.red, self.alpha) as u32) << 24)
            | ((unpremultiply(self.green, self.alpha) as u32) << 16)
            | ((unpremultiply(self.blue, self.alpha) as u32) << 8)
            | self.alpha as u32
    }
}

const fn unpremultiply(channel: u8, alpha: u8) -> u8 {
    let value = (channel as u32 * 255 + alpha as u32 / 2) / alpha as u32;
    if value > 255 { 255 } else { value as u8 }
}
