use crate::compiler::{compile, types::*};
use crate::grouper::group;
use crate::parser::parse;

fn score_from(source: &str) -> crate::ast::grouped::Score {
    let doc = parse(source, "test").unwrap();
    group(doc).unwrap()
}

/// Lyrics-part document with one track.
fn lyrics_doc(score_content: &str) -> String {
    format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nS = notes lyrics\n\n[score]\n{score_content}"
    )
}

/// Minimal one-part (notes) document. `score_content` is everything after `[score]\n`.
fn notes_doc(score_content: &str) -> String {
    format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nS = notes\n\n[score]\n{score_content}"
    )
}

/// Two-part (notes) document.
fn two_part_doc(score_content: &str) -> String {
    format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nS = notes\nA = notes\n\n[score]\n{score_content}"
    )
}

#[test]
fn single_quarter_note_produces_one_note_head_element() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    assert!(!blocks.is_empty());
    let row = &blocks[0].rows[0];
    let note_heads: Vec<_> = row
        .elements
        .iter()
        .filter(|e| matches!(e.content, ElementContent::NoteHead { .. }))
        .collect();
    assert_eq!(note_heads.len(), 1);
}

#[test]
fn bar_line_is_last_element_in_row() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let row = &blocks[0].rows[0];
    let last = row.elements.last().unwrap();
    assert_eq!(last.content, ElementContent::BarLine);
}

#[test]
fn bpm_decoration_on_first_measure() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=100)\n1\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let has_bpm = blocks[0]
        .decorations
        .iter()
        .any(|d| matches!(d, Decoration::Bpm(100)));
    assert!(has_bpm);
}

#[test]
fn two_measures_produce_two_blocks() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n\n2\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    assert_eq!(blocks.len(), 2);
}

#[test]
fn eighth_notes_produce_underline_elements() {
    // 2_ means eighth note (duration=2 quarter-beats) in jianpu syntax
    // Two eighth notes fill one beat; padded with rests to complete 4/4
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n2_ 2_ 0 0 0\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let row = &blocks[0].rows[0];
    let underlines: Vec<_> = row
        .elements
        .iter()
        .filter(|e| matches!(e.content, ElementContent::Underline { .. }))
        .collect();
    assert!(!underlines.is_empty(), "expected at least one underline");
}

#[test]
fn time_signature_appears_as_decoration() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let has_ts = blocks[0].decorations.iter().any(|d| {
        matches!(
            d,
            Decoration::TimeSignature {
                numerator: 4,
                denominator: 4
            }
        )
    });
    assert!(has_ts);
}

#[test]
fn ditto_rows_are_skipped() {
    // Two parts rendered in both measures (no ditto)
    let score = score_from(&two_part_doc("(time=4/4 key=C4 bpm=120)\n1\n3\n\n2\n4\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].rows.len(), 2);
}

#[test]
fn bar_number_decoration_without_label() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n\n2\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let bar1_num = blocks[0]
        .decorations
        .iter()
        .find(|d| matches!(d, Decoration::BarNumber(_)));
    assert!(
        matches!(bar1_num, Some(Decoration::BarNumber(1))),
        "first measure should have BarNumber(1)"
    );
    let bar2_num = blocks[1]
        .decorations
        .iter()
        .find(|d| matches!(d, Decoration::BarNumber(_)));
    assert!(
        matches!(bar2_num, Some(Decoration::BarNumber(2))),
        "second measure should have BarNumber(2)"
    );
}

#[test]
fn section_label_measure_has_no_bar_number() {
    let score = score_from(&notes_doc(
        "(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1\n",
    ));
    let result = compile(&score);
    let blocks = result.blocks;
    let has_bar_num = blocks[0]
        .decorations
        .iter()
        .any(|d| matches!(d, Decoration::BarNumber(_)));
    assert!(!has_bar_num, "labeled measure should not have a bar number");
    let has_label = blocks[0]
        .decorations
        .iter()
        .any(|d| matches!(d, Decoration::SectionLabel(_)));
    assert!(has_label, "labeled measure should have SectionLabel");
}

#[test]
fn rest_produces_rest_element() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n0\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let row = &blocks[0].rows[0];
    let rests: Vec<_> = row
        .elements
        .iter()
        .filter(|e| matches!(e.content, ElementContent::Rest { .. }))
        .collect();
    assert_eq!(rests.len(), 1);
}

