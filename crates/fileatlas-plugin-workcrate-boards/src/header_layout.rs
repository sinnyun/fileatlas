//! Fixed native header anchors. No per-field text measurements or wrapping layout.
pub(crate) const TEXT_INSET: f64 = 24.;
pub(crate) const TEXT_FONT: &str = "Microsoft YaHei";
pub(crate) const PAGE_NUMBER_SIZE: f64 = 12.;
pub(crate) const PROJECT_NAME_SIZE: f64 = 20.;
pub(crate) const GROUP_SIZE: f64 = 13.;
pub(crate) const PRIMARY_CENTER: f64 = 34.;
pub(crate) const BAND_RADIUS: f64 = 10.;
pub(crate) const CHIP_RADIUS: f64 = 7.;
// Native document ink, independent of the application's UI theme.
pub(crate) const COLOR_TEXT: &str = "#000000";
pub(crate) const COLOR_BAND: &str = "#eef4fd";
pub(crate) const COLOR_CHIP: &str = "#ffffff";
pub(crate) const COLOR_CHIP_EDGE: &str = "#d8e5f4";
pub(crate) const ROLE_PAGE_NUMBER: &str = "page_number";
pub(crate) const ROLE_PROJECT_NAME: &str = "project_name";
pub(crate) const ROLE_DETAILS: &str = "details";

#[derive(Clone, Copy)]
pub(crate) struct Geometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}
impl Geometry {
    pub fn split(&self) -> f64 {
        self.x + self.width * 0.33
    }
    pub fn right(&self) -> f64 {
        self.x + self.width - TEXT_INSET
    }
    pub fn category_x(&self) -> f64 {
        self.x + 68.
    }
    pub fn centre_y(&self) -> f64 {
        self.y + PRIMARY_CENTER
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchors_are_independent_of_field_content_and_follow_artboard_origin() {
        let g = Geometry {
            x: 200.,
            y: 100.,
            width: 1190.,
        };
        assert_eq!(g.right(), 1366.);
        assert_eq!(g.category_x(), 268.);
        assert_eq!(g.centre_y(), 134.);
        assert!(g.split() < g.right());
    }
}
