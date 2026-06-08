use crate::layout::types::{
    GridContent, GridElement, HorizontalAlignment, Page, VerticalAlignment,
};

/// Must match PAGE_MARGIN in layout/mod.rs — padding applied on every edge.
const PAGE_MARGIN: f32 = 25.0;

pub fn render(pages: &[Page], row_height: u32, note_number_width: u32) -> Vec<String> {
    pages
        .iter()
        .map(|page| render_page(page, row_height, note_number_width))
        .collect()
}

struct PageRenderContext {
    row_height: f32,
    note_number_width: f32,
    base_font_size: f32,
    cjk_font_size: f32,
    page_width: f32,
    page_height: f32,
    usable_width: f32,
}

impl PageRenderContext {
    fn new(page: &Page, row_height: u32, note_number_width: u32) -> Self {
        let row_height = row_height as f32;
        let note_number_width = note_number_width as f32;
        let page_width = page.page_width_pt;
        Self {
            row_height,
            note_number_width,
            base_font_size: row_height * 0.6,
            cjk_font_size: row_height * 0.6 * 1.2,
            page_width,
            page_height: 842.0,
            usable_width: page_width - 2.0 * PAGE_MARGIN,
        }
    }
}

fn render_page(page: &Page, row_height: u32, note_number_width: u32) -> String {
    let ctx = PageRenderContext::new(page, row_height, note_number_width);
    let mut elements = String::new();
    render_page_header(page, &ctx, &mut elements);
    render_row_groups(page, &ctx, &mut elements);
    render_page_footer(page, &ctx, &mut elements);

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 595 842">{elements}</svg>"#
    )
}

fn render_page_header(page: &Page, ctx: &PageRenderContext, elements: &mut String) {
    let title_y = PAGE_MARGIN + ctx.row_height * 0.75;
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
        ctx.page_width / 2.0,
        title_y,
        ctx.row_height * 1.5,
        escape_xml(&page.header.title)
    ));

    let subtitle_author_y = PAGE_MARGIN + ctx.row_height * 2.25;
    if let Some(subtitle) = &page.header.subtitle {
        elements.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
            ctx.page_width / 2.0,
            subtitle_author_y,
            ctx.base_font_size,
            escape_xml(subtitle)
        ));
    }
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="end" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
        ctx.page_width - PAGE_MARGIN,
        subtitle_author_y,
        ctx.base_font_size,
        escape_xml(&page.header.author)
    ));
}

fn render_row_groups(page: &Page, ctx: &PageRenderContext, elements: &mut String) {
    for row_group in &page.row_groups {
        let column_width = ctx.usable_width / row_group.width_in_columns as f32;
        for element in row_group.elements.iter() {
            render_grid_element(element, column_width, ctx, elements);
        }
    }
}

fn render_page_footer(page: &Page, ctx: &PageRenderContext, elements: &mut String) {
    let footer_y = ctx.page_height - PAGE_MARGIN - ctx.row_height * 0.5;
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif">{}/{}</text>"#,
        ctx.page_width / 2.0,
        footer_y,
        ctx.row_height * 0.75,
        page.footer.page,
        page.footer.total
    ));
}

#[derive(Copy, Clone)]
struct ElementCoords {
    x: f32,
    y: f32,
    base_x: f32,
    base_y: f32,
}

fn element_position(element: &GridElement, column_width: f32, row_height: f32) -> ElementCoords {
    let col = element.position.column as f32;
    let row = element.position.row as f32;
    let base_x = col * column_width + PAGE_MARGIN;
    let base_y = PAGE_MARGIN + row * row_height;

    let x = match element.horizontal_alignment {
        HorizontalAlignment::Left => base_x,
        HorizontalAlignment::Center => base_x + column_width / 2.0,
        HorizontalAlignment::Right => base_x + column_width,
    };
    let y = match element.vertical_alignment {
        VerticalAlignment::Top => base_y,
        VerticalAlignment::Center => base_y + row_height / 2.0,
        VerticalAlignment::Bottom => base_y + row_height,
    };

    ElementCoords {
        x,
        y,
        base_x,
        base_y,
    }
}

