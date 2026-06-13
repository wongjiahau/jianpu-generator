pub mod types;
pub use types::*;

mod slur_chains;
use slur_chains::{extend_note_chains, PartCrossState, PendingSlurOpen, SlurKey};

use crate::ast::grouped::{
    GroupedChordNote, GroupedNote, GroupedRest, MultiPartMeasure, NoteEvent, PartSlice, Score,
};
use crate::ast::parsed::PartKind;

struct PartSliceResult {
    elements: Vec<ColumnElement>,
    final_tie: bool,
    final_tie_column: Option<u32>,
    final_tie_measure: Option<usize>,
    final_slur_key: Option<SlurKey>,
    final_pending_opens: Vec<Option<PendingSlurOpen>>,
}

pub fn compile(score: &Score) -> CompileResult {
    let max_parts = score
        .measures
        .iter()
        .map(|m| m.parts.len())
        .max()
        .unwrap_or(0);
    let mut cross_states: Vec<PartCrossState> =
        (0..max_parts).map(|_| PartCrossState::new()).collect();

    let mut slur_spans: Vec<SlurSpan> = Vec::new();
    let blocks = score
        .measures
        .iter()
        .enumerate()
        .map(|(measure_index, measure)| {
            compile_measure(
                measure,
                measure_index + 1,
                measure_index,
                &mut cross_states,
                &mut slur_spans,
            )
        })
        .collect();

    CompileResult { blocks, slur_spans }
}

fn compile_measure(
    measure: &MultiPartMeasure,
    bar_number: usize,
    measure_index: usize,
    cross_states: &mut Vec<PartCrossState>,
    slur_spans: &mut Vec<SlurSpan>,
) -> MeasureBlock {
    while cross_states.len() < measure.parts.len() {
        cross_states.push(PartCrossState::new());
    }

    let decorations = collect_decorations(measure, bar_number);
    let mut rows: Vec<MeasureRow> = Vec::new();
    for (part_idx, part_row) in measure.parts.iter().enumerate() {
        let Some(cs) = cross_states.get(part_idx) else {
            continue;
        };
        let init_tie = cs.prev_tie;
        let init_tie_column = cs.prev_tie_column;
        let init_tie_measure = cs.prev_tie_measure;
        let init_key = cs.prev_slur_key.clone();
        let init_pending_opens = cs.clone_pending_opens();

        let slice_result = compile_part_slice(
            part_row.slice(),
            init_tie,
            init_tie_column,
            init_tie_measure,
            init_key,
            init_pending_opens,
            measure_index,
            part_idx,
            slur_spans,
        );

        let Some(cs) = cross_states.get_mut(part_idx) else {
            continue;
        };
        cs.prev_tie = slice_result.final_tie;
        cs.prev_tie_column = slice_result.final_tie_column;
        cs.prev_tie_measure = slice_result.final_tie_measure;
        cs.prev_slur_key = slice_result.final_slur_key;
        cs.pending_slur_opens = slice_result.final_pending_opens;

        match part_row.rendered_slice() {
            Some(_) => {
                let label = part_row.name().cloned().unwrap_or_default();
                let id = RowId(
                    part_row
                        .name()
                        .cloned()
                        .unwrap_or_else(|| format!("__anon_{part_idx}")),
                );
                rows.push(MeasureRow {
                    id,
                    label,
                    elements: slice_result.elements,
                });
            }
            None => {
                if let Some(last) = rows.last_mut() {
                    let ditto_label = part_row.name().map(String::as_str).unwrap_or("");
                    if !ditto_label.is_empty() {
                        last.label.push_str(", ");
                        last.label.push_str(ditto_label);
                    }
                }
            }
        }
    }
    if rows.len() == 1 && measure.parts.len() > 1 {
        if let Some(row) = rows.get_mut(0) {
            row.label = "[ALL]".to_string();
        }
    }
    MeasureBlock { rows, decorations }
}

fn collect_decorations(measure: &MultiPartMeasure, bar_number: usize) -> Vec<Decoration> {
    let mut decorations = Vec::new();
    if let Some(bpm) = measure.bpm {
        decorations.push(Decoration::Bpm(bpm));
    }
    if let Some(ts) = &measure.time_signature {
        decorations.push(Decoration::TimeSignature {
            numerator: ts.numerator as u32,
            denominator: ts.denominator as u32,
        });
    }
    if let Some(label) = &measure.label {
        decorations.push(Decoration::SectionLabel(label.clone()));
    }
    if measure.label.is_none() {
        decorations.push(Decoration::BarNumber(bar_number as u32));
    }
    decorations
}

