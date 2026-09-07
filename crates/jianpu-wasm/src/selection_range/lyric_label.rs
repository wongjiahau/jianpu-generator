use crate::types::LyricSpanOut;

use super::helpers::{lyric_measure_index, LyricEndpoint, VerseMeasureSpan};
use super::types::{ClickableElementId, LyricCellOut, ResolveSelectionRangeResponse};

/// `LyricLabel ↔ LyricLabel` (same verse only) and `Lyric ↔ LyricLabel`. See
/// [`lyric_label_range`] and [`lyric_lyric_label_range`] for each rule's own
/// doc comment.
pub(crate) fn resolve(
    _note_spans: &[crate::types::NoteSpanOut],
    lyric_spans: &[LyricSpanOut],
    anchor: &ClickableElementId,
    current: &ClickableElementId,
) -> Option<ResolveSelectionRangeResponse> {
    match (anchor, current) {
        // `LyricLabel ↔ LyricLabel`, same verse — system-agnostic,
        // mirroring the cross-part `Note ↔ Note` arm: range resolution has
        // no notion of "system" at all, so a pair whose labels sit in
        // different systems (different `(measure_index_start,
        // measure_index_end)`) resolves exactly like a same-system pair,
        // just with the measure range spanning `min(anchor_start,
        // current_start)` .. `max(anchor_end, current_end)` instead of
        // collapsing to one shared span. This *is* the same-system case's
        // rule too — when both labels share one system, `anchor_start ==
        // current_start` and `anchor_end == current_end`, so the min/max
        // are that shared span — so there's no separate same-system arm to
        // keep in sync. A different-verse pair is still left unresolved
        // (falls through to `Err`, same as before) — a cross-verse selection
        // isn't this rule's concern.
        (
            ClickableElementId::LyricLabel {
                source_part_index: anchor_part,
                verse: anchor_verse,
                measure_index_start: anchor_start,
                measure_index_end: anchor_end,
            },
            ClickableElementId::LyricLabel {
                source_part_index: current_part,
                verse: current_verse,
                measure_index_start: current_start,
                measure_index_end: current_end,
            },
        ) if anchor_verse == current_verse => Some(lyric_label_range(
            lyric_spans,
            VerseMeasureSpan {
                part: *anchor_part,
                verse: *anchor_verse,
                start: *anchor_start,
                end: *anchor_end,
            },
            VerseMeasureSpan {
                part: *current_part,
                verse: *current_verse,
                start: *current_start,
                end: *current_end,
            },
        )),
        // `Lyric ↔ LyricLabel` — the one label-mixed pair where *both*
        // sides carry verse info. See `lyric_lyric_label_range`'s own doc
        // comment.
        (
            ClickableElementId::Lyric {
                source_part_index: lyric_part,
                note_id: lyric_note_id,
                verse: lyric_verse,
            },
            ClickableElementId::LyricLabel {
                source_part_index: label_part,
                verse: label_verse,
                measure_index_start: label_start,
                measure_index_end: label_end,
            },
        )
        | (
            ClickableElementId::LyricLabel {
                source_part_index: label_part,
                verse: label_verse,
                measure_index_start: label_start,
                measure_index_end: label_end,
            },
            ClickableElementId::Lyric {
                source_part_index: lyric_part,
                note_id: lyric_note_id,
                verse: lyric_verse,
            },
        ) => Some(lyric_lyric_label_range(
            lyric_spans,
            LyricEndpoint {
                part: *lyric_part,
                note_id: *lyric_note_id,
                verse: *lyric_verse,
            },
            VerseMeasureSpan {
                part: *label_part,
                verse: *label_verse,
                start: *label_start,
                end: *label_end,
            },
        )),
        _ => None,
    }
}

/// `LyricLabel ↔ LyricLabel`'s rule — derive `part_range`/`measure_range`
/// straight from the two labels' own fields (their shared `verse`, no
/// range needed), no span lookup required. A lyric-label sweep only ever
/// selects lyric syllables — no note cells, mirroring
/// `lyricCellsForLyricLabels`'s output type (unlike `PartLabel ↔
/// PartLabel`, which selects notes first and lyrics only as a secondary
/// row).
fn lyric_label_range(
    lyric_spans: &[LyricSpanOut],
    anchor: VerseMeasureSpan,
    current: VerseMeasureSpan,
) -> ResolveSelectionRangeResponse {
    let part_start = anchor.part.min(current.part);
    let part_end = anchor.part.max(current.part);
    let measure_start = anchor.start.min(current.start);
    let measure_end = anchor.end.max(current.end);

    let lyric_cells = lyric_spans
        .iter()
        .filter(|span| {
            span.source_part_index >= part_start
                && span.source_part_index <= part_end
                && span.verse == anchor.verse
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

/// Backs the `Lyric ↔ LyricLabel` arm — mirroring cross-part `Lyric ↔
/// Lyric`'s own `verse_range` idea rather than requiring an exact match
/// the way the plain `LyricLabel ↔ LyricLabel` rule still does, this
/// ranges over `verse_range` alongside `part_range`/`measure_range` (the
/// `Lyric` side's single measure treated as its own `[measure_index,
/// measure_index]` span, looked up from `lyric_spans`). Collapses to a
/// single verse when both sides share one, so this one function also
/// covers that case without a separate rule. `note_cells` stays empty
/// always, mirroring `Lyric ↔ Lyric`/`LyricLabel ↔ LyricLabel` — a
/// lyric-only gesture never reaches into the note row. `Err` if the
/// `Lyric` endpoint's own span can't be found (shouldn't happen for a
/// valid click-derived ID; guarded rather than panicking, mirroring this
/// crate's other cross-scope arms).
fn lyric_lyric_label_range(
    lyric_spans: &[LyricSpanOut],
    lyric: LyricEndpoint,
    label: VerseMeasureSpan,
) -> ResolveSelectionRangeResponse {
    let Some(lyric_measure) =
        lyric_measure_index(lyric_spans, lyric.part, lyric.note_id, lyric.verse)
    else {
        return ResolveSelectionRangeResponse::Err;
    };

    let part_start = lyric.part.min(label.part);
    let part_end = lyric.part.max(label.part);
    let measure_start = lyric_measure.min(label.start);
    let measure_end = lyric_measure.max(label.end);
    let verse_start = lyric.verse.min(label.verse);
    let verse_end = lyric.verse.max(label.verse);

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