fn render_grid_element(
    element: &GridElement,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let coords = element_position(element, column_width, ctx.row_height);
    render_grid_content(&element.content, coords, column_width, ctx, elements);
}

fn render_grid_content(
    content: &GridContent,
    coords: ElementCoords,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let ElementCoords {
        x,
        y,
        base_x,
        base_y,
    } = coords;

    match content {
        GridContent::NoteHead {
            pitch,
            octave,
            dotted,
        } => render_note_head(pitch, *octave, *dotted, coords, column_width, ctx, elements),
        GridContent::Rest => render_rest(x, y, ctx, elements),
        GridContent::DurationUnderlines { levels } => {
            render_duration_underlines(levels, base_y, column_width, ctx, elements);
        }
        GridContent::LowerOctaveDots {
            count,
            underline_count,
        } => render_lower_octave_dots(*count, *underline_count, x, base_y, ctx, elements),
        GridContent::Lyric { text, is_cjk } => render_lyric(text, *is_cjk, x, y, ctx, elements),
        GridContent::TieOrSlurCurve {
            from_column,
            to_column,
        } => render_tie_or_slur_curve(
            *from_column,
            *to_column,
            y,
            base_y,
            column_width,
            ctx,
            elements,
        ),
        GridContent::Extension => render_extension(x, y, ctx, elements),
        GridContent::BarLine { height_in_rows } => {
            render_bar_line(x, base_y, *height_in_rows, ctx, elements);
        }
        GridContent::TimeSignatureLabel {
            numerator,
            denominator,
        } => render_time_signature_label(
            numerator,
            denominator,
            base_x,
            y,
            column_width,
            ctx,
            elements,
        ),
        GridContent::BpmLabel { bpm } => {
            render_bpm_label(*bpm, base_x, y, column_width, ctx, elements);
        }
        GridContent::PartLabel { text } => render_part_label(text, x, y, ctx, elements),
        GridContent::HorizontalBar {
            from_column,
            to_column,
        } => render_horizontal_bar(*from_column, *to_column, base_y, column_width, elements),
        GridContent::BarNumber { number } => render_bar_number(*number, x, y, ctx, elements),
        GridContent::SectionLabel { text } => render_section_label(text, x, y, ctx, elements),
        GridContent::ChordSymbol { text } => render_chord_symbol(text, x, y, ctx, elements),
    }
}

fn render_rest(x: f32, y: f32, ctx: &PageRenderContext, elements: &mut String) {
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="monospace">0</text>"#,
        x, y, ctx.base_font_size
    ));
}

fn render_lyric(
    text: &str,
    is_cjk: bool,
    x: f32,
    y: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let font_size = if is_cjk {
        ctx.cjk_font_size
    } else {
        ctx.base_font_size
    };
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="hanging" font-family="sans-serif">{}</text>"#,
        x, y, font_size, escape_xml(text)
    ));
}

fn render_extension(x: f32, y: f32, ctx: &PageRenderContext, elements: &mut String) {
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="monospace">-</text>"#,
        x, y, ctx.base_font_size
    ));
}

fn render_bar_line(
    x: f32,
    base_y: f32,
    height_in_rows: u32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let line_y2 = base_y + height_in_rows as f32 * ctx.row_height;
    elements.push_str(&format!(
        r#"<line x1="{x:.1}" y1="{base_y:.1}" x2="{x:.1}" y2="{line_y2:.1}" stroke="black" stroke-width="0.5"/>"#
    ));
}

fn render_part_label(text: &str, x: f32, y: f32, ctx: &PageRenderContext, elements: &mut String) {
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="start" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
        x, y, ctx.base_font_size * 0.8, escape_xml(text)
    ));
}

fn render_bar_number(number: u32, x: f32, y: f32, ctx: &PageRenderContext, elements: &mut String) {
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="start" dominant-baseline="ideographic" font-family="sans-serif">{}</text>"#,
        x, y, ctx.base_font_size * 0.6, number
    ));
}