// ── Per-part beam state ───────────────────────────────────────────────────────

struct BeamEntry {
    column: u32,
    underline_count: u32,
    duration: u32,
}

fn flush_beam_buffer(buffer: &mut Vec<BeamEntry>, elements: &mut Vec<ColumnElement>) {
    if buffer.is_empty() {
        return;
    }
    let underlines = compute_underline_levels(buffer);
    elements.extend(underlines);
    buffer.clear();
}

fn compute_underline_levels(buffer: &[BeamEntry]) -> Vec<ColumnElement> {
    let (Some(first), Some(last)) = (buffer.first(), buffer.last()) else {
        return Vec::new();
    };
    let mut result = Vec::new();

    result.push(ColumnElement {
        column: first.column,
        content: ElementContent::Underline {
            from_column: first.column,
            to_column: last.column + last.duration,
            last_head_column: last.column,
            level: 0,
        },
    });

    let mut run_start: Option<u32> = None;
    let mut run_end: u32 = 0;
    let mut run_last_head: u32 = 0;
    for entry in buffer {
        if entry.underline_count >= 2 {
            if run_start.is_none() {
                run_start = Some(entry.column);
            }
            run_end = entry.column + entry.duration;
            run_last_head = entry.column;
        } else if let Some(start) = run_start.take() {
            result.push(ColumnElement {
                column: start,
                content: ElementContent::Underline {
                    from_column: start,
                    to_column: run_end,
                    last_head_column: run_last_head,
                    level: 1,
                },
            });
        }
    }
    if let Some(start) = run_start {
        result.push(ColumnElement {
            column: start,
            content: ElementContent::Underline {
                from_column: start,
                to_column: run_end,
                last_head_column: run_last_head,
                level: 1,
            },
        });
    }

    result
}

// ── Part slice compiler ───────────────────────────────────────────────────────

struct PartState<'a> {
    elements: &'a mut Vec<ColumnElement>,
    beam_buf: &'a mut Vec<BeamEntry>,
    pending_chains: &'a mut Vec<Vec<(u32, SlurKey)>>,
    pending_slur_opens: &'a mut Vec<Option<PendingSlurOpen>>,
    slur_spans: &'a mut Vec<SlurSpan>,
    prev_tie: &'a mut bool,
    prev_tie_column: &'a mut Option<u32>,
    prev_tie_measure: &'a mut Option<usize>,
    prev_slur_key: &'a mut Option<SlurKey>,
    col: &'a mut u32,
    cross_measure_open: &'a mut bool,
    measure_index: usize,
    part_index: usize,
}