#[test]
fn bar_line_column_equals_total_duration() {
    // "1 2 3 4" = four quarter notes, each duration=4 → total 16 quarter-beats
    // Bar line should appear at column 16
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1 2 3 4\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let row = &blocks[0].rows[0];
    let bar_line = row
        .elements
        .iter()
        .find(|e| matches!(e.content, ElementContent::BarLine))
        .unwrap();
    assert_eq!(
        bar_line.column, 16,
        "bar line should be at column 16 for four quarter notes"
    );
}

#[test]
fn all_parts_ditto_except_first_produces_all_label() {
    let score = score_from(
        "[metadata]
title=\"t\"
author=\"a\"

[parts]
Soprano (S) = notes
Alto (A) = notes
Tenor (T) = notes

[score]
(time=4/4 key=C4 bpm=120)
1 2 3 4
\"
\"
",
    );
    let result = compile(&score);
    let blocks = result.blocks;
    assert_eq!(
        blocks[0].rows.len(),
        1,
        "all ditto parts should collapse to one row"
    );
    assert_eq!(
        blocks[0].rows[0].label, "[ALL]",
        "label should be [ALL] when all parts except first are ditto"
    );
}

#[test]
fn extended_note_produces_note_dash_at_each_extra_beat() {
    // "1- 2-" = two half notes filling a 4/4 measure (8+8=16 quarter-beats).
    // Each half note should produce one NoteDash at the beat following the note head.
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1- 2-\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let row = &blocks[0].rows[0];
    let dashes: Vec<_> = row
        .elements
        .iter()
        .filter(|e| matches!(e.content, ElementContent::NoteDash))
        .collect();
    assert_eq!(
        dashes.len(),
        2,
        "two half notes should produce two NoteDash elements"
    );
    assert_eq!(dashes[0].column, 4, "first NoteDash should be at column 4");
    assert_eq!(
        dashes[1].column, 12,
        "second NoteDash should be at column 12"
    );
}

#[test]
fn note_head_column_is_zero_indexed() {
    // First note in measure should be at column 0
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n"));
    let result = compile(&score);
    let blocks = result.blocks;
    let row = &blocks[0].rows[0];
    let note_head = row
        .elements
        .iter()
        .find(|e| matches!(e.content, ElementContent::NoteHead { .. }))
        .unwrap();
    assert_eq!(note_head.column, 0);
}

