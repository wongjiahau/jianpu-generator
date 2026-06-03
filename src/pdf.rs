use crate::error::{JianPuError, Span};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
use std::collections::HashMap;

pub fn write_pdf(svgs: &[String]) -> Result<Vec<u8>, JianPuError> {
    if svgs.is_empty() {
        return Ok(Vec::new());
    }

    let mut options = svg2pdf::usvg::Options::default();
    {
        let db = options.fontdb_mut();
        db.load_font_data(include_bytes!("../fonts/SourceHanSansSC-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../fonts/SourceHanSansTC-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../fonts/NotoSansMono-Regular.ttf").to_vec());
        db.set_sans_serif_family("Source Han Sans SC");
        db.set_monospace_family("Noto Sans Mono");
    }

    let conversion_options = svg2pdf::ConversionOptions::default();
    let mut alloc = Ref::new(1);

    let catalog_id = alloc.bump();
    let page_tree_id = alloc.bump();

    let num_pages = svgs.len();
    let page_ids: Vec<Ref> = (0..num_pages).map(|_| alloc.bump()).collect();
    let content_ids: Vec<Ref> = (0..num_pages).map(|_| alloc.bump()).collect();

    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id)
        .count(num_pages as i32)
        .kids(page_ids.iter().copied());

    let svg_name = Name(b"Svg");

    for (i, svg_str) in svgs.iter().enumerate() {
        let tree = svg2pdf::usvg::Tree::from_str(svg_str, &options).map_err(|e| {
            JianPuError::new(Span::new(0, 0), format!("SVG parse error: {}", e))
        })?;

        let page_width = tree.size().width();
        let page_height = tree.size().height();

        let (svg_chunk, svg_ref) =
            svg2pdf::to_chunk(&tree, conversion_options).map_err(|e| {
                JianPuError::new(
                    Span::new(0, 0),
                    format!("SVG to PDF chunk failed: {}", e),
                )
            })?;

        // Renumber the chunk's internal refs so they don't conflict with our allocator.
        let mut map = HashMap::new();
        let svg_chunk = svg_chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_ref_new = map[&svg_ref];

        pdf.extend(&svg_chunk);

        // Content stream: scale the 1×1 XObject to fill the page.
        let mut content = Content::new();
        content.transform([page_width, 0.0, 0.0, page_height, 0.0, 0.0]);
        content.x_object(svg_name);
        let content_bytes = content.finish();

        pdf.stream(content_ids[i], &content_bytes).finish();

        let mut page = pdf.page(page_ids[i]);
        page.media_box(Rect::new(0.0, 0.0, page_width, page_height));
        page.parent(page_tree_id);
        page.contents(content_ids[i]);
        let mut resources = page.resources();
        resources.x_objects().pair(svg_name, svg_ref_new);
        resources.finish();
        page.finish();
    }

    Ok(pdf.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{grouper, layout, parser, renderer};

    fn make_pdf(score_str: &str, lyrics_str: &str) -> Vec<u8> {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 {}\n\n[lyrics]\n{}\n",
            score_str, lyrics_str
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        let score = grouper::group(doc).unwrap();
        let cell_size = score.metadata.cell_size;
        let pages = layout::layout(&score, 595.0, 842.0);
        let svgs = renderer::render(&pages, cell_size);
        write_pdf(&svgs).unwrap()
    }

    #[test]
    fn produces_non_empty_pdf_bytes() {
        let bytes = make_pdf("1 2 3 4", "a b c d");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn pdf_starts_with_pdf_header() {
        let bytes = make_pdf("1 2 3 4", "a b c d");
        assert!(bytes.starts_with(b"%PDF"));
    }
}
