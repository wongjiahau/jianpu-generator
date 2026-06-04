use crate::ast::parsed::{ParsedDocument, ParsedPart, ParsedScore};
use crate::error::{JianPuError, Span};

pub mod lyrics;
pub mod metadata_parser;
pub mod score;
pub mod section_splitter;

pub fn parse(input: &str, filename: &str) -> Result<ParsedDocument, JianPuError> {
    use section_splitter::{split_sections, SectionKind};

    let sections = split_sections(input)?;

    let mut raw_metadata: Option<(String, usize)> = None;
    let mut raw_scores: Vec<(Option<String>, String, usize)> = Vec::new();

    let doc_span = Span::new(0, input.len());

    for section in sections {
        match section.kind {
            SectionKind::Metadata => {
                if raw_metadata.is_some() {
                    return Err(JianPuError::new(doc_span.clone(), "duplicate [metadata] section"));
                }
                raw_metadata = Some((section.content, section.content_offset));
            }
            SectionKind::Score => {
                raw_scores.push((None, section.content, section.content_offset));
            }
        }
    }

    let (meta_content, meta_offset) = raw_metadata
        .ok_or_else(|| JianPuError::new(doc_span.clone(), "missing [metadata] section"))?;

    if raw_scores.is_empty() {
        return Err(JianPuError::new(doc_span, "missing [score] section"));
    }

    let metadata = metadata_parser::parse_metadata(&meta_content, meta_offset)?;

    let mut parts = Vec::new();
    for (i, (name, score_content, score_offset)) in raw_scores.into_iter().enumerate() {
        let tokens = score::tokenizer::tokenize(&score_content, score_offset);
        let events = score::token_parser::parse_tokens(tokens)?;

        // Directives are only allowed in the first part
        if i > 0 {
            use crate::ast::parsed::ScoreEvent;
            for spanned in &events {
                match &spanned.value {
                    ScoreEvent::BpmChange(_)
                    | ScoreEvent::KeyChange(_)
                    | ScoreEvent::TimeSignatureChange { .. } => {
                        return Err(JianPuError::new(
                            spanned.span.clone(),
                            "directives (bpm, key, time signature) are only allowed in the first part's score section".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
        }

        parts.push(ParsedPart {
            name,
            score: ParsedScore { events },
            lyrics: None,
        });
    }

    Ok(ParsedDocument {
        filename: filename.to_string(),
        metadata,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        "[metadata]\ntitle = \"hello world\"\nauthor = \"foo\"\n\n",
        "[score]\nbpm=120 1=C4 4/4 1 2 _3 _4\n"
    );

    #[test]
    fn parses_full_document() {
        let doc = parse(SAMPLE, "test.jianpu").unwrap();
        assert_eq!(doc.metadata.title, "hello world");
        assert_eq!(doc.metadata.author, "foo");
        assert_eq!(doc.parts.len(), 1);
        // 4/4 time sig + bpm + key + 4 notes = 7 events
        assert_eq!(doc.parts[0].score.events.len(), 7);
        assert!(doc.parts[0].lyrics.is_none());
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "[unknown]\nfoo\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_duplicate_score_section() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n[score]\n1 2 3 4\n[score]\n5 6 7 1\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_missing_metadata_section() {
        let input = "[score]\n4/4 1 2 3 4\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_named_score_section() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n[score:Soprano]\n4/4 1 2 3 4\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn single_unnamed_part() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4\n";
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 1);
        assert_eq!(doc.parts[0].name, None);
        assert!(doc.parts[0].lyrics.is_none());
    }
}
