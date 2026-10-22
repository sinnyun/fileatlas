//! Pure geometry for the WorkCrate-owned material header inside the native document.
//!
//! The official program owns text metrics, so every position here is expressed as the
//! document box the caller must land a node in, derived from the widths and heights the
//! official `document.node` report. Nothing in this module talks to VectorCraft.
use std::collections::BTreeMap;

pub(crate) const TEXT_INSET: f64 = 24.;
pub(crate) const PAGE_NUMBER_SIZE: f64 = 12.;
pub(crate) const PROJECT_NAME_SIZE: f64 = 20.;
pub(crate) const GROUP_SIZE: f64 = 13.;
pub(crate) const SUB_SIZE: f64 = 11.;
pub(crate) const PRIMARY_CENTER: f64 = 34.;
pub(crate) const SUB_FIRST_CENTER: f64 = 64.;
pub(crate) const SUB_STEP: f64 = 18.;
pub(crate) const SUB_VALUE_COLUMN: f64 = 100.;
pub(crate) const GROUP_GAP: f64 = 8.;
pub(crate) const NAME_GROUP_MIN_GAP: f64 = 16.;
pub(crate) const BAND_BOTTOM_PAD: f64 = 12.;
/// The gap between the page edge and the pale band that carries the header.
pub(crate) const BAND_INSET: f64 = 8.;
pub(crate) const BAND_RADIUS: f64 = 10.;
/// The page number sits in its own white chip, so its text is inset from the chip edge.
pub(crate) const CHIP_PAD_X: f64 = 8.;
pub(crate) const CHIP_PAD_Y: f64 = 5.;
pub(crate) const CHIP_RADIUS: f64 = 7.;
pub(crate) const NAME_CHIP_GAP: f64 = 12.;
pub(crate) const MAX_SUB_ROWS: usize = 12;

pub(crate) const COLOR_PRIMARY: &str = "#18212f";
pub(crate) const COLOR_MUTED: &str = "#6b7683";
pub(crate) const COLOR_BAND: &str = "#eef4fd";
pub(crate) const COLOR_CHIP: &str = "#ffffff";
pub(crate) const COLOR_CHIP_EDGE: &str = "#d8e5f4";
pub(crate) const COLOR_ACCENT: &str = "#2f6fd0";

pub(crate) const ROLE_PAGE_NUMBER: &str = "page_number";
pub(crate) const ROLE_PAGE_CHIP: &str = "page_number_chip";
pub(crate) const ROLE_PROJECT_NAME: &str = "project_name";
pub(crate) const ROLE_SEPARATOR: &str = "separator";
pub(crate) const ROLE_BAND: &str = "band";

/// The official box of one text node, as reported by `document.node`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Measured {
    pub width: f64,
    pub height: f64,
}

/// One page field that can appear in the header.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Field<'a> {
    pub role: &'a str,
    pub label: &'a str,
    pub text: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Band {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The white rounded chip behind the page number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Chip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placement {
    pub role: String,
    pub text: String,
    pub size: f64,
    pub color: &'static str,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub rows: Vec<Placement>,
    pub band: Band,
    pub chip: Option<Chip>,
    /// Whether material / size / quantity kept their place on the primary row.
    pub group_in_primary: bool,
}

pub(crate) fn label_role(role: &str) -> String {
    format!("label:{role}")
}

pub(crate) fn separator_role(index: usize) -> String {
    format!("{ROLE_SEPARATOR}:{index}")
}

fn measured(map: &BTreeMap<String, Measured>, role: &str) -> Result<Measured, HeaderLayoutError> {
    map.get(role)
        .copied()
        .ok_or_else(|| HeaderLayoutError::Unmeasured(role.to_owned()))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HeaderLayoutError {
    /// The official program has not reported a box for a role the layout must place.
    Unmeasured(String),
    /// Text that would print past the page edge: the band cannot hold it at this size.
    TooWide(String),
    TooManyRows(usize),
}

impl std::fmt::Display for HeaderLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unmeasured(role) => write!(f, "header text size missing for `{role}`"),
            Self::TooWide(role) => write!(f, "header text `{role}` is wider than the page"),
            Self::TooManyRows(rows) => write!(f, "more than {rows} header detail rows"),
        }
    }
}

/// The right edge of the left block: page-number chip then project name, each starting where
/// the previous one ended.
fn primary_left_extent(
    x: f64,
    page_number: &str,
    project_name: &str,
    measured_sizes: &BTreeMap<String, Measured>,
) -> Result<f64, HeaderLayoutError> {
    let mut extent = x + TEXT_INSET;
    if !page_number.is_empty() {
        extent +=
            measured(measured_sizes, ROLE_PAGE_NUMBER)?.width + CHIP_PAD_X * 2. + NAME_CHIP_GAP;
    }
    if !project_name.is_empty() {
        extent += measured(measured_sizes, ROLE_PROJECT_NAME)?.width;
    }
    Ok(extent)
}

