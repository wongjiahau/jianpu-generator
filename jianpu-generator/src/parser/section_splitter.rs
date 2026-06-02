use crate::error::{JianPuError, Span};
use itertools::Itertools;

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

pub fn split_sections(
    input: &str,
) -> Result<Vec<RawSection>, JianPuError> {
    // Create indexed lines with their byte offsets
    let line_items: Vec<_> = input
        .lines()
        .scan(0usize, |byte_offset, line| {
            let offset = *byte_offset;
            *byte_offset += line.len() + 1; // +1 for '\n'
            Some((offset, line))
        })
        .collect();

    // Group consecutive lines by whether they start a section header
    let grouped: Vec<_> = line_items
        .iter()
        .chunk_by(|(_, line)| line.starts_with('[') && line.ends_with(']'))
        .into_iter()
        .map(|(is_header, group)| (is_header, group.collect::<Vec<_>>()))
        .collect();

    // Process pairs: header group followed by content group
    grouped
        .into_iter()
        .tuples()
        .map(|((is_header, header_items), (_is_content, content_items))| {
            if !is_header || header_items.is_empty() {
                return Err(JianPuError::new(
                    Span::new(0, 0),
                    "internal error: expected header group".to_string(),
                ));
            }

            let (byte_offset, header_line) = header_items[0];
            let kind_str = &header_line[1..header_line.len() - 1];
            let kind = parse_section_kind(kind_str, *byte_offset)?;
            let content_offset = byte_offset + header_line.len() + 1;

            let content = content_items
                .iter()
                .map(|(_, line)| *line)
                .join("\n");
            let content = if content.is_empty() {
                content
            } else {
                content + "\n"
            };

            Ok(RawSection {
                kind,
                content,
                content_offset,
            })
        })
        .collect()
}

fn parse_section_kind(
    kind_str: &str,
    byte_offset: usize,
) -> Result<SectionKind, JianPuError> {
    match kind_str {
        "metadata" => Ok(SectionKind::Metadata),
        "score" => Ok(SectionKind::Score),
        "lyrics" => Ok(SectionKind::Lyrics),
        _ => Err(JianPuError::new(
            Span::new(byte_offset, byte_offset + kind_str.len() + 2), // +2 for '[]'
            format!("unknown section: [{}]", kind_str),
        )),
    }
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
}
