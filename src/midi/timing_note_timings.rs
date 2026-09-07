use crate::ast::grouped::Score;
use crate::error::IrrecoverableError;
use crate::filters::apply_track_filter;

use super::navigation::{
    expand_navigation_with_note_positions, filter_expanded_tracks, ExpandedMeasureOrigin,
};
use super::timing::{measure_tick_boundaries_and_tempo, ticks_to_seconds};
use super::timing_note_events::{
    build_written_note_id_lookup, record_measure_note_timings, MeasureTimingContext,
    PartTimingCursor,
};
use super::timing_range::build_measure_range_score;
use super::TPQ;
use crate::compiler::compile;

/// Physically drops parts `visible_tracks` excludes, mirroring
/// [`crate::filters::apply_track_filter`], the same removal the *rendered*
/// SVG's `compile()` call sees (see `render_svgs_from_source_filtered_with_lyrics`).
/// Timing/note-id bookkeeping must be built from a score shaped this way —
/// not the fully unfiltered written score — so a leading all-rest run that's
/// only all-rest once a hidden part's notes are removed collapses into the
/// same single `MultiMeasureRest` block/`note_id` the render does (see
/// `compiler::merge_rest_runs`). `None` returns `score` unchanged (no
/// allocation).
///
/// Distinct from `enabled_tracks`/[`filter_expanded_tracks`]: that mutes a
/// possibly-narrower subset of these *visible* parts for one clip's audio
/// only (e.g. range-select playback) without removing anything from the
/// render, so it must never influence block/note_id structure — it's applied
/// afterward, to the expanded timeline, on top of whatever this function
/// already resolved.
fn visible_score(score: &Score, visible_tracks: Option<&[String]>) -> Score {
    if visible_tracks.is_none() {
        return score.clone();
    }
    let mut score = score.clone();
    apply_track_filter(&mut score, visible_tracks);
    score
}

/// Identity of one sounding note/rest's elapsed-seconds extent, matching the
/// `(source_part_index, note_id)` key stamped onto `ColumnElement`s by the
/// compiler (see `compiler::types::ColumnElement::note_id`) and surfaced in
/// rendered SVG via `renderer::new_types::Tag::Note`. A tie that spans a
/// measure boundary is merged into a single `NoteTiming` (its `end_s` is the
/// tied-to note's own end), mirroring how the MIDI writer merges tied notes
/// into one NoteOn/NoteOff pair.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteTiming {
    pub source_part_index: usize,
    pub note_id: usize,
    pub start_s: f64,
    pub end_s: f64,
}

/// Maps each written measure index to the index of the compiled block it
/// ended up in — consecutive all-rest written measures may be collapsed by
/// the compiler into a single `MultiMeasureRest` glyph
/// (`compiler::merge_rest_runs`), so several written measures can share one
/// block.
fn written_measure_to_block(written_blocks: &[crate::compiler::MeasureBlock]) -> Vec<usize> {
    written_blocks
        .iter()
        .enumerate()
        .flat_map(|(block_index, block)| {
            std::iter::repeat_n(block_index, block.represents_measures)
        })
        .collect()
}

/// The number of `PartTimingCursor`s needed to cover every written part,
/// keyed by *written* part index (stable across playback occurrences), not
/// by position within a given occurrence's (possibly omission-shrunk)
/// `parts` vec — so a tie held open across an expanded measure boundary
/// stays attached to the right part even if a neighboring occurrence's
/// `(-abbrev ...)` omissions shifted positions around.
fn new_part_timing_cursors(score: &Score) -> Vec<PartTimingCursor> {
    let max_written_parts = score
        .measures
        .iter()
        .map(|m| m.parts.len())
        .max()
        .unwrap_or(0);
    (0..max_written_parts)
        .map(|_| PartTimingCursor::new())
        .collect()
}