fn render_section_label(
    text: &str,
    x: f32,
    y: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="start" dominant-baseline="ideographic" font-style="italic" font-weight="bold" font-family="sans-serif">{}</text>"#,
        x, y, ctx.base_font_size * 1.2, escape_xml(text)
    ));
}

fn render_chord_symbol(text: &str, x: f32, y: f32, ctx: &PageRenderContext, elements: &mut String) {
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="start" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
        x, y, ctx.base_font_size * 0.75, escape_xml(text)
    ));
}

fn render_note_head(
    pitch: &crate::ast::parsed::JianPuPitch,
    octave: i8,
    dotted: bool,
    coords: ElementCoords,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let ElementCoords { x, y, base_y, .. } = coords;
    let digit = pitch_to_digit(pitch);
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="monospace">{}</text>"#,
        x, y, ctx.base_font_size, digit
    ));
    if dotted {
        let dot_radius = ctx.row_height * 0.06;
        let dot_x = x + column_width * 0.5;
        elements.push_str(&format!(
            r#"<circle cx="{dot_x:.1}" cy="{y:.1}" r="{dot_radius:.1}" fill="black"/>"#
        ));
    }
    let dot_radius = ctx.row_height * 0.08;
    let dot_spacing = dot_radius * 3.0;
    for i in 0..octave {
        let dot_y = base_y - dot_radius - (i as f32) * dot_spacing;
        elements.push_str(&format!(
            r#"<circle cx="{x:.1}" cy="{dot_y:.1}" r="{dot_radius:.1}" fill="black"/>"#
        ));
    }
}

fn render_duration_underlines(
    levels: &[crate::layout::types::UnderlineSpan],
    base_y: f32,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    for (i, span) in levels.iter().enumerate() {
        let line_x1 = span.from_column as f32 * column_width + column_width * 0.1 + PAGE_MARGIN;
        let line_x2 = span.last_head_column as f32 * column_width
            + column_width * 0.5
            + ctx.note_number_width * 0.5
            + PAGE_MARGIN;
        let line_y = base_y + ctx.row_height * 0.1 + (i as f32) * (ctx.row_height * 0.15);
        elements.push_str(&format!(
            r#"<line x1="{line_x1:.1}" y1="{line_y:.1}" x2="{line_x2:.1}" y2="{line_y:.1}" stroke="black" stroke-width="1"/>"#
        ));
    }
}

fn render_lower_octave_dots(
    count: u32,
    underline_count: u8,
    x: f32,
    base_y: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let dot_radius = ctx.row_height * 0.08;
    for i in 0..count {
        let slot = underline_count as f32 + i as f32;
        let dot_y = base_y + ctx.row_height * 0.1 + slot * (ctx.row_height * 0.15);
        elements.push_str(&format!(
            r#"<circle cx="{x:.1}" cy="{dot_y:.1}" r="{dot_radius:.1}" fill="black"/>"#
        ));
    }
}

fn render_tie_or_slur_curve(
    from_column: u32,
    to_column: u32,
    y: f32,
    base_y: f32,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let x1 = (from_column as f32 + 0.5) * column_width + PAGE_MARGIN;
    let x2 = (to_column as f32 + 0.5) * column_width + PAGE_MARGIN;
    let cy = base_y - ctx.row_height * 0.3;
    elements.push_str(&format!(
        r#"<path d="M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}" fill="none" stroke="black" stroke-width="1"/>"#,
        x1, y, (x1 + x2) / 2.0, cy, x2, y
    ));
}

