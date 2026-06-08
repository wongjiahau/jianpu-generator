use crate::ast::parsed::ParsedMetadata;
use crate::error::{JianPuError, Span};

fn parse_positive_u32(key: &str, value: &str, line_span: &Span) -> Result<u32, JianPuError> {
    let parsed = value.parse::<u32>().map_err(|_| {
        JianPuError::new(
            line_span.clone(),
            format!("{key} must be a positive integer, got: {value}"),
        )
    })?;
    if parsed == 0 {
        return Err(JianPuError::new(
            line_span.clone(),
            format!("{key} must be greater than zero"),
        ));
    }
    Ok(parsed)
}

pub fn parse_metadata(content: &str, base_offset: usize) -> Result<ParsedMetadata, JianPuError> {
    let mut title: Option<String> = None;
    let mut subtitle: Option<String> = None;
    let mut author: Option<String> = None;
    let mut row_height: Option<u32> = None;
    let mut max_columns: Option<u32> = None;
    let mut label_width: Option<u32> = None;
    let mut note_number_width: Option<u32> = None;
    let mut byte_offset = base_offset;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            byte_offset += line.len() + 1;
            continue;
        }

        let line_span = Span::new(byte_offset, byte_offset + line.len());

        let (key_raw, value_raw) = trimmed.split_once('=').ok_or_else(|| {
            JianPuError::new(
                line_span.clone(),
                format!("expected key = value, got: {trimmed}"),
            )
        })?;

        let key = key_raw.trim();
        let value = value_raw.trim().trim_matches('"');

        match key {
            "title" => title = Some(value.to_string()),
            "subtitle" => subtitle = Some(value.to_string()),
            "author" => author = Some(value.to_string()),
            "row height" => {
                row_height = Some(parse_positive_u32("row height", value, &line_span)?);
            }
            "max columns" => {
                max_columns = Some(parse_positive_u32("max columns", value, &line_span)?);
            }
            "label width" => {
                label_width = Some(parse_positive_u32("label width", value, &line_span)?);
            }
            "note number width" => {
                note_number_width =
                    Some(parse_positive_u32("note number width", value, &line_span)?);
            }
            _ => {
                return Err(JianPuError::new(
                    line_span,
                    format!("unknown metadata field: {key}"),
                ))
            }
        }

        byte_offset += line.len() + 1;
    }

    let zero_span = Span::new(base_offset, base_offset);

    Ok(ParsedMetadata {
        title: title
            .ok_or_else(|| JianPuError::new(zero_span.clone(), "missing required field: title"))?,
        subtitle,
        author: author
            .ok_or_else(|| JianPuError::new(zero_span, "missing required field: author"))?,
        row_height,
        max_columns,
        label_width,
        note_number_width,
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
        assert_eq!(meta.row_height, None);
        assert_eq!(meta.max_columns, None);
        assert_eq!(meta.label_width, None);
    }

    #[test]
    fn parses_optional_row_height() {
        let content = "title = \"t\"\nauthor = \"a\"\nrow height = 16\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.row_height, Some(16));
    }

    #[test]
    fn parses_optional_max_columns() {
        let content = "title = \"t\"\nauthor = \"a\"\nmax columns = 32\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.max_columns, Some(32));
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
    fn rejects_parts_field_in_metadata() {
        let content = "title = \"t\"\nauthor = \"a\"\nparts = notes: lyrics:\n";
        let err = parse_metadata(content, 0).unwrap_err();
        assert!(err.message.contains("unknown metadata field: parts"));
    }

    #[test]
    fn rejects_invalid_row_height() {
        let content = "title = \"t\"\nauthor = \"a\"\nrow height = abc\n";
        assert!(parse_metadata(content, 0).is_err());
    }

    #[test]
    fn rejects_invalid_max_columns() {
        let content = "title = \"t\"\nauthor = \"a\"\nmax columns = 0\n";
        assert!(parse_metadata(content, 0).is_err());
    }

    #[test]
    fn parses_optional_subtitle() {
        let content = "title = \"hello\"\nauthor = \"foo\"\nsubtitle = \"sub\"\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.subtitle, Some("sub".to_string()));
    }

    #[test]
    fn subtitle_defaults_to_none() {
        let content = "title = \"t\"\nauthor = \"a\"\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.subtitle, None);
    }

    #[test]
    fn rejects_row_height_with_underscore() {
        let content = "title = \"t\"\nauthor = \"a\"\nrow_height = 20\n";
        assert!(parse_metadata(content, 0).is_err());
    }

    #[test]
    fn parses_label_width() {
        let content = "title = \"t\"\nauthor = \"a\"\nlabel width = 60\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.label_width, Some(60));
    }

    #[test]
    fn label_width_defaults_to_none() {
        let content = "title = \"t\"\nauthor = \"a\"\n";
        let meta = parse_metadata(content, 0).unwrap();
        assert_eq!(meta.label_width, None);
    }
}
