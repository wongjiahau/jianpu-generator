use crate::compiler::types::MeasureBlock;
use crate::grid_layout::highlight::measure_column_bounds;
use crate::grid_layout::layout::{
    block_column_width, directive_line_should_emit, is_lyric_row, LABEL_COLS, MUSIC_START_COL,
};
use crate::grid_layout::playback_cursor::{compute_all_playback_cursor_targets, note_row_spans};
use crate::grid_layout::system_walk::for_each_system;
use crate::grid_layout::types::{
    BarLineClickTarget, BarNumberClickTarget, GridElement, Header, LyricClickTarget,
    LyricLabelClickTarget, MeasureClickTarget, MeasureHighlight, MeasureRange,
    PartLabelClickTarget, PlaybackCursorTarget,
};
use std::collections::HashMap;

use super::highlight::{compute_error_highlight_infos, compute_measure_highlights_for_range};

#[path = "click_targets_lyric.rs"]
mod click_targets_lyric;
pub(crate) use click_targets_lyric::{
    compute_all_lyric_click_targets, compute_all_lyric_label_click_targets,
};

pub(crate) fn compute_all_measure_click_targets(
    page_systems: &[Vec<Vec<MeasureBlock>>],
    tuplet_bracket_map: &HashMap<(usize, usize), Vec<GridElement>>,
    header: &Header,
    base: f32,
    hide_system_dividers: bool,
) -> Vec<(usize, MeasureClickTarget)> {
    let mut global_measure_index: usize = 0;
    let mut results: Vec<(usize, MeasureClickTarget)> = Vec::new();

    for_each_system(
        page_systems,
        tuplet_bracket_map,
        header,
        base,
        hide_system_dividers,
        |page_idx, system, row_offset, _tuplet_part_indices, musical_row_count| {
            let row_start = row_offset;
            let row_end = row_offset + musical_row_count.saturating_sub(1);

            let mut col_offset: u32 = MUSIC_START_COL;
            let last_block_idx = system.len().saturating_sub(1);
            for (block_idx, block) in system.iter().enumerate() {
                let col_w = block_column_width(block);
                let (column_start, column_end) = measure_column_bounds(
                    col_offset,
                    col_w,
                    block_idx == 0,
                    block_idx == last_block_idx,
                );
                results.push((
                    page_idx,
                    MeasureClickTarget {
                        row_start,
                        row_end,
                        column_start,
                        column_end,
                        measure_index: global_measure_index,
                        measure_index_end: global_measure_index
                            + block.represents_measures.saturating_sub(1),
                    },
                ));
                col_offset += col_w;
                global_measure_index += block.represents_measures;
            }
        },
    );
    results
}

/// One [`BarNumberClickTarget`] per bar number actually drawn in every
/// system, keyed by page index like `compute_all_measure_click_targets`.
/// Walks `page_systems` in the same order (and with the same
/// `global_measure_index` accumulation) as `compute_all_measure_click_targets`
/// and `make_decoration_row`, so a target's `measure_index`/`measure_index_end`
/// and its `column` (the block's own leading-barline column) always agree
/// with what's actually drawn there.
pub(crate) fn compute_all_bar_number_click_targets(
    page_systems: &[Vec<Vec<MeasureBlock>>],
    tuplet_bracket_map: &HashMap<(usize, usize), Vec<GridElement>>,
    header: &Header,
    base: f32,
    hide_system_dividers: bool,
) -> Vec<(usize, BarNumberClickTarget)> {
    let mut global_measure_index: usize = 0;
    let mut results: Vec<(usize, BarNumberClickTarget)> = Vec::new();

    for_each_system(
        page_systems,
        tuplet_bracket_map,
        header,
        base,
        hide_system_dividers,
        |page_idx, system, row_offset, _tuplet_part_indices, _musical_row_count| {
            // The decoration row is always the row immediately above
            // `row_offset` when the system draws one — see
            // `for_each_system`'s doc comment. In practice every non-empty
            // system does (every block compiled from real source always
            // carries a `DirectiveLine` decoration — see
            // `compiler::collect_decorations`), but `row` is only ever read
            // below when a block actually has a decoration to draw, so an
            // artificial system with none (as in some tests) is harmless.
            let row = row_offset.saturating_sub(1);

            let mut leading_barline_col = LABEL_COLS;
            for (index, block) in system.iter().enumerate() {
                if let Some(dec) = block.decorations.first() {
                    if directive_line_should_emit(index, dec) {
                        results.push((
                            page_idx,
                            BarNumberClickTarget {
                                row,
                                column: leading_barline_col,
                                measure_index: global_measure_index,
                                measure_index_end: global_measure_index
                                    + block.represents_measures.saturating_sub(1),
                            },
                        ));
                    }
                }
                leading_barline_col += block_column_width(block);
                global_measure_index += block.represents_measures;
            }
        },
    );
    results
}

