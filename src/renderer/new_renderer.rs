use crate::ast::parsed::JianPuPitch;
use crate::compositor::types::{
    AbsoluteContent, AbsoluteElement, AbsolutePage, DominantBaseline, FontFamily, FontWeight,
    TextAnchor,
};
use crate::render_config::RenderConfig;
use crate::renderer::new_types::{SvgDocument, SvgElement, SvgKind};

pub fn render_new(pages: &[AbsolutePage], config: &RenderConfig) -> Vec<SvgDocument> {
    pages.iter().map(|page| render_page(page, config)).collect()
}

fn render_page(page: &AbsolutePage, config: &RenderConfig) -> SvgDocument {
    let row_height = config.row_height as f32;
    let base_font_size = row_height * 0.6;
    let cjk_font_size = base_font_size * 1.2;
    let note_number_width = config.note_number_width as f32;

    let elements = page
        .elements
        .iter()
        .flat_map(|elem| {
            render_element(
                elem,
                &row_height,
                &base_font_size,
                &cjk_font_size,
                &note_number_width,
            )
        })
        .collect();

    SvgDocument {
        width_pt: page.width_pt,
        height_pt: page.height_pt,
        elements,
    }
}

fn render_element(
    elem: &AbsoluteElement,
    row_height: &f32,
    base_font_size: &f32,
    cjk_font_size: &f32,
    note_number_width: &f32,
) -> Vec<SvgElement> {
    match &elem.content {
        AbsoluteContent::NoteHead {
            pitch,
            octave,
            dotted,
        } => render_note_head(
            elem,
            pitch,
            *octave,
            *dotted,
            row_height,
            base_font_size,
            note_number_width,
        ),
        AbsoluteContent::Rest { dotted } => {
            render_rest(elem, *dotted, row_height, base_font_size, note_number_width)
        }
        AbsoluteContent::ChordSymbol(s) => render_chord_symbol(elem, s, base_font_size),
        AbsoluteContent::Underline { width, level: _ } => render_underline(elem, width),
        AbsoluteContent::TieOrSlur { width } => render_tie_or_slur(elem, width, row_height),
        AbsoluteContent::BarLine { height } => render_bar_line(elem, height),
        AbsoluteContent::HorizontalLine { width } => render_horizontal_line(elem, width),
        AbsoluteContent::Lyric(s) => render_lyric(elem, s, base_font_size, cjk_font_size),
        AbsoluteContent::Text {
            content,
            font_size,
            anchor,
            baseline,
            font,
            weight,
            italic,
        } => {
            vec![SvgElement {
                x: elem.x,
                y: elem.y,
                variant: "text",
                kind: SvgKind::Text {
                    content: content.clone(),
                    font_size: *font_size,
                    anchor: *anchor,
                    baseline: *baseline,
                    font: *font,
                    weight: *weight,
                    italic: *italic,
                },
            }]
        }
    }
}

fn render_note_head(
    elem: &AbsoluteElement,
    pitch: &JianPuPitch,
    octave: i8,
    dotted: bool,
    row_height: &f32,
    base_font_size: &f32,
    note_number_width: &f32,
) -> Vec<SvgElement> {
    let mut results = Vec::new();

    // 1. Note digit
    results.push(SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "note-head",
        kind: SvgKind::Text {
            content: pitch_to_digit(pitch).to_string(),
            font_size: *base_font_size,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::Monospace,
            weight: FontWeight::Normal,
            italic: false,
        },
    });

    // 2. Dotted note: circle to the right
    if dotted {
        let dot_radius = row_height * 0.06;
        let dot_x = elem.x + note_number_width * 1.5;
        results.push(SvgElement {
            x: dot_x,
            y: elem.y,
            variant: "note-head",
            kind: SvgKind::Circle { r: dot_radius },
        });
    }

    // 3. Upper octave dots (octave > 0): circles above
    if octave > 0 {
        let dot_radius = row_height * 0.08;
        let dot_spacing = dot_radius * 3.0;
        let gap = dot_radius * 2.0;
        for i in 0..octave {
            let dot_y = elem.y - base_font_size / 2.0 - dot_radius - gap - (i as f32) * dot_spacing;
            results.push(SvgElement {
                x: elem.x,
                y: dot_y,
                variant: "note-head",
                kind: SvgKind::Circle { r: dot_radius },
            });
        }
    }

    // 4. Lower octave dots (octave < 0): circles below
    if octave < 0 {
        let dot_radius = row_height * 0.08;
        let dot_spacing = dot_radius * 3.0;
        for i in 0..(-octave) {
            let dot_y = elem.y + base_font_size / 2.0 + dot_radius + (i as f32) * dot_spacing;
            results.push(SvgElement {
                x: elem.x,
                y: dot_y,
                variant: "note-head",
                kind: SvgKind::Circle { r: dot_radius },
            });
        }
    }

    results
}