#[allow(clippy::too_many_arguments)]
fn compile_part_slice(
    slice: &PartSlice,
    initial_prev_tie: bool,
    initial_prev_tie_column: Option<u32>,
    initial_prev_tie_measure: Option<usize>,
    initial_prev_slur_key: Option<SlurKey>,
    initial_pending_opens: Vec<Option<PendingSlurOpen>>,
    measure_index: usize,
    part_index: usize,
    slur_spans: &mut Vec<SlurSpan>,
) -> PartSliceResult {
    let mut elements: Vec<ColumnElement> = Vec::new();
    let mut beam_buf: Vec<BeamEntry> = Vec::new();
    let mut pending_chains: Vec<Vec<(u32, SlurKey)>> = Vec::new();
    let mut pending_slur_opens: Vec<Option<PendingSlurOpen>> = initial_pending_opens;
    let mut prev_tie = initial_prev_tie;
    let mut prev_tie_column: Option<u32> = initial_prev_tie_column;
    let mut prev_tie_measure: Option<usize> = initial_prev_tie_measure;
    let mut prev_slur_key: Option<SlurKey> = initial_prev_slur_key;
    let mut col: u32 = 0;
    let measure_col_start: u32 = 0;
    let mut cross_measure_open = initial_prev_tie;

    let mut lyrics_iter = slice.lyrics.as_ref().map(|l| l.syllables.iter());

    let mut state = PartState {
        elements: &mut elements,
        beam_buf: &mut beam_buf,
        pending_chains: &mut pending_chains,
        pending_slur_opens: &mut pending_slur_opens,
        slur_spans,
        prev_tie: &mut prev_tie,
        prev_tie_column: &mut prev_tie_column,
        prev_tie_measure: &mut prev_tie_measure,
        prev_slur_key: &mut prev_slur_key,
        col: &mut col,
        cross_measure_open: &mut cross_measure_open,
        measure_index,
        part_index,
    };

    for event in &slice.notes.events {
        match event {
            NoteEvent::Note(note) => {
                compile_note(
                    &mut state,
                    note,
                    measure_col_start,
                    &mut lyrics_iter,
                    slice.kind,
                );
            }
            NoteEvent::Rest(rest) => {
                compile_rest(&mut state, rest, measure_col_start);
            }
            NoteEvent::Chord(chord) => {
                compile_chord(&mut state, chord, measure_col_start);
            }
        }
    }

    flush_beam_buffer(state.beam_buf, state.elements);

    // Extract values from state before the end-of-measure chain flush (avoids borrow conflicts).
    let final_tie = *state.prev_tie;
    let final_tie_column = *state.prev_tie_column;
    let final_tie_measure = *state.prev_tie_measure;
    let final_key = state.prev_slur_key.clone();
    // End state borrow so we can access `pending_slur_opens`, `pending_chains`, etc. directly.
    let _ = state;

    // Flush remaining chains at end of measure.
    // Multi-note chains (len > 1): slur is still open across the measure boundary.
    //   Preserve the original origin (don't emit an arc yet).
    // Single-note chains (len == 1): cross-measure open, save as PendingSlurOpen
    //   only if no existing origin is already present.
    for (depth, chain) in pending_chains.iter().enumerate() {
        if chain.len() > 1 {
            // Slur is still open across the measure boundary.
            // Preserve the original origin (or use first chain note if none exists).
            // Do NOT emit an arc — flush_chain would produce a premature span.
            let origin = pending_slur_opens
                .get(depth)
                .and_then(|o| o.as_ref())
                .map(|o| (o.measure_index, o.from_column))
                .or_else(|| chain.first().map(|(c, _)| (measure_index, *c)));
            while pending_slur_opens.len() <= depth {
                pending_slur_opens.push(None);
            }
            if let (Some(origin), Some(slot)) = (origin, pending_slur_opens.get_mut(depth)) {
                *slot = Some(PendingSlurOpen {
                    measure_index: origin.0,
                    from_column: origin.1,
                });
            }
        } else if let Some((chain_col, _)) = chain.first() {
            // Only save if no cross-measure origin is already preserved.
            while pending_slur_opens.len() <= depth {
                pending_slur_opens.push(None);
            }
            if pending_slur_opens
                .get(depth)
                .and_then(|o| o.as_ref())
                .is_none()
            {
                if let Some(slot) = pending_slur_opens.get_mut(depth) {
                    *slot = Some(PendingSlurOpen {
                        measure_index,
                        from_column: *chain_col,
                    });
                }
            }
        }
    }

    elements.push(ColumnElement {
        column: col,
        content: ElementContent::BarLine,
    });

    PartSliceResult {
        elements,
        final_tie,
        final_tie_column,
        final_tie_measure,
        final_slur_key: final_key,
        final_pending_opens: pending_slur_opens,
    }
}

