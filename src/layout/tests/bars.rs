use super::*;

#[test]
fn part_label_and_barline_variants_exist() {
    let _ = GridContent::PartLabel {
        text: "Soprano".to_string(),
    };
    let _ = GridContent::BarLine { height_in_rows: 1 };
}

#[test]
fn two_part_layout_emits_part_labels() {
    let score = make_two_part_score("1 2 3 4", "5 6 7 1");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::PartLabel { .. }))
        .collect();
    assert_eq!(labels.len(), 2, "expected one PartLabel per named part");
}

#[test]
fn two_part_layout_has_note_heads_for_both_parts() {
    let score = make_two_part_score("1 2 3 4", "5 6 7 1");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let note_heads: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
        .collect();
    assert_eq!(note_heads.len(), 8, "expected 4 notes per part × 2 parts");
}

#[test]
fn two_part_layout_emits_directives_once_on_directive_row() {
    let score = make_two_part_score("1 2 3 4", "5 6 7 1");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let time_sig_labels = collect_time_sig_labels(&pages);
    let bpm_labels = collect_bpm_labels(&pages);
    assert_eq!(
        time_sig_labels.len(),
        1,
        "time signature label should appear once per change, not per notes part"
    );
    assert_eq!(
        bpm_labels.len(),
        1,
        "BPM label should appear once per change, not per notes part"
    );
}

#[test]
fn single_named_part_produces_part_label() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let labels: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::PartLabel { .. }))
        .collect();
    assert_eq!(labels.len(), 1);
    if let GridContent::PartLabel { text } = &labels[0].content {
        assert_eq!(text, "Melody");
    } else {
        panic!("expected PartLabel");
    }
}

#[test]
fn horizontal_bar_variant_exists() {
    let _ = GridContent::HorizontalBar {
        from_column: 0,
        to_column: 12,
    };
}

#[test]
fn left_bar_line_emitted_at_start_of_first_system_line() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    // label_cols=2 (named single part), header_rows=2, directive row → part_row_base = 4
    let left_bars: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
        .collect();
    assert_eq!(
        left_bars.len(),
        1,
        "expected one left bar for a single system line"
    );
    assert_eq!(
        left_bars[0].position.row, 4,
        "left bar should be at part_row_base when directive row is present"
    );
}

#[test]
fn left_bar_line_emitted_for_each_system_line_on_wrap() {
    // First measure: 16 (notes) + 1 (bar) = 17 cols
    // Second measure: 16 + 1 = 17 cols; 17+17=34 > 28 → wraps → 2 system lines
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, 300.0, A4_HEIGHT);
    let left_bars: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
        .collect();
    assert_eq!(left_bars.len(), 2, "expected one left bar per system line");
}

#[test]
fn bottom_bar_emitted_at_end_of_system_line() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let bottom_bars: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
        .collect();
    assert_eq!(
        bottom_bars.len(),
        1,
        "expected one bottom bar for a single system line"
    );
    // effective_row_group_height = 4 + 1 directive row = 5; row = header_rows + 5 = 7
    assert_eq!(
        bottom_bars[0].position.row, 7,
        "bottom bar row should be current_row_offset + effective_row_group_height"
    );
    if let GridContent::HorizontalBar {
        from_column,
        to_column,
    } = &bottom_bars[0].content
    {
        assert_eq!(*from_column, 0);
        // 2 (left bar col) + 16 (notes) + 1 (end bar) + 1 = 20
        assert_eq!(
            *to_column, 20,
            "to_column should equal current_col at flush time"
        );
    } else {
        panic!("expected HorizontalBar");
    }
}

#[test]
fn bottom_bar_emitted_for_each_system_line_on_wrap() {
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, 300.0, A4_HEIGHT);
    let bottom_bars: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
        .collect();
    assert_eq!(
        bottom_bars.len(),
        2,
        "expected one bottom bar per system line"
    );
}

