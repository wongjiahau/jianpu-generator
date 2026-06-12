pub mod types;
pub use types::*;

use crate::ast::grouped::{
    GroupedChordNote, GroupedNote, GroupedRest, MultiPartMeasure, NoteEvent, PartSlice, Score,
};
use crate::ast::parsed::{Extension, JianPuPitch, PartKind, TriadQuality};

/// Per-part state carried across measure boundaries.
struct PartCrossState {
    prev_tie: bool,
    prev_slur_key: Option<SlurKey>,
}

pub fn compile(score: &Score) -> Vec<MeasureBlock> {
    let max_parts = score
        .measures
        .iter()
        .map(|m| m.parts.len())
        .max()
        .unwrap_or(0);
    let mut cross_states: Vec<PartCrossState> = (0..max_parts)
        .map(|_| PartCrossState {
            prev_tie: false,
            prev_slur_key: None,
        })
        .collect();

    score
        .measures
        .iter()
        .enumerate()
        .map(|(idx, measure)| compile_measure(measure, idx + 1, &mut cross_states))
        .collect()
}

fn compile_measure(
    measure: &MultiPartMeasure,
    bar_number: usize,
    cross_states: &mut Vec<PartCrossState>,
) -> MeasureBlock {
    while cross_states.len() < measure.parts.len() {
        cross_states.push(PartCrossState {
            prev_tie: false,
            prev_slur_key: None,
        });
    }

    let decorations = collect_decorations(measure, bar_number);
    let mut rows: Vec<MeasureRow> = Vec::new();
    for (part_idx, part_row) in measure.parts.iter().enumerate() {
        let (init_tie, init_key) = cross_states
            .get(part_idx)
            .map(|cs| (cs.prev_tie, cs.prev_slur_key.clone()))
            .unwrap_or((false, None));
        let (elements, final_tie, final_key) =
            compile_part_slice(part_row.slice(), init_tie, init_key);
        if let Some(cs) = cross_states.get_mut(part_idx) {
            cs.prev_tie = final_tie;
            cs.prev_slur_key = final_key;
        }

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
                    elements,
                });
            }
            None => {
                // Ditto row: append its label to the source row's label.
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

    // Level 0: spans all notes in the beam group
    result.push(ColumnElement {
        column: first.column,
        content: ElementContent::Underline {
            from_column: first.column,
            to_column: last.column + last.duration,
            last_head_column: last.column,
            level: 0,
        },
    });

    // Level 1: one span per maximal contiguous sub-run with underline_count >= 2
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

// ── Slur / tie chain state ────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum SlurKey {
    Pitch(JianPuPitch),
    Chord {
        degree: JianPuPitch,
        triad: TriadQuality,
        extension: Option<Extension>,
        bass_degree: Option<JianPuPitch>,
    },
    Rest,
}

impl SlurKey {
    fn from_chord(chord: &GroupedChordNote) -> Self {
        SlurKey::Chord {
            degree: chord.degree.clone(),
            triad: chord.triad.clone(),
            extension: chord.extension.clone(),
            bass_degree: chord.bass.as_ref().map(|b| b.degree.clone()),
        }
    }
}

fn extend_note_chains(
    chains: &mut Vec<Vec<(u32, SlurKey)>>,
    membership: u8,
    continuation: u8,
    col: u32,
    key: &SlurKey,
    elements: &mut Vec<ColumnElement>,
) {
    while chains.len() < membership as usize {
        chains.push(Vec::new());
    }
    for chain in chains.iter_mut().take(membership as usize) {
        chain.push((col, key.clone()));
    }
    for depth in (continuation as usize)..(membership as usize) {
        if let Some(chain) = chains.get(depth) {
            if chain.len() > 1 {
                flush_chain(chain, elements);
            }
        }
        if let Some(chain) = chains.get_mut(depth) {
            chain.clear();
        }
    }
}

fn flush_chain(chain: &[(u32, SlurKey)], elements: &mut Vec<ColumnElement>) {
    if chain.len() <= 1 {
        return;
    }

    let has_key_change = chain
        .windows(2)
        .any(|w| matches!((w.first(), w.get(1)), (Some(a), Some(b)) if a.1 != b.1));

    if has_key_change {
        if let (Some(first), Some(last)) = (chain.first(), chain.last()) {
            elements.push(ColumnElement {
                column: first.0,
                content: ElementContent::TieOrSlur {
                    from_column: first.0,
                    to_column: last.0,
                },
            });
        }
    }

    // Tie arc for each consecutive same-pitch pair
    for w in chain.windows(2) {
        if let (Some(prev), Some(next)) = (w.first(), w.get(1)) {
            if prev.1 == next.1 {
                elements.push(ColumnElement {
                    column: prev.0,
                    content: ElementContent::TieOrSlur {
                        from_column: prev.0,
                        to_column: next.0,
                    },
                });
            }
        }
    }
}

