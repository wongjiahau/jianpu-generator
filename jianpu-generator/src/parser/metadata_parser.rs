use crate::ast::parsed::ParsedMetadata;
use crate::error::{JianPuError, Span};

pub fn parse_metadata(
    content: &str,
    base_offset: usize,
) -> Result<ParsedMetadata, JianPuError> {
    let mut title: Option<String> = None;
    let mut author: Option<String> = None;
    let mut cell_size: Option<u32> = None;
    let mut byte_offset = base_offset;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            byte_offset += line.len() + 1;
            continue;
        }

        let line_span = Span::new(byte_offset, byte_offset + line.len());

        let (key_raw, value_raw) = trimmed.split_once('=').ok_or_else(|| {
            JianPuError::new(line_span.clone(), format!("expected key = value, got: {}", trimmed))
        })?;

        let key = key_raw.trim();
        let value = value_raw.trim().trim_matches('"');

        match key {
            "title" => title = Some(value.to_string()),
            "author" => author = Some(value.to_string()),
            "cell_size" => {
                let parsed = value.parse::<u32>().map_err(|_| {
                    JianPuError::new(
                        line_span.clone(),
                        format!("cell_size must be a positive integer, got: {}", value),
                    )
                })?;
                if parsed == 0 {
                    return Err(JianPuError::new(
                        line_span.clone(),
                        "cell_size must be greater than zero".to_string(),
                    ));
                }
                cell_size = Some(parsed);
            }
            _ => {
                return Err(JianPuError::new(
                    line_span,
                    format!("unknown metadata field: {}", key),
                ))
            }
        }

        byte_offset += line.len() + 1;
    }

    let zero_span = Span::new(base_offset, base_offset);

    Ok(ParsedMetadata {
        title: title
            .ok_or_else(|| JianPuError::new(zero_span.clone(), "missing required field: title"))?,
        author: author
            .ok_or_else(|| JianPuError::new(zero_span, "missing required field: author"))?,
        cell_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_title_and_author() {
        let content = "title = \"hello world\"\nauthor = \"foo\"\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.title, "hello world");
        assert_eq!(meta.author, "foo");
        assert_eq!(meta.cell_size, None);
    }

    #[test]
    fn parses_optional_cell_size() {
        let content = "title = \"t\"\nauthor = \"a\"\ncell_size = 16\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.cell_size, Some(16));
    }

    #[test]
    fn rejects_missing_title() {
        let content = "author = \"foo\"\n";
        assert!(parse_metadata(content, 0).is_err());
    }

    #[test]
    fn rejects_missing_author() {
        let content = "title = \"foo\"\n";
        assert!(parse_metadata(content, 0).is_err());
    }

    #[test]
    fn rejects_unknown_field() {
        let content = "title = \"t\"\nauthor = \"a\"\nfoo = \"bar\"\n";
        assert!(parse_metadata(content, 0).is_err());
    }

    #[test]
    fn rejects_invalid_cell_size() {
        let content = "title = \"t\"\nauthor = \"a\"\ncell_size = abc\n";
        assert!(parse_metadata(content, 0).is_err());
    }
}