#[test]
fn left_bar_line_emitted_at_correct_column_for_named_parts() {
    // Named two-part score: label_cols = ceil(label_width / row_height) = ceil(40/24) = 2
    // Left bar at column=2, height_in_rows = 1 + (2-1)*4 = 5
    let score = make_two_part_score("1 2 3 4", "5 6 7 1");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let left_bars: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
        .collect();
    assert_eq!(
        left_bars.len(),
        1,
        "expected one left bar for named two-part score"
    );
    assert_eq!(
        left_bars[0].position.row, 4,
        "left bar should be at part_row_base when directive row is present"
    );
    if let GridContent::BarLine { height_in_rows } = &left_bars[0].content {
        assert_eq!(
            *height_in_rows, 5,
            "left bar height should be bar_height = row_group_height-1 = 6-1 = 5 (directive row is above the staff)"
        );
    } else {
        panic!("expected BarLine");
    }
}

#[test]
fn left_bar_line_has_correct_height_for_single_part() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let left_bars: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
        .collect();
    assert_eq!(left_bars.len(), 1);
    if let GridContent::BarLine { height_in_rows } = &left_bars[0].content {
        assert_eq!(
            *height_in_rows, 3,
            "single-part left bar height should be bar_height = row_group_height-1 = 4-1 = 3 (directive row is above the staff)"
        );
    } else {
        panic!("expected BarLine");
    }
}

#[test]
fn bar_number_emitted_at_start_of_each_row_group() {
    // First measure: 16 (notes) + 1 (bar) = 17 cols, fits in max_columns=28.
    // Second measure: 16 + 1 = 17 cols; 17+17=34 > 28 → wraps → two row groups.
    let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);

    let bar_numbers: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
        .collect();

    // One BarNumber per row group (2 row groups total)
    assert_eq!(bar_numbers.len(), 2, "expected one BarNumber per row group");

    // First row group: bar 1, at column 2 (label_cols=2), meta row with directive row = 3
    if let GridContent::BarNumber { number } = bar_numbers[0].content {
        assert_eq!(number, 1, "first row group must start at bar 1");
    }
    assert_eq!(bar_numbers[0].position.column, 2);
    assert_eq!(
        bar_numbers[0].position.row, 3,
        "row = header_rows + 1 with directive row"
    );
    assert_eq!(
        bar_numbers[0].horizontal_alignment,
        HorizontalAlignment::Left
    );
    assert_eq!(bar_numbers[0].vertical_alignment, VerticalAlignment::Bottom);

    // Second row group: bar 2, no directive row; row = header_rows + first effective height = 2 + 5 = 7
    if let GridContent::BarNumber { number } = bar_numbers[1].content {
        assert_eq!(number, 2, "second row group must start at bar 2");
    }
    assert_eq!(bar_numbers[1].position.column, 2);
    assert_eq!(
        bar_numbers[1].position.row, 7,
        "row = 2 + 5 after first row group with directive row"
    );
}

#[test]
fn bar_number_emitted_on_first_row_group_even_without_wrap() {
    // A single measure fits in one row group — no wrap occurs.
    // Bar number 1 should still be emitted at the start of that row group.
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);

    let bar_numbers: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
        .collect();

    assert_eq!(
        bar_numbers.len(),
        1,
        "expected one BarNumber for a single row group"
    );
    if let GridContent::BarNumber { number } = bar_numbers[0].content {
        assert_eq!(number, 1, "bar number must be 1 for the first row group");
    }
    assert_eq!(bar_numbers[0].position.column, 2);
    assert_eq!(
        bar_numbers[0].position.row, 3,
        "row = header_rows + 1 with directive row"
    );
    assert_eq!(
        bar_numbers[0].horizontal_alignment,
        HorizontalAlignment::Left
    );
    assert_eq!(bar_numbers[0].vertical_alignment, VerticalAlignment::Bottom);
}