fn render_rest(
    elem: &AbsoluteElement,
    dotted: bool,
    row_height: &f32,
    base_font_size: &f32,
    note_number_width: &f32,
) -> Vec<SvgElement> {
    let mut results = Vec::new();

    // "0" text
    results.push(SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "rest",
        kind: SvgKind::Text {
            content: "0".to_string(),
            font_size: *base_font_size,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::Monospace,
            weight: FontWeight::Normal,
            italic: false,
        },
    });

    // Optional dot
    if dotted {
        let dot_radius = row_height * 0.06;
        let dot_x = elem.x + note_number_width * 1.5;
        results.push(SvgElement {
            x: dot_x,
            y: elem.y,
            variant: "rest",
            kind: SvgKind::Circle { r: dot_radius },
        });
    }

    results
}

fn render_chord_symbol(elem: &AbsoluteElement, s: &str, base_font_size: &f32) -> Vec<SvgElement> {
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "chord-symbol",
        kind: SvgKind::Text {
            content: s.to_string(),
            font_size: *base_font_size,
            anchor: TextAnchor::Start,
            baseline: DominantBaseline::Middle,
            font: FontFamily::Monospace,
            weight: FontWeight::Normal,
            italic: false,
        },
    }]
}

fn render_horizontal_line(elem: &AbsoluteElement, width: &f32) -> Vec<SvgElement> {
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "horizontal-line",
        kind: SvgKind::Line {
            x2: elem.x + width,
            y2: elem.y,
            stroke_width: 0.5,
        },
    }]
}

fn render_underline(elem: &AbsoluteElement, width: &f32) -> Vec<SvgElement> {
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "underline",
        kind: SvgKind::Line {
            x2: elem.x + width,
            y2: elem.y,
            stroke_width: 1.0,
        },
    }]
}

fn render_tie_or_slur(elem: &AbsoluteElement, width: &f32, row_height: &f32) -> Vec<SvgElement> {
    let cx = elem.x + width / 2.0;
    let cy = elem.y - row_height * 0.3;
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "tie-or-slur",
        kind: SvgKind::Path {
            control_x: cx,
            control_y: cy,
            end_x: elem.x + width,
            end_y: elem.y,
            stroke_width: 1.0,
        },
    }]
}

fn render_bar_line(elem: &AbsoluteElement, height: &f32) -> Vec<SvgElement> {
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "bar-line",
        kind: SvgKind::Line {
            x2: elem.x,
            y2: elem.y + height,
            stroke_width: 0.5,
        },
    }]
}

fn render_lyric(
    elem: &AbsoluteElement,
    s: &str,
    base_font_size: &f32,
    cjk_font_size: &f32,
) -> Vec<SvgElement> {
    let is_cjk = s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c));
    let font_size = if is_cjk {
        *cjk_font_size
    } else {
        *base_font_size
    };
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: "lyric",
        kind: SvgKind::Text {
            content: s.to_string(),
            font_size,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Hanging,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    }]
}

fn pitch_to_digit(pitch: &JianPuPitch) -> char {
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
