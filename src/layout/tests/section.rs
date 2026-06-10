use super::*;

#[test]
fn section_label_renders_below_directive_row_when_both_present() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n",
    );
    let pages = parse_and_layout(input);
    let all: Vec<_> = pages[0].row_groups[0].elements.iter().collect();
    let time_row = all
        .iter()
        .find(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
        .unwrap()
        .position
        .row;
    let label_row = all
        .iter()
        .find(|e| matches!(&e.content, GridContent::SectionLabel { text } if text == "Verse 1"))
        .unwrap()
        .position
        .row;
    assert!(
        label_row > time_row,
        "section label must be below directive row"
    );
}

#[test]
fn section_label_element_emitted_at_correct_position() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n",
    );
    let pages = parse_and_layout(input);
    let all_elements: Vec<_> = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let label_el = all_elements
        .iter()
        .find(|e| matches!(&e.content, GridContent::SectionLabel { text } if text == "Verse 1"));
    assert!(label_el.is_some(), "expected SectionLabel element");
    let el = label_el.unwrap();
    assert_eq!(el.horizontal_alignment, HorizontalAlignment::Left);
    assert_eq!(el.vertical_alignment, VerticalAlignment::Bottom);
    let has_bar_number = all_elements
        .iter()
        .any(|e| matches!(&e.content, GridContent::BarNumber { .. }));
    assert!(
        !has_bar_number,
        "bar number must be suppressed when section label is present"
    );
}

#[test]
fn chord_row_emits_chord_symbol_element() {
    use crate::layout::types::GridContent;
    let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nchord = chord\nMelody = notes\n\n[score]\n(time=4/4 key=C4 bpm=120)\n1 - - -\n1---\n";
    let doc = crate::parser::parse(input, "test.jianpu").unwrap();
    let score = crate::grouper::group(doc).unwrap();
    let pages = layout(&score, 595.0, 842.0);
    let all_elements: Vec<_> = pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let chord_symbols: Vec<_> = all_elements
        .iter()
        .filter(|e| matches!(e.content, GridContent::ChordSymbol { .. }))
        .collect();
    assert!(
        !chord_symbols.is_empty(),
        "expected at least one ChordSymbol element"
    );
    if let GridContent::ChordSymbol { text } = &chord_symbols[0].content {
        assert_eq!(text, "1");
    }
}

#[test]
fn no_section_label_when_not_declared() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
    );
    let pages = parse_and_layout(input);
    let has_label = pages[0]
        .row_groups
        .iter()
        .flat_map(|rg| rg.elements.iter())
        .any(|e| matches!(&e.content, GridContent::SectionLabel { .. }));
    assert!(!has_label, "expected no SectionLabel element");
}
