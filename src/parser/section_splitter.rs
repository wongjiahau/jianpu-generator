use crate::error::{JianPuError, Span};

#[derive(Clone)]
pub struct RawSection {
    pub kind: SectionKind,
    pub content: String,
    /// Byte offset in the original source where this section's content begins.
    pub content_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SectionKind {
    Metadata,
    Parts,
    Score,
}

pub fn split_sections(input: &str) -> Result<Vec<RawSection>, JianPuError> {
    let mut sections: Vec<RawSection> = Vec::new();
    let mut current_kind: Option<SectionKind> = None;
    let mut current_content = String::new();
    let mut current_content_offset: usize = 0;
    let mut byte_offset: usize = 0;

    for line in input.lines() {
        let line_len = line.len() + 1; // +1 for '\n'

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(kind) = current_kind.take() {
                sections.push(RawSection {
                    kind,
                    content: current_content.clone(),
                    content_offset: current_content_offset,
                });
                current_content.clear();
            }
            let kind_str = &line[1..line.len() - 1];
            current_kind = Some(match kind_str {
                "metadata" => SectionKind::Metadata,
                "parts" => SectionKind::Parts,
                "score" => SectionKind::Score,
                _ => {
                    return Err(JianPuError::new(
                        Span::new(byte_offset, byte_offset + line.len()),
                        format!("unknown section: [{kind_str}]"),
                    ))
                }
            });
            current_content_offset = byte_offset + line_len;
        } else if current_kind.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }

        byte_offset += line_len;
    }

    if let Some(kind) = current_kind {
        sections.push(RawSection {
            kind,
            content: current_content,
            content_offset: current_content_offset,
        });
    }

    validate_section_order(&sections)
}

fn validate_section_order(sections: &[RawSection]) -> Result<Vec<RawSection>, JianPuError> {
    let expected = [
        SectionKind::Metadata,
        SectionKind::Parts,
        SectionKind::Score,
    ];
    if sections.len() != expected.len() {
        return Err(JianPuError::new(
            Span::new(0, 0),
            format!(
                "expected exactly 3 sections ([metadata], [parts], [score]), got {}",
                sections.len()
            ),
        ));
    }
    for (section, exp) in sections.iter().zip(expected.iter()) {
        if &section.kind != exp {
            return Err(JianPuError::new(
                Span::new(0, 0),
                "sections must appear in order: [metadata], [parts], [score]".to_string(),
            ));
        }
    }
    Ok(sections.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_section_input(score: &str) -> String {
        format!("[metadata]\ntitle = \"hi\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n{score}")
    }

    #[test]
    fn splits_metadata_parts_and_score() {
        let input = three_section_input("1 2 3\n");
        let sections = split_sections(&input).unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].kind, SectionKind::Metadata);
        assert_eq!(sections[1].kind, SectionKind::Parts);
        assert_eq!(sections[2].kind, SectionKind::Score);
        assert_eq!(sections[1].content.trim(), "Melody = notes lyrics");
        assert_eq!(sections[2].content.trim(), "1 2 3");
    }

    #[test]
    fn rejects_lyrics_section() {
        let input = "[metadata]\ntitle=\"t\"\n[lyrics]\nfoo\n";
        assert!(split_sections(input).is_err());
    }

    #[test]
    fn rejects_named_score_section() {
        let input = "[score:Soprano]\n1 2 3\n";
        assert!(split_sections(input).is_err());
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "[unknown]\nfoo\n";
        assert!(split_sections(input).is_err());
    }

    #[test]
    fn rejects_parts_after_score() {
        let input = "[metadata]\nt\n[parts]\nMelody = notes\n[score]\n1\n[parts]\nX = notes\n";
        assert!(split_sections(input).is_err());
    }

    #[test]
    fn rejects_missing_parts_section() {
        let input = "[metadata]\ntitle=\"t\"\n\n[score]\n1\n";
        assert!(split_sections(input).is_err());
    }

    #[test]
    fn content_offset_points_past_header_line() {
        let input = "[metadata]\ntitle = \"hi\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n";
        let sections = split_sections(input).unwrap();
        assert_eq!(sections[0].content_offset, 11);
    }

    #[test]
    fn handles_header_with_no_content() {
        let input = "[metadata]\ntitle = \"hi\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n";
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[2].kind, SectionKind::Score);
        assert_eq!(sections[2].content.trim(), "");
    }
}
