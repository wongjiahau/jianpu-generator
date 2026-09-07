use super::{shift_part_octave, shift_range_octave, ByteRange};

fn source_with_score(parts_body: &str, score_body: &str) -> String {
    format!(
        "# metadata\ntitle = \"t\"\nauthor = \"a\"\n\n# parts\n{parts_body}\n\n# score\ntime=4/4 key=C4 bpm=120\n{score_body}\n"
    )
}

#[test]
fn shifts_every_note_uniformly() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let result = shift_part_octave(&source, "M", 1);
    assert!(result.contains("[M] 1' 2' 3' 4'"), "got:\n{result}");
}

#[test]
fn shifts_down_from_existing_marker() {
    let source = source_with_score("Melody [M] = notes", "[M] 1' 2' 3' 4'\n");
    let result = shift_part_octave(&source, "M", -1);
    assert!(result.contains("[M] 1 2 3 4"), "got:\n{result}");
}

#[test]
fn tied_notes_shift_together_and_stay_tied() {
    let source = source_with_score("Melody [M] = notes", "[M] 1~ 1 5 5\n");
    let result = shift_part_octave(&source, "M", 1);
    assert!(result.contains("[M] 1'~ 1' 5' 5'"), "got:\n{result}");
}

#[test]
fn drops_marker_when_net_octave_is_zero() {
    let source = source_with_score("Melody [M] = notes", "[M] 1' 2' 3' 4'\n");
    let result = shift_part_octave(&source, "M", -1);
    assert!(
        !result.contains('\''),
        "octave markers should be fully dropped:\n{result}"
    );
}

#[test]
fn preserves_duration_and_dash_suffixes() {
    let source = source_with_score("Melody [M] = notes", "[M] 1_ 2_ 3- 4\n");
    let result = shift_part_octave(&source, "M", 1);
    assert!(result.contains("[M] 1'_ 2'_ 3'- 4'"), "got:\n{result}");
}

#[test]
fn preserves_dot_suffix() {
    let source = source_with_score("Melody [M] = notes", "[M] 1. 2. 3\n");
    let result = shift_part_octave(&source, "M", 1);
    assert!(result.contains("[M] 1'. 2'. 3'"), "got:\n{result}");
}

#[test]
fn unknown_abbreviation_returns_source_unchanged() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let result = shift_part_octave(&source, "NOMATCH", 1);
    assert_eq!(result, source);
}

#[test]
fn follow_part_returns_source_unchanged() {
    let source = source_with_score(
        "Melody [M] = notes\nChords [C] = follow[M]",
        "[M] 1 2 3 4\n",
    );
    let result = shift_part_octave(&source, "C", 1);
    assert_eq!(result, source);
}

#[test]
fn zero_delta_returns_source_unchanged() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let result = shift_part_octave(&source, "M", 0);
    assert_eq!(result, source);
}

/// One `ByteRange` covering the entire span `[start_needle, end_needle)`
/// (the substring starting at `start_needle`'s first occurrence in `source`
/// through the end of `end_needle`'s first occurrence at/after that point).
fn range_between(source: &str, start_needle: &str, end_needle: &str) -> ByteRange {
    let start = source.find(start_needle).unwrap() as u32;
    let end_start = source[start as usize..].find(end_needle).unwrap() + start as usize;
    let end = (end_start + end_needle.len()) as u32;
    ByteRange {
        start_byte: start,
        end_byte: end,
    }
}

#[test]
fn range_shifts_only_notes_overlapping_the_byte_range() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let range = range_between(&source, "2 3", "2 3");
    let result = shift_range_octave(&source, &[range], 1).source;
    assert!(result.contains("[M] 1 2' 3' 4"), "got:\n{result}");
}

#[test]
fn range_spanning_two_parts_shifts_notes_in_both() {
    let source = source_with_score(
        "Melody [M] = notes\nBass [B] = notes",
        "[M] 1 2 3 4\n[B] 5 6 7 1\n",
    );
    // From the last Melody note through the first Bass note.
    let range = range_between(&source, "4\n[B] 5", "4\n[B] 5");
    let result = shift_range_octave(&source, &[range], 1).source;
    assert!(result.contains("[M] 1 2 3 4'"), "got:\n{result}");
    assert!(result.contains("[B] 5' 6 7 1"), "got:\n{result}");
}

#[test]
fn range_spanning_two_measures_shifts_notes_in_both() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n\n[M] 5 6 7 1\n");
    // From the last note of measure 1 through the first note of measure 2.
    let range = range_between(&source, "4\n\n[M] 5", "4\n\n[M] 5");
    let result = shift_range_octave(&source, &[range], 1).source;
    assert!(result.contains("[M] 1 2 3 4'"), "got:\n{result}");
    assert!(result.contains("[M] 5' 6 7 1"), "got:\n{result}");
}

#[test]
fn range_with_no_overlapping_notes_returns_source_unchanged() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let result = shift_range_octave(
        &source,
        &[ByteRange {
            start_byte: 0,
            end_byte: 0,
        }],
        1,
    );
    assert_eq!(result.source, source);
    assert!(result.ranges.is_empty());
}

#[test]
fn range_zero_delta_returns_source_unchanged() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let len = source.len() as u32;
    let result = shift_range_octave(
        &source,
        &[ByteRange {
            start_byte: 0,
            end_byte: len,
        }],
        0,
    );
    assert_eq!(result.source, source);
    assert!(result.ranges.is_empty());
}

#[test]
fn empty_ranges_list_returns_source_unchanged() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let result = shift_range_octave(&source, &[], 1);
    assert_eq!(result.source, source);
    assert!(result.ranges.is_empty());
}

