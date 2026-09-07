use crate::types::LyricSpanOut;

use super::helpers::{lyric_measure_index, LyricEndpoint};
use super::types::{ClickableElementId, LyricCellOut, ResolveSelectionRangeResponse};

/// `Lyric ↔ Lyric`, every scope — same-part-and-verse, same-part
/// cross-verse, and cross-part. See [`same_part_same_verse`],
/// [`same_part_cross_verse`], and [`cross_part`] for each rule's own doc
/// comment.
pub(crate) fn resolve(
    _note_spans: &[crate::types::NoteSpanOut],
    lyric_spans: &[LyricSpanOut],
    anchor: &ClickableElementId,
    current: &ClickableElementId,
) -> Option<ResolveSelectionRangeResponse> {
    match (anchor, current) {
        (
            ClickableElementId::Lyric {
                source_part_index: anchor_part,
                note_id: anchor_id,
                verse: anchor_verse,
            },
            ClickableElementId::Lyric {
                source_part_index: current_part,
                note_id: current_id,
                verse: current_verse,
            },
        ) if anchor_part == current_part && anchor_verse == current_verse => {
            Some(same_part_same_verse(
                lyric_spans,
                *anchor_part,
                *anchor_verse,
                *anchor_id,
                *current_id,
            ))
        }
        // `Lyric ↔ Lyric`, same part, different verse — the syllable-row
        // counterpart to `LyricLabel ↔ LyricLabel`'s own cross-verse gap
        // (that one is unaffected by this arm; it's the label row, not the
        // syllable row). Falls through from the same-part-and-verse guard
        // above. See `same_part_cross_verse`'s own doc comment.
        (
            ClickableElementId::Lyric {
                source_part_index: anchor_part,
                note_id: anchor_id,
                verse: anchor_verse,
            },
            ClickableElementId::Lyric {
                source_part_index: current_part,
                note_id: current_id,
                verse: current_verse,
            },
        ) if anchor_part == current_part => Some(same_part_cross_verse(
            lyric_spans,
            *anchor_part,
            *anchor_id,
            *anchor_verse,
            *current_id,
            *current_verse,
        )),
        // `Lyric ↔ Lyric`, different part (any verse pairing, including
        // same-verse-different-part) — falls through from the same-part
        // guards above. See `cross_part`'s own doc comment.
        (
            ClickableElementId::Lyric {
                source_part_index: anchor_part,
                note_id: anchor_id,
                verse: anchor_verse,
            },
            ClickableElementId::Lyric {
                source_part_index: current_part,
                note_id: current_id,
                verse: current_verse,
            },
        ) => Some(cross_part(
            lyric_spans,
            LyricEndpoint {
                part: *anchor_part,
                note_id: *anchor_id,
                verse: *anchor_verse,
            },
            LyricEndpoint {
                part: *current_part,
                note_id: *current_id,
                verse: *current_verse,
            },
        )),
        _ => None,
    }
}

/// Same-part, same-verse `Lyric ↔ Lyric` — ranges by `note_id` alone,
/// within the shared `verse`. No note cells — mirrors the same-part
/// `Note ↔ Note` arm's note that an index range has no notion of "row".
fn same_part_same_verse(
    lyric_spans: &[LyricSpanOut],
    part: usize,
    verse: usize,
    anchor_id: usize,
    current_id: usize,
) -> ResolveSelectionRangeResponse {
    let range_start = anchor_id.min(current_id);
    let range_end = anchor_id.max(current_id);

    let lyric_cells = lyric_spans
        .iter()
        .filter(|span| {
            span.source_part_index == part
                && span.verse == verse
                && span.note_id >= range_start
                && span.note_id <= range_end
        })
        .map(|span| LyricCellOut {
            source_part_index: span.source_part_index,
            note_id: span.note_id,
            verse: span.verse,
        })
        .collect();

    ResolveSelectionRangeResponse::Ok {
        note_cells: Vec::new(),
        lyric_cells,
    }
}

