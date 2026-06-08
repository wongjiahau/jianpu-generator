use crate::ast::parsed::ParsedDocument;
use crate::error::{JianPuError, Span};

pub mod lyrics;
pub mod metadata_parser;
pub mod parts_parser;
pub mod score;
pub mod section_splitter;

pub fn parse(input: &str, filename: &str) -> Result<ParsedDocument, JianPuError> {
    use section_splitter::{split_sections, SectionKind};

    let sections = split_sections(input)?;
    let doc_span = Span::new(0, input.len());

    let mut raw_metadata: Option<(String, usize)> = None;
    let mut raw_parts: Option<(String, usize)> = None;
    let mut raw_score: Option<(String, usize)> = None;

    for section in sections {
        match section.kind {
            SectionKind::Metadata => {
                if raw_metadata.is_some() {
                    return Err(JianPuError::new(doc_span, "duplicate [metadata] section"));
                }
                raw_metadata = Some((section.content, section.content_offset));
            }
            SectionKind::Parts => {
                if raw_parts.is_some() {
                    return Err(JianPuError::new(doc_span, "duplicate [parts] section"));
                }
                raw_parts = Some((section.content, section.content_offset));
            }
            SectionKind::Score => {
                if raw_score.is_some() {
                    return Err(JianPuError::new(doc_span, "duplicate [score] section"));
                }
                raw_score = Some((section.content, section.content_offset));
            }
        }
    }

    let (meta_content, meta_offset) = raw_metadata
        .ok_or_else(|| JianPuError::new(doc_span.clone(), "missing [metadata] section"))?;
    let (parts_content, parts_offset) =
        raw_parts.ok_or_else(|| JianPuError::new(doc_span.clone(), "missing [parts] section"))?;
    let (score_content, score_offset) =
        raw_score.ok_or_else(|| JianPuError::new(doc_span, "missing [score] section"))?;

    let metadata = metadata_parser::parse_metadata(&meta_content, meta_offset)?;
    let declarations = parts_parser::parse_parts(&parts_content, parts_offset)?;
    let tracks = score::interleaved_parser::parse(&score_content, score_offset, &declarations)?;

    Ok(ParsedDocument {
        filename: filename.to_string(),
        metadata,
        declarations,
        tracks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::{ParsedNotesTrack, ParsedTrack};

    fn notes_track(doc: &ParsedDocument) -> &ParsedNotesTrack {
        doc.tracks
            .iter()
            .find_map(|t| match t {
                ParsedTrack::Notes(n) => Some(n),
                ParsedTrack::Chord(_) => None,
            })
            .expect("expected a notes track")
    }

    #[test]
    fn parses_full_document() {
        let input = concat!(
            "[metadata]\ntitle = \"hello world\"\nauthor = \"foo\"\n\n",
            "[parts]\nMelody = notes lyrics\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n你好wo rld\n"
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.metadata.title, "hello world");
        assert_eq!(doc.metadata.author, "foo");
        assert_eq!(doc.declarations.len(), 1);
        assert_eq!(doc.tracks.len(), 1);
        let notes = notes_track(&doc);
        assert_eq!(notes.score.events.len(), 7);
        assert_eq!(notes.lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "[unknown]\nfoo\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_duplicate_score_section() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n",
            "[parts]\nMelody = notes\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n\n",
            "[score]\n5 6 7 1\n",
        );
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_missing_metadata_section() {
        let input = concat!(
            "[parts]\nMelody = notes\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n"
        );
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn parses_two_named_parts() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n",
            "[parts]\nSoprano = notes\nAlto = notes\n\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.tracks.len(), 2);
        let soprano = doc
            .tracks
            .iter()
            .find_map(|t| match t {
                ParsedTrack::Notes(n) if n.abbreviation == "Soprano" => Some(n),
                _ => None,
            })
            .unwrap();
        let alto = doc
            .tracks
            .iter()
            .find_map(|t| match t {
                ParsedTrack::Notes(n) if n.abbreviation == "Alto" => Some(n),
                _ => None,
            })
            .unwrap();
        assert!(soprano.lyrics.is_none());
        assert!(alto.lyrics.is_none());
    }

    #[test]
    fn single_unnamed_part_remains_compatible() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n",
            "[parts]\nMelody = notes lyrics\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n"
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.tracks.len(), 1);
        let notes = notes_track(&doc);
        assert_eq!(notes.abbreviation, "Melody");
        assert!(notes.lyrics.is_some());
    }
}
