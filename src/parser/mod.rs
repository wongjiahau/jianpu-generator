use crate::ast::parsed::ParsedDocument;
use crate::error::{JianPuError, Span};

pub mod lyrics;
pub mod metadata_parser;
pub mod score;
pub mod section_splitter;

pub fn parse(input: &str, filename: &str) -> Result<ParsedDocument, JianPuError> {
    use section_splitter::{split_sections, SectionKind};

    let sections = split_sections(input)?;
    let doc_span = Span::new(0, input.len());

    let mut raw_metadata: Option<(String, usize)> = None;
    let mut raw_score: Option<(String, usize)> = None;

    for section in sections {
        match section.kind {
            SectionKind::Metadata => {
                if raw_metadata.is_some() {
                    return Err(JianPuError::new(
                        doc_span.clone(),
                        "duplicate [metadata] section",
                    ));
                }
                raw_metadata = Some((section.content, section.content_offset));
            }
            SectionKind::Score => {
                if raw_score.is_some() {
                    return Err(JianPuError::new(
                        doc_span.clone(),
                        "duplicate [score] section",
                    ));
                }
                raw_score = Some((section.content, section.content_offset));
            }
        }
    }

    let (meta_content, meta_offset) = raw_metadata
        .ok_or_else(|| JianPuError::new(doc_span.clone(), "missing [metadata] section"))?;
    let (score_content, score_offset) =
        raw_score.ok_or_else(|| JianPuError::new(doc_span, "missing [score] section"))?;

    let metadata = metadata_parser::parse_metadata(&meta_content, meta_offset)?;
    let parts_decl = metadata.parts.clone();
    let parts = score::interleaved_parser::parse(&score_content, score_offset, &parts_decl)?;

    Ok(ParsedDocument {
        filename: filename.to_string(),
        metadata,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_document() {
        let input = concat!(
            "[metadata]\ntitle = \"hello world\"\nauthor = \"foo\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n你好wo rld\n"
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.metadata.title, "hello world");
        assert_eq!(doc.metadata.author, "foo");
        assert_eq!(doc.parts.len(), 1);
        // 3 directive events + 4 notes = 7
        assert_eq!(doc.parts[0].score.events.len(), 7);
        // 4 syllables
        assert_eq!(doc.parts[0].lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "[unknown]\nfoo\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_duplicate_score_section() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n\n",
            "[score]\n5 6 7 1\n",
        );
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_missing_metadata_section() {
        let input = "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn parses_two_named_parts() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 2);
        assert_eq!(doc.parts[0].name, Some("Soprano".to_string()));
        assert_eq!(doc.parts[1].name, Some("Alto".to_string()));
        assert!(doc.parts[0].lyrics.is_none());
        assert!(doc.parts[1].lyrics.is_none());
    }

    #[test]
    fn single_unnamed_part_remains_compatible() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n"
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 1);
        assert_eq!(doc.parts[0].name, None);
        assert!(doc.parts[0].lyrics.is_some());
    }
}
