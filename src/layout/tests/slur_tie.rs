use super::*;

#[test]
fn two_different_notes_emit_one_slur() {
    // 1~ 2: different pitches → one slur from col 3 to col 7
    let score = make_score("(12) 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let curves = collect_curves(&pages);
    assert_eq!(curves.len(), 1);
    assert_eq!(curves[0], (3, 7));
}

#[test]
fn three_note_slur_emits_one_curve() {
    // 3~2~1: all different pitches → one slur from col 3 to col 11
    let score = make_score("(321) 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let curves = collect_curves(&pages);
    assert_eq!(curves.len(), 1);
    assert_eq!(curves[0], (3, 11));
}

#[test]
fn mixed_chain_emits_slur_and_tie() {
    // (433) 2: chain [4@3, 3@7, 3@11]
    // → one slur from 3 to 11 (pitch change exists)
    // → one tie from 7 to 11 (same-pitch pair 3~3)
    let score = make_score("(433) 2", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let mut curves = collect_curves(&pages);
    curves.sort();
    assert_eq!(curves.len(), 2);
    assert_eq!(curves[0], (3, 11)); // slur
    assert_eq!(curves[1], (7, 11)); // tie
}

#[test]
fn nested_group_emits_outer_and_inner_slurs() {
    let score = make_score_raw(
        "(time=3/4 key=C4 bpm=120)\n(3= (2_1_)) 2_ 2_ 2_ 1=\na b c d e f g\n",
        "",
    );
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let mut curves = collect_curves(&pages);
    curves.sort_by_key(|(from, to)| (to - from, *from));
    assert_eq!(curves.len(), 2);
    let (inner_from, inner_to) = curves[0];
    let (outer_from, outer_to) = curves[1];
    assert!(inner_to - inner_from < outer_to - outer_from);
    assert!(inner_from >= outer_from && inner_to <= outer_to);
}

#[test]
fn same_pitch_chain_emits_only_tie() {
    // (11) 2 3: same pitches → one tie, no slur
    let score = make_score("(11) 2 3", "a b c");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let curves = collect_curves(&pages);
    assert_eq!(curves.len(), 1);
    assert_eq!(curves[0], (3, 7));
}

#[test]
fn cross_measure_tie_emits_right_half_arc_on_line_wrap() {
    // With default max_columns=28:
    // Measure 1: 1 (left bar col) + 16 (notes) + 1 (end bar) = 18 cols
    // Measure 2: 16 + 1 = 17 cols → 18+16=34 > 28 → wraps to new line
    // 3~ at col 15 in measure 1 should produce a right-half arc ending at the bar line (col 18).
    let score = make_score("0 0 0 (3 | 3) 0 0 0", "a");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let curves = collect_curves(&pages);
    assert!(
        !curves.is_empty(),
        "expected right-half tie arc when cross-measure tie wraps to new line"
    );
    // The right-half arc starts at the tied note (col 15) and ends at the bar line (col 19).
    assert!(
        curves.iter().any(|&(from, to)| from == 15 && to == 19),
        "expected right-half arc from col 15 to col 19; got: {curves:?}"
    );
}

#[test]
fn cross_measure_tie_continuation_does_not_consume_lyric_on_line_wrap() {
    // The continuation note (3 in measure 2) must NOT consume a lyric syllable
    // because prev_tie is preserved across the line boundary.
    // Only the 3~ note in measure 1 should consume a lyric.
    let score = make_score("0 0 0 (3 | 3) 0 0 0", "a");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let lyrics = collect_lyric_positions(&pages);
    assert_eq!(
        lyrics.len(),
        1,
        "continuation note across line break must not consume a lyric syllable; got: {lyrics:?}"
    );
    assert_eq!(lyrics[0].1, "a");
}
