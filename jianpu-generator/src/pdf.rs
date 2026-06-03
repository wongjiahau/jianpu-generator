use crate::error::{JianPuError, Span};

pub fn write_pdf(svgs: &[String]) -> Result<Vec<u8>, JianPuError> {
    if svgs.is_empty() {
        return Ok(Vec::new());
    }

    if svgs.len() > 1 {
        eprintln!(
            "warning: multi-page scores are not yet fully supported; outputting page 1 of {}",
            svgs.len()
        );
    }

    let svg = &svgs[0];
    let mut options = svg2pdf::usvg::Options::default();
    {
        let db = options.fontdb_mut();
        db.load_font_data(include_bytes!("../fonts/SourceHanSansSC-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../fonts/SourceHanSansTC-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../fonts/NotoSansMono-Regular.ttf").to_vec());
        db.set_sans_serif_family("Source Han Sans SC");
        db.set_monospace_family("Noto Sans Mono");
    }
    let tree = svg2pdf::usvg::Tree::from_str(svg, &options).map_err(|e| {
        JianPuError::new(Span::new(0, 0), format!("SVG parse error: {}", e))
    })?;

    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions::default())
        .map_err(|e| {
            JianPuError::new(
                Span::new(0, 0),
                format!("SVG to PDF conversion failed: {}", e),
            )
        })
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
