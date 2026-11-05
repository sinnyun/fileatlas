//! Pure gallery geometry. Rendering consumes placements without deciding order.

use fileatlas_plugin_workspaces::GalleryLayout;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placement {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Geometry {
    pub placements: Vec<Placement>,
    pub content_height: f32,
}

pub(super) fn arrange(
    layout: GalleryLayout,
    ratios: &[f32],
    width: f32,
    preferred: f32,
    gap: f32,
) -> Geometry {
    let width = width.max(1.);
    let preferred = preferred.max(1.);
    let ratios = ratios.iter().map(|r| valid_ratio(*r)).collect::<Vec<_>>();
    match layout {
        GalleryLayout::Grid | GalleryLayout::List => Geometry {
            placements: Vec::new(),
            content_height: 0.,
        },
        GalleryLayout::Waterfall => waterfall(&ratios, width, preferred, gap),
        GalleryLayout::Justified => justified(&ratios, width, preferred, gap),
    }
}

fn valid_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.
    }
}

fn waterfall(ratios: &[f32], width: f32, preferred: f32, gap: f32) -> Geometry {
    let columns = ((width + gap) / (preferred + gap)).floor().max(1.) as usize;
    let card_width = ((width - gap * (columns.saturating_sub(1)) as f32) / columns as f32).max(1.);
    let mut bottoms: Vec<f32> = vec![0.; columns];
    let mut placements = Vec::with_capacity(ratios.len());
    for (index, ratio) in ratios.iter().enumerate() {
        let col = bottoms
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let height = card_width / ratio;
        placements.push(Placement {
            index,
            x: col as f32 * (card_width + gap),
            y: bottoms[col],
            width: card_width,
            height,
        });
        bottoms[col] += height + 38. + gap;
    }
    Geometry {
        placements,
        content_height: bottoms.into_iter().fold(0., f32::max).max(0.)
            - if ratios.is_empty() { 0. } else { gap },
    }
}

fn justified(ratios: &[f32], width: f32, target_height: f32, gap: f32) -> Geometry {
    let mut placements = Vec::with_capacity(ratios.len());
    let mut start = 0;
    let mut y = 0.;
    while start < ratios.len() {
        let mut end = start;
        let mut sum = 0.;
        while end < ratios.len() {
            let next = sum + ratios[end];
            let count = end - start + 1;
            let height = (width - gap * (count.saturating_sub(1)) as f32) / next;
            if end > start && height < target_height * 0.78 {
                break;
            }
            sum = next;
            end += 1;
            if height <= target_height {
                break;
            }
        }
        let count = end - start;
        let ideal = (width - gap * count.saturating_sub(1) as f32) / sum.max(0.01);
        let height = if end == ratios.len() {
            target_height.min(ideal)
        } else {
            ideal
        };
        let mut x = 0.;
        for (offset, ratio) in ratios[start..end].iter().enumerate() {
            let card_width = ratio * height;
            placements.push(Placement {
                index: start + offset,
                x,
                y,
                width: card_width,
                height,
            });
            x += card_width + gap;
        }
        y += height + 38. + gap;
        start = end;
    }
    Geometry {
        placements,
        content_height: (y - gap).max(0.),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waterfall_places_each_card_in_the_shortest_column_without_distorting_ratio() {
        let result = arrange(GalleryLayout::Waterfall, &[1., 2., 1.], 400., 180., 10.);
        assert_eq!(result.placements.len(), 3);
        assert!(result.placements[2].y > result.placements[1].y);
        for (placement, ratio) in result.placements.iter().zip([1., 2., 1.]) {
            assert!((placement.width / placement.height - ratio).abs() < 0.001);
        }
    }

    #[test]
    fn justified_full_row_preserves_ratios_and_fills_width() {
        let result = arrange(GalleryLayout::Justified, &[1., 2., 1., 1.], 600., 120., 16.);
        assert_eq!(result.placements.len(), 4);
        let row_y = result.placements[0].y;
        assert!(
            result
                .placements
                .iter()
                .all(|item| (item.y - row_y).abs() < 0.001)
        );
        for (placement, ratio) in result.placements.iter().zip([1., 2., 1., 1.]) {
            assert!((placement.width / placement.height - ratio).abs() < 0.001);
        }
        let Some(last) = result.placements.last() else {
            return;
        };
        assert!((last.x + last.width - 600.).abs() < 0.001);
    }
}
