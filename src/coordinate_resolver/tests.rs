use crate::ast::parsed::JianPuPitch;
use crate::compositor::types::AbsoluteContent;
use crate::coordinate_resolver::resolve::resolve;
use crate::grid_layout::types::{GridContent, GridElement, GridPage, GridRow, HAlign, VAlign};

fn single_row_page(element: GridElement) -> GridPage {
    GridPage {
        width_pt: 595.0,
        height_pt: 842.0,
        rows: vec![GridRow {
            height_pt: 30.0,
            column_count: 10,
            elements: vec![element],
        }],
    }
}

#[test]
fn resolve_empty_pages_returns_empty() {
    assert!(resolve(&[], 12.0).is_empty());
}

#[test]
fn note_head_halign_center_has_x_at_center_of_column() {
    // usable = 595 - 50 = 545, col_width = 545/10 = 54.5
    // column=0, halign=Center → x = 25 + 0*54.5 + 54.5*0.5 = 52.25
    let el = GridElement {
        column: 0,
        column_span: 1,
        halign: HAlign::Center,
        valign: VAlign::Center,
        content: GridContent::NoteHead {
            pitch: JianPuPitch::One,
            octave: 0,
            dotted: false,
        },
    };
    let page = single_row_page(el);
    let abs = resolve(&[page], 12.0);
    let note = abs[0]
        .elements
        .iter()
        .find(|e| matches!(e.content, AbsoluteContent::NoteHead { .. }))
        .expect("should have NoteHead");
    let col_width = (595.0 - 50.0) / 10.0; // 54.5
    let expected_x = 25.0 + 0.0 * col_width + col_width * 0.5;
    assert!(
        (note.x - expected_x).abs() < 0.01,
        "x={} expected={expected_x}",
        note.x
    );
}

#[test]
fn valign_top_places_y_at_row_top() {
    let el = GridElement {
        column: 0,
        column_span: 1,
        halign: HAlign::Start,
        valign: VAlign::Top,
        content: GridContent::HorizontalLine,
    };
    let page = GridPage {
        width_pt: 595.0,
        height_pt: 842.0,
        rows: vec![
            GridRow {
                height_pt: 10.0,
                column_count: 1,
                elements: vec![],
            },
            GridRow {
                height_pt: 20.0,
                column_count: 1,
                elements: vec![el],
            },
        ],
    };
    let abs = resolve(&[page], 12.0);
    let line = abs[0]
        .elements
        .iter()
        .find(|e| matches!(e.content, AbsoluteContent::HorizontalLine { .. }))
        .expect("should have HorizontalLine");
    // row_y = PAGE_MARGIN + 10.0 = 35.0; VAlign::Top → y = row_y
    assert!((line.y - 35.0).abs() < 0.01, "y={}", line.y);
}

#[test]
fn halign_end_places_x_at_right_of_column_span() {
    let el = GridElement {
        column: 0,
        column_span: 1,
        halign: HAlign::End,
        valign: VAlign::Center,
        content: GridContent::Text {
            content: "Author".to_string(),
            font_size: 12.0,
            bold: false,
            italic: false,
        },
    };
    let page = single_row_page(el);
    let abs = resolve(&[page], 12.0);
    let text = abs[0]
        .elements
        .iter()
        .find(
            |e| matches!(&e.content, AbsoluteContent::Text { content, .. } if content == "Author"),
        )
        .expect("should have Text");
    let col_width = (595.0 - 50.0) / 10.0;
    let expected_x = 25.0 + col_width; // Start + 1*col_width = 25 + 54.5
    assert!(
        (text.x - expected_x).abs() < 0.01,
        "x={} expected={expected_x}",
        text.x
    );
}

#[test]
fn octave_dot_grid_content_emits_nothing() {
    let el = GridElement {
        column: 0,
        column_span: 1,
        halign: HAlign::Center,
        valign: VAlign::Center,
        content: GridContent::OctaveDot,
    };
    let page = single_row_page(el);
    let abs = resolve(&[page], 12.0);
    assert!(
        abs[0].elements.is_empty(),
        "OctaveDot should emit no AbsoluteElement"
    );
}
