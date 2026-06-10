use super::*;

#[test]
fn consecutive_eighth_notes_at_beat_start_share_one_underline() {
    // _2 _2 fills beat 1 (qb 0–3); 0 0 0 are quarter rests filling the rest of 4/4
    // Total: 2+2+4+4+4 = 16 quarter-beats ✓
    let score = make_score("2_ 2_ 0 0 0", "a b");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let groups = collect_underline_levels(&pages);
    assert_eq!(groups.len(), 1, "expected one beam group");
    assert_eq!(groups[0].len(), 1, "expected one underline level");
    assert_eq!(groups[0][0].from_column, 3);
    assert_eq!(groups[0][0].to_column, 7);
}

#[test]
fn eighth_rest_and_note_within_same_beat_share_one_underline() {
    // 0(4qb) _0(2qb) _2(2qb) _2(2qb) _0(2qb) 0(4qb) = 16qb ✓
    // Beat 2: _0 rest + _2 note → share one underline (same beat, rest joins beam buffer)
    // Beat 3: _2 note + _0 rest → share one underline (same beat)
    let score = make_score("0 0_ 2_ 2_ 0_ 0", "a b");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let groups = collect_underline_levels(&pages);
    assert_eq!(
        groups.len(),
        2,
        "expected two underline groups (one per beat)"
    );
    // group[0]: beat 2 — _0 rest + _2 note
    assert_eq!(groups[0][0].from_column, 7);
    assert_eq!(groups[0][0].to_column, 11);
    // group[1]: beat 3 — _2 note + _0 rest
    assert_eq!(groups[1][0].from_column, 11);
    assert_eq!(groups[1][0].to_column, 15);
}

#[test]
fn mixed_eighth_and_sixteenth_notes_produce_two_underline_levels() {
    // _1(2qb) =2(1qb) =3(1qb) fills beat 1 exactly; 0 0 0 fill 12 more qb = 16 total ✓
    // Level 1: spans all three notes (col 3–7)
    // Level 2: spans only the sixteenth sub-run =2,=3 (col 5–7)
    let score = make_score("1_ 2= 3= 0 0 0", "a b c");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let groups = collect_underline_levels(&pages);
    assert_eq!(groups.len(), 1, "expected one beam group");
    assert_eq!(groups[0].len(), 2, "expected two underline levels");
    assert_eq!(groups[0][0].from_column, 3);
    assert_eq!(groups[0][0].to_column, 7);
    assert_eq!(groups[0][1].from_column, 5);
    assert_eq!(groups[0][1].to_column, 7);
}

#[test]
fn sixteenth_note_and_sixteenth_rests_share_one_beat_group() {
    // =1(1qb) =0(1qb) =0(1qb) =0(1qb) fills beat 1; 0 0 0 fills the remaining 12qb = 16 total ✓
    // All four fit within beat 1 → joined in one beam group with two underline levels.
    let score = make_score("1= 0= 0= 0= 0 0 0", "a");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let groups = collect_underline_levels(&pages);
    assert_eq!(
        groups.len(),
        1,
        "expected one beam group (note + rests share a beat)"
    );
    assert_eq!(groups[0].len(), 2, "expected two underline levels");
    // Level 1 and level 2 both span the whole beat (cols 3–7)
    assert_eq!(
        groups[0][0],
        UnderlineSpan {
            from_column: 3,
            to_column: 7,
            last_head_column: 6
        }
    );
    assert_eq!(
        groups[0][1],
        UnderlineSpan {
            from_column: 3,
            to_column: 7,
            last_head_column: 6
        }
    );
}

#[test]
fn eighth_rest_underline_connects_to_following_sixteenth_notes() {
    // _0(2qb) =1(1qb) =2(1qb) fills beat 1 exactly (2+1+1=4qb); 0 0 0 fills 12 more = 16 total ✓
    // _0 rest should join the beam buffer and share the level-1 underline with =1 and =2.
    let score = make_score("0_ 1= 2= 0 0 0", "a b");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let groups = collect_underline_levels(&pages);
    assert_eq!(groups.len(), 1, "expected one beam group spanning the beat");
    assert_eq!(groups[0].len(), 2, "expected two underline levels");
    // Level 1 spans all three (col 3–7)
    assert_eq!(groups[0][0].from_column, 3);
    assert_eq!(groups[0][0].to_column, 7);
    // Level 2 spans only =1 and =2 (col 5–7)
    assert_eq!(groups[0][1].from_column, 5);
    assert_eq!(groups[0][1].to_column, 7);
}

