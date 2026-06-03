use crate::error::{JianPuError, Span};

pub struct RawSection {
    pub kind: SectionKind,
    pub content: String,
    /// Byte offset in the original source where this section's content begins.
    pub content_offset: usize,
}

#[derive(Debug, PartialEq)]
pub enum SectionKind {
    Metadata,
    Score,
    Lyrics,
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
                "score" => SectionKind::Score,
                "lyrics" => SectionKind::Lyrics,
                _ => {
                    return Err(JianPuError::new(
                        Span::new(byte_offset, byte_offset + line.len()),
                        format!("unknown section: [{}]", kind_str),
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

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_three_sections() {
        let input = r#"[metadata]
title = "hi"

[score]
1 2 3

[lyrics]
你好
"#;
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].kind, SectionKind::Metadata);
        assert_eq!(sections[0].content.trim(), "title = \"hi\"");
        assert_eq!(sections[1].kind, SectionKind::Score);
        assert_eq!(sections[1].content.trim(), "1 2 3");
        assert_eq!(sections[2].kind, SectionKind::Lyrics);
        assert_eq!(sections[2].content.trim(), "你好");
    }

    #[test]
    fn rejects_unknown_section() {
        let input = r#"[unknown]
foo
"#;
        assert!(split_sections(input).is_err());
    }

    #[test]
    fn content_offset_points_past_header_line() {
        let input = r#"[metadata]
title = "hi"
"#;
        let sections = split_sections(input).unwrap();
        // "[metadata]\n" is 11 bytes
        assert_eq!(sections[0].content_offset, 11);
    }

    #[test]
    fn handles_header_with_no_content() {
        let input = "[metadata]\ntitle = \"hi\"\n\n[score]\n";
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].kind, SectionKind::Score);
        assert_eq!(sections[1].content.trim(), "");
    }

    #[test]
    fn handles_consecutive_headers() {
        // [score] immediately after [metadata] with no content in between
        let input = "[metadata]\n[score]\n1 2 3\n[lyrics]\nfoo\n";
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].kind, SectionKind::Metadata);
        assert_eq!(sections[0].content.trim(), "");
        assert_eq!(sections[1].kind, SectionKind::Score);
        assert_eq!(sections[1].content.trim(), "1 2 3");
    }
}
