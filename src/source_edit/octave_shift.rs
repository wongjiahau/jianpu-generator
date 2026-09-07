//! Bulk-rewrites the `'`/`,` octave markers on every note belonging to one
//! part, so a part transcribed an octave too high/low can be corrected
//! without editing each note by hand.
//!
//! Infallible / best-effort, mirroring [`super::update_part_declaration`]'s
//! "not found -> unchanged" convention: an unknown abbreviation, a
//! `follow[X]` part (which has no notes of its own), or any parse failure
//! returns `source` unchanged.

use crate::ast::parsed::{ParsedMeasureSlot, ParsedTrack, ScoreEvent};
use crate::error::Span;
use crate::parser;

/// Shifts every note in the part named `abbreviation` by `delta` octaves,
/// rewriting each note's `'`/`,` marker run in place.
pub fn shift_part_octave(source: &str, abbreviation: &str, delta: i8) -> String {
    if delta == 0 {
        return source.to_string();
    }

    let Ok(document) = parser::parse(source, "input.jianpu", &[]) else {
        return source.to_string();
    };

    let is_follow = document
        .declarations
        .iter()
        .find(|decl| decl.abbreviation == abbreviation)
        .is_none_or(|decl| decl.follow_target.is_some());
    if is_follow {
        return source.to_string();
    }

    let Some(ParsedTrack::Timed(track)) = document.tracks.iter().find(|track| {
        let ParsedTrack::Timed(track) = track;
        track.abbreviation == abbreviation
    }) else {
        return source.to_string();
    };

    let edits = collect_octave_shift_edits(source, std::iter::once(track), delta, |_| true);
    apply_octave_shift_edits(source, edits)
}

/// One `[start_byte, end_byte)` span of the editor's selection. A Monaco
/// multicursor selection (e.g. one produced by clicking a part label, which
/// selects that part's notes across every measure in the system) surfaces as
/// several disjoint `ByteRange`s rather than one contiguous span — see
/// [`shift_range_octave`]'s doc comment for why that distinction matters.
#[derive(Debug, Clone, Copy)]
pub struct ByteRange {
    pub start_byte: u32,
    pub end_byte: u32,
}

/// [`shift_range_octave`]'s return value: the rewritten source, plus the
/// caller's own input `ranges` remapped forward through the edits — the
/// editor toolbar's "shift selection" action re-selects these afterward
/// (see that field's doc comment for why the caller can't just reuse its
/// own pre-edit `ranges` verbatim for that).
#[derive(Debug, Clone)]
pub struct ShiftRangeOctaveResult {
    pub source: String,
    /// The caller's input `ranges`, in the same order and count, each
    /// remapped to its new position in the post-edit text. A range's length
    /// can change (e.g. a two-note contiguous drag selection whose notes'
    /// `'`/`,` marker runs each grow or shrink by a different amount) but its
    /// *shape* — how many ranges there are, contiguous vs. disjoint — is
    /// always preserved, so re-selecting these restores exactly what the
    /// caller had selected, just shifted onto the new text. Re-applying the
    /// caller's *pre-edit* `ranges` to the new text directly wouldn't work:
    /// each range's byte offsets only stay valid for text preceding every
    /// edit that lands before it.
    pub ranges: Vec<ByteRange>,
}

/// Shifts every note whose span overlaps *any* of `ranges` by `delta`
/// octaves, across every part — the "selection octave" toolbar action,
/// distinct from [`shift_part_octave`]'s whole-part scope.
///
/// Takes a *list* of ranges rather than one `[start_byte, end_byte)` pair
/// because a multicursor selection (e.g. clicking a part label, which
/// selects that part's notes across every measure in its system) is
/// generally disjoint: collapsing it to a single min/max span would sweep in
/// unrelated notes/parts sitting between the disjoint pieces (e.g. another
/// part's line in between two selected measures of this one).
///
/// Notes belonging to a `follow[X]` part have no events of their own (see
/// [`shift_part_octave`]'s doc comment), so they're naturally left alone
/// without any extra check here.
pub fn shift_range_octave(source: &str, ranges: &[ByteRange], delta: i8) -> ShiftRangeOctaveResult {
    let unchanged = || ShiftRangeOctaveResult {
        source: source.to_string(),
        ranges: Vec::new(),
    };

    if delta == 0 || ranges.is_empty() {
        return unchanged();
    }

    let Ok(document) = parser::parse(source, "input.jianpu", &[]) else {
        return unchanged();
    };

    let tracks = document
        .tracks
        .iter()
        .map(|ParsedTrack::Timed(track)| track);

    let edits = collect_octave_shift_edits(source, tracks, delta, |span| {
        ranges.iter().any(|range| {
            let (start_byte, end_byte) = (range.start_byte as usize, range.end_byte as usize);
            span.start < end_byte && span.end > start_byte
        })
    });
    if edits.is_empty() {
        return unchanged();
    }

    let new_ranges = remap_ranges(ranges, &edits);
    ShiftRangeOctaveResult {
        source: apply_octave_shift_edits(source, edits),
        ranges: new_ranges,
    }
}

/// Which end of an input range [`remap_offset`] is resolving, controlling how
/// it behaves when the offset falls *inside* an edit's old span (rather than
/// cleanly before or after it): a range's start biases left to the edit's new
/// start, its end biases right to the edit's new end, so a range that begins
/// or ends mid-note still comes back covering that note's full new text
/// instead of collapsing to a zero-width point inside it.
#[derive(Clone, Copy)]
enum Bias {
    Left,
    Right,
}