#[test]
fn cross_measure_tie_does_not_consume_lyric_slot_for_continuation_note() {
    // Bar 1: "1 2 3 (4" has 4 lyric slots → "ha ta ba na"
    // Bar 2: "4) 5 6 7" → note 4 is a tie continuation, only 3 lyric slots → "sa da ko"
    // "sa" must be assigned to note 5 (column 4), not note 4) (column 0).
    let score = score_from(&lyrics_doc(concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 (4\n",
        "ha ta ba na\n",
        "\n",
        "4) 5 6 7\n",
        "sa da ko\n",
    )));
    let result = compile(&score);
    let blocks = result.blocks;
    let bar2 = &blocks[1].rows[0];
    // "sa" should be at column 4 (note 5, after the tied note 4 at column 0)
    let lyrics: Vec<_> = bar2
        .elements
        .iter()
        .filter_map(|e| {
            if let ElementContent::Lyric(text) = &e.content {
                Some((e.column, text.as_str()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        lyrics,
        vec![(4, "sa"), (8, "da"), (12, "ko")],
        "lyrics should be assigned to notes 5, 6, 7 (columns 4, 8, 12), not to the tied continuation note 4"
    );
}

#[test]
fn same_measure_slur_emits_slur_span() {
    // "(4 5)" open on note 4 (col 0), close on note 5 (col 4).
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n(4 5) 0 0\n"));
    let result = compile(&score);
    assert!(
        result.slur_spans.iter().any(|s| {
            s.part_index == 0
                && s.from_measure == 0
                && s.from_column == 0
                && s.to_measure == 0
                && s.to_column == 4
        }),
        "expected SlurSpan (measure=0, col=0) → (measure=0, col=4), got: {:?}",
        result.slur_spans
    );
}

#[test]
fn cross_measure_slur_emits_single_slur_span() {
    // Bar 1: "1 2 3 (4" — slur opens on note 4 at col 12.
    // Bar 2: "5) 6 7 1" — slur closes on note 5 at col 0.
    let score = score_from(&notes_doc(concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 (4\n",
        "\n",
        "5) 6 7 1\n",
    )));
    let result = compile(&score);
    assert!(
        result.slur_spans.iter().any(|s| {
            s.part_index == 0
                && s.from_measure == 0
                && s.from_column == 12
                && s.to_measure == 1
                && s.to_column == 0
        }),
        "expected SlurSpan (measure=0, col=12) → (measure=1, col=0), got: {:?}",
        result.slur_spans
    );
    assert!(
        result
            .slur_spans
            .iter()
            .all(|s| s.from_column != 16 && s.to_column != 16),
        "no slur span should touch barline col 16, got: {:?}",
        result.slur_spans
    );
}

#[test]
fn cross_measure_tie_emits_single_slur_span() {
    // Bar 1: "1 2 3 (4" — note 4 at col 12 has tie=true (same pitch on both sides).
    // Bar 2: "4) 5 6 7" — note 4 at col 0 closes the tie.
    let score = score_from(&notes_doc(concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 (4\n",
        "\n",
        "4) 5 6 7\n",
    )));
    let result = compile(&score);
    assert!(
        result.slur_spans.iter().any(|s| {
            s.part_index == 0
                && s.from_measure == 0
                && s.from_column == 12
                && s.to_measure == 1
                && s.to_column == 0
        }),
        "expected SlurSpan (measure=0, col=12) → (measure=1, col=0), got: {:?}",
        result.slur_spans
    );
}

#[test]
fn cross_measure_slur_closing_on_extension_dash() {
    // Bar 1: "1 2 3 (4" — slur opens on note 4 at col 12.
    // Bar 2: "5 -) - -" — slur closes at the extension dash at col 4.
    let score = score_from(&notes_doc(concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 (4\n",
        "\n",
        "5 -) - -\n",
    )));
    let result = compile(&score);
    assert!(
        result.slur_spans.iter().any(|s| {
            s.part_index == 0
                && s.from_measure == 0
                && s.from_column == 12
                && s.to_measure == 1
                && s.to_column == 4
        }),
        "expected SlurSpan (measure=0, col=12) → (measure=1, col=4), got: {:?}",
        result.slur_spans
    );
    assert!(
        result.slur_spans.iter().all(|s| s.to_column != 16),
        "no slur span should end at barline col 16"
    );
}

#[test]
fn three_measure_slur_emits_single_slur_span() {
    // Bar 1: "(1 2 3 4" — slur opens on note 1 at col 0, multiple notes in slur.
    // Bar 2: "5 6 7 1" — all notes in slur continue.
    // Bar 3: "2) 3 4 5" — slur closes on note 2 at col 0.
    let score = score_from(&notes_doc(concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "(1 2 3 4\n",
        "\n",
        "5 6 7 1\n",
        "\n",
        "2) 3 4 5\n",
    )));
    let result = compile(&score);
    assert!(
        result.slur_spans.iter().any(|s| {
            s.part_index == 0
                && s.from_measure == 0
                && s.from_column == 0
                && s.to_measure == 2
                && s.to_column == 0
        }),
        "expected SlurSpan (measure=0, col=0) → (measure=2, col=0), got: {:?}",
        result.slur_spans
    );
}

#[test]
fn three_measure_slur_with_single_note_middle_measure() {
    // Bar 1: "1 2 3 (4" — slur opens on note 4 at col 12.
    // Bar 2: "5 6 7 1" — single measure with all notes in slur continuation.
    // Bar 3: "2) 3 4 5" — slur closes on note 2 at col 0.
    let score = score_from(&notes_doc(concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 (4\n",
        "\n",
        "5 6 7 1\n",
        "\n",
        "2) 3 4 5\n",
    )));
    let result = compile(&score);
    assert!(
        result.slur_spans.iter().any(|s| {
            s.part_index == 0
                && s.from_measure == 0
                && s.from_column == 12
                && s.to_measure == 2
                && s.to_column == 0
        }),
        "expected SlurSpan (measure=0, col=12) → (measure=2, col=0), got: {:?}",
        result.slur_spans
    );
}