fn compile_note(
    state: &mut PartState<'_>,
    note: &GroupedNote,
    measure_col_start: u32,
    lyrics_iter: &mut Option<std::slice::Iter<'_, crate::ast::parsed::Syllable>>,
    kind: PartKind,
) {
    state.elements.push(ColumnElement {
        column: *state.col,
        content: ElementContent::NoteHead {
            pitch: note.pitch.clone(),
            octave: note.octave,
            dotted: note.dotted,
        },
    });

    let underline_count = match note.duration {
        1 => 2,
        2 | 3 => 1,
        _ => 0,
    };

    if underline_count == 0 {
        flush_beam_buffer(state.beam_buf, state.elements);
    }

    let slur_key = SlurKey::Pitch(note.pitch.clone());
    extend_note_chains(
        state.pending_chains,
        state.pending_slur_opens,
        note.group_membership,
        note.group_continuation,
        *state.col,
        &slur_key,
        state.slur_spans,
        state.measure_index,
        state.part_index,
    );
    if let Some(close_offset) = note.slur_group_close_at_duration {
        if note.group_membership > 0 {
            extend_note_chains(
                state.pending_chains,
                state.pending_slur_opens,
                note.group_membership,
                0,
                *state.col + close_offset,
                &SlurKey::Rest,
                state.slur_spans,
                state.measure_index,
                state.part_index,
            );
        }
    }

    let is_tie_continuation = *state.prev_tie && state.prev_slur_key.as_ref() == Some(&slur_key);

    // Cross-measure tie close: emit SlurSpan using saved tie origin.
    if *state.cross_measure_open && is_tie_continuation {
        if let (Some(from_col), Some(from_measure)) =
            (*state.prev_tie_column, *state.prev_tie_measure)
        {
            state.slur_spans.push(SlurSpan {
                part_index: state.part_index,
                from_measure,
                from_column: from_col,
                to_measure: state.measure_index,
                to_column: *state.col,
            });
        }
        *state.cross_measure_open = false;
    }

    if kind == PartKind::NotesWithLyrics && !is_tie_continuation {
        if let Some(ref mut iter) = lyrics_iter {
            if let Some(syllable) = iter.next() {
                state.elements.push(ColumnElement {
                    column: *state.col,
                    content: ElementContent::Lyric(syllable.text.clone()),
                });
            }
        }
    }

    // Save tie origin before advancing column.
    if note.tie {
        *state.prev_tie_column = Some(*state.col);
        *state.prev_tie_measure = Some(state.measure_index);
    } else {
        *state.prev_tie_column = None;
        *state.prev_tie_measure = None;
    }
    *state.prev_tie = note.tie;
    *state.prev_slur_key = Some(slur_key);

    if !note.dotted {
        let note_col = *state.col;
        for dash_col in (note_col + 4..note_col + note.duration).step_by(4) {
            state.elements.push(ColumnElement {
                column: dash_col,
                content: ElementContent::NoteDash,
            });
        }
    }

    if underline_count > 0 {
        state.beam_buf.push(BeamEntry {
            column: *state.col,
            underline_count,
            duration: note.duration,
        });
    }

    *state.col += note.duration;

    let beat_position = *state.col - measure_col_start;
    if underline_count > 0 && beat_position % 4 == 0 {
        flush_beam_buffer(state.beam_buf, state.elements);
    }
}

fn compile_rest(state: &mut PartState<'_>, rest: &GroupedRest, measure_col_start: u32) {
    let underline_count = match rest.duration {
        1 => 2,
        2 => 1,
        _ => 0,
    };

    if underline_count == 0 {
        flush_beam_buffer(state.beam_buf, state.elements);
    }

    state.elements.push(ColumnElement {
        column: *state.col,
        content: ElementContent::Rest {
            dotted: rest.dotted,
        },
    });

    if rest.group_membership > 0 {
        extend_note_chains(
            state.pending_chains,
            state.pending_slur_opens,
            rest.group_membership,
            rest.group_continuation,
            *state.col,
            &SlurKey::Rest,
            state.slur_spans,
            state.measure_index,
            state.part_index,
        );
    }

    if underline_count > 0 {
        state.beam_buf.push(BeamEntry {
            column: *state.col,
            underline_count,
            duration: rest.duration,
        });
    }

    *state.col += rest.duration;
    *state.prev_tie = false;
    *state.prev_tie_column = None;
    *state.prev_tie_measure = None;
    *state.prev_slur_key = None;

    let beat_position = *state.col - measure_col_start;
    if underline_count > 0 && beat_position % 4 == 0 {
        flush_beam_buffer(state.beam_buf, state.elements);
    }
}

fn compile_chord(state: &mut PartState<'_>, chord: &GroupedChordNote, measure_col_start: u32) {
    let text = chord.format_symbol();
    state.elements.push(ColumnElement {
        column: *state.col,
        content: ElementContent::ChordSymbol(text),
    });

    let underline_count = match chord.duration {
        1 => 2,
        2 | 3 => 1,
        _ => 0,
    };

    if underline_count == 0 {
        flush_beam_buffer(state.beam_buf, state.elements);
    }

    let slur_key = SlurKey::from_chord(chord);
    extend_note_chains(
        state.pending_chains,
        state.pending_slur_opens,
        chord.group_membership,
        chord.group_continuation,
        *state.col,
        &slur_key,
        state.slur_spans,
        state.measure_index,
        state.part_index,
    );

    if chord.tie {
        *state.prev_tie_column = Some(*state.col);
        *state.prev_tie_measure = Some(state.measure_index);
    } else {
        *state.prev_tie_column = None;
        *state.prev_tie_measure = None;
    }
    *state.prev_tie = chord.tie;
    *state.prev_slur_key = Some(slur_key);

    if underline_count > 0 {
        state.beam_buf.push(BeamEntry {
            column: *state.col,
            underline_count,
            duration: chord.duration,
        });
    }

    *state.col += chord.duration;

    let beat_position = *state.col - measure_col_start;
    if underline_count > 0 && beat_position % 4 == 0 {
        flush_beam_buffer(state.beam_buf, state.elements);
    }
}

#[cfg(test)]
mod tests;