fn render_time_signature_label(
    numerator: &u8,
    denominator: &u8,
    base_x: f32,
    y: f32,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let slot_width = 2.0 * column_width;
    let center_x = base_x + slot_width / 2.0;
    let numerator_y = y - ctx.row_height * 0.25;
    let rule_y = y;
    let denominator_y = y + ctx.row_height * 0.25;
    let rule_x1 = base_x + slot_width * 0.2;
    let rule_x2 = base_x + slot_width * 0.8;
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
        center_x, numerator_y, ctx.base_font_size, numerator
    ));
    elements.push_str(&format!(
        r#"<line x1="{rule_x1:.1}" y1="{rule_y:.1}" x2="{rule_x2:.1}" y2="{rule_y:.1}" stroke="black" stroke-width="1"/>"#
    ));
    elements.push_str(&format!(
        r#"<text x="{:.1}" y="{:.1}" font-size="{:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif">{}</text>"#,
        center_x, denominator_y, ctx.base_font_size, denominator
    ));
}

fn render_bpm_label(
    bpm: u32,
    base_x: f32,
    y: f32,
    column_width: f32,
    ctx: &PageRenderContext,
    elements: &mut String,
) {
    let slot_width = 2.0 * column_width;
    let center_x = base_x + slot_width / 2.0;
    let small_font_size = ctx.base_font_size * 0.6;
    elements.push_str(&format!(
        r#"<text x="{center_x:.1}" y="{y:.1}" font-size="{small_font_size:.1}" text-anchor="middle" dominant-baseline="middle" font-family="sans-serif">♩={bpm}</text>"#
    ));
}