/// Elapsed-seconds start/end of every sounding note, rest, or chord actually
/// heard when `score` is played back, keyed by `(source_part_index,
/// note_id)` — the same identity `ColumnElement::note_id` uses.
///
/// Playback order follows `# sequence`/D.C.-al-Coda-Fine navigation exactly
/// like [`super::write_midi`] does (via
/// [`expand_navigation_with_note_positions`]), so a repeated or reordered
/// written measure produces one [`NoteTiming`] per occurrence it's actually
/// played — all sharing the written event's `(source_part_index, note_id)`,
/// since that identity names *which written note*, not which time it sounds.
/// `note_id`s themselves are computed once over the *written* score
/// ([`build_written_note_id_lookup`]), so they agree with `ColumnElement`
/// regardless of how many times playback repeats them.
///
/// Ties across measures are merged into a single `NoteTiming` per occurrence
/// (matching how the compiler reuses a note's id for its tie continuation,
/// and how the MIDI writer merges the underlying NoteOn/NoteOff pair) rather
/// than producing one entry per tied fragment. Likewise, a run of consecutive
/// all-rest written measures the compiler collapses into one
/// `MultiMeasureRest` glyph (`compiler::merge_rest_runs`) produces a single
/// `NoteTiming` spanning the whole run, using the glyph's own `note_id`
/// (`MeasureRow::first_note_id`), rather than one entry per underlying
/// measure.
///
/// `visible_tracks` names which parts are actually rendered (the part
/// visibility toggle's state, i.e. what `apply_track_filter` would remove
/// before `compile()` for the SVG render — see [`visible_score`]).
/// `note_id_lookup`/`written_blocks` are built from that visibility-filtered
/// score, so a leading all-rest run that only becomes all-rest once a hidden
/// part's notes are removed collapses into the same single
/// `MultiMeasureRest` block/`note_id` the render does. `None` keeps every
/// part (matching an unfiltered render).
///
/// `enabled_tracks` separately mutes playback down to a further, possibly
/// narrower subset of the *visible* parts for this one clip only (e.g. the
/// web app's note range-select playback), without disturbing
/// `source_part_index` or block structure: it's applied to the *expanded*
/// timeline via [`filter_expanded_tracks`], strictly after the
/// visibility-filtered `note_id_lookup`/`written_blocks` are built — so a
/// kept part still reports the index it has in the visibility-filtered
/// render, matching the rendered SVG's `data-note-id`. `None` keeps every
/// visible part.
pub fn note_timings_seconds(
    score: &Score,
    visible_tracks: Option<&[String]>,
    enabled_tracks: Option<&[String]>,
) -> Result<Vec<NoteTiming>, IrrecoverableError> {
    let score = visible_score(score, visible_tracks);
    let score = &score;
    let note_id_lookup = build_written_note_id_lookup(score);
    let written_blocks = compile(score).blocks;
    let block_lookup = written_measure_to_block(&written_blocks);

    let (mut expanded, mut origins) = expand_navigation_with_note_positions(score)?;
    filter_expanded_tracks(&mut expanded, &mut origins, enabled_tracks);
    // Tick boundaries/tempo are built over the expanded (playback-order)
    // measures, matching `write_midi`.
    let (measure_start_ticks, tempo_changes) =
        measure_tick_boundaries_and_tempo(&expanded.measures)?;

    let mut cursors = new_part_timing_cursors(score);
    // (source_part_index, note_id, start_tick, end_tick)
    let mut results: Vec<(usize, usize, u32, u32)> = Vec::new();

    for (measure, (tick_window, origin)) in expanded
        .measures
        .iter()
        .zip(measure_start_ticks.windows(2).zip(origins.iter()))
    {
        let Some(ctx) = measure_timing_context(
            tick_window,
            origin,
            &block_lookup,
            &written_blocks,
            &note_id_lookup,
        ) else {
            continue;
        };
        record_measure_note_timings(measure, ctx, &mut cursors, &mut results);
    }

    Ok(results
        .into_iter()
        .map(
            |(source_part_index, note_id, start_tick, end_tick)| NoteTiming {
                source_part_index,
                note_id,
                start_s: ticks_to_seconds(start_tick, &tempo_changes, TPQ),
                end_s: ticks_to_seconds(end_tick, &tempo_changes, TPQ),
            },
        )
        .collect())
}