/// One [`BarLineClickTarget`] per bar line actually drawn in every system —
/// one before the first block (leading, `prev = None`) plus one after every
/// block (trailing, `next = None` only for the last) — `system.len() + 1`
/// per system. Walks `page_systems` in the same order and with the same
/// `global_measure_index`/`col_offset` accumulation as
/// `compute_all_measure_click_targets`, so a boundary's `column` always
/// lines up with the measures it sits between.
///
/// The actual rendered bar-line glyph at a system boundary doesn't sit at
/// the raw `col_offset` value — it's drawn at the same column
/// `measure_column_bounds` uses for the flanking measures' own click-target
/// edge there (a leading system divider is padded a full column left of
/// `col_offset`, an internal one half a column left, a closing one sits
/// exactly at `col_offset`), so `column` mirrors that same
/// `is_first_block`/`is_last_block` adjustment rather than the plain
/// accumulator, or this target's rect would be centered off the real line.
pub(crate) fn compute_all_bar_line_click_targets(
    page_systems: &[Vec<Vec<MeasureBlock>>],
    tuplet_bracket_map: &HashMap<(usize, usize), Vec<GridElement>>,
    header: &Header,
    base: f32,
    hide_system_dividers: bool,
) -> Vec<(usize, BarLineClickTarget)> {
    let mut global_measure_index: usize = 0;
    let mut results: Vec<(usize, BarLineClickTarget)> = Vec::new();

    for_each_system(
        page_systems,
        tuplet_bracket_map,
        header,
        base,
        hide_system_dividers,
        |page_idx, system, row_offset, _tuplet_part_indices, musical_row_count| {
            let row_start = row_offset;
            let row_end = row_offset + musical_row_count.saturating_sub(1);
            let mut col_offset: u32 = MUSIC_START_COL;

            results.push((
                page_idx,
                BarLineClickTarget {
                    row_start,
                    row_end,
                    column: col_offset as f32 - 1.0,
                    measure_index_next: Some(global_measure_index),
                    measure_index_prev: None,
                },
            ));

            let last_block_idx = system.len().saturating_sub(1);
            for (block_idx, block) in system.iter().enumerate() {
                col_offset += block_column_width(block);
                global_measure_index += block.represents_measures;
                let is_last_block = block_idx == last_block_idx;
                results.push((
                    page_idx,
                    BarLineClickTarget {
                        row_start,
                        row_end,
                        column: col_offset as f32 - if is_last_block { 0.0 } else { 0.5 },
                        measure_index_next: (!is_last_block).then_some(global_measure_index),
                        measure_index_prev: Some(global_measure_index.saturating_sub(1)),
                    },
                ));
            }
        },
    );
    results
}

/// Filters a `compute_all_*_click_target`/`compute_all_playback_cursor_targets`
/// result down to the entries for one page — shared by every `*_on_page` call
/// site in `grid_layout/layout.rs`.
pub(crate) fn targets_on_page<T: Clone>(targets: &[(usize, T)], page_idx: usize) -> Vec<T> {
    targets
        .iter()
        .filter(|(p, _)| *p == page_idx)
        .map(|(_, t)| t.clone())
        .collect()
}

/// One [`PartLabelClickTarget`] per labeled part row in every system, keyed
/// by page index like `compute_all_measure_click_targets`. The
/// `measure_index_start`/`measure_index_end` given to each system's labels
/// come from the same `global_measure_index` accumulation
/// `compute_all_measure_click_targets` performs — both functions walk
/// `page_systems` in identical order, so the running total agrees at every
/// system boundary.
pub(crate) fn compute_all_part_label_click_targets(
    page_systems: &[Vec<Vec<MeasureBlock>>],
    tuplet_bracket_map: &HashMap<(usize, usize), Vec<GridElement>>,
    header: &Header,
    base: f32,
    hide_system_dividers: bool,
) -> Vec<(usize, PartLabelClickTarget)> {
    let mut global_measure_index: usize = 0;
    let mut results: Vec<(usize, PartLabelClickTarget)> = Vec::new();

    for_each_system(
        page_systems,
        tuplet_bracket_map,
        header,
        base,
        hide_system_dividers,
        |page_idx, system, row_offset, tuplet_part_indices, _musical_row_count| {
            let part_spans = note_row_spans(system, row_offset, tuplet_part_indices);

            let measure_index_start = global_measure_index;
            for block in system {
                global_measure_index += block.represents_measures;
            }
            let measure_index_end = global_measure_index.saturating_sub(1);

            if let Some(first) = system.first() {
                for (part_idx, span) in part_spans.iter().enumerate() {
                    let Some(part_template) = first.rows.get(part_idx) else {
                        continue;
                    };
                    // Mirror `expand_note_part`'s own guard: only a note row
                    // ever gets a `RowLabel` drawn (a standalone `lyrics`
                    // part's row, or an absorbed verse row, never does, even
                    // though it may still carry a non-empty `label`).
                    if part_template.label.is_empty() || is_lyric_row(part_template) {
                        continue;
                    }
                    results.push((
                        page_idx,
                        PartLabelClickTarget {
                            row_start: span.row_start,
                            // Stops at the note row's own rows — unlike
                            // `PlaybackCursorTarget`, this rect must not
                            // reach into a following lyric verse row, since
                            // that row draws its own `LyricLabelClickTarget`
                            // rect in the same label gutter column; absorbing
                            // it here would make this rect's hover fill
                            // visually paint over that separate label too. A
                            // part-label range selection still selects the
                            // lyrics under it (see `lyricCellsForPartLabels`),
                            // which is resolved from `source_part_index` and
                            // the measure range, not from this rect's height.
                            row_end: span.click_row_end,
                            source_part_index: part_template.source_part_index,
                            measure_index_start,
                            measure_index_end,
                        },
                    ));
                }
            }
        },
    );
    results
}