/// `note_id` numbering is shared across a part's verses the same way it's
/// shared between a part's notes and lyrics (the fact the same-part
/// `Note ↔ Lyric` arm already leans on), so this ranges `note_id` exactly
/// like `same_part_same_verse`, plus a second, independent range over
/// `verse` — verse acts as a row index here, the same role
/// `sourcePartIndex` plays in `PartLabel ↔ PartLabel`. A click-and-click
/// selection already behaves this way for a straight vertical sweep
/// (verse rows render as stacked bands in increasing verse order under a
/// part, so the anchor-to-current span naturally covers a contiguous verse
/// range) — this rule matches that rather than diverging from it. See
/// `PLAN-clickable-element-id-selection.md`'s "cross-verse (same part)"
/// writeup and `lyric-range-select-crosses-verse.feature`.
fn same_part_cross_verse(
    lyric_spans: &[LyricSpanOut],
    part: usize,
    anchor_id: usize,
    anchor_verse: usize,
    current_id: usize,
    current_verse: usize,
) -> ResolveSelectionRangeResponse {
    let note_id_start = anchor_id.min(current_id);
    let note_id_end = anchor_id.max(current_id);
    let verse_start = anchor_verse.min(current_verse);
    let verse_end = anchor_verse.max(current_verse);

    let lyric_cells = lyric_spans
        .iter()
        .filter(|span| {
            span.source_part_index == part
                && span.verse >= verse_start
                && span.verse <= verse_end
                && span.note_id >= note_id_start
                && span.note_id <= note_id_end
        })
        .map(|span| LyricCellOut {
            source_part_index: span.source_part_index,
            note_id: span.note_id,
            verse: span.verse,
        })
        .collect();

    ResolveSelectionRangeResponse::Ok {
        note_cells: Vec::new(),
        lyric_cells,
    }
}

/// No shared `note_id` axis across parts, so this reuses the cross-part
/// `Note ↔ Note`/`Note ↔ Lyric` arms' `measure_index`-range pattern for its
/// column axis: each endpoint's own `measure_index`, looked up from
/// `lyric_spans` by its own `(source_part_index, note_id, verse)`. Unlike
/// those arms, the row axis here can't collapse to "the one `Lyric`
/// endpoint's own verse" (`Note ↔ Lyric`'s trick) or "every verse"
/// (`PartLabel ↔ PartLabel`'s trick) — both endpoints are `Lyric` and can
/// each carry a different verse — so it gets its own `verse` range instead,
/// exactly like `same_part_cross_verse` uses. When both endpoints share one
/// verse, that range collapses to a single value, so this one function also
/// covers the same-verse-different-part case without a separate rule. See
/// `PLAN-clickable-element-id-selection.md`'s "cross-part (any verse
/// pairing)" writeup and `lyric-range-select-crosses-part.feature`.
///
/// `Err` if either endpoint's own span can't be found — shouldn't happen
/// for a valid click-derived ID, but guarded rather than panicking,
/// mirroring this crate's other cross-part arms.
fn cross_part(
    lyric_spans: &[LyricSpanOut],
    anchor: LyricEndpoint,
    current: LyricEndpoint,
) -> ResolveSelectionRangeResponse {
    let anchor_measure =
        lyric_measure_index(lyric_spans, anchor.part, anchor.note_id, anchor.verse);
    let current_measure =
        lyric_measure_index(lyric_spans, current.part, current.note_id, current.verse);
    let (Some(anchor_measure), Some(current_measure)) = (anchor_measure, current_measure) else {
        return ResolveSelectionRangeResponse::Err;
    };

    let part_start = anchor.part.min(current.part);
    let part_end = anchor.part.max(current.part);
    let verse_start = anchor.verse.min(current.verse);
    let verse_end = anchor.verse.max(current.verse);
    let measure_start = anchor_measure.min(current_measure);
    let measure_end = anchor_measure.max(current_measure);

    let lyric_cells = lyric_spans
        .iter()
        .filter(|span| {
            span.source_part_index >= part_start
                && span.source_part_index <= part_end
                && span.verse >= verse_start
                && span.verse <= verse_end
                && span.measure_index >= measure_start
                && span.measure_index <= measure_end
        })
        .map(|span| LyricCellOut {
            source_part_index: span.source_part_index,
            note_id: span.note_id,
            verse: span.verse,
        })
        .collect();

    ResolveSelectionRangeResponse::Ok {
        note_cells: Vec::new(),
        lyric_cells,
    }
}