/// Same as [`note_timings_seconds`], but scoped to a measure range and
/// relative to the start of that range, matching the audio clip returned by
/// [`super::write_midi_for_measure_range`].
///
/// `start_pos`/`end_pos` are *playback positions* — i.e. already resolved
/// against `# sequence`/D.C.-al-Coda-Fine navigation, exactly like
/// [`super::expand_for_measure_range`] resolves them for
/// [`super::write_midi_for_measure_range`]'s caller — not raw written
/// measure indices. Unlike that caller, `score` here must still be the
/// *original, unexpanded* written score: this function re-derives the
/// expanded (playback-order) timeline itself via
/// [`expand_navigation_with_note_positions`] so it can look `note_id`s up
/// against `build_written_note_id_lookup(score)`/`compile(score).blocks` —
/// both computed once over the *visibility-filtered* score (see
/// [`visible_score`]), so they agree with the `note_id` `ColumnElement`s
/// carry in the rendered SVG regardless of how navigation reorders playback,
/// and so a leading all-rest run that's only all-rest once a hidden part is
/// removed still collapses to the same `MultiMeasureRest` block/`note_id`
/// the render sees. If `end_pos` falls outside the expanded timeline (only
/// possible if the caller derived `start_pos`/`end_pos` from a different
/// score), this returns an empty result rather than panicking. See
/// [`note_timings_seconds`] for how `enabled_tracks` differs from
/// `visible_tracks`.
pub fn note_timings_seconds_for_range(
    score: &Score,
    start_pos: usize,
    end_pos: usize,
    visible_tracks: Option<&[String]>,
    enabled_tracks: Option<&[String]>,
) -> Result<Vec<NoteTiming>, IrrecoverableError> {
    if score.measures.is_empty() || start_pos > end_pos {
        return Ok(Vec::new());
    }

    let score = visible_score(score, visible_tracks);
    let score = &score;
    let note_id_lookup = build_written_note_id_lookup(score);
    let written_blocks = compile(score).blocks;
    let block_lookup = written_measure_to_block(&written_blocks);

    let (mut expanded, mut origins) = expand_navigation_with_note_positions(score)?;
    filter_expanded_tracks(&mut expanded, &mut origins, enabled_tracks);
    if end_pos >= expanded.measures.len() {
        return Ok(Vec::new());
    }
    let Some(range_score) = build_measure_range_score(&expanded, start_pos, end_pos) else {
        return Ok(Vec::new());
    };

    let (measure_start_ticks, tempo_changes) =
        measure_tick_boundaries_and_tempo(&range_score.measures)?;

    let mut cursors = new_part_timing_cursors(score);
    let mut results: Vec<(usize, usize, u32, u32)> = Vec::new();

    for (measure, (tick_window, origin)) in range_score.measures.iter().zip(
        measure_start_ticks
            .windows(2)
            .zip(origins.iter().skip(start_pos)),
    ) {
        let Some(ctx) = measure_timing_context(
            tick_window,
            origin,
            &block_lookup,
            &written_blocks,
            &note_id_lookup,
        ) else {
            continue;
        };
        record_measure_note_timings(measure, ctx, &mut cursors, &mut results);
    }

    Ok(results
        .into_iter()
        .map(
            |(source_part_index, note_id, start_tick, end_tick)| NoteTiming {
                source_part_index,
                note_id,
                start_s: ticks_to_seconds(start_tick, &tempo_changes, TPQ),
                end_s: ticks_to_seconds(end_tick, &tempo_changes, TPQ),
            },
        )
        .collect())
}

