use crate::compositor::types::{AbsoluteContent, AbsoluteElement};
use crate::grid_layout::types::GridRow;
use crate::grid_layout::PAGE_MARGIN;

/// The per-page layout data every row-range resolver below needs, bundled
/// into one value so `resolve_row_range_geometry` stays under the repo's
/// max-argument-count lint (it would otherwise need 8 parameters). Shared
/// with the click-target resolvers in `highlight_click_targets` — the same
/// row-range math backs both highlight rects and click hit targets.
#[derive(Clone, Copy)]
pub(super) struct RowLayoutContext<'a> {
    pub(super) rows: &'a [GridRow],
    pub(super) row_tops: &'a [f32],
    pub(super) usable_width: f32,
    pub(super) part_label_width_pt: f32,
}

/// Pixel geometry of one row range (`row_start..=row_end`) restricted to one
/// column range (`column_start..column_end`) — the shared math behind every
/// row-range-shaped highlight/click target below (measure highlight, error
/// highlight, playback cursor, note/measure/part-label click targets). `None`
/// when `row_start` or `row_end` falls outside `ctx.rows`.
pub(super) struct RowRangeGeometry {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

pub(super) fn resolve_row_range_geometry(
    row_start: usize,
    row_end: usize,
    column_start: f32,
    column_end: f32,
    ctx: RowLayoutContext,
) -> Option<RowRangeGeometry> {
    let RowLayoutContext {
        rows,
        row_tops,
        usable_width,
        part_label_width_pt,
    } = ctx;
    let start_row = rows.get(row_start)?;
    let y = row_tops.get(row_start)?;
    if row_end >= rows.len() {
        return None;
    }
    let geometry = start_row.column_geometry(usable_width, part_label_width_pt);
    let x = PAGE_MARGIN + geometry.x_start(column_start);
    let width = geometry.x_start(column_end) - geometry.x_start(column_start);
    let height = rows
        .get(row_start..=row_end)
        .map(|slice| slice.iter().map(|row| row.height_pt).sum())
        .unwrap_or(0.0);
    Some(RowRangeGeometry {
        x,
        y: *y,
        width,
        height,
    })
}

pub(super) fn resolve_measure_highlights(
    highlights: &[crate::grid_layout::types::MeasureHighlight],
    rows: &[GridRow],
    row_tops: &[f32],
    usable_width: f32,
    part_label_width_pt: f32,
) -> Vec<AbsoluteElement> {
    let ctx = RowLayoutContext {
        rows,
        row_tops,
        usable_width,
        part_label_width_pt,
    };
    highlights
        .iter()
        .filter_map(|h| {
            let geometry = resolve_row_range_geometry(
                h.row_start,
                h.row_end,
                h.column_start,
                h.column_end,
                ctx,
            )?;
            Some(AbsoluteElement {
                x: geometry.x,
                y: geometry.y,
                content: AbsoluteContent::MeasureHighlight {
                    width: geometry.width,
                    height: geometry.height,
                },
            })
        })
        .collect()
}

pub(super) fn resolve_error_highlights(
    highlights: &[crate::grid_layout::types::MeasureHighlight],
    rows: &[GridRow],
    row_tops: &[f32],
    usable_width: f32,
    part_label_width_pt: f32,
) -> Vec<AbsoluteElement> {
    let ctx = RowLayoutContext {
        rows,
        row_tops,
        usable_width,
        part_label_width_pt,
    };
    highlights
        .iter()
        .filter_map(|h| {
            let geometry = resolve_row_range_geometry(
                h.row_start,
                h.row_end,
                h.column_start,
                h.column_end,
                ctx,
            )?;
            Some(AbsoluteElement {
                x: geometry.x,
                y: geometry.y,
                content: AbsoluteContent::ErrorHighlight {
                    width: geometry.width,
                    height: geometry.height,
                },
            })
        })
        .collect()
}

pub(super) fn resolve_playback_cursor_target(
    target: &crate::grid_layout::types::PlaybackCursorTarget,
    rows: &[GridRow],
    row_tops: &[f32],
    usable_width: f32,
    part_label_width_pt: f32,
) -> Option<AbsoluteElement> {
    let ctx = RowLayoutContext {
        rows,
        row_tops,
        usable_width,
        part_label_width_pt,
    };
    let geometry = resolve_row_range_geometry(
        target.row_start,
        target.row_end,
        target.column_start,
        target.column_end,
        ctx,
    )?;
    Some(AbsoluteElement {
        x: geometry.x,
        y: geometry.y,
        content: AbsoluteContent::PlaybackCursorTarget {
            width: geometry.width,
            height: geometry.height,
            source_part_index: target.source_part_index,
            note_id: target.note_id,
        },
    })
}
