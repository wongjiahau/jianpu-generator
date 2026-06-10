use crate::ast::grouped::{MultiPartMeasure, PartRow, Score};
use crate::layout::types::{
    Footer, GridContent, GridElement, GridPosition, Header, HorizontalAlignment, Page, RowGroup,
    VerticalAlignment,
};

use super::{
    flush_beam_buffer, measure_beat_width, measure_column_width, part_row_height, BeamBufferEntry,
    SlurKey, PAGE_MARGIN,
};

#[path = "part_emit.rs"]
mod part_emit;

use part_emit::{emit_timed_part, PartNoteState};

fn measure_has_directive_labels(measure: &MultiPartMeasure) -> bool {
    measure.time_signature.is_some() || measure.bpm.is_some()
}

/// Which parts of a measure render: true = ditto (suppressed), false = active.
/// All measures sharing a system line must have an identical pattern.
fn measure_ditto_pattern(measure: &MultiPartMeasure) -> Vec<bool> {
    measure.parts.iter().map(PartRow::is_ditto).collect()
}

fn line_has_any_directive_labels(
    measures: &[MultiPartMeasure],
    start_idx: usize,
    columns_per_row: u32,
    line_start_col: u32,
) -> bool {
    let mut col = line_start_col;
    let line_pattern = measures.get(start_idx).map(measure_ditto_pattern);
    for measure in measures.get(start_idx..).into_iter().flatten() {
        if line_pattern.as_deref() != Some(&measure_ditto_pattern(measure)) {
            break;
        }
        if measure_has_directive_labels(measure) {
            return true;
        }
        let width = measure_column_width(measure);
        if col.saturating_add(width) > columns_per_row {
            break;
        }
        col = col.saturating_add(width);
    }
    false
}

pub(crate) struct LayoutEngine<'a> {
    score: &'a Score,
    page_width_pt: f32,
    columns_per_row: u32,
    /// Height of the current line's part rows. Recomputed per line: ditto
    /// parts occupy no rows, so lines with different ditto patterns differ.
    row_group_height: u32,
    bar_height: u32,
    /// Ditto pattern (one bool per part) of the line being filled. A measure
    /// with a different pattern cannot share the line and forces a wrap.
    current_line_pattern: Vec<bool>,
    /// Per-part row heights of the current line, used to locate part rows
    /// when flushing buffers at a line wrap.
    line_part_heights: Vec<u32>,
    has_named_parts: bool,
    label_cols: u32,
    header_rows: u32,
    pages: Vec<Page>,
    current_page_row_groups: Vec<RowGroup>,
    current_elements: Vec<GridElement>,
    current_col: u32,
    current_row_offset: u32,
    is_line_start: bool,
    bar_number: u32,
    measure_index: usize,
    line_has_directive_row: bool,
    effective_row_group_height: u32,
    current_page_rows_used: u32,
    max_rows_per_page: u32,
    per_part_states: Vec<PerPartLayoutState>,
}

struct PerPartLayoutState {
    prev_tie: bool,
    prev_slur_key: Option<SlurKey>,
    beam_buffer: Vec<BeamBufferEntry>,
    pending_chains: Vec<Vec<(u32, SlurKey)>>,
    chain_row: u32,
    cross_line_tie: Option<SlurKey>,
}

