use crate::ast::parsed::JianPuPitch;
use crate::compiler::types::{ColumnElement, ElementContent, MeasureRow, RowId};
use crate::grid_layout::types::Header;
use crate::grid_layout::types::{GridContent, GridRow, VAlign};
use crate::render_config::RenderConfig;

#[test]
fn column_width_pt_divides_evenly() {
    let row = GridRow {
        height_pt: 30.0,
        column_count: 10,
        elements: vec![],
    };
    assert_eq!(row.column_width_pt(500.0), 50.0);
}

#[test]
fn column_width_pt_with_label_columns() {
    // 4 label cols + 16 musical cols = 20 total; usable=400 → 20pt each
    let row = GridRow {
        height_pt: 30.0,
        column_count: 20,
        elements: vec![],
    };
    assert_eq!(row.column_width_pt(400.0), 20.0);
}

fn note_row(id: &str) -> MeasureRow {
    MeasureRow {
        id: RowId(id.to_string()),
        label: id.to_string(),
        elements: vec![ColumnElement {
            column: 0,
            content: ElementContent::NoteHead {
                pitch: JianPuPitch::One,
                octave: 0,
                dotted: false,
            },
        }],
    }
}

fn chord_row(id: &str) -> MeasureRow {
    MeasureRow {
        id: RowId(id.to_string()),
        label: id.to_string(),
        elements: vec![ColumnElement {
            column: 0,
            content: ElementContent::ChordSymbol("Am".to_string()),
        }],
    }
}

fn lyric_row(id: &str) -> MeasureRow {
    MeasureRow {
        id: RowId(id.to_string()),
        label: id.to_string(),
        elements: vec![ColumnElement {
            column: 0,
            content: ElementContent::Lyric("la".to_string()),
        }],
    }
}

use crate::compiler::types::{CompileResult, MeasureBlock};
use crate::grid_layout::layout::{
    chord_part_sub_row_heights, expand_system_to_rows, is_chord_only_row, is_lyric_row, layout,
    note_part_sub_row_heights, pack_into_systems,
};
use std::collections::HashMap;

fn make_block(row_id: &str, bar_col: u32) -> MeasureBlock {
    MeasureBlock {
        rows: vec![MeasureRow {
            id: RowId(row_id.to_string()),
            label: row_id.to_string(),
            elements: vec![
                ColumnElement {
                    column: 0,
                    content: ElementContent::NoteHead {
                        pitch: JianPuPitch::One,
                        octave: 0,
                        dotted: false,
                    },
                },
                ColumnElement {
                    column: bar_col,
                    content: ElementContent::BarLine,
                },
            ],
        }],
        decorations: vec![],
    }
}

fn cfg() -> RenderConfig {
    RenderConfig {
        row_height: 30,
        label_width: 0,
        note_number_width: 12,
        max_columns: 8,
    }
}

#[test]
fn is_lyric_row_detects_lyric() {
    assert!(is_lyric_row(&lyric_row("L")));
    assert!(!is_lyric_row(&note_row("S")));
}

#[test]
fn is_chord_only_row_detects_chord() {
    assert!(is_chord_only_row(&chord_row("C")));
    assert!(!is_chord_only_row(&note_row("S")));
    assert!(!is_chord_only_row(&lyric_row("L")));
}

#[test]
fn note_part_sub_row_heights_sums_correctly() {
    let heights = note_part_sub_row_heights(30.0);
    // arc + above_dot + note_head + below_dot + ul + ul
    // = 9.0 + 7.5 + 30.0 + 7.5 + 4.5 + 4.5 = 63.0
    let sum: f32 = heights.iter().sum();
    assert!((sum - 63.0).abs() < 0.001, "sum={sum}");
    assert_eq!(heights.len(), 6);
}

#[test]
fn chord_part_sub_row_heights_has_four_rows() {
    let heights = chord_part_sub_row_heights(30.0);
    assert_eq!(heights.len(), 4);
}

#[test]
fn single_block_is_one_system() {
    let blocks = vec![make_block("S", 3)]; // 4 columns
    let systems = pack_into_systems(&blocks, &cfg());
    assert_eq!(systems.len(), 1);
    assert_eq!(systems[0].len(), 1);
}

#[test]
fn blocks_exceeding_max_columns_split_into_two_systems() {
    // Each block is 4 cols wide; max=8 → fits 2 per system
    let blocks = vec![make_block("S", 3), make_block("S", 3), make_block("S", 3)];
    let systems = pack_into_systems(&blocks, &cfg());
    assert_eq!(systems.len(), 2);
    assert_eq!(systems[0].len(), 2);
    assert_eq!(systems[1].len(), 1);
}

#[test]
fn different_row_ids_start_new_system() {
    let blocks = vec![make_block("A", 3), make_block("B", 3)];
    let systems = pack_into_systems(&blocks, &cfg());
    assert_eq!(systems.len(), 2);
}

