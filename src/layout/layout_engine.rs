use crate::ast::grouped::{
    ChordSlice, GroupedChordEvent, GroupedNote, GroupedRest, MultiPartMeasure, NoteEvent, PartRow,
    PartSlice, Score,
};
use crate::ast::parsed::{JianPuPitch, Syllable};
use crate::layout::types::{
    Footer, GridContent, GridElement, GridPosition, Header, HorizontalAlignment, Page, RowGroup,
    VerticalAlignment,
};
use crate::utils::is_cjk_char;

use super::{
    compute_prefix_width, flush_beam_buffer, flush_chain, format_chord_symbol,
    measure_column_width, part_row_height, BeamBufferEntry, PAGE_MARGIN,
};

pub(crate) struct LayoutEngine<'a> {
    score: &'a Score,
    page_width_pt: f32,
    columns_per_row: u32,
    row_group_height: u32,
    bar_height: u32,
    has_named_parts: bool,
    label_cols: u32,
    header_rows: u32,
    row_groups_per_page: u32,
    pages: Vec<Page>,
    current_page_row_groups: Vec<RowGroup>,
    current_elements: Vec<GridElement>,
    current_col: u32,
    current_row_offset: u32,
    is_line_start: bool,
    bar_number: u32,
    per_part_states: Vec<PerPartLayoutState>,
}

struct PerPartLayoutState {
    prev_tie: bool,
    prev_pitch: Option<JianPuPitch>,
    beam_buffer: Vec<BeamBufferEntry>,
    pending_chain: Vec<(u32, JianPuPitch)>,
    chain_row: u32,
    cross_line_tie: Option<JianPuPitch>,
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
            .map(|m| {
                m.parts
                    .iter()
                    .filter(|p| matches!(p, PartRow::Notes(_)))
                    .count()
            })
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
        let row_groups_per_page =
            ((usable_height / row_height) as u32 - reserved_rows) / row_group_height;

