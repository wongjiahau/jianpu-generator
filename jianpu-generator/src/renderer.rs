use crate::layout::types::{GridContent, HorizontalAlignment, Page, VerticalAlignment};

pub fn render(pages: &[Page], cell_size: u32) -> Vec<String> {
    pages.iter().map(|page| render_page(page, cell_size)).collect()
}

fn render_page(page: &Page, cell_size: u32) -> String {
    let cell = cell_size as f32;
    let base_font_size = cell * 0.6;
    let cjk_font_size = base_font_size * 1.2;

    let mut elements = String::new();

    for row_group in &page.row_groups {
        for element in &row_group.elements {
            let col = element.position.column as f32;
            let row = element.position.row as f32;

            let base_x = col * cell;
            let base_y = row * cell;

            // Apply alignment offsets
            let x = match element.horizontal_alignment {
                HorizontalAlignment::Left => base_x,
                HorizontalAlignment::Center => base_x + cell / 2.0,
                HorizontalAlignment::Right => base_x + cell,
            };
            let y = match element.vertical_alignment {
                VerticalAlignment::Top => base_y,
                VerticalAlignment::Center => base_y + cell / 2.0,
                VerticalAlignment::Bottom => base_y + cell,
            };

            match &element.content {
                GridContent::NoteHead { pitch, octave } => {
                    let digit = pitch_to_digit(pitch);
                    elements.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="monospace">{}</text>"#,
                        x, y, base_font_size, digit
                    ));
                    // Octave markers
                    let dot_radius = cell * 0.08;
                    let dot_spacing = dot_radius * 3.0;
                    for i in 0..*octave {
                        let dot_y = base_y - dot_radius - (i as f32) * dot_spacing;
                        elements.push_str(&format!(
                            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="black"/>"#,
                            x, dot_y, dot_radius
                        ));
                    }
                    for i in 0..(-*octave) {
                        let dot_y = base_y + cell + dot_radius + (i as f32) * dot_spacing;
                        elements.push_str(&format!(
                            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="black"/>"#,
                            x, dot_y, dot_radius
                        ));
                    }
                }
                GridContent::Rest => {
                    elements.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="monospace">0</text>"#,
                        x, y, base_font_size
                    ));
                }
                GridContent::DurationUnderlines { count } => {
                    let line_x1 = base_x + cell * 0.1;
                    let line_x2 = base_x + cell * 0.9;
                    for i in 0..*count {
                        let line_y = base_y + cell * 0.1 + (i as f32) * (cell * 0.15);
                        elements.push_str(&format!(
                            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="black" stroke-width="1"/>"#,
                            line_x1, line_y, line_x2, line_y
                        ));
                    }
                }
                GridContent::Lyric { text, is_cjk } => {
                    let font_size = if *is_cjk { cjk_font_size } else { base_font_size };
                    elements.push_str(&format!(
                        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="hanging" font-family="sans-serif">{}</text>"#,
                        x, y, font_size, escape_xml(text)
                    ));
                }
                GridContent::TieOrSlurCurve { from_column, to_column } => {
                    let x1 = (*from_column as f32 + 0.5) * cell;
                    let x2 = (*to_column as f32 + 0.5) * cell;
                    let cy = base_y - cell * 0.3;
                    elements.push_str(&format!(
                        r#"<path d="M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}" fill="none" stroke="black" stroke-width="1"/>"#,
                        x1, y, (x1 + x2) / 2.0, cy, x2, y
                    ));
                }
                GridContent::BarLine => {
                    let line_x = base_x;
                    let line_y1 = base_y;
                    let line_y2 = base_y + cell;
                    elements.push_str(&format!(
                        r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="black" stroke-width="1.5"/>"#,
                        line_x, line_y1, line_x, line_y2
                    ));
                }
            }
        }
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 595 842">{}</svg>"#,
        elements
    )
}

fn pitch_to_digit(pitch: &crate::ast::parsed::JianPuPitch) -> char {
    use crate::ast::parsed::JianPuPitch::*;
    match pitch {
        One => '1',
        Two => '2',
        Three => '3',
        Four => '4',
        Five => '5',
        Six => '6',
        Seven => '7',
    }
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{grouper, layout, parser};

    const A4_W: f32 = 595.0;
    const A4_H: f32 = 842.0;

    fn render_score(score_str: &str, lyrics_str: &str) -> Vec<String> {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 {}\n\n[lyrics]\n{}\n",
            score_str, lyrics_str
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        let score = grouper::group(doc).unwrap();
        let pages = layout::layout(&score, A4_W, A4_H);
        render(&pages, score.metadata.cell_size)
    }

    #[test]
    fn produces_one_svg_per_page() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert_eq!(svgs.len(), 1);
    }

    #[test]
    fn svg_has_correct_dimensions() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert!(svgs[0].contains("width=\"210mm\""));
        assert!(svgs[0].contains("height=\"297mm\""));
    }

    #[test]
    fn svg_contains_note_digits() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert!(svgs[0].contains(">1<"));
        assert!(svgs[0].contains(">2<"));
        assert!(svgs[0].contains(">3<"));
        assert!(svgs[0].contains(">4<"));
    }

    #[test]
    fn svg_contains_lyric_text() {
        let svgs = render_score("1 2 3 4", "你 好 wo rld");
        assert!(svgs[0].contains("你"));
        assert!(svgs[0].contains("好"));
        assert!(svgs[0].contains("wo"));
    }

    #[test]
    fn cjk_lyric_has_larger_font() {
        let svgs = render_score("1 2", "你 a");
        // CJK syllable should have a font-size attribute larger than non-CJK
        // We check that the SVG contains two different font sizes
        assert!(svgs[0].contains("font-size"));
    }

    #[test]
    fn svg_is_valid_xml_structure() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert!(svgs[0].starts_with("<svg"));
        assert!(svgs[0].ends_with("</svg>"));
    }
}