fn make_system_single_note_block() -> Vec<MeasureBlock> {
    vec![make_block("S", 3)] // 4 musical cols, bar at compiler col 3
}

#[test]
fn note_block_expands_to_six_sub_rows() {
    let rows = expand_system_to_rows(&make_system_single_note_block(), 30.0, &HashMap::new());
    // 1 note part × 6 sub-rows, no lyric
    assert_eq!(rows.len(), 6);
}

#[test]
fn note_head_element_is_in_sub_row_index_2() {
    let rows = expand_system_to_rows(&make_system_single_note_block(), 30.0, &HashMap::new());
    let note_row = &rows[2]; // note-head sub-row
    let has_note = note_row
        .elements
        .iter()
        .any(|e| matches!(e.content, GridContent::NoteHead { .. }));
    assert!(has_note, "note head should be in sub-row 2");
}

#[test]
fn bar_line_element_has_positive_height_pt() {
    let rows = expand_system_to_rows(&make_system_single_note_block(), 30.0, &HashMap::new());
    let bar = rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .find(|e| matches!(e.content, GridContent::BarLine { .. }));
    let bar = bar.expect("should have a BarLine element");
    if let GridContent::BarLine { height_pt } = bar.content {
        assert!(height_pt > 0.0, "height_pt={height_pt}");
    }
}

#[test]
fn row_label_is_in_note_head_sub_row_at_column_0_span_4() {
    let rows = expand_system_to_rows(&make_system_single_note_block(), 30.0, &HashMap::new());
    let note_row = &rows[2];
    let label = note_row
        .elements
        .iter()
        .find(|e| matches!(e.content, GridContent::RowLabel(_)));
    let label = label.expect("note-head row should have RowLabel");
    assert_eq!(label.column, 0);
    assert_eq!(label.column_span, 4);
}

#[test]
fn column_count_is_label_cols_plus_musical_cols() {
    let rows = expand_system_to_rows(&make_system_single_note_block(), 30.0, &HashMap::new());
    // 4 label cols + 4 musical cols (bar at col 3 → block width=4)
    assert_eq!(rows[0].column_count, 8);
}

// ── decoration row helpers ────────────────────────────────────────────────────

fn make_block_with_decorations(
    decorations: Vec<crate::compiler::types::Decoration>,
) -> MeasureBlock {
    use crate::compiler::types::MeasureBlock;
    MeasureBlock {
        rows: vec![MeasureRow {
            id: RowId("S".to_string()),
            label: "S".to_string(),
            elements: vec![
                ColumnElement {
                    column: 0,
                    content: ElementContent::NoteHead {
                        pitch: JianPuPitch::One,
                        octave: 0,
                        dotted: false,
                    },
                },
                ColumnElement {
                    column: 3,
                    content: ElementContent::BarLine,
                },
            ],
        }],
        decorations,
    }
}

// ── layout() tests ────────────────────────────────────────────────────────────

fn hdr() -> Header {
    Header {
        title: "Song".to_string(),
        subtitle: None,
        author: "Me".to_string(),
    }
}

fn cfg_wide() -> RenderConfig {
    RenderConfig {
        row_height: 30,
        label_width: 0,
        note_number_width: 12,
        max_columns: 48,
    }
}