// ── Part slice compiler ───────────────────────────────────────────────────────

/// Mutable state carried through one measure's worth of note events for a single part.
struct PartState<'a> {
    elements: &'a mut Vec<ColumnElement>,
    beam_buf: &'a mut Vec<BeamEntry>,
    pending_chains: &'a mut Vec<Vec<(u32, SlurKey)>>,
    prev_tie: &'a mut bool,
    prev_slur_key: &'a mut Option<SlurKey>,
    col: &'a mut u32,
    /// True when this measure started with an inherited open tie from the previous measure.
    /// Consumed (set false) after the first close-arc is emitted.
    cross_measure_open: &'a mut bool,
}

fn compile_part_slice(
    slice: &PartSlice,
    initial_prev_tie: bool,
    initial_prev_slur_key: Option<SlurKey>,
) -> (Vec<ColumnElement>, bool, Option<SlurKey>) {
    let mut elements: Vec<ColumnElement> = Vec::new();
    let mut beam_buf: Vec<BeamEntry> = Vec::new();
    let mut pending_chains: Vec<Vec<(u32, SlurKey)>> = Vec::new();
    let mut prev_tie = initial_prev_tie;
    let mut prev_slur_key: Option<SlurKey> = initial_prev_slur_key;
    let mut col: u32 = 0;
    let measure_col_start: u32 = 0;
    let mut cross_measure_open = initial_prev_tie;

    let mut lyrics_iter = slice.lyrics.as_ref().map(|l| l.syllables.iter());

    let mut state = PartState {
        elements: &mut elements,
        beam_buf: &mut beam_buf,
        pending_chains: &mut pending_chains,
        prev_tie: &mut prev_tie,
        prev_slur_key: &mut prev_slur_key,
        col: &mut col,
        cross_measure_open: &mut cross_measure_open,
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

    // Flush any remaining beam
    flush_beam_buffer(state.beam_buf, state.elements);

    // Flush any remaining chains (end of measure).
    // Open chains (len == 1) are cross-measure: emit an arc from the tied note to the barline.
    let barline_col = *state.col;
    for chain in state.pending_chains.iter() {
        if chain.len() > 1 {
            flush_chain(chain, state.elements);
        } else if let Some((chain_col, _)) = chain.first() {
            state.elements.push(ColumnElement {
                column: *chain_col,
                content: ElementContent::TieOrSlur {
                    from_column: *chain_col,
                    to_column: barline_col,
                },
            });
        }
    }

    let final_tie = *state.prev_tie;
    let final_key = state.prev_slur_key.clone();

    // Bar line at end
    elements.push(ColumnElement {
        column: col,
        content: ElementContent::BarLine,
    });

    (elements, final_tie, final_key)
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
        note.group_membership,
        note.group_continuation,
        *state.col,
        &slur_key,
        state.elements,
    );
    // When the slur group closes on an extension within this note, close the chain at the
    // extension-dash column rather than letting it become a cross-measure open chain.
    if let Some(close_offset) = note.slur_group_close_at_duration {
        if note.group_membership > 0 {
            extend_note_chains(
                state.pending_chains,
                note.group_membership,
                0,
                *state.col + close_offset,
                &SlurKey::Rest,
                state.elements,
            );
        }
    }

    let is_tie_continuation = *state.prev_tie && state.prev_slur_key.as_ref() == Some(&slur_key);

    // Emit close arc for cross-measure tie continuation (first continuation note only).
    if *state.cross_measure_open && is_tie_continuation {
        state.elements.push(ColumnElement {
            column: *state.col,
            content: ElementContent::TieOrSlurClose {
                to_column: *state.col,
            },
        });
        *state.cross_measure_open = false;
    }

    // Emit lyric for NotesWithLyrics parts (skip tie-continuation notes)
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

    *state.prev_tie = note.tie;
    *state.prev_slur_key = Some(slur_key);

    // Emit a visual dash at each beat column within the note's span beyond the first.
    // Dotted notes carry duration visually via the dot; only non-dotted notes get dashes.
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
            rest.group_membership,
            rest.group_continuation,
            *state.col,
            &SlurKey::Rest,
            state.elements,
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
        chord.group_membership,
        chord.group_continuation,
        *state.col,
        &slur_key,
        state.elements,
    );

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