/// The whole `material · size · quantity` run width, separators and gaps included.
fn group_extent(
    group: &[Field<'_>],
    measured_sizes: &BTreeMap<String, Measured>,
) -> Result<f64, HeaderLayoutError> {
    let mut width = 0.;
    for field in group {
        width += measured(measured_sizes, field.role)?.width;
    }
    if group.len() > 1 {
        let separator = measured(measured_sizes, &separator_role(0))?.width;
        width += (group.len() as f64 - 1.) * (separator + GROUP_GAP * 2.);
    }
    Ok(width)
}

/// Whether the material / size / quantity run still shares the primary row. The caller only
/// measures detail-row labels for the fields this answers `false` for.
pub(crate) fn group_fits(
    x: f64,
    width: f64,
    page_number: &str,
    project_name: &str,
    group: &[Field<'_>],
    measured_sizes: &BTreeMap<String, Measured>,
) -> Result<bool, HeaderLayoutError> {
    if group.is_empty() {
        return Ok(false);
    }
    let right = x + width - TEXT_INSET;
    let left_extent = primary_left_extent(x, page_number, project_name, measured_sizes)?;
    Ok(
        group_extent(group, measured_sizes)?
            <= (right - (left_extent + NAME_GROUP_MIN_GAP)).max(0.),
    )
}

/// Place the header band: page number and project name on the left, material / size /
/// quantity sharing the primary row on the right when `group_in_primary`, and everything
/// else in label/value rows below. `group` and `sub` carry only the fields with a value.
pub(crate) fn build(
    x: f64,
    y: f64,
    width: f64,
    page_number: &str,
    project_name: &str,
    group: &[Field<'_>],
    sub: &[Field<'_>],
    group_in_primary: bool,
    measured_sizes: &BTreeMap<String, Measured>,
) -> Result<Layout, HeaderLayoutError> {
    let mut placements = Vec::new();
    let left = x + TEXT_INSET;
    let right = x + width - TEXT_INSET;
    let primary_center = y + PRIMARY_CENTER;

    let mut primary_left_extent = left;
    let mut chip = None;
    if !page_number.is_empty() {
        let size = measured(measured_sizes, ROLE_PAGE_NUMBER)?;
        let boxed = Chip {
            x: left - CHIP_PAD_X,
            y: primary_center - (size.height + CHIP_PAD_Y * 2.) / 2.,
            width: size.width + CHIP_PAD_X * 2.,
            height: size.height + CHIP_PAD_Y * 2.,
            radius: CHIP_RADIUS,
        };
        placements.push(placed(
            ROLE_PAGE_NUMBER.to_owned(),
            page_number,
            PAGE_NUMBER_SIZE,
            COLOR_ACCENT,
            left,
            primary_center,
            size,
        ));
        primary_left_extent = boxed.x + boxed.width + NAME_CHIP_GAP;
        chip = Some(boxed);
    }
    if !project_name.is_empty() {
        let size = measured(measured_sizes, ROLE_PROJECT_NAME)?;
        placements.push(placed(
            ROLE_PROJECT_NAME.to_owned(),
            project_name,
            PROJECT_NAME_SIZE,
            COLOR_PRIMARY,
            primary_left_extent,
            primary_center,
            size,
        ));
    }

    let mut sub_rows: Vec<Field> = sub.to_vec();
    if group_in_primary {
        let mut cursor = right;
        for (index, field) in group.iter().enumerate().rev() {
            let size = measured(measured_sizes, field.role)?;
            cursor -= size.width;
            placements.push(placed(
                field.role.to_owned(),
                field.text,
                GROUP_SIZE,
                COLOR_ACCENT,
                cursor,
                primary_center,
                size,
            ));
            if index > 0 {
                let separator = measured(measured_sizes, &separator_role(index - 1))?;
                cursor -= GROUP_GAP;
                placements.push(placed(
                    separator_role(index - 1),
                    "·",
                    GROUP_SIZE,
                    COLOR_MUTED,
                    cursor - separator.width,
                    primary_center,
                    separator,
                ));
                cursor -= GROUP_GAP + separator.width;
            }
        }
    } else {
        sub_rows.extend_from_slice(group);
    }

    if sub_rows.len() > MAX_SUB_ROWS {
        return Err(HeaderLayoutError::TooManyRows(MAX_SUB_ROWS));
    }
    for (index, field) in sub_rows.iter().enumerate() {
        let center = y + SUB_FIRST_CENTER + index as f64 * SUB_STEP;
        let label = measured(measured_sizes, &label_role(field.role))?;
        placements.push(placed(
            label_role(field.role),
            field.label,
            SUB_SIZE,
            COLOR_MUTED,
            left,
            center,
            label,
        ));
        let value = measured(measured_sizes, field.role)?;
        placements.push(placed(
            field.role.to_owned(),
            field.text,
            SUB_SIZE,
            COLOR_PRIMARY,
            left + SUB_VALUE_COLUMN.max(label.width + 8.),
            center,
            value,
        ));
    }

    let mut bottom = primary_center;
    if let Some(chip) = &chip {
        bottom = bottom.max(chip.y + chip.height);
    }
    for placement in &placements {
        if placement.left + placement.width > right + 0.5 {
            return Err(HeaderLayoutError::TooWide(placement.role.clone()));
        }
        bottom = bottom.max(placement.top + placement.height);
    }
    let band_height = (bottom - y) + BAND_BOTTOM_PAD;
    Ok(Layout {
        rows: placements,
        band: Band {
            x: x + BAND_INSET,
            y: y + BAND_INSET,
            width: width - BAND_INSET * 2.,
            height: band_height - BAND_INSET,
        },
        chip,
        group_in_primary,
    })
}

fn placed(
    role: String,
    text: &str,
    size: f64,
    color: &'static str,
    left: f64,
    center: f64,
    measured: Measured,
) -> Placement {
    Placement {
        role,
        text: text.to_owned(),
        size,
        color,
        left,
        top: center - measured.height / 2.,
        width: measured.width,
        height: measured.height,
    }
}

#[cfg(test)]
mod tests;
