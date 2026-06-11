use crate::compiler::{compile, types::*};
use crate::grouper::group;
use crate::parser::parse;

fn score_from(source: &str) -> crate::ast::grouped::Score {
    let doc = parse(source, "test").unwrap();
    group(doc).unwrap()
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
    let blocks = compile(&score);
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
    let blocks = compile(&score);
    let row = &blocks[0].rows[0];
    let last = row.elements.last().unwrap();
    assert_eq!(last.content, ElementContent::BarLine);
}

#[test]
fn bpm_decoration_on_first_measure() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=100)\n1\n"));
    let blocks = compile(&score);
    let has_bpm = blocks[0]
        .decorations
        .iter()
        .any(|d| matches!(d, Decoration::Bpm(100)));
    assert!(has_bpm);
}

#[test]
fn two_measures_produce_two_blocks() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n\n2\n"));
    let blocks = compile(&score);
    assert_eq!(blocks.len(), 2);
}

#[test]
fn eighth_notes_produce_underline_elements() {
    // 2_ means eighth note (duration=2 quarter-beats) in jianpu syntax
    // Two eighth notes fill one beat; padded with rests to complete 4/4
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n2_ 2_ 0 0 0\n"));
    let blocks = compile(&score);
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
    let blocks = compile(&score);
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
    let blocks = compile(&score);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].rows.len(), 2);
}

#[test]
fn bar_number_decoration_without_label() {
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n\n2\n"));
    let blocks = compile(&score);
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
    let blocks = compile(&score);
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
    let blocks = compile(&score);
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
    let blocks = compile(&score);
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
    let blocks = compile(&score);
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
fn note_head_column_is_zero_indexed() {
    // First note in measure should be at column 0
    let score = score_from(&notes_doc("(time=4/4 key=C4 bpm=120)\n1\n"));
    let blocks = compile(&score);
    let row = &blocks[0].rows[0];
    let note_head = row
        .elements
        .iter()
        .find(|e| matches!(e.content, ElementContent::NoteHead { .. }))
        .unwrap();
    assert_eq!(note_head.column, 0);
}