pub(crate) struct HighlightAndClickInfos {
    pub(crate) highlight_infos: Vec<(usize, MeasureHighlight)>,
    pub(crate) error_highlight_infos: Vec<(usize, MeasureHighlight)>,
    pub(crate) all_click_target_infos: Vec<(usize, MeasureClickTarget)>,
    pub(crate) all_playback_cursor_target_infos: Vec<(usize, PlaybackCursorTarget)>,
    pub(crate) all_part_label_click_target_infos: Vec<(usize, PartLabelClickTarget)>,
    pub(crate) all_lyric_click_target_infos: Vec<(usize, LyricClickTarget)>,
    pub(crate) all_lyric_label_click_target_infos: Vec<(usize, LyricLabelClickTarget)>,
    pub(crate) all_bar_number_click_target_infos: Vec<(usize, BarNumberClickTarget)>,
    pub(crate) all_bar_line_click_target_infos: Vec<(usize, BarLineClickTarget)>,
}

pub(crate) struct HighlightAndClickInfosParams<'a> {
    pub(crate) blocks: &'a [MeasureBlock],
    pub(crate) page_systems: &'a [Vec<Vec<MeasureBlock>>],
    pub(crate) tuplet_bracket_map: &'a HashMap<(usize, usize), Vec<GridElement>>,
    pub(crate) header: &'a Header,
    pub(crate) base: f32,
    pub(crate) hide_system_dividers: bool,
    pub(crate) highlighted_measure_ranges: Option<Vec<MeasureRange>>,
}

pub(crate) fn compute_highlight_and_click_infos(
    params: &HighlightAndClickInfosParams<'_>,
) -> HighlightAndClickInfos {
    let HighlightAndClickInfosParams {
        blocks,
        page_systems,
        tuplet_bracket_map,
        header,
        base,
        hide_system_dividers,
        highlighted_measure_ranges,
    } = params;
    let (blocks, page_systems, tuplet_bracket_map, header, base, hide_system_dividers) = (
        *blocks,
        *page_systems,
        *tuplet_bracket_map,
        *header,
        *base,
        *hide_system_dividers,
    );
    let highlight_infos = highlighted_measure_ranges
        .as_ref()
        .map(Vec::as_slice)
        .map(|ranges| {
            compute_measure_highlights_for_range(
                page_systems,
                tuplet_bracket_map,
                ranges,
                header,
                base,
                hide_system_dividers,
            )
        })
        .unwrap_or_default();
    let error_highlight_infos = compute_error_highlight_infos(
        blocks,
        page_systems,
        tuplet_bracket_map,
        header,
        base,
        hide_system_dividers,
    );

    // Every `compute_all_*_click_targets` below shares this same
    // `(page_systems, tuplet_bracket_map, header, base, hide_system_dividers)`
    // signature (see `for_each_system`), so a local macro collapses each
    // call to one line instead of a 6-line block, keeping this function
    // under the repo's max-line-count lint.
    macro_rules! compute_all {
        ($f:expr) => {
            $f(
                page_systems,
                tuplet_bracket_map,
                header,
                base,
                hide_system_dividers,
            )
        };
    }
    let all_click_target_infos = compute_all!(compute_all_measure_click_targets);
    let all_playback_cursor_target_infos = compute_all!(compute_all_playback_cursor_targets);
    let all_part_label_click_target_infos = compute_all!(compute_all_part_label_click_targets);
    let all_lyric_click_target_infos = compute_all!(compute_all_lyric_click_targets);
    let all_lyric_label_click_target_infos = compute_all!(compute_all_lyric_label_click_targets);
    let all_bar_number_click_target_infos = compute_all!(compute_all_bar_number_click_targets);
    let all_bar_line_click_target_infos = compute_all!(compute_all_bar_line_click_targets);

    HighlightAndClickInfos {
        highlight_infos,
        error_highlight_infos,
        all_click_target_infos,
        all_playback_cursor_target_infos,
        all_part_label_click_target_infos,
        all_lyric_click_target_infos,
        all_lyric_label_click_target_infos,
        all_bar_number_click_target_infos,
        all_bar_line_click_target_infos,
    }
}
