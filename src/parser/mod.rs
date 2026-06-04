use crate::ast::parsed::{ParsedDocument, ParsedLyrics, ParsedPart, ParsedScore};
use crate::error::{JianPuError, Span};

pub mod lyrics;
pub mod metadata_parser;
pub mod score;
pub mod section_splitter;

fn section_label(kind: &str, name: &Option<String>) -> String {
    format!("[{}{}]", kind, name.as_deref().map(|n| format!(":{}", n)).unwrap_or_default())
}

pub fn parse(input: &str, filename: &str) -> Result<ParsedDocument, JianPuError> {
    use section_splitter::{split_sections, SectionKind};

    let sections = split_sections(input)?;

    let mut raw_metadata: Option<(String, usize)> = None;
    let mut raw_scores: Vec<(Option<String>, String, usize)> = Vec::new();
    let mut raw_lyrics: Vec<(Option<String>, String)> = Vec::new();

    let doc_span = Span::new(0, input.len());

    for section in sections {
        match section.kind {
            SectionKind::Metadata => {
                if raw_metadata.is_some() {
                    return Err(JianPuError::new(doc_span.clone(), "duplicate [metadata] section"));
                }
                raw_metadata = Some((section.content, section.content_offset));
            }
            SectionKind::Score { name } => {
                if raw_scores.iter().any(|(n, _, _)| n == &name) {
                    return Err(JianPuError::new(
                        doc_span.clone(),
                        format!("duplicate {} section", section_label("score", &name)),
                    ));
                }
                raw_scores.push((name, section.content, section.content_offset));
            }
            SectionKind::Lyrics { name } => {
                if raw_lyrics.iter().any(|(n, _)| n == &name) {
                    return Err(JianPuError::new(
                        doc_span.clone(),
                        format!("duplicate {} section", section_label("lyrics", &name)),
                    ));
                }
                // Orphan check: lyrics name must match a score name
                if !raw_scores.iter().any(|(n, _, _)| n == &name) {
                    return Err(JianPuError::new(
                        doc_span.clone(),
                        format!(
                            "orphan {} section: no matching {} found",
                            section_label("lyrics", &name),
                            section_label("score", &name),
                        ),
                    ));
                }
                raw_lyrics.push((name, section.content));
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

        let lyrics = raw_lyrics
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, content)| ParsedLyrics {
                syllables: lyrics::tokenizer::tokenize_lyrics(content),
            });

        parts.push(ParsedPart {
            name,
            score: ParsedScore { events },
            lyrics,
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
        "[score]\nbpm=120 1=C4 4/4 1 2 _3 _4\n\n",
        "[lyrics]\n你好wo rld\n"
    );

    #[test]
    fn parses_full_document() {
        let doc = parse(SAMPLE, "test.jianpu").unwrap();
        assert_eq!(doc.metadata.title, "hello world");
        assert_eq!(doc.metadata.author, "foo");
        assert_eq!(doc.parts.len(), 1);
        // 4/4 time sig + bpm + key + 4 notes = 7 events
        assert_eq!(doc.parts[0].score.events.len(), 7);
        // 4 syllables: 你 好 wo rld
        assert_eq!(doc.parts[0].lyrics.as_ref().unwrap().syllables.len(), 4);
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

    #[test]
    fn parses_two_named_parts() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n",
            "[score:Soprano]\n4/4 1 2 3 4\n",
            "[lyrics:Soprano]\na b c d\n",
            "[score:Alto]\n5 6 7 1\n",
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 2);
        assert_eq!(doc.parts[0].name, Some("Soprano".to_string()));
        assert_eq!(doc.parts[1].name, Some("Alto".to_string()));
        assert!(doc.parts[0].lyrics.is_some());
        assert!(doc.parts[1].lyrics.is_none());
    }

    #[test]
    fn single_unnamed_part_remains_compatible() {
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4\n\n[lyrics]\na b c d\n";
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 1);
        assert_eq!(doc.parts[0].name, None);
        assert!(doc.parts[0].lyrics.is_some());
    }

    #[test]
    fn rejects_orphan_lyrics_section() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n",
            "[score]\n4/4 1 2 3 4\n",
            "[lyrics:Alto]\na b c d\n",
        );
        let err = parse(input, "test.jianpu").unwrap_err();
        assert!(err.message.contains("orphan"), "expected orphan error, got: {}", err.message);
    }

    #[test]
    fn rejects_duplicate_score_section_by_name() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n",
            "[score:S]\n4/4 1 2 3 4\n",
            "[score:S]\n4/4 5 6 7 1\n",
        );
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_duplicate_lyrics_section_by_name() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n",
            "[score:S]\n4/4 1 2 3 4\n",
            "[lyrics:S]\na b c d\n",
            "[lyrics:S]\ne f g h\n",
        );
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_directive_in_non_first_part() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n",
            "[score:Soprano]\n4/4 1 2 3 4\n",
            "[score:Alto]\nbpm=90 5 6 7 1\n",
        );
        assert!(parse(input, "test.jianpu").is_err());
    }
}
