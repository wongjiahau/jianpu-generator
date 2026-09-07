use crate::ast::grouped::NoteEvent;
use crate::compiler::visible_part_indices;
use crate::error::IrrecoverableError;

/// Source byte range of one sounded event (note/chord/percussion hit) or rest,
/// keyed the same way the compiled SVG's `data-part-index`/`data-note-id`
/// attributes are (see `renderer::new_renderer::render_playback_cursor_target`),
/// so a click hit-test on the SVG can be mapped straight back to source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSourceSpan {
    /// Index into `MultiPartMeasure::parts` for this event's part, matching
    /// the compiled `part_index`/`source_part_index` used throughout the
    /// renderer and MIDI pipeline.
    pub source_part_index: usize,
    /// Same id a tied run of notes shares in `ColumnElement::note_id`: a tie
    /// continuation reuses the id of the note it continues from rather than
    /// allocating a fresh one.
    pub note_id: usize,
    /// Index into `Score.measures`.
    pub measure_index: usize,
    /// Inclusive start byte of this event's token in the original source.
    /// Always `Some` in practice (every `NoteEvent` variant, including
    /// `Rest`, carries an `event_span`); kept `Option` so a future event
    /// kind without a mappable token has somewhere to signal that.
    pub start: Option<usize>,
    /// Exclusive end byte of this event's token in the original source.
    pub end: Option<usize>,
}

/// Result of [`list_note_spans_from_source`].
pub struct NoteSpansResult {
    /// Source byte span of every note/chord/percussion/rest event, in score
    /// order (measure, then part, then event).
    pub spans: Vec<NoteSourceSpan>,
}

/// Per-part running state mirroring the id/tie bookkeeping
/// `compiler::part_slice::compile_timed_unit` performs during compilation,
/// so the note ids produced here line up 1-to-1 with `ColumnElement::note_id`.
#[derive(Default, Clone, Copy)]
struct PartCounterState {
    next_note_id: usize,
    prev_tie: bool,
    prev_tie_note_id: Option<usize>,
}

/// Return the source byte span of every note/chord/percussion/rest event in
/// the compiled score, one entry per event, with note ids matching the
/// compiled `ColumnElement::note_id` values (including tie-continuation reuse).
///
/// `enabled_tracks` must mirror whatever the caller passed to the render
/// pipeline (see `render_svgs_with_parts`/`apply_track_filter`): hiding a
/// part `Vec::retain`s it out of every measure's `parts` before compiling,
/// which shifts every later part's index down by one. Every `source_part_index`
/// emitted here matches the SVG's `data-part-index` only when the same
/// filter is applied here too — passing `None` when tracks are actually
/// hidden would emit indices for the *unfiltered* part list, which no longer
/// agree with the compacted indices the hidden-aware SVG renders.
pub fn list_note_spans_from_source(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
) -> Result<NoteSpansResult, IrrecoverableError> {
    let mut score = crate::compile(source, filename, &[])?;
    crate::filters::apply_track_filter(&mut score, enabled_tracks);

    let max_parts = score
        .measures
        .iter()
        .map(|m| m.parts.len())
        .max()
        .unwrap_or(0);
    let mut states: Vec<PartCounterState> = vec![PartCounterState::default(); max_parts];

    let mut spans = Vec::new();
    for (measure_index, measure) in score.measures.iter().enumerate() {
        let visible = visible_part_indices(measure);
        for (part_idx, part_row) in measure.parts.iter().enumerate() {
            if !visible.contains(&part_idx) {
                continue;
            }
            let Some(state) = states.get_mut(part_idx) else {
                continue;
            };
            for event in &part_row.slice().notes.events {
                let tentative_id = state.next_note_id;
                state.next_note_id += 1;

                let (span, tie_to_next) = match event {
                    NoteEvent::Note(note) => (Some(note.event_span), note.tie_to_next()),
                    NoteEvent::Chord(chord) => (Some(chord.event_span), chord.tie_to_next()),
                    NoteEvent::Percussion(hit) => (Some(hit.event_span), hit.tie_to_next()),
                    NoteEvent::Rest(rest) => (Some(rest.event_span), false),
                };

                let note_id = if state.prev_tie {
                    state.prev_tie_note_id.unwrap_or(tentative_id)
                } else {
                    tentative_id
                };

                spans.push(NoteSourceSpan {
                    source_part_index: part_idx,
                    note_id,
                    measure_index,
                    start: span.map(|s| s.start),
                    end: span.map(|s| s.end),
                });

                state.prev_tie = tie_to_next;
                state.prev_tie_note_id = if tie_to_next { Some(note_id) } else { None };
            }
        }
    }

    Ok(NoteSpansResult { spans })
}

/// One selected `(source_part_index, note_id)` cell — see `NoteSourceSpan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteCell {
    pub source_part_index: usize,
    pub note_id: usize,
}

/// One contiguous selected byte range within a single part's single
/// measure, ready to become a Monaco multicursor selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSelectionRun {
    pub source_part_index: usize,
    pub measure_index: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Groups a range-selected set of `(source_part_index, note_id)` cells into
/// contiguous per-`(part, measure)` source byte runs, folding each selected
/// cell's byte span into the running min/max for its `(part, measure)` key.
/// A cell with no mappable `NoteSourceSpan` (`start`/`end` both `None`,
/// which no current event kind produces, but is left possible for a future
/// one) is skipped rather than letting it break an otherwise-contiguous
/// run. Output is sorted by `(source_part_index, measure_index)`.
pub fn group_selected_notes_into_contiguous_runs(
    selected_cells: &[NoteCell],
    note_spans: &[NoteSourceSpan],
) -> Vec<NoteSelectionRun> {
    let selected: std::collections::HashSet<NoteCell> = selected_cells.iter().copied().collect();

    let mut runs_by_part_measure: std::collections::HashMap<(usize, usize), NoteSelectionRun> =
        std::collections::HashMap::new();
    for span in note_spans {
        let cell = NoteCell {
            source_part_index: span.source_part_index,
            note_id: span.note_id,
        };
        if !selected.contains(&cell) {
            continue;
        }
        let (Some(start), Some(end)) = (span.start, span.end) else {
            continue; // rest: skip, don't break contiguity
        };

        runs_by_part_measure
            .entry((span.source_part_index, span.measure_index))
            .and_modify(|run| {
                run.start_byte = run.start_byte.min(start);
                run.end_byte = run.end_byte.max(end);
            })
            .or_insert(NoteSelectionRun {
                source_part_index: span.source_part_index,
                measure_index: span.measure_index,
                start_byte: start,
                end_byte: end,
            });
    }

    let mut runs: Vec<NoteSelectionRun> = runs_by_part_measure.into_values().collect();
    runs.sort_by_key(|run| (run.source_part_index, run.measure_index));
    runs
}

#[cfg(test)]
#[path = "note_spans_tests.rs"]
mod tests;