impl<'a> LayoutEngine<'a> {
    pub(crate) fn new(score: &'a Score, page_width_pt: f32, page_height_pt: f32) -> Self {
        let row_height = score.metadata.row_height as f32;
        let columns_per_row = score.metadata.max_columns;

        let row_group_height: u32 = score
            .measures
            .first()
            .map(|m| m.parts.iter().map(part_row_height).sum::<u32>())
            .unwrap_or(3)
            .max(3);
        let bar_height: u32 = row_group_height - 1;

        let num_notes_parts = score
            .measures
            .first()
            .map(|m| m.parts.len())
            .unwrap_or(1)
            .max(1) as u32;

        let has_named_parts = score
            .measures
            .first()
            .map(|m| m.parts.iter().any(|p| p.name().is_some()))
            .unwrap_or(false);
        let label_cols: u32 = if has_named_parts {
            ((score.metadata.label_width as f32 / row_height).ceil()) as u32
        } else {
            0
        };

        let header_rows: u32 = if score.metadata.subtitle.is_some() {
            3
        } else {
            2
        };
        let footer_rows: u32 = 1;
        let reserved_rows = header_rows + footer_rows;
        let usable_height = page_height_pt - 2.0 * PAGE_MARGIN;
        let max_rows_per_page = ((usable_height / row_height) as u32).saturating_sub(reserved_rows);

        let num_notes_parts_usize = num_notes_parts as usize;
        Self {
            score,
            page_width_pt,
            columns_per_row,
            row_group_height,
            bar_height,
            current_line_pattern: Vec::new(),
            line_part_heights: Vec::new(),
            has_named_parts,
            label_cols,
            header_rows,
            pages: Vec::new(),
            current_page_row_groups: Vec::new(),
            current_elements: Vec::new(),
            current_col: label_cols,
            current_row_offset: header_rows,
            is_line_start: true,
            bar_number: 1,
            measure_index: 0,
            line_has_directive_row: false,
            effective_row_group_height: row_group_height,
            current_page_rows_used: 0,
            max_rows_per_page,
            per_part_states: (0..num_notes_parts_usize)
                .map(|_| PerPartLayoutState {
                    prev_tie: false,
                    prev_slur_key: None,
                    beam_buffer: Vec::new(),
                    pending_chains: Vec::new(),
                    chain_row: 0,
                    cross_line_tie: None,
                })
                .collect(),
        }
    }

    pub(crate) fn layout(mut self) -> Vec<Page> {
        while let Some(measure) = self.score.measures.get(self.measure_index) {
            self.wrap_line_if_needed(measure);
            self.refresh_line_row_state();
            self.emit_line_start_elements(measure);
            self.emit_section_label(measure);
            let note_col_start = self.emit_measure_directives(measure);
            self.emit_measure_content(measure, note_col_start);
            self.measure_index += 1;
        }
        self.finalize_pages()
    }

    fn refresh_line_row_state(&mut self) {
        if !self.is_line_start {
            return;
        }
        if let Some(measure) = self.score.measures.get(self.measure_index) {
            self.current_line_pattern = measure_ditto_pattern(measure);
            self.line_part_heights = measure.parts.iter().map(part_row_height).collect();
            self.row_group_height = self.line_part_heights.iter().sum::<u32>().max(3);
            self.bar_height = self.row_group_height - 1;
        }
        self.line_has_directive_row = line_has_any_directive_labels(
            &self.score.measures,
            self.measure_index,
            self.columns_per_row,
            self.label_cols,
        );
        self.effective_row_group_height =
            self.row_group_height + u32::from(self.line_has_directive_row);
        if !self.current_page_row_groups.is_empty()
            && self.current_page_rows_used + self.effective_row_group_height
                > self.max_rows_per_page
        {
            self.flush_page();
        }
    }

    fn meta_row(&self) -> u32 {
        self.current_row_offset + u32::from(self.line_has_directive_row)
    }

    fn part_row_base(&self) -> u32 {
        self.current_row_offset + 1 + u32::from(self.line_has_directive_row)
    }

    fn flush_page(&mut self) {
        if self.current_page_row_groups.is_empty() {
            return;
        }
        self.pages.push(Page {
            header: self.make_header(),
            footer: Footer {
                page: self.pages.len() as u32 + 1,
                total: 0,
            },
            row_groups: std::mem::take(&mut self.current_page_row_groups),
            page_width_pt: self.page_width_pt,
        });
        self.current_row_offset = self.header_rows;
        self.current_page_rows_used = 0;
    }

    fn make_header(&self) -> Header {
        Header {
            title: self.score.metadata.title.clone(),
            subtitle: self.score.metadata.subtitle.clone(),
            author: self.score.metadata.author.clone(),
        }
    }

    fn push_bottom_system_bar(&mut self) {
        self.current_elements.push(GridElement {
            position: GridPosition {
                column: 0,
                row: self.current_row_offset + self.effective_row_group_height,
            },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            content: GridContent::HorizontalBar {
                from_column: 0,
                to_column: self.current_col,
            },
        });
    }

