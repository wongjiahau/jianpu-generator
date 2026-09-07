use crate::selection_range::resolve_selection_range_response;
use crate::selection_range::types::{
    ClickableElementId, LyricCellOut, NoteCellOut, ResolveSelectionRangeResponse,
};

use super::test_helpers::{fixture, lyric, lyric_cell, lyric_span, note, note_cell, note_span};

/// Table-driven case: given `(anchor, current)`, resolving a `Note ↔ Lyric`
/// cross-row range must produce exactly the expected note/lyric cells —
/// `note_cells` unrestricted by verse (there's only one row of notes),
/// `lyric_cells` restricted to the `Lyric` endpoint's own verse (see
/// `note_lyric::cross_part`'s doc comment) — regardless of which side is
/// the `Note` and which is the `Lyric`, and regardless of anchor/current
/// order.
fn assert_note_lyric_range(
    anchor: &ClickableElementId,
    current: &ClickableElementId,
    expected_note_cells: &[NoteCellOut],
    expected_lyric_cells: &[LyricCellOut],
) {
    let (note_spans, lyric_spans) = fixture();
    let response = resolve_selection_range_response(&note_spans, &lyric_spans, anchor, current);
    match response {
        ResolveSelectionRangeResponse::Ok {
            note_cells,
            lyric_cells,
        } => {
            assert_eq!(note_cells, expected_note_cells);
            assert_eq!(lyric_cells, expected_lyric_cells);
        }
        ResolveSelectionRangeResponse::Err => panic!("expected Ok, got Err"),
    }
}

#[test]
fn same_part_note_lyric_range_note_anchor() {
    // Same part — ranges by `note_id`, not `measure_index` (see the
    // same-part arm's doc comment): `note_id` range [0, 2] picks up every
    // part-0 note and verse-0 syllable in that range, regardless of which
    // measure each happens to sit in.
    assert_note_lyric_range(
        &note(0, 0),
        &lyric(0, 2, 0),
        &[note_cell(0, 0), note_cell(0, 1), note_cell(0, 2)],
        &[
            lyric_cell(0, 0, 0),
            lyric_cell(0, 1, 0),
            lyric_cell(0, 2, 0),
        ],
    );
}

#[test]
fn same_part_note_lyric_range_lyric_anchor() {
    // Same pair as above with the `Lyric`/`Note` roles swapped between
    // anchor and current — same result.
    assert_note_lyric_range(
        &lyric(0, 0, 0),
        &note(0, 2),
        &[note_cell(0, 0), note_cell(0, 1), note_cell(0, 2)],
        &[
            lyric_cell(0, 0, 0),
            lyric_cell(0, 1, 0),
            lyric_cell(0, 2, 0),
        ],
    );
}

#[test]
fn same_part_note_lyric_range_uses_note_id_not_measure_index() {
    // Distinguishes the same-part arm (note_id-based) from the cross-part
    // arm below it (measure_index-based): note_id 5 shares measure_index 1
    // with note_id 1 (both part 0), but sits outside the anchor/current
    // note_id range [0, 1]. If the cross-part arm's measure-range rule ever
    // fired for a same-part pair — the exact regression this row's own
    // `note-lyric-cross-range-select.feature` caught, since a jianpu measure
    // routinely holds several notes — it would wrongly include note_id 5
    // (and any verse-0 lyric on it) via its measure match; the guard on the
    // arm above ensures it never does.
    let note_spans = vec![
        note_span(0, 0, 0),
        note_span(0, 1, 1),
        note_span(0, 5, 1),
        note_span(0, 2, 2),
    ];
    let mut lyric_spans = vec![lyric_span(0, 0, 0, 0), lyric_span(0, 1, 0, 1)];
    // A verse-1 syllable on note_id 1 (in range) also proves `lyric_cells`
    // stays scoped to the `Lyric` endpoint's own verse — mirrors
    // `LyricLabel ↔ LyricLabel`'s single-verse scoping.
    lyric_spans.push(lyric_span(0, 1, 1, 1));
    // A verse-0 syllable on note_id 5 (out of range) proves the range check
    // itself, not just the verse filter, excludes it.
    lyric_spans.push(lyric_span(0, 5, 0, 1));

    let anchor = note(0, 0);
    let current = lyric(0, 1, 0);
    let response = resolve_selection_range_response(&note_spans, &lyric_spans, &anchor, &current);

    match response {
        ResolveSelectionRangeResponse::Ok {
            note_cells,
            lyric_cells,
        } => {
            assert_eq!(note_cells, vec![note_cell(0, 0), note_cell(0, 1)]);
            assert_eq!(lyric_cells, vec![lyric_cell(0, 0, 0), lyric_cell(0, 1, 0)]);
        }
        ResolveSelectionRangeResponse::Err => panic!("expected Ok, got Err"),
    }
}

#[test]
fn cross_part_note_lyric_range() {
    // Different parts — no shared `note_id` axis to range over, so this
    // falls back to `note_lyric::cross_part`'s measure-range rule (see
    // that function's doc comment): part_range = [0, 1], measure_range =
    // [0, 1] (note 1,3's measure 1, lyric 0,0's measure 0) — excludes
    // part 0's measure-2 note/syllable; part 1 has no lyric at all in the
    // fixture, so lyric_cells stays part-0-only.
    assert_note_lyric_range(
        &note(1, 3),
        &lyric(0, 0, 0),
        &[note_cell(0, 0), note_cell(0, 1), note_cell(1, 3)],
        &[lyric_cell(0, 0, 0), lyric_cell(0, 1, 0)],
    );
}

#[test]
fn cross_part_note_lyric_range_excludes_other_verses() {
    // A verse-1 syllable sitting inside the swept measure/part range is
    // still excluded — the selection only ever covered the `Lyric` endpoint's
    // own verse-0 row, not every verse.
    let (mut note_spans, mut lyric_spans) = fixture();
    note_spans.push(note_span(1, 4, 1));
    lyric_spans.push(lyric_span(1, 4, 1, 1));

    let anchor = note(1, 3);
    let current = lyric(0, 0, 0);
    let response = resolve_selection_range_response(&note_spans, &lyric_spans, &anchor, &current);

    match response {
        ResolveSelectionRangeResponse::Ok { lyric_cells, .. } => {
            assert_eq!(lyric_cells, vec![lyric_cell(0, 0, 0), lyric_cell(0, 1, 0)]);
        }
        ResolveSelectionRangeResponse::Err => panic!("expected Ok, got Err"),
    }
}