#[test]
fn layout_single_block_produces_one_page() {
    let blocks = vec![make_block("S", 3)];
    let compile_result = CompileResult {
        blocks,
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    assert_eq!(pages.len(), 1);
}

#[test]
fn layout_page_has_correct_dimensions() {
    let blocks = vec![make_block("S", 3)];
    let compile_result = CompileResult {
        blocks,
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    assert!((pages[0].width_pt - 595.0).abs() < 0.001);
    assert!((pages[0].height_pt - 842.0).abs() < 0.001);
}

#[test]
fn layout_rows_include_header_and_footer() {
    let blocks = vec![make_block("S", 3)];
    let compile_result = CompileResult {
        blocks,
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    // At minimum: header title row, header subtitle+author row, footer row
    assert!(pages[0].rows.len() >= 3, "len={}", pages[0].rows.len());
}

#[test]
fn layout_page_total_height_does_not_exceed_page_height() {
    let blocks: Vec<_> = (0..10).map(|_| make_block("S", 3)).collect();
    let compile_result = CompileResult {
        blocks,
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    for page in &pages {
        let total: f32 = page.rows.iter().map(|r| r.height_pt).sum();
        assert!(
            total <= page.height_pt,
            "total={total} > page={}",
            page.height_pt
        );
    }
}

#[test]
fn layout_with_bpm_decoration_has_decoration_row() {
    use crate::compiler::types::{Decoration, MeasureBlock};
    let block = MeasureBlock {
        rows: vec![MeasureRow {
            id: RowId("S".to_string()),
            label: "S".to_string(),
            elements: vec![
                ColumnElement {
                    column: 0,
                    content: ElementContent::NoteHead {
                        pitch: JianPuPitch::One,
                        octave: 0,
                        dotted: false,
                    },
                },
                ColumnElement {
                    column: 3,
                    content: ElementContent::BarLine,
                },
            ],
        }],
        decorations: vec![Decoration::Bpm(120)],
    };
    let compile_result = CompileResult {
        blocks: vec![block],
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    let has_bpm = pages[0]
        .rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .any(|e| matches!(e.content, GridContent::Bpm(120)));
    assert!(has_bpm, "should have Bpm(120) element");
}

#[test]
fn decoration_row_has_fixed_column_count() {
    use crate::compiler::types::Decoration;
    let block = make_block_with_decorations(vec![Decoration::Bpm(120)]);
    let compile_result = CompileResult {
        blocks: vec![block],
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    let deco_row = pages[0]
        .rows
        .iter()
        .find(|r| {
            r.elements
                .iter()
                .any(|e| matches!(e.content, GridContent::Bpm(_)))
        })
        .expect("should have a decoration row with Bpm");
    assert_eq!(
        deco_row.column_count, 12,
        "decoration row should use fixed DECO_COLS=12"
    );
}

#[test]
fn decoration_items_start_at_column_1() {
    use crate::compiler::types::Decoration;
    let block = make_block_with_decorations(vec![Decoration::Bpm(120)]);
    let compile_result = CompileResult {
        blocks: vec![block],
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    let bpm_el = pages[0]
        .rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .find(|e| matches!(e.content, GridContent::Bpm(_)))
        .expect("should have Bpm element");
    assert_eq!(bpm_el.column, 1, "first decoration should be at column 1");
}

#[test]
fn section_label_ordered_before_bpm_regardless_of_declaration_order() {
    use crate::compiler::types::Decoration;
    // Bpm declared first — SectionLabel should still win column 1
    let block = make_block_with_decorations(vec![
        Decoration::Bpm(120),
        Decoration::SectionLabel("A".to_string()),
    ]);
    let compile_result = CompileResult {
        blocks: vec![block],
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    let section_col = pages[0]
        .rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .find(|e| matches!(e.content, GridContent::SectionLabel(_)))
        .expect("should have SectionLabel element")
        .column;
    let bpm_col = pages[0]
        .rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .find(|e| matches!(e.content, GridContent::Bpm(_)))
        .expect("should have Bpm element")
        .column;
    assert!(
        section_col < bpm_col,
        "SectionLabel (col {section_col}) should come before Bpm (col {bpm_col})"
    );
}

#[test]
fn multiple_decorations_occupy_consecutive_columns_starting_at_1() {
    use crate::compiler::types::Decoration;
    let block = make_block_with_decorations(vec![
        Decoration::Bpm(120),
        Decoration::TimeSignature {
            numerator: 4,
            denominator: 4,
        },
    ]);
    let compile_result = CompileResult {
        blocks: vec![block],
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    let bpm_col = pages[0]
        .rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .find(|e| matches!(e.content, GridContent::Bpm(_)))
        .expect("should have Bpm element")
        .column;
    let time_col = pages[0]
        .rows
        .iter()
        .flat_map(|r| r.elements.iter())
        .find(|e| matches!(e.content, GridContent::TimeSignature { .. }))
        .expect("should have TimeSignature element")
        .column;
    assert_eq!(bpm_col, 1, "Bpm should be at column 1");
    assert_eq!(time_col, 2, "TimeSignature should be at column 2");
}

#[test]
fn footer_row_fills_remaining_page_height() {
    let blocks = vec![make_block("S", 3)];
    let compile_result = CompileResult {
        blocks,
        slur_spans: vec![],
    };
    let page_height = 842.0_f32;
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, page_height);
    let page = &pages[0];
    let non_footer_height: f32 = page.rows[..page.rows.len() - 1]
        .iter()
        .map(|r| r.height_pt)
        .sum();
    let footer_height = page.rows.last().unwrap().height_pt;
    let expected = page_height - 2.0 * crate::grid_layout::PAGE_MARGIN - non_footer_height;
    assert!(
        (footer_height - expected).abs() < 0.001,
        "footer_height={footer_height} expected={expected}"
    );
}

#[test]
fn footer_element_valign_is_bottom() {
    let blocks = vec![make_block("S", 3)];
    let compile_result = CompileResult {
        blocks,
        slur_spans: vec![],
    };
    let pages = layout(&compile_result, &cfg_wide(), &hdr(), 595.0, 842.0);
    let footer_row = pages[0].rows.last().unwrap();
    assert!(
        footer_row
            .elements
            .iter()
            .all(|e| e.valign == VAlign::Bottom),
        "footer elements should be VAlign::Bottom"
    );
}