    fn commit_row_group(&mut self) {
        if let Some(elements) =
            nonempty::NonEmpty::from_vec(std::mem::take(&mut self.current_elements))
        {
            self.current_page_row_groups.push(RowGroup {
                elements,
                height_in_rows: self.effective_row_group_height,
                width_in_columns: self.current_col,
            });
            self.current_page_rows_used += self.effective_row_group_height;
        }
    }

    fn maybe_start_new_page(&mut self) {
        if self.current_page_rows_used >= self.max_rows_per_page {
            self.flush_page();
        }
    }

    fn wrap_line_if_needed(&mut self, measure: &MultiPartMeasure) {
        let measure_width = measure_column_width(measure);
        let width_overflow = self.current_col + measure_width > self.columns_per_row;
        // Every measure on a line shares one vertical layout, so a measure
        // whose ditto pattern differs from the line's must start a new line.
        let pattern_change =
            !self.is_line_start && measure_ditto_pattern(measure) != self.current_line_pattern;

        if !width_overflow && !pattern_change {
            return;
        }

        // Flush offsets must come from the current line's part heights, not
        // the incoming measure's — their ditto patterns may differ.
        let mut flush_row_cursor = self.current_row_offset;
        for part_idx_flush in 0..self.per_part_states.len() {
            if let Some(state) = self.per_part_states.get_mut(part_idx_flush) {
                flush_beam_buffer(
                    &mut state.beam_buffer,
                    flush_row_cursor,
                    &mut self.current_elements,
                );
            }
            flush_row_cursor += self
                .line_part_heights
                .get(part_idx_flush)
                .copied()
                .unwrap_or(0);
        }

        for (part_idx_tie, _part_row) in measure.parts.iter().enumerate() {
            if let Some(state) = self.per_part_states.get_mut(part_idx_tie) {
                for chain in &state.pending_chains {
                    if let Some(last) = chain.last() {
                        let to_col = self.current_col.saturating_sub(1);
                        if last.0 < to_col {
                            self.current_elements.push(GridElement {
                                position: GridPosition {
                                    column: last.0,
                                    row: state.chain_row,
                                },
                                horizontal_alignment: HorizontalAlignment::Left,
                                vertical_alignment: VerticalAlignment::Top,
                                content: GridContent::TieOrSlurCurve {
                                    from_column: last.0,
                                    to_column: to_col,
                                },
                            });
                        }
                        state.cross_line_tie = Some(last.1.clone());
                    }
                }
            }
        }
        for state in self.per_part_states.iter_mut() {
            state.pending_chains.clear();
        }

        self.push_bottom_system_bar();
        self.commit_row_group();
        self.current_col = self.label_cols;
        self.current_row_offset += self.effective_row_group_height;
        self.is_line_start = true;
        self.line_has_directive_row = false;
        self.effective_row_group_height = self.row_group_height;
        self.maybe_start_new_page();
    }

    fn emit_line_start_elements(&mut self, measure: &MultiPartMeasure) {
        if !self.is_line_start {
            return;
        }

        let part_base = self.part_row_base();

        self.current_elements.push(GridElement {
            position: GridPosition {
                column: self.label_cols,
                row: part_base,
            },
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            content: GridContent::BarLine {
                height_in_rows: self.bar_height,
            },
        });
        if measure.label.is_none() {
            self.current_elements.push(GridElement {
                position: GridPosition {
                    column: self.label_cols,
                    row: self.meta_row(),
                },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Bottom,
                content: GridContent::BarNumber {
                    number: self.bar_number,
                },
            });
        }
        self.current_col = self.label_cols + 1;

        if self.has_named_parts {
            let mut row_cursor = self.part_row_base() - 1;
            for part_row in &measure.parts {
                if let Some(name) = part_row.rendered_slice().and_then(|s| s.name.as_ref()) {
                    self.current_elements.push(GridElement {
                        position: GridPosition {
                            column: 0,
                            row: row_cursor + 1,
                        },
                        horizontal_alignment: HorizontalAlignment::Left,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::PartLabel { text: name.clone() },
                    });
                }
                row_cursor += part_row_height(part_row);
            }
        }
        self.is_line_start = false;
    }