#[test]
fn pure_sixteenth_beat_group_has_two_underlines() {
    // =1 =2 =3 =4 fills one beat exactly (4×1qb = 4qb); 0 0 0 fills 12 more qb = 16 total ✓
    // All four notes are sixteenth (underline_count=2): level-1 spans 5–9, level-2 also 5–9.
    let score = make_score("1= 2= 3= 4= 0 0 0", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let groups = collect_underline_levels(&pages);
    assert_eq!(groups.len(), 1, "expected one beam group spanning the beat");
    assert_eq!(
        groups[0].len(),
        2,
        "pure-sixteenth group must produce two underline levels"
    );
    assert_eq!(
        groups[0][0],
        UnderlineSpan {
            from_column: 3,
            to_column: 7,
            last_head_column: 6
        }
    );
    assert_eq!(
        groups[0][1],
        UnderlineSpan {
            from_column: 3,
            to_column: 7,
            last_head_column: 6
        }
    );
}

#[test]
fn tied_notes_share_one_lyric_syllable() {
    // 3~3 is a tie (same pitch): both notes share one syllable.
    // (33) 1 2 with lyrics "a b c":
    //   3 (col 3) → "a",  second 3 (col 7) → no lyric,  1 (col 11) → "b",  2 (col 15) → "c"
    let score = make_score("(33) 1 2", "a b c");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    assert_eq!(
        collect_lyric_positions(&pages),
        vec![
            (3, "a".to_string()),
            (11, "b".to_string()),
            (15, "c".to_string())
        ],
    );
}

#[test]
fn slurred_notes_each_get_a_lyric_syllable() {
    // 4~3~3: 4→3 is a slur (different pitch, each gets a syllable),
    //        3→3 is a tie (same pitch, second 3 shares the syllable of first 3).
    // So "(433) 2" with lyrics "a b c" assigns: 4→"a", first 3→"b", second 3→no lyric, 2→"c"
    let score = make_score("(433) 2", "a b c");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    assert_eq!(
        collect_lyric_positions(&pages),
        vec![
            (3, "a".to_string()),
            (7, "b".to_string()),
            (15, "c".to_string())
        ],
    );
}

#[test]
fn dash_lyric_is_rendered() {
    // "1 2 3 4" with lyrics "你 - 好 a": note 1→"你", note 2→"-", note 3→"好", note 4→"a"
    let score = make_score("1 2 3 4", "你 - 好 a");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    assert_eq!(
        collect_lyric_positions(&pages),
        vec![
            (3, "你".to_string()),
            (7, "-".to_string()),
            (11, "好".to_string()),
            (15, "a".to_string())
        ],
    );
}

#[test]
fn half_beat_note_has_duration_underline() {
    // 3/4 bar avoids 4/4 grouping rules: 2 eighth notes separated by 2 quarter notes.
    // _1 and 4_ are each flushed as separate beam groups → 2 DurationUnderlines elements.
    let score = make_score_raw("(time=3/4 key=C4 bpm=120)\n1_ 3 3 4_\na b c d\n", "");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let underlines: Vec<_> = all_elements.iter()
        .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if levels.len() == 1))
        .collect();
    assert_eq!(underlines.len(), 2); // one for _1, one for 4_
}

#[test]
fn dotted_half_beat_note_has_one_underline() {
    // _1* = dotted eighth (duration 3). Should get 1 underline like a plain eighth.
    // 3 + 1 + 4 + 4 + 4 = 16 quarter-beats = one full 4/4 bar.
    let score = make_score("1_. 2= 3 3 3", "a b c d e");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let underlines: Vec<_> = all_elements
        .iter()
        .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if !levels.is_empty()))
        .collect();
    assert!(
        !underlines.is_empty(),
        "dotted eighth note must produce at least one underline"
    );
}

#[test]
fn dotted_note_head_has_dotted_flag() {
    // _1* note head should have dotted=true in the layout element.
    let score = make_score("1_. 2= 3 3 3", "a b c d e");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let dotted_heads: Vec<_> = all_elements
        .iter()
        .filter(|e| matches!(&e.content, GridContent::NoteHead { dotted: true, .. }))
        .collect();
    assert_eq!(
        dotted_heads.len(),
        1,
        "exactly one note head should be dotted"
    );
}

#[test]
fn lower_octave_note_emits_lower_octave_dots_element() {
    // "1." = pitch 1, 1-beat note (duration=4), octave -1
    // underline_count for duration=4 is 0
    let score = make_score("1, 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let lower_dots: Vec<_> = all_elements
        .iter()
        .filter(|e| matches!(e.content, GridContent::LowerOctaveDots { .. }))
        .collect();
    assert_eq!(lower_dots.len(), 1, "expected one LowerOctaveDots element");
    if let GridContent::LowerOctaveDots {
        count,
        underline_count,
    } = &lower_dots[0].content
    {
        assert_eq!(*count, 1);
        assert_eq!(*underline_count, 0, "1-beat note has 0 underlines");
    }
    assert_eq!(
        lower_dots[0].position.row, 5,
        "LowerOctaveDots must be in absolute row 5 with directive row"
    );
    assert_eq!(lower_dots[0].vertical_alignment, VerticalAlignment::Top);
}

#[test]
fn unchanged_labels_do_not_repeat_after_line_wrap() {
    // Wrapping is controlled by max_columns (default 28), not page width.
    // First measure: 16 (notes) + 1 (bar) = 17 cols — fits in 28.
    // Second measure: 16 + 1 = 17 cols — 17 + 17 = 34 > 28 → wraps after first measure.
    // Same time sig and BPM on second measure → no repeat labels.
    // Total TimeSignatureLabel count across the whole score should be exactly 1.
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, 300.0, A4_HEIGHT);
    let time_sig_labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
        .collect();
    assert_eq!(
        time_sig_labels.len(),
        1,
        "time signature label must not repeat on wrapped lines, got {}",
        time_sig_labels.len()
    );
}