fn render_horizontal_bar(
    from_column: u32,
    to_column: u32,
    base_y: f32,
    column_width: f32,
    elements: &mut String,
) {
    let x1 = from_column as f32 * column_width + PAGE_MARGIN;
    let x2 = to_column as f32 * column_width + PAGE_MARGIN;
    elements.push_str(&format!(
        r#"<line x1="{x1:.1}" y1="{base_y:.1}" x2="{x2:.1}" y2="{base_y:.1}" stroke="black" stroke-width="0.35"/>"#
    ));
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{score_str}\n{lyrics_str}\n"
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        let score = grouper::group(doc).unwrap();
        let pages = layout::layout(&score, A4_W, A4_H);
        render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        )
    }

    #[test]
    fn section_label_renders_in_svg() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\na b c d\n",
        );
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        assert!(
            svgs[0].contains("Verse 1"),
            "expected section label 'Verse 1' in SVG"
        );
        assert!(
            svgs[0].contains("font-style=\"italic\""),
            "expected italic style on section label"
        );
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
        let svgs = render_score("1 2 3 4", "你 a b c");
        let svg = &svgs[0];
        let font_size_14 = svg.contains("font-size=\"14.4\"");
        let font_size_17 = svg.contains("font-size=\"17.3\"");
        assert!(
            font_size_14 && font_size_17,
            "Expected both base (14.4) and CJK (17.3) font sizes in SVG, got: {}",
            &svg[..svg.len().min(500)]
        );
    }

    #[test]
    fn svg_is_valid_xml_structure() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert!(svgs[0].starts_with("<svg"));
        assert!(svgs[0].ends_with("</svg>"));
    }

    #[test]
    fn lower_octave_note_renders_dot_below_note() {
        let svgs = render_score("1. 2 3 4", "a b c d");
        assert!(
            svgs[0].contains(r#"cy="123.4""#),
            "1-beat lower-octave dot must be at slot 0 (cy=123.4)"
        );
    }

    #[test]
    fn quarter_beat_lower_octave_dot_is_below_two_underlines() {
        let score_str = "=1. =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1";
        let lyrics_str = "a b c d e f g h i j k l m n o p";
        let svgs = render_score(score_str, lyrics_str);
        assert!(
            svgs[0].contains(r#"cy="130.6""#),
            "quarter-beat lower-octave dot must be at slot 2 (cy=130.6)"
        );
    }

    #[test]
    fn svg_contains_title_and_author() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert!(svgs[0].contains(">t<"), "expected title 't' in SVG");
        assert!(svgs[0].contains(">a<"), "expected author 'a' in SVG");
    }

    #[test]
    fn svg_contains_page_number() {
        let svgs = render_score("1 2 3 4", "a b c d");
        assert!(svgs[0].contains("1/1"), "expected page number '1/1' in SVG");
    }

    #[test]
    fn time_signature_label_renders_numerator_and_denominator_text() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=2/4 key=C4 bpm=120)\n3 5\na b\n",
        );
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        let svg = &svgs[0];
        assert!(
            svg.contains(">2<"),
            "expected numerator 2 in SVG for 2/4 time signature"
        );
        assert!(
            svg.contains(">4<"),
            "expected denominator 4 in SVG for 2/4 time signature"
        );
    }

    #[test]
    fn bpm_label_renders_beats_per_minute_text() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=75)\n1 2 3 4\na b c d\n",
        );
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        let svg = &svgs[0];
        assert!(
            svg.contains("♩=75"),
            "expected BPM label text '♩=75' in SVG output"
        );
    }

    #[test]
    fn multi_part_svg_contains_both_part_labels() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano lyrics:Soprano notes:Alto lyrics:Alto\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n5 6 7 1\ne f g h\n",
        );
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        assert!(
            svgs[0].contains("Soprano"),
            "expected 'Soprano' label in SVG"
        );
        assert!(svgs[0].contains("Alto"), "expected 'Alto' label in SVG");
    }

    #[test]
    fn horizontal_bar_renders_horizontal_line() {
        use crate::layout::types::*;
        use nonempty::nonempty;
        let page = Page {
            header: Header {
                title: "t".to_string(),
                subtitle: None,
                author: "a".to_string(),
            },
            footer: Footer { page: 1, total: 1 },
            page_width_pt: A4_W,
            row_groups: vec![RowGroup {
                height_in_rows: 4,
                width_in_columns: 16,
                elements: nonempty![GridElement {
                    position: GridPosition { column: 0, row: 6 },
                    horizontal_alignment: HorizontalAlignment::Left,
                    vertical_alignment: VerticalAlignment::Top,
                    content: GridContent::HorizontalBar {
                        from_column: 0,
                        to_column: 16
                    },
                }],
            }],
        };
        let svgs = render(&[page], 24, 8);
        assert!(
            svgs[0].contains(r#"x1="25.0" y1="169.0" x2="570.0" y2="169.0""#),
            "expected horizontal line at y=169.0 spanning full content width;\nSVG snippet: {}",
            &svgs[0][..svgs[0].len().min(800)]
        );
    }

    #[test]
    fn section_label_escapes_xml_special_chars() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120 label=\"A&B\")\n1 2 3 4\na b c d\n",
        );
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        assert!(
            svgs[0].contains("A&amp;B"),
            "expected XML-escaped label in SVG"
        );
        assert!(!svgs[0].contains("A&B\""), "expected raw & to be escaped");
    }

    #[test]
    fn bar_number_renders_as_small_text_above_left_bar() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
        );
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        let svg = &svgs[0];
        assert!(
            svg.contains(">1<") || svg.contains(">1 <"),
            "expected bar number 1 in SVG output"
        );
        assert!(
            svg.contains(">2<") || svg.contains(">2 <"),
            "expected bar number 2 in SVG output"
        );
        assert!(
            svg.contains("font-size=\"8.6\""),
            "expected bar number font-size 8.6 in SVG; snippet: {}",
            &svg[..svg.len().min(800)]
        );
    }

    #[test]
    fn chord_symbol_renders_as_svg_text() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = chord: notes:\n\n[score]\n(time=4/4 key=C4 bpm=120)\n1m7 - 4 5\n1 - 1 1\n";
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        assert!(
            svgs[0].contains("1m⁷"),
            "expected rendered chord symbol '1m⁷' in SVG"
        );
    }

    #[test]
    fn chord_symbol_with_sharp_renders_unicode() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = chord: notes:\n\n[score]\n(time=4/4 key=C4 bpm=120)\n1# - - -\n1 - - -\n";
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = crate::layout::layout(&score, A4_W, A4_H);
        let svgs = render(
            &pages,
            score.metadata.row_height,
            score.metadata.note_number_width,
        );
        assert!(svgs[0].contains("1♯"), "expected '1♯' in SVG");
    }
}