    fn emit_section_label(&mut self, measure: &MultiPartMeasure) {
        if let Some(label_text) = &measure.label {
            self.current_elements.push(GridElement {
                position: GridPosition {
                    column: self.current_col,
                    row: self.meta_row(),
                },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Bottom,
                content: GridContent::SectionLabel {
                    text: label_text.clone(),
                },
            });
        }
    }

    fn emit_measure_directives(&mut self, measure: &MultiPartMeasure) -> u32 {
        let note_col_start = self.current_col;

        if self.line_has_directive_row {
            let directive_row = self.current_row_offset;
            let mut dc = note_col_start;

            if let Some(ts) = &measure.time_signature {
                self.current_elements.push(GridElement {
                    position: GridPosition {
                        column: dc,
                        row: directive_row,
                    },
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                    content: GridContent::TimeSignatureLabel {
                        numerator: ts.numerator,
                        denominator: ts.denominator,
                    },
                });
                dc += 2;
            }

            if let Some(bpm) = measure.bpm {
                self.current_elements.push(GridElement {
                    position: GridPosition {
                        column: dc,
                        row: directive_row,
                    },
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                    content: GridContent::BpmLabel { bpm },
                });
            }
        }

        note_col_start
    }

    fn max_notes_width(measure: &MultiPartMeasure) -> u32 {
        measure
            .parts
            .iter()
            .map(|row| measure_beat_width(row.slice()))
            .max()
            .unwrap_or(0)
    }

    fn emit_measure_content(&mut self, measure: &MultiPartMeasure, note_col_start: u32) {
        let max_notes_width = Self::max_notes_width(measure);
        let mut main_row_cursor = self.part_row_base() - 1;

        for (notes_idx, part_row_enum) in measure.parts.iter().enumerate() {
            let part_row_offset = main_row_cursor;
            let Some(part_slice) = part_row_enum.rendered_slice() else {
                // Ditto rows render nothing and occupy no vertical space.
                continue;
            };
            if let Some(state) = self.per_part_states.get_mut(notes_idx) {
                let mut part_state = PartNoteState {
                    elements: &mut self.current_elements,
                    label_cols: self.label_cols,
                    beam_buf: &mut state.beam_buffer,
                    pending_chains: &mut state.pending_chains,
                    chain_row: &mut state.chain_row,
                    prev_tie: &mut state.prev_tie,
                    prev_slur_key: &mut state.prev_slur_key,
                    cross_line_tie: &mut state.cross_line_tie,
                };
                emit_timed_part(&mut part_state, part_slice, part_row_offset, note_col_start);
            }
            main_row_cursor += part_row_height(part_row_enum);
        }

        let bar_col = note_col_start + max_notes_width;
        self.current_elements.push(GridElement {
            position: GridPosition {
                column: bar_col,
                row: self.part_row_base(),
            },
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            content: GridContent::BarLine {
                height_in_rows: self.bar_height,
            },
        });
        self.current_col = bar_col + 1;
        self.bar_number += 1;
    }

    fn finalize_pages(mut self) -> Vec<Page> {
        if !self.current_elements.is_empty() {
            self.push_bottom_system_bar();
        }
        self.commit_row_group();
        if !self.current_page_row_groups.is_empty() {
            self.pages.push(Page {
                header: self.make_header(),
                footer: Footer {
                    page: self.pages.len() as u32 + 1,
                    total: 0,
                },
                row_groups: std::mem::take(&mut self.current_page_row_groups),
                page_width_pt: self.page_width_pt,
            });
        }

        if self.pages.is_empty() {
            self.pages.push(Page {
                header: self.make_header(),
                footer: Footer { page: 1, total: 1 },
                row_groups: Vec::new(),
                page_width_pt: self.page_width_pt,
            });
        }

        let total = self.pages.len() as u32;
        for page in &mut self.pages {
            page.footer.total = total;
        }

        self.pages
    }
}
