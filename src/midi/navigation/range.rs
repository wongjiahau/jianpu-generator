use crate::ast::grouped::Score;
use crate::error::IrrecoverableError;

use super::{build_expanded, expand_navigation_with_origins};

/// Smallest position `>= min_pos` in `origins` whose value equals
/// `written_index`, i.e. the earliest playback position (at or after
/// `min_pos`) at which the given written measure is played.
pub fn earliest_playback_position(
    origins: &[usize],
    written_index: usize,
    min_pos: usize,
) -> Option<usize> {
    origins
        .iter()
        .enumerate()
        .skip(min_pos)
        .find(|(_, &origin)| origin == written_index)
        .map(|(pos, _)| pos)
}

/// Resolves a `start..=end` range of **`# sequence` entry indices** (0-based
/// positions in `score.sequence`, i.e. the order entries are written in
/// `# sequence` — not written measure indices) directly into their playback
/// position range, by summing each entry's own span length instead of
/// searching `origins` for the earliest/last occurrence of a written measure
/// index.
///
/// This sidesteps an ambiguity `expand_for_measure_range` can't resolve on
/// its own: when a label recurs in `# sequence` (e.g. `A, B(-x), B`), every
/// occurrence of `B` shares the same written measure range, so matching by
/// written index alone always finds the *first* occurrence — the second
/// `B` (or a `B(-x)`-then-`B` swap) is unreachable that way. Since the
/// caller already knows which entry it means (e.g. the sequence-jump
/// toolbar's clicked button index), passing that index directly
/// picks the exact occurrence, omissions included.
///
/// Callers must first check `score.sequence` is `Some` and that
/// `entry_range` indexes into it (see [`expand_for_measure_range`], the
/// only caller) — an out-of-range `entry_range` silently produces a
/// meaningless position rather than an error.
fn expand_for_sequence_entry_range(
    score: &Score,
    entry_range: std::ops::RangeInclusive<usize>,
) -> (Score, usize, usize) {
    let (entry_start, entry_end) = (*entry_range.start(), *entry_range.end());
    let idx: Vec<(usize, &[String])> = score
        .sequence
        .iter()
        .flatten()
        .flat_map(|span| (span.start..=span.end).map(move |i| (i, span.omit_parts.as_slice())))
        .collect();
    let mut position = 0usize;
    let mut start_pos = 0usize;
    let mut end_pos = 0usize;
    for (i, span) in score.sequence.iter().flatten().enumerate() {
        let len = span.end - span.start + 1;
        if i == entry_start {
            start_pos = position;
        }
        if i == entry_end {
            end_pos = position + len - 1;
        }
        position += len;
    }
    let (expanded, _origins) = build_expanded(score, &idx);
    (expanded, start_pos, end_pos)
}

/// Maps a `start..=end` written range to a playback position range: maps
/// `start` to its earliest playback position at or after position 0.
///
/// - If `extend_to_last_occurrence` is `true`, `end` is mapped to its *last*
///   occurrence at or after `start`'s position — the final time `end` is
///   reached in the performance tail starting at `start`. This is what makes
///   "play written measure X through the last written measure" (the web
///   app's "play from current measure", which always passes the score's
///   literal last written measure as `end`) follow every repeat/jump instead
///   of stopping at `end`'s first occurrence.
/// - If `false`, `end` is mapped to its *earliest* occurrence at or after
///   `start`'s position, so that selecting an exact written range (e.g. the
///   web app's "play current measure", where `start == end`) plays only
///   that occurrence instead of overrunning into a later repeat pass.
/// - If `respect_sequence` is `false`, `# sequence`
///   (including any `(-abbrev ...)` part omissions it applies to a given
///   occurrence) are ignored entirely: the range is returned unchanged
///   against the literal written score, so e.g. "play current measure"
///   always plays the measure exactly as written, regardless of which
///   occurrence(s) of it a `# sequence` entry might otherwise select.
/// - `sequence_entry_range`, when `Some`, names the exact `# sequence`
///   entry/entries to play by their 0-based index into `score.sequence`
///   (see [`expand_for_sequence_entry_range`]) instead of resolving
///   `start_index`/`end_index` by written-index search — the only way to
///   correctly select a specific occurrence of a repeated label. Ignored
///   when `score.sequence` is `None`.
///
/// Falls back to the original written range if either endpoint has no
/// reachable position, or if `start_index > end_index`.
pub fn expand_for_measure_range(
    score: &Score,
    start_index: usize,
    end_index: usize,
    extend_to_last_occurrence: bool,
    respect_sequence: bool,
    sequence_entry_range: Option<std::ops::RangeInclusive<usize>>,
) -> Result<(Score, usize, usize), IrrecoverableError> {
    // Checked ahead of the `start_index > end_index` guard below: an entry
    // range is resolved directly from `score.sequence`, independent of
    // `start_index`/`end_index`, so it must not be skipped just because the
    // selected entries' *written* measure indices happen to run in reverse
    // (e.g. selecting `Y, X` in `X, Y(-b), Y, X`, where `Y`'s written measure
    // comes after `X`'s despite `Y` being selected first).
    if respect_sequence {
        if let Some(spans) = &score.sequence {
            if let Some(entry_range) = sequence_entry_range {
                if *entry_range.start() <= *entry_range.end() && *entry_range.end() < spans.len() {
                    return Ok(expand_for_sequence_entry_range(score, entry_range));
                }
            }
        }
    }
    if start_index > end_index {
        return Ok((score.clone(), start_index, end_index));
    }
    if !respect_sequence {
        return Ok((score.clone(), start_index, end_index));
    }
    let (expanded, origins) = expand_navigation_with_origins(score)?;
    let mapped = earliest_playback_position(&origins, start_index, 0).and_then(|start_pos| {
        let mut end_positions = origins
            .iter()
            .enumerate()
            .skip(start_pos)
            .filter(|(_, &origin)| origin == end_index);
        let end_pos = if extend_to_last_occurrence {
            end_positions.next_back()
        } else {
            end_positions.next()
        };
        end_pos.map(|(pos, _)| (start_pos, pos))
    });
    match mapped {
        Some((start_pos, end_pos)) => Ok((expanded, start_pos, end_pos)),
        None => Ok((score.clone(), start_index, end_index)),
    }
}
