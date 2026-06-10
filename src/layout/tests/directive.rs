use super::*;

#[test]
fn time_and_bpm_labels_emit_on_directive_row_above_meta_row() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let time_labels = collect_time_sig_labels(&pages);
    let bpm_labels = collect_bpm_labels(&pages);
    assert_eq!(time_labels.len(), 1);
    assert_eq!(bpm_labels.len(), 1);
    assert_eq!(
        time_labels[0].position.row, 2,
        "time signature should be on directive row (header_rows)"
    );
    assert_eq!(
        bpm_labels[0].position.row, 2,
        "BPM should be on directive row (header_rows)"
    );
    let bar_numbers: Vec<_> = pages[0].row_groups[0]
        .elements
        .iter()
        .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
        .collect();
    assert_eq!(bar_numbers.len(), 1);
    assert_eq!(
        bar_numbers[0].position.row, 3,
        "bar number should be on meta row (header_rows + 1)"
    );
}

#[test]
fn continuation_line_without_directive_changes_omits_directive_row() {
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, 300.0, A4_HEIGHT);
    let bar_numbers: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
        .collect();
    assert_eq!(bar_numbers.len(), 2);
    assert_eq!(bar_numbers[0].position.row, 3, "first line bar on meta row");
    assert_eq!(
        bar_numbers[1].position.row, 7,
        "wrapped line without directive changes should not add extra row"
    );
}

#[test]
fn first_measure_emits_time_signature_label_at_column_zero() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
        .collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].position.column, 3);
    if let GridContent::TimeSignatureLabel {
        numerator,
        denominator,
    } = &labels[0].content
    {
        assert_eq!(*numerator, 4);
        assert_eq!(*denominator, 4);
    } else {
        panic!("expected TimeSignatureLabel");
    }
}

#[test]
fn first_measure_emits_bpm_label_at_column_two() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
        .collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].position.column, 5);
    if let GridContent::BpmLabel { bpm } = &labels[0].content {
        assert_eq!(*bpm, 120);
    } else {
        panic!("expected BpmLabel");
    }
}

#[test]
fn note_heads_start_at_measure_column_not_after_directive_labels() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let note_heads: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
        .collect();
    assert_eq!(
        note_heads[0].position.column, 3,
        "notes start at label_cols+1; time/BPM on directive row share the same column"
    );
}

#[test]
fn notes_align_when_directives_present_on_first_measure_only() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nmax columns = 48\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
    );
    let pages = parse_and_layout(input);
    let note_heads: Vec<_> = pages[0].row_groups[0]
        .elements
        .iter()
        .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
        .collect();
    let measure1_start = note_heads[0].position.column;
    let measure2_start = note_heads[4].position.column;
    assert_eq!(measure1_start, 3);
    assert_eq!(
        measure2_start, 20,
        "second measure notes should continue after first measure bar, not reserve directive columns"
    );
}

#[test]
fn unchanged_time_signature_emits_no_second_label() {
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
        .collect();
    assert_eq!(labels.len(), 1, "only one time signature label expected for two measures with identical time signature on the same line");
}

#[test]
fn unchanged_bpm_emits_no_second_label() {
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
        .collect();
    assert_eq!(
        labels.len(),
        1,
        "only one BPM label expected for two measures with identical BPM on the same line"
    );
}

#[test]
fn mid_line_time_signature_change_uses_directive_row_for_whole_line() {
    // Wide max_columns keeps both measures on one system line.
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nmax columns = 48\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1=\n",
        "\n(time=3/4)\n1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1= 1=\n",
    );
    let pages = parse_and_layout(input);
    let time_labels = collect_time_sig_labels(&pages);
    assert_eq!(time_labels.len(), 2);
    assert_eq!(
        time_labels[0].position.row, time_labels[1].position.row,
        "both time labels on same directive row when measures share a system line"
    );
    assert!(
        time_labels[1].position.column > time_labels[0].position.column,
        "second time label at later measure column"
    );
}

#[test]
fn wrapped_time_signature_change_gets_directive_row_on_new_line() {
    let score = make_score_raw(
        "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(time=3/4)\n1 2 3\ne f g\n",
        "",
    );
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let time_labels = collect_time_sig_labels(&pages);
    assert_eq!(time_labels.len(), 2);
    assert_eq!(time_labels[0].position.row, 2, "first line directive row");
    assert_eq!(
        time_labels[1].position.row, 7,
        "wrapped line directive row after first row group with directive row"
    );
}

#[test]
fn time_signature_change_emits_second_label() {
    // Two bars: first 4/4 (4 quarter notes), second 3/4 (3 quarter notes), each with lyrics.
    let score = make_score_raw(
        "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(time=3/4)\n1 2 3\ne f g\n",
        "",
    );
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
        .collect();
    assert_eq!(
        labels.len(),
        2,
        "expected one label per distinct time signature, got positions: {:?}",
        labels
            .iter()
            .map(|e| (e.position.column, e.position.row))
            .collect::<Vec<_>>()
    );
}

#[test]
fn bpm_change_emits_second_label() {
    // Two bars: first at bpm=120, second at bpm=90, each with lyrics.
    let score = make_score_raw(
        "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(bpm=90)\n5 6 7 1\ne f g h\n",
        "",
    );
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
        .collect();
    assert_eq!(
        labels.len(),
        2,
        "expected one BPM label per distinct BPM value"
    );
}

#[test]
fn header_is_populated_on_every_page() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    assert!(!pages.is_empty());
    for page in &pages {
        assert_eq!(page.header.title, "t");
        assert_eq!(page.header.author, "a");
        assert_eq!(page.header.subtitle, None);
    }
}

#[test]
fn footer_page_numbers_are_correct() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let total = pages.len() as u32;
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page.footer.page, i as u32 + 1);
        assert_eq!(page.footer.total, total);
    }
}

#[test]
fn produces_at_least_one_page() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    assert!(!pages.is_empty());
}

#[test]
fn note_heads_are_present() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let note_heads: Vec<_> = all_elements
        .iter()
        .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
        .collect();
    assert_eq!(note_heads.len(), 4);
}

#[test]
fn lyrics_are_present() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let lyrics: Vec<_> = all_elements
        .iter()
        .filter(|e| matches!(e.content, GridContent::Lyric { .. }))
        .collect();
    assert_eq!(lyrics.len(), 4);
}