/// Remaps each of the caller's input `ranges` forward through `edits` to its
/// new `[start, end)` byte span in the post-edit text — the piece
/// [`shift_range_octave`] hands back so the caller can re-select exactly what
/// it had selected, just shifted onto the new text, preserving the input
/// ranges' count and shape (unlike remapping each *edit's own* span forward,
/// which would turn one contiguous multi-note selection into one disjoint
/// range per shifted note).
fn remap_ranges(ranges: &[ByteRange], edits: &[(Span, String)]) -> Vec<ByteRange> {
    let mut ascending: Vec<&(Span, String)> = edits.iter().collect();
    ascending.sort_by_key(|(span, _)| span.start);

    ranges
        .iter()
        .map(|range| ByteRange {
            start_byte: remap_offset(range.start_byte, &ascending, Bias::Left),
            end_byte: remap_offset(range.end_byte, &ascending, Bias::Right),
        })
        .collect()
}

/// Maps one byte offset in the pre-edit source to its position in the
/// post-edit text, given `edits_ascending` (sorted by span start). Walks the
/// edits in source order, accumulating how much the total byte length has
/// drifted (grown or shrunk) from every edit fully preceding `offset`; an
/// edit whose old span straddles `offset` is resolved via `bias` instead of
/// arithmetic, since `offset` has no well-defined position inside text that
/// got replaced wholesale.
fn remap_offset(offset: u32, edits_ascending: &[&(Span, String)], bias: Bias) -> u32 {
    let mut byte_length_delta: i64 = 0;
    for (span, replacement) in edits_ascending {
        if span.end as u32 <= offset {
            byte_length_delta += replacement.len() as i64 - (span.end - span.start) as i64;
            continue;
        }
        if span.start as u32 <= offset {
            let new_start = (span.start as i64 + byte_length_delta) as u32;
            return match bias {
                Bias::Left => new_start,
                Bias::Right => new_start + replacement.len() as u32,
            };
        }
        break;
    }
    (offset as i64 + byte_length_delta) as u32
}

/// Gathers the `(span, replacement)` edits for every note across `tracks`
/// that satisfies `span_matches`, shifted by `delta` octaves. Shared between
/// [`shift_part_octave`] (called with a single track and an always-true
/// predicate) and [`shift_range_octave`] (every track, filtered by byte
/// overlap).
fn collect_octave_shift_edits<'a>(
    source: &str,
    tracks: impl Iterator<Item = &'a crate::ast::parsed::ParsedTimedTrack>,
    delta: i8,
    span_matches: impl Fn(Span) -> bool,
) -> Vec<(Span, String)> {
    tracks
        .flat_map(|track| track.measure_slots.iter())
        .filter_map(|slot| match slot {
            ParsedMeasureSlot::Real { events } => Some(events),
            ParsedMeasureSlot::EmptyNote { .. } => None,
        })
        .flatten()
        .filter_map(|spanned| {
            let ScoreEvent::Note(note) = &spanned.value else {
                return None;
            };
            if !span_matches(spanned.span) {
                return None;
            }
            let new_octave = note.octave.saturating_add(delta);
            if new_octave == note.octave {
                return None;
            }
            let text = source.get(spanned.span.start..spanned.span.end)?;
            // A tied continuation written as a bare repeat atom (`_`/`=`/`r`,
            // see `parse_repeat_unit`) copies its pitch *and* octave from the
            // note it repeats, with no pitch digit or `'`/`,` marker of its
            // own in the source — there's nowhere in its span to attach a
            // marker. Leave it untouched: re-parsing after the anchor note's
            // own marker is rewritten picks up the new octave automatically.
            if !text.starts_with(|c: char| c.is_ascii_digit()) {
                return None;
            }
            Some((spanned.span, rewrite_octave_marker(text, new_octave)))
        })
        .collect()
}

/// Applies `edits` (as produced by [`collect_octave_shift_edits`]) to
/// `source`, rewriting from the end backwards so earlier spans stay valid.
fn apply_octave_shift_edits(source: &str, mut edits: Vec<(Span, String)>) -> String {
    if edits.is_empty() {
        return source.to_string();
    }

    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));

    let mut result = source.to_string();
    for (span, replacement) in edits {
        result.replace_range(span.start..span.end, &replacement);
    }
    result
}

/// Rewrites one note token's `'`/`,` octave marker to reflect `new_octave`,
/// preserving every other suffix character (duration/tie/dot) and their
/// relative order. The marker is re-inserted immediately after the
/// pitch+accidental head, matching the convention used throughout this
/// codebase's `.jianpu` sources (octave marker before duration/tie suffixes).
fn rewrite_octave_marker(text: &str, new_octave: i8) -> String {
    let split_at = text
        .char_indices()
        .find(|(_, c)| matches!(c, '_' | '=' | '-' | '.' | '~' | '\'' | ','))
        .map_or(text.len(), |(index, _)| index);
    let (head, remainder) = text.split_at(split_at);

    let remainder_without_octave: String = remainder
        .chars()
        .filter(|c| !matches!(c, '\'' | ','))
        .collect();

    let marker = match new_octave.cmp(&0) {
        std::cmp::Ordering::Greater => "'".repeat(new_octave as usize),
        std::cmp::Ordering::Less => ",".repeat((-new_octave) as usize),
        std::cmp::Ordering::Equal => String::new(),
    };

    format!("{head}{marker}{remainder_without_octave}")
}

#[cfg(test)]
#[path = "octave_shift_tests.rs"]
mod octave_shift_tests;
