use crate::ast::grouped::NoteEvent;
use crate::compiler::visible_part_indices;
use crate::error::IrrecoverableError;

/// Source byte range of one lyric syllable — see `ast::parsed::Syllable::span`.
/// Keyed the same way the compiled SVG's `data-part-index`/`data-note-id`/
/// `data-verse` attributes are (see `renderer::new_renderer::render_lyric_click_target`),
/// so a click hit-test on the SVG can be mapped straight back to source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricSourceSpan {
    /// Index into `MultiPartMeasure::parts` for this syllable's part, matching
    /// the compiled `part_index`/`source_part_index` used throughout the
    /// renderer.
    pub source_part_index: usize,
    /// Same id the syllable's underlying note carries in
    /// `ColumnElement::note_id` (see `compiler::types::ElementContent::Lyric`).
    pub note_id: usize,
    /// 0-indexed verse this syllable belongs to, disambiguating multiple
    /// syllables that share the same `note_id` across a part's verse lines.
    pub verse: usize,
    /// Index into `Score.measures`.
    pub measure_index: usize,
    /// Inclusive start byte of this syllable's own token in the original source.
    pub start: usize,
    /// Exclusive end byte of this syllable's own token in the original source.
    pub end: usize,
}

/// Result of [`list_lyric_spans_from_source`].
pub struct LyricSpansResult {
    /// Source byte span of every lyric syllable, in score order (measure,
    /// then part, then verse, then note).
    pub spans: Vec<LyricSourceSpan>,
}

/// Per-part running state mirroring the id/tie bookkeeping
/// `compiler::part_slice::process_events` performs during compilation, so the
/// note ids produced here line up 1-to-1 with `ColumnElement::note_id` (and
/// thus with `note_spans::list_note_spans_from_source`'s own replay).
#[derive(Default, Clone, Copy)]
struct PartCounterState {
    next_note_id: usize,
    prev_tie: bool,
    prev_tie_note_id: Option<usize>,
}

/// Return the source byte span of every lyric syllable in the compiled
/// score, one entry per syllable, with note ids matching the compiled
/// `ColumnElement::note_id` values. Only notes/chords parts with lyrics
/// carry per-syllable spans (`ElementContent::Lyric`); `ElementContent::LyricLine`
/// (whole-line verses with no per-note identity) is currently unreachable —
/// see its doc comment — so it's never emitted here.
///
/// `enabled_tracks` must mirror whatever the caller passed to the render
/// pipeline — see `note_spans::list_note_spans_from_source`'s doc comment
/// for why: without it, `source_part_index` here would disagree with the
/// hidden-aware SVG's compacted `data-part-index` for every part declared
/// after a hidden one.
pub fn list_lyric_spans_from_source(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
) -> Result<LyricSpansResult, IrrecoverableError> {
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
            let slice = part_row.slice();
            let mut lyrics_iters: Vec<_> =
                slice.lyrics.iter().map(|l| l.syllables.iter()).collect();

            for event in &slice.notes.events {
                let tentative_id = state.next_note_id;
                state.next_note_id += 1;

                let is_tie_continuation = state.prev_tie;
                let tie_to_next = match event {
                    NoteEvent::Note(note) => note.tie_to_next(),
                    NoteEvent::Chord(chord) => chord.tie_to_next(),
                    NoteEvent::Percussion(hit) => hit.tie_to_next(),
                    NoteEvent::Rest(_) => false,
                };

                let note_id = if is_tie_continuation {
                    state.prev_tie_note_id.unwrap_or(tentative_id)
                } else {
                    tentative_id
                };

                // Mirrors `process_events`'s own gate exactly: only a
                // non-tie-continuation `Note` event on a part carrying lyrics
                // ever consumes a syllable from each verse.
                if !lyrics_iters.is_empty()
                    && !is_tie_continuation
                    && matches!(event, NoteEvent::Note(_))
                {
                    for (verse, it) in lyrics_iters.iter_mut().enumerate() {
                        if let Some(syllable) = it.next() {
                            spans.push(LyricSourceSpan {
                                source_part_index: part_idx,
                                note_id,
                                verse,
                                measure_index,
                                start: syllable.span.start,
                                end: syllable.span.end,
                            });
                        }
                    }
                }

                state.prev_tie = tie_to_next;
                state.prev_tie_note_id = if tie_to_next { Some(note_id) } else { None };
            }
        }
    }

    Ok(LyricSpansResult { spans })
}

/// One selected `(source_part_index, note_id, verse)` cell — see
/// `LyricSourceSpan`. Structurally identical to `note_spans::NoteCell` but
/// kept as its own type: a lyric cell and a note cell sharing the same
/// underlying note number are not interchangeable, they're just keyed by the
/// same note for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LyricCell {
    pub source_part_index: usize,
    pub note_id: usize,
    pub verse: usize,
}

/// One contiguous selected byte range within a single verse line of a
/// single part's single measure, ready to become a Monaco multicursor
/// selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricSelectionRun {
    pub source_part_index: usize,
    pub measure_index: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Groups a range-selected set of `(source_part_index, note_id, verse)` cells
/// into contiguous per-`(part, verse, measure)` source byte runs. Grouped by
/// verse as well as part/measure — even though the output `LyricSelectionRun`
/// doesn't carry `verse`, each verse's syllables live on their own source
/// line, so merging two different verses' spans into one run would produce a
/// byte range spanning unrelated text between them. Output is sorted by
/// `(source_part_index, measure_index)`.
pub fn group_selected_lyrics_into_contiguous_runs(
    selected_cells: &[LyricCell],
    lyric_spans: &[LyricSourceSpan],
) -> Vec<LyricSelectionRun> {
    let selected: std::collections::HashSet<LyricCell> = selected_cells.iter().copied().collect();

    let mut runs_by_part_verse_measure: std::collections::HashMap<
        (usize, usize, usize),
        LyricSelectionRun,
    > = std::collections::HashMap::new();
    for span in lyric_spans {
        let cell = LyricCell {
            source_part_index: span.source_part_index,
            note_id: span.note_id,
            verse: span.verse,
        };
        if !selected.contains(&cell) {
            continue;
        }

        runs_by_part_verse_measure
            .entry((span.source_part_index, span.verse, span.measure_index))
            .and_modify(|run| {
                run.start_byte = run.start_byte.min(span.start);
                run.end_byte = run.end_byte.max(span.end);
            })
            .or_insert(LyricSelectionRun {
                source_part_index: span.source_part_index,
                measure_index: span.measure_index,
                start_byte: span.start,
                end_byte: span.end,
            });
    }

    let mut runs: Vec<LyricSelectionRun> = runs_by_part_verse_measure.into_values().collect();
    // `start_byte` is included as a tiebreaker (not just `source_part_index`/
    // `measure_index`) because two different verses of the same part/measure
    // produce two separate runs that would otherwise sort in unstable
    // `HashMap` iteration order relative to each other.
    runs.sort_by_key(|run| (run.source_part_index, run.measure_index, run.start_byte));
    runs
}

#[cfg(test)]
#[path = "lyric_spans_tests.rs"]
mod tests;
