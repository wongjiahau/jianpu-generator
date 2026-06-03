use crate::ast::parsed::ParsedDocument;
use crate::error::JianPuError;

pub mod lyrics;
pub mod metadata_parser;
pub mod score;
pub mod section_splitter;

pub fn parse(input: &str, filename: &str) -> Result<ParsedDocument, JianPuError> {
    use crate::error::Span;
    use section_splitter::{split_sections, SectionKind};

    let sections = split_sections(input)?;

    let mut raw_metadata: Option<(String, usize)> = None;
    let mut raw_score: Option<(String, usize)> = None;
    let mut raw_lyrics: Option<(String, usize)> = None;

    for section in sections {
        match section.kind {
            SectionKind::Metadata => {
                if raw_metadata.is_some() {
                    return Err(JianPuError::new(Span::new(0, input.len()), "duplicate [metadata] section"));
                }
                raw_metadata = Some((section.content, section.content_offset));
            }
            SectionKind::Score => {
                if raw_score.is_some() {
                    return Err(JianPuError::new(Span::new(0, input.len()), "duplicate [score] section"));
                }
                raw_score = Some((section.content, section.content_offset));
            }
            SectionKind::Lyrics => {
                if raw_lyrics.is_some() {
                    return Err(JianPuError::new(Span::new(0, input.len()), "duplicate [lyrics] section"));
                }
                raw_lyrics = Some((section.content, section.content_offset));
            }
        }
    }

    let doc_span = Span::new(0, input.len());

    let (meta_content, meta_offset) = raw_metadata.ok_or_else(|| {
        JianPuError::new(doc_span.clone(), "missing [metadata] section")
    })?;
    let (score_content, score_offset) = raw_score.ok_or_else(|| {
        JianPuError::new(doc_span.clone(), "missing [score] section")
    })?;
    let (lyrics_content, _) = raw_lyrics.ok_or_else(|| {
        JianPuError::new(doc_span, "missing [lyrics] section")
    })?;

    let metadata = metadata_parser::parse_metadata(&meta_content, meta_offset)?;

    let tokens = score::tokenizer::tokenize(&score_content, score_offset);
    let score_events = score::token_parser::parse_tokens(tokens)?;

    let lyrics = lyrics::tokenizer::tokenize_lyrics(&lyrics_content);

    Ok(ParsedDocument {
        filename: filename.to_string(),
        metadata,
        score_events,
        lyrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[metadata]
title = "hello world"
author = "foo"

[score]
bpm=120 1=C4 4/4 1 2 _3 _4

[lyrics]
你好wo rld
"#;

    #[test]
    fn parses_full_document() {
        let doc = parse(SAMPLE, "test.jianpu").unwrap();
        assert_eq!(doc.metadata.title, "hello world");
        assert_eq!(doc.metadata.author, "foo");
        // 4/4 time sig + bpm + key + 4 notes = 7 events
        assert_eq!(doc.score_events.len(), 7);
        // 4 syllables: 你 好 wo rld
        assert_eq!(doc.lyrics.len(), 4);
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "[unknown]\nfoo\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_duplicate_score_section() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n[score]\n1 2 3 4\n[score]\n5 6 7 1\n[lyrics]\na\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_missing_metadata_section() {
        let input = "[score]\n4/4 1 2 3 4\n[lyrics]\na b c d\n";
        assert!(parse(input, "test.jianpu").is_err());
    }
}