/// Regression test for a part-label-click bug: clicking a part label in a
/// system with more than one measure produces one *disjoint* Monaco
/// selection per measure (a multicursor selection), not one contiguous
/// span. Passing every disjoint range shifts notes in both measures of the
/// clicked part, while leaving the other part's line — which sits, in
/// source order, *between* the two selected ranges — untouched. Collapsing
/// the two ranges to a single min/max span would incorrectly sweep in that
/// other part's line too; this test guards against that regression.
#[test]
fn multiple_disjoint_ranges_shift_notes_in_each_without_touching_the_part_in_between() {
    let source = source_with_score(
        "Melody [M] = notes\nHarmony [H] = notes",
        "[M] 1 2\n[H] 5 6\n\n[M] 3 4\n[H] 7 1\n",
    );
    let measure_zero_melody = range_between(&source, "1 2", "1 2");
    let measure_one_melody = range_between(&source, "3 4", "3 4");
    let response = shift_range_octave(&source, &[measure_zero_melody, measure_one_melody], 1);
    let result = &response.source;
    assert!(result.contains("[M] 1' 2'"), "got:\n{result}");
    assert!(result.contains("[M] 3' 4'"), "got:\n{result}");
    assert!(result.contains("[H] 5 6"), "got:\n{result}");
    assert!(result.contains("[H] 7 1\n"), "got:\n{result}");
}

/// Regression test for the part-label-click bug's *second* symptom: once
/// the editor re-selects [`shift_range_octave`]'s returned `ranges` and the
/// user shifts again, every one of those ranges must still land exactly on
/// its measure — not drift out of place. Restoring a stale *pre-edit*
/// selection instead (what the editor used to do) works for the first
/// measure but drifts for every one after it, since each earlier measure's
/// marker runs can grow or shrink by a different amount than the selection
/// was captured for.
///
/// Also guards the *shape* of the returned ranges: this input is a genuinely
/// disjoint multicursor selection (one range per measure, as a part-label
/// click produces), so the response must keep exactly that many ranges,
/// each remapped onto its own measure's new text — not collapsed, and not
/// exploded into one range per shifted note.
#[test]
fn returned_ranges_preserve_a_disjoint_multicursor_selections_shape() {
    let source = source_with_score(
        "Melody [M] = notes\nHarmony [H] = notes",
        "[M] 1 2\n[H] 5 6\n\n[M] 3 4\n[H] 7 1\n",
    );
    let measure_zero_melody = range_between(&source, "1 2", "1 2");
    let measure_one_melody = range_between(&source, "3 4", "3 4");
    let response = shift_range_octave(&source, &[measure_zero_melody, measure_one_melody], 1);

    let texts_at_returned_ranges: Vec<&str> = response
        .ranges
        .iter()
        .map(|range| &response.source[range.start_byte as usize..range.end_byte as usize])
        .collect();
    assert_eq!(texts_at_returned_ranges, vec!["1' 2'", "3' 4'"]);

    // Shifting a second time using exactly the returned ranges (as the
    // editor's re-applied selection would) must shift every measure again.
    let second = shift_range_octave(&response.source, &response.ranges, 1);
    assert!(
        second.source.contains("[M] 1'' 2''"),
        "got:\n{}",
        second.source
    );
    assert!(
        second.source.contains("[M] 3'' 4''"),
        "got:\n{}",
        second.source
    );
    assert!(second.source.contains("[H] 5 6"), "got:\n{}", second.source);
    assert!(
        second.source.contains("[H] 7 1\n"),
        "got:\n{}",
        second.source
    );
}

/// A single contiguous drag selection spanning multiple notes (as opposed to
/// the disjoint per-measure multicursor above) must come back as a single
/// contiguous range covering the whole shifted span — not split into one
/// range per shifted note, which would visibly narrow the user's selection
/// even though every originally-selected note still got shifted.
#[test]
fn a_contiguous_multi_note_range_stays_a_single_range() {
    let source = source_with_score("Melody [M] = notes", "[M] 1 2 3 4\n");
    let range = range_between(&source, "1 2 3 4", "1 2 3 4");
    let response = shift_range_octave(&source, &[range], 1);

    assert!(
        response.source.contains("[M] 1' 2' 3' 4'"),
        "got:\n{}",
        response.source
    );
    assert_eq!(response.ranges.len(), 1);
    let only_range = response.ranges[0];
    let text_at_range =
        &response.source[only_range.start_byte as usize..only_range.end_byte as usize];
    assert_eq!(text_at_range, "1' 2' 3' 4'");
}

/// Regression test for a tied note written with the bare-repeat-atom
/// shorthand (`_`/`=`/`r`, which repeats the previous pitched note's pitch
/// *and* octave with no digit or `'`/`,` marker of its own in the source —
/// see `parse_repeat_unit`'s doc comment). Shifting octave up then back down
/// must restore the original source exactly: since a repeat atom has no
/// pitch-head character to attach a marker to, `rewrite_octave_marker` used
/// to insert the marker *before* the atom (e.g. `_` -> `'_`), producing
/// malformed source that doesn't round-trip back to a bare `_`.
#[test]
fn shifting_up_then_down_restores_a_tied_repeat_atom_note() {
    let source = source_with_score("Melody [M] = notes", "[M] 1~ _ 5 5\n");
    let up = shift_part_octave(&source, "M", 1);
    let down = shift_part_octave(&up, "M", -1);
    assert_eq!(down, source, "roundtrip failed\nup:\n{up}\ndown:\n{down}");
}
