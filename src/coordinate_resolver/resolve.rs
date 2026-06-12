use crate::compositor::types::{
    AbsoluteContent, AbsoluteElement, AbsolutePage, DominantBaseline, FontFamily, FontWeight,
    TextAnchor,
};
use crate::grid_layout::types::{GridContent, GridPage, HAlign, VAlign};
use crate::grid_layout::PAGE_MARGIN;

pub fn resolve(pages: &[GridPage], note_number_width: f32) -> Vec<AbsolutePage> {
    pages
        .iter()
        .map(|page| resolve_page(page, note_number_width))
        .collect()
}

fn resolve_page(page: &GridPage, note_number_width: f32) -> AbsolutePage {
    let usable_width = page.width_pt - 2.0 * PAGE_MARGIN;
    let mut elements: Vec<AbsoluteElement> = Vec::new();
    let mut row_y = PAGE_MARGIN;

    for row in &page.rows {
        let col_width = row.column_width_pt(usable_width);
        for el in &row.elements {
            let x_start = PAGE_MARGIN + el.column as f32 * col_width;
            let span_width = el.column_span as f32 * col_width;
            let x = match el.halign {
                HAlign::Start => x_start,
                HAlign::Center => x_start + span_width * 0.5,
                HAlign::End => x_start + span_width,
            };
            let y = match el.valign {
                VAlign::Top => row_y,
                VAlign::Center => row_y + row.height_pt * 0.5,
                VAlign::Bottom => row_y + row.height_pt,
            };
            // Beam underlines span from the left edge of the first note character
            // to the right edge of the last note character so adjacent beat groups
            // have a natural gap (the inter-note spacing minus the note width).
            if let GridContent::Underline { level } = &el.content {
                let note_center_x = x_start + col_width * 0.5;
                let ul_x = note_center_x - note_number_width * 0.5;
                let ul_width = (el.column_span as f32 - 1.0) * col_width + note_number_width;
                elements.push(AbsoluteElement {
                    x: ul_x,
                    y,
                    content: AbsoluteContent::Underline {
                        width: ul_width,
                        level: *level,
                    },
                });
                continue;
            }
            if let Some(content) = grid_to_absolute(&el.content, span_width, el.halign) {
                elements.push(AbsoluteElement { x, y, content });
            }
        }
        row_y += row.height_pt;
    }

    AbsolutePage {
        width_pt: page.width_pt,
        height_pt: page.height_pt,
        elements,
    }
}

fn text_anchor(halign: HAlign) -> TextAnchor {
    match halign {
        HAlign::Start => TextAnchor::Start,
        HAlign::Center => TextAnchor::Middle,
        HAlign::End => TextAnchor::End,
    }
}

#[allow(clippy::too_many_lines)]
fn grid_to_absolute(
    content: &GridContent,
    span_width: f32,
    halign: HAlign,
) -> Option<AbsoluteContent> {
    match content {
        GridContent::NoteHead {
            pitch,
            octave,
            dotted,
        } => Some(AbsoluteContent::NoteHead {
            pitch: pitch.clone(),
            octave: *octave,
            dotted: *dotted,
        }),
        GridContent::Rest { dotted } => Some(AbsoluteContent::Rest { dotted: *dotted }),
        GridContent::NoteDash => Some(AbsoluteContent::Text {
            content: "\u{2014}".to_string(),
            font_size: 12.0,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::Monospace,
            weight: FontWeight::Normal,
            italic: false,
        }),
        GridContent::OctaveDot => None,
        GridContent::ChordSymbol(s) => Some(AbsoluteContent::ChordSymbol(s.clone())),
        GridContent::Underline { level } => Some(AbsoluteContent::Underline {
            width: span_width,
            level: *level,
        }),
        GridContent::TieOrSlur => Some(AbsoluteContent::TieOrSlur { width: span_width }),
        GridContent::TieOrSlurClose => Some(AbsoluteContent::TieOrSlur {
            width: span_width * 0.5,
        }),
        GridContent::BarLine { height_pt } => Some(AbsoluteContent::BarLine { height: *height_pt }),
        GridContent::HorizontalLine => Some(AbsoluteContent::HorizontalLine { width: span_width }),
        GridContent::RowLabel(s) => Some(AbsoluteContent::Text {
            content: s.clone(),
            font_size: 12.0,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        }),
        GridContent::LyricSyllable(s) => Some(AbsoluteContent::Lyric(s.clone())),
        GridContent::Bpm(bpm) => Some(AbsoluteContent::Text {
            content: format!("\u{2669}={bpm}"),
            font_size: 12.0,
            anchor: TextAnchor::Start,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        }),
        GridContent::TimeSignature {
            numerator,
            denominator,
        } => Some(AbsoluteContent::Text {
            content: format!("{numerator}/{denominator}"),
            font_size: 12.0,
            anchor: TextAnchor::Start,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        }),
        GridContent::SectionLabel(s) => Some(AbsoluteContent::Text {
            content: s.clone(),
            font_size: 12.0,
            anchor: TextAnchor::Start,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Bold,
            italic: true,
        }),
        GridContent::BarNumber(n) => Some(AbsoluteContent::Text {
            content: n.to_string(),
            font_size: 10.0,
            anchor: TextAnchor::Start,
            baseline: DominantBaseline::Ideographic,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        }),
        GridContent::Text {
            content,
            font_size,
            bold,
            italic,
        } => Some(AbsoluteContent::Text {
            content: content.clone(),
            font_size: *font_size,
            anchor: text_anchor(halign),
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: if *bold {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            },
            italic: *italic,
        }),
    }
}
