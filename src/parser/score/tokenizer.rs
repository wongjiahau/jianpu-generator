/// A raw token is a non-whitespace string extracted from the score content,
/// paired with its byte offset in the original source.
pub struct RawToken {
    pub text: String,
    pub offset: usize,
}

/// Split score content into raw tokens, filtering out `|` bar-line separators.
pub fn tokenize(content: &str, base_offset: usize) -> Vec<RawToken> {
    let mut tokens = Vec::new();
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        // Collect the full token (consecutive non-whitespace)
        let start = i;
        let mut end = i + c.len_utf8();
        while let Some(&(j, nc)) = chars.peek() {
            if nc.is_whitespace() {
                break;
            }
            end = j + nc.len_utf8();
            chars.next();
            if nc == '~' {
                break; // `~` ends the current token; the next char starts a new token
            }
        }
        let text = content[start..end].to_string();
        if text == "|" {
            continue; // bar-line separator, ignore
        }
        tokens.push(RawToken {
            text,
            offset: base_offset + start,
        });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_by_whitespace() {
        let tokens = tokenize("1 2 3", 0);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "1");
        assert_eq!(tokens[1].text, "2");
        assert_eq!(tokens[2].text, "3");
    }

    #[test]
    fn filters_bar_lines() {
        let tokens = tokenize("1 2 | 3", 0);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[2].text, "3");
    }

    #[test]
    fn records_byte_offsets() {
        let tokens = tokenize("1 2 3", 0);
        assert_eq!(tokens[0].offset, 0);
        assert_eq!(tokens[1].offset, 2);
        assert_eq!(tokens[2].offset, 4);
    }

    #[test]
    fn base_offset_is_added() {
        let tokens = tokenize("1 2", 100);
        assert_eq!(tokens[0].offset, 100);
        assert_eq!(tokens[1].offset, 102);
    }

    #[test]
    fn splits_on_tilde() {
        let tokens = tokenize("4~3~3", 0);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "4~");
        assert_eq!(tokens[1].text, "3~");
        assert_eq!(tokens[2].text, "3");
    }

    #[test]
    fn handles_multiline_content() {
        let tokens = tokenize("1 2\n3 4\n", 0);
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[2].text, "3");
        assert_eq!(tokens[2].offset, 4); // past "1 2\n"
    }
}
