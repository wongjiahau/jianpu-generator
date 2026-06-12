use crate::layout::new_types::Page;

use super::{
    AbsoluteContent, AbsoluteElement, DominantBaseline, FontFamily, FontWeight, TextAnchor,
    PAGE_MARGIN,
};

pub(super) fn emit_header(
    page: &Page,
    row_height_pt: f32,
    base_font_size: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let page_width_pt = page.page_width_pt;
    let title_y = PAGE_MARGIN + row_height_pt * 0.75;
    out.push(AbsoluteElement {
        x: page_width_pt / 2.0,
        y: title_y,
        content: AbsoluteContent::Text {
            content: page.header.title.clone(),
            font_size: row_height_pt * 1.5,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    });

    let subtitle_author_y = PAGE_MARGIN + row_height_pt * 2.25;
    if let Some(subtitle) = &page.header.subtitle {
        out.push(AbsoluteElement {
            x: page_width_pt / 2.0,
            y: subtitle_author_y,
            content: AbsoluteContent::Text {
                content: subtitle.clone(),
                font_size: row_height_pt * 0.8,
                anchor: TextAnchor::Middle,
                baseline: DominantBaseline::Middle,
                font: FontFamily::SansSerif,
                weight: FontWeight::Normal,
                italic: true,
            },
        });
    }
    out.push(AbsoluteElement {
        x: page_width_pt - PAGE_MARGIN,
        y: subtitle_author_y,
        content: AbsoluteContent::Text {
            content: page.header.author.clone(),
            font_size: base_font_size,
            anchor: TextAnchor::End,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    });
}

pub(super) fn emit_footer(
    page: &Page,
    row_height_pt: f32,
    base_font_size: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let footer_y = page.page_height_pt - PAGE_MARGIN - row_height_pt * 0.5;
    out.push(AbsoluteElement {
        x: page.page_width_pt / 2.0,
        y: footer_y,
        content: AbsoluteContent::Text {
            content: format!("{}/{}", page.footer.page, page.footer.total),
            font_size: base_font_size,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    });
}