        let num_notes_parts_usize = num_notes_parts as usize;
        Self {
            score,
            page_width_pt,
            columns_per_row,
            row_group_height,
            bar_height,
            has_named_parts,
            label_cols,
            header_rows,
            row_groups_per_page,
            pages: Vec::new(),
            current_page_row_groups: Vec::new(),
            current_elements: Vec::new(),
            current_col: label_cols,
            current_row_offset: header_rows,
            is_line_start: true,
            bar_number: 1,
            per_part_states: (0..num_notes_parts_usize)
                .map(|_| PerPartLayoutState {
                    prev_tie: false,
                    prev_pitch: None,
                    beam_buffer: Vec::new(),
                    pending_chain: Vec::new(),
                    chain_row: 0,
                    cross_line_tie: None,
                })
                .collect(),
        }
    }

    pub(crate) fn layout(mut self) -> Vec<Page> {
        for measure in &self.score.measures {
            self.wrap_line_if_needed(measure);
            self.emit_line_start_elements(measure);
            self.emit_section_label(measure);
            let note_col_start = self.emit_measure_directives(measure);
            self.emit_measure_content(measure, note_col_start);
        }
        self.finalize_pages()
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
                row: self.current_row_offset + self.row_group_height,
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
                height_in_rows: self.row_group_height,
                width_in_columns: self.current_col,
            });
        }
    }

    fn maybe_start_new_page(&mut self) {
        if self.current_page_row_groups.len() >= self.row_groups_per_page as usize {
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
            self.current_row_offset = self.header_rows;
        }
    }

    fn wrap_line_if_needed(&mut self, measure: &MultiPartMeasure) {
        let prefix_width = compute_prefix_width(measure);
        let measure_width = measure_column_width(measure);

        if self.current_col + prefix_width + measure_width <= self.columns_per_row {
            return;
        }

        let mut notes_idx_flush = 0usize;
        let mut flush_row_cursor = self.current_row_offset;
        for part_row in measure.parts.iter() {
            if let PartRow::Notes(_) = part_row {
                if let Some(state) = self.per_part_states.get_mut(notes_idx_flush) {
                    flush_beam_buffer(
                        &mut state.beam_buffer,
                        flush_row_cursor,
                        &mut self.current_elements,
                    );
                }
                notes_idx_flush += 1;
            }
            flush_row_cursor += part_row_height(part_row);
        }

        let mut notes_idx_tie = 0usize;
        for part_row in measure.parts.iter() {
            if let PartRow::Notes(_) = part_row {
                if let Some(state) = self.per_part_states.get_mut(notes_idx_tie) {
                    if let Some(last) = state.pending_chain.last() {
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
                notes_idx_tie += 1;
            }
        }
        for state in self.per_part_states.iter_mut() {
            state.pending_chain.clear();
        }

        self.push_bottom_system_bar();
        self.commit_row_group();
        self.current_col = self.label_cols;
        self.current_row_offset += self.row_group_height;
        self.is_line_start = true;
        self.maybe_start_new_page();
    }

    fn emit_line_start_elements(&mut self, measure: &MultiPartMeasure) {
        if !self.is_line_start {
            return;
        }

        self.current_elements.push(GridElement {
            position: GridPosition {
                column: self.label_cols,
                row: self.current_row_offset + 1,
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
                    row: self.current_row_offset,
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
            let mut row_cursor = self.current_row_offset;
            for part_row in &measure.parts {
                if let Some(name) = part_row.name() {
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
                    row: self.current_row_offset,
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
        let directive_col_start = self.current_col;
        let mut directive_advance = 0u32;
        let mut directive_row_cursor = self.current_row_offset;
        let mut is_first_directive_part = true;

        for part_row_enum in measure.parts.iter() {
            if let PartRow::Notes(_) = part_row_enum {
                let mut dc = directive_col_start;

                if let Some(ts) = &measure.time_signature {
                    self.current_elements.push(GridElement {
                        position: GridPosition {
                            column: dc,
                            row: directive_row_cursor + 1,
                        },
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::TimeSignatureLabel {
                            numerator: ts.numerator,
                            denominator: ts.denominator,
                        },
                    });
                    dc += 2;
                    if is_first_directive_part {
                        directive_advance += 2;
                    }
                }

                if let Some(bpm) = measure.bpm {
                    self.current_elements.push(GridElement {
                        position: GridPosition {
                            column: dc,
                            row: directive_row_cursor + 1,
                        },
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::BpmLabel { bpm },
                    });
                    if is_first_directive_part {
                        directive_advance += 2;
                    }
                }
                is_first_directive_part = false;
            }
            directive_row_cursor += part_row_height(part_row_enum);
        }

        self.current_col = directive_col_start + directive_advance;
        self.current_col
    }

    fn max_notes_width(measure: &MultiPartMeasure) -> u32 {
        measure
            .parts
            .iter()
            .filter_map(|row| {
                if let PartRow::Notes(p) = row {
                    Some(p)
                } else {
                    None
                }
            })
            .map(|part| {
                part.notes
                    .events
                    .iter()
                    .map(|n| match n {
                        NoteEvent::Note(note) => note.duration,
                        NoteEvent::Rest(rest) => rest.duration,
                    })
                    .sum::<u32>()
            })
            .max()
            .unwrap_or(0)
    }

    fn emit_measure_content(&mut self, measure: &MultiPartMeasure, note_col_start: u32) {
        let max_notes_width = Self::max_notes_width(measure);
        let mut notes_idx = 0usize;
        let mut main_row_cursor = self.current_row_offset;

        for part_row_enum in measure.parts.iter() {
            let part_row_offset = main_row_cursor;
            match part_row_enum {
                PartRow::Notes(part_slice) => {
                    if let Some(state) = self.per_part_states.get_mut(notes_idx) {
                        let mut part_state = PartNoteState {
                            elements: &mut self.current_elements,
                            label_cols: self.label_cols,
                            beam_buf: &mut state.beam_buffer,
                            pending_chain: &mut state.pending_chain,
                            chain_row: &mut state.chain_row,
                            prev_tie: &mut state.prev_tie,
                            prev_pitch: &mut state.prev_pitch,
                            cross_line_tie: &mut state.cross_line_tie,
                        };
                        emit_notes_part(
                            &mut part_state,
                            part_slice,
                            part_row_offset,
                            note_col_start,
                        );
                    }
                    notes_idx += 1;
                    main_row_cursor += part_row_height(part_row_enum);
                }
                PartRow::Chord(chord_slice) => {
                    emit_chord_part(
                        &mut self.current_elements,
                        chord_slice,
                        main_row_cursor,
                        note_col_start,
                    );
                    main_row_cursor += 2;
                }
            }
        }

        let bar_col = note_col_start + max_notes_width;
        self.current_elements.push(GridElement {
            position: GridPosition {
                column: bar_col,
                row: self.current_row_offset + 1,
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

fn emit_chord_part(
    elements: &mut Vec<GridElement>,
    chord_slice: &ChordSlice,
    main_row_cursor: u32,
    note_col_start: u32,
) {
    let mut col = note_col_start;
    for event in &chord_slice.events {
        match event {
            GroupedChordEvent::Chord(chord) => {
                let text = format_chord_symbol(chord);
                elements.push(GridElement {
                    position: GridPosition {
                        column: col,
                        row: main_row_cursor + 1,
                    },
                    horizontal_alignment: HorizontalAlignment::Left,
                    vertical_alignment: VerticalAlignment::Center,
                    content: GridContent::ChordSymbol { text },
                });
                col += chord.duration;
            }
            GroupedChordEvent::Rest(dur) => {
                col += dur;
            }
        }
    }
}

struct PartNoteState<'a> {
    elements: &'a mut Vec<GridElement>,
    label_cols: u32,
    beam_buf: &'a mut Vec<BeamBufferEntry>,
    pending_chain: &'a mut Vec<(u32, JianPuPitch)>,
    chain_row: &'a mut u32,
    prev_tie: &'a mut bool,
    prev_pitch: &'a mut Option<JianPuPitch>,
    cross_line_tie: &'a mut Option<JianPuPitch>,
}

fn emit_notes_part(
    state: &mut PartNoteState<'_>,
    part_slice: &PartSlice,
    part_row_offset: u32,
    note_col_start: u32,
) {
    let mut col = note_col_start;
    let measure_col_start_for_part = note_col_start;

    if state.pending_chain.is_empty() {
        *state.chain_row = part_row_offset + 1;
    }

    let mut lyrics_iter = part_slice.lyrics.as_ref().map(|l| l.syllables.iter());

    for note_event in &part_slice.notes.events {
        match note_event {
            NoteEvent::Note(note) => {
                emit_grouped_note(
                    state,
                    note,
                    &mut col,
                    part_row_offset,
                    measure_col_start_for_part,
                    &mut lyrics_iter,
                );
            }
            NoteEvent::Rest(rest) => {
                emit_grouped_rest(
                    state,
                    rest,
                    &mut col,
                    part_row_offset,
                    measure_col_start_for_part,
                );
            }
        }
    }

    flush_beam_buffer(state.beam_buf, part_row_offset, state.elements);
}

fn push_note_head_elements(
    elements: &mut Vec<GridElement>,
    note: &GroupedNote,
    col: u32,
    part_row_offset: u32,
) {
    elements.push(GridElement {
        position: GridPosition {
            column: col,
            row: part_row_offset + 1,
        },
        horizontal_alignment: HorizontalAlignment::Center,
        vertical_alignment: VerticalAlignment::Center,
        content: GridContent::NoteHead {
            pitch: note.pitch.clone(),
            octave: note.octave,
            dotted: note.dotted,
        },
    });

    if note.octave < 0 {
        let dot_underline_count = match note.duration {
            1 => 2u8,
            2 | 3 => 1u8,
            _ => 0u8,
        };
        elements.push(GridElement {
            position: GridPosition {
                column: col,
                row: part_row_offset + 2,
            },
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Top,
            content: GridContent::LowerOctaveDots {
                count: (-note.octave) as u32,
                underline_count: dot_underline_count,
            },
        });
    }

    if note.duration > 4 {
        let extra_beats = (note.duration - 4) / 4;
        for i in 0..extra_beats {
            elements.push(GridElement {
                position: GridPosition {
                    column: col + 4 + i * 4,
                    row: part_row_offset + 1,
                },
                horizontal_alignment: HorizontalAlignment::Center,
                vertical_alignment: VerticalAlignment::Center,
                content: GridContent::Extension,
            });
        }
    }
}

fn emit_grouped_note(
    state: &mut PartNoteState<'_>,
    note: &GroupedNote,
    col: &mut u32,
    part_row_offset: u32,
    measure_col_start_for_part: u32,
    lyrics_iter: &mut Option<std::slice::Iter<'_, Syllable>>,
) {
    push_note_head_elements(state.elements, note, *col, part_row_offset);

    let underline_count = match note.duration {
        1 => 2,
        2 | 3 => 1,
        _ => 0,
    };

    if underline_count == 0 {
        flush_beam_buffer(state.beam_buf, part_row_offset, state.elements);
    }

    state.pending_chain.push((*col, note.pitch.clone()));

    let is_tie_continuation = *state.prev_tie && state.prev_pitch.as_ref() == Some(&note.pitch);

    if state.cross_line_tie.is_some() {
        if is_tie_continuation && *col > state.label_cols {
            state.elements.push(GridElement {
                position: GridPosition {
                    column: state.label_cols,
                    row: *state.chain_row,
                },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Top,
                content: GridContent::TieOrSlurCurve {
                    from_column: state.label_cols,
                    to_column: *col,
                },
            });
        }
        *state.cross_line_tie = None;
    }

    if !is_tie_continuation {
        if let Some(ref mut iter) = lyrics_iter {
            if let Some(syllable) = iter.next() {
                let is_cjk = syllable
                    .text
                    .chars()
                    .next()
                    .map(is_cjk_char)
                    .unwrap_or(false);
                state.elements.push(GridElement {
                    position: GridPosition {
                        column: *col,
                        row: part_row_offset + 3,
                    },
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Top,
                    content: GridContent::Lyric {
                        text: syllable.text.clone(),
                        is_cjk,
                    },
                });
            }
        }
    }
    *state.prev_tie = note.tie;
    *state.prev_pitch = Some(note.pitch.clone());

    if underline_count > 0 {
        state.beam_buf.push(BeamBufferEntry {
            column: *col,
            underline_count,
            duration: note.duration,
        });
    }

    *col += note.duration;

    let beat_position = *col - measure_col_start_for_part;
    if underline_count > 0 && beat_position % 4 == 0 {
        flush_beam_buffer(state.beam_buf, part_row_offset, state.elements);
    }

    if !note.tie {
        flush_chain(state.pending_chain, *state.chain_row, state.elements);
        state.pending_chain.clear();
    }
}

fn emit_grouped_rest(
    state: &mut PartNoteState<'_>,
    rest: &GroupedRest,
    col: &mut u32,
    part_row_offset: u32,
    measure_col_start_for_part: u32,
) {
    let rest_underline_count = match rest.duration {
        1 => 2,
        2 => 1,
        _ => 0,
    };
    if rest_underline_count == 0 {
        flush_beam_buffer(state.beam_buf, part_row_offset, state.elements);
    }
    state.elements.push(GridElement {
        position: GridPosition {
            column: *col,
            row: part_row_offset + 1,
        },
        horizontal_alignment: HorizontalAlignment::Center,
        vertical_alignment: VerticalAlignment::Center,
        content: GridContent::Rest,
    });
    if rest_underline_count > 0 {
        state.beam_buf.push(BeamBufferEntry {
            column: *col,
            underline_count: rest_underline_count,
            duration: rest.duration,
        });
    }
    *col += rest.duration;
    *state.prev_tie = false;
    *state.cross_line_tie = None;
    let beat_position = *col - measure_col_start_for_part;
    if rest_underline_count > 0 && beat_position % 4 == 0 {
        flush_beam_buffer(state.beam_buf, part_row_offset, state.elements);
    }
}