/// Same as [`note_timings_seconds_for_range`], but for a range that ignores
/// `# sequence`/D.C.-al-Coda-Fine navigation entirely (`respect_sequence:
/// false` in [`super::MeasureRangeSelection`]) — the "play current measure"
/// case. `start_index`/`end_index` are literal indices into `score.measures`
/// (not playback positions), matching what
/// [`super::expand_for_measure_range`] returns unchanged when
/// `respect_sequence` is `false`. Unlike [`note_timings_seconds_for_range`],
/// this never re-derives the expanded (playback-order) timeline: each
/// measure's own written index is its origin, so a written measure that also
/// happens to recur elsewhere in `# sequence` (e.g. selecting "C" when the
/// sequence is `A, B, B, C`) still resolves to its own `note_id`s rather than
/// to whichever measure occupies that same position in the expanded
/// timeline. See [`note_timings_seconds`] for how `visible_tracks` (applied
/// before `note_id_lookup`/`written_blocks` are built, see
/// [`visible_score`]) differs from `enabled_tracks`.
pub fn note_timings_seconds_for_literal_range(
    score: &Score,
    start_index: usize,
    end_index: usize,
    visible_tracks: Option<&[String]>,
    enabled_tracks: Option<&[String]>,
) -> Result<Vec<NoteTiming>, IrrecoverableError> {
    if score.measures.is_empty() || start_index > end_index {
        return Ok(Vec::new());
    }

    let score = visible_score(score, visible_tracks);
    let score = &score;
    let note_id_lookup = build_written_note_id_lookup(score);
    let written_blocks = compile(score).blocks;
    let block_lookup = written_measure_to_block(&written_blocks);

    let Some(mut range_score) = build_measure_range_score(score, start_index, end_index) else {
        return Ok(Vec::new());
    };
    let mut origins: Vec<ExpandedMeasureOrigin> = range_score
        .measures
        .iter()
        .enumerate()
        .map(|(measure_offset, measure)| ExpandedMeasureOrigin {
            written_measure_index: start_index + measure_offset,
            part_written_indices: (0..measure.parts.len()).collect(),
        })
        .collect();
    filter_expanded_tracks(&mut range_score, &mut origins, enabled_tracks);

    let (measure_start_ticks, tempo_changes) =
        measure_tick_boundaries_and_tempo(&range_score.measures)?;

    let mut cursors = new_part_timing_cursors(score);
    let mut results: Vec<(usize, usize, u32, u32)> = Vec::new();

    for (measure, (tick_window, origin)) in range_score
        .measures
        .iter()
        .zip(measure_start_ticks.windows(2).zip(origins.iter()))
    {
        let Some(ctx) = measure_timing_context(
            tick_window,
            origin,
            &block_lookup,
            &written_blocks,
            &note_id_lookup,
        ) else {
            continue;
        };
        record_measure_note_timings(measure, ctx, &mut cursors, &mut results);
    }

    Ok(results
        .into_iter()
        .map(
            |(source_part_index, note_id, start_tick, end_tick)| NoteTiming {
                source_part_index,
                note_id,
                start_s: ticks_to_seconds(start_tick, &tempo_changes, TPQ),
                end_s: ticks_to_seconds(end_tick, &tempo_changes, TPQ),
            },
        )
        .collect())
}

/// Builds one measure's [`MeasureTimingContext`] from a length-2
/// `measure_start_ticks.windows(2)` slice and its navigation origin. Returns
/// `None` if any of the invariants documented on [`MeasureTimingContext`]
/// don't hold (which shouldn't happen for a `tick_window`/`origin` pair
/// produced by this module's own callers), so the caller can skip the
/// measure rather than panic.
fn measure_timing_context<'a>(
    tick_window: &[u32],
    origin: &'a ExpandedMeasureOrigin,
    block_lookup: &'a [usize],
    written_blocks: &'a [crate::compiler::MeasureBlock],
    note_id_lookup: &'a std::collections::HashMap<(usize, usize, usize), usize>,
) -> Option<MeasureTimingContext<'a, impl Fn(usize) -> usize + 'a>> {
    let &[measure_start_tick, measure_end_tick] = tick_window else {
        return None;
    };
    let block_index = *block_lookup.get(origin.written_measure_index)?;
    let block = written_blocks.get(block_index)?;
    Some(MeasureTimingContext {
        written_measure_index: origin.written_measure_index,
        part_written_index: |part_idx| {
            origin
                .part_written_indices
                .get(part_idx)
                .copied()
                .unwrap_or(part_idx)
        },
        measure_start_tick,
        measure_end_tick,
        block_index,
        block,
        note_id_lookup,
    })
}
