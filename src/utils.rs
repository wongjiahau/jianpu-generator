use crate::ast::parsed::Syllable;

/// Returns true if `c` is a CJK or Japanese/Korean character.
/// Covers Hiragana, Katakana, CJK Extension A, CJK Unified Ideographs, Hangul.
pub fn is_cjk_char(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x309F |  // Hiragana
        0x30A0..=0x30FF |  // Katakana
        0x3400..=0x4DBF |  // CJK Extension A
        0x4E00..=0x9FFF |  // CJK Unified Ideographs
        0xAC00..=0xD7AF    // Hangul
    )
}

pub fn tokenize_lyrics(content: &str) -> Vec<Syllable> {
    let mut raw: Vec<Syllable> = Vec::new();
    let mut current_latin = String::new();

    // Flush the current latin buffer as a syllable (if non-empty).
    let flush = |current_latin: &mut String, raw: &mut Vec<Syllable>| {
        let trimmed = current_latin.trim().to_string();
        if !trimmed.is_empty() {
            raw.push(Syllable { text: trimmed, held: false });
        }
        current_latin.clear();
    };

    for c in content.chars() {
        if is_cjk_char(c) {
            flush(&mut current_latin, &mut raw);
            raw.push(Syllable { text: c.to_string(), held: false });
        } else if c == '-' {
            // `-` is a special delimiter — flush any pending latin, then push a
            // dedicated dash syllable.  Do NOT accumulate it into current_latin
            // so that consecutive dashes each produce their own token.
            flush(&mut current_latin, &mut raw);
            raw.push(Syllable { text: "-".to_string(), held: false });
        } else if c.is_whitespace() {
            flush(&mut current_latin, &mut raw);
        } else {
            current_latin.push(c);
        }
    }

    // Flush remaining latin
    flush(&mut current_latin, &mut raw);

    // Post-process: each `-` token marks the previous syllable as held.
    let mut result: Vec<Syllable> = Vec::new();
    for syllable in raw {
        if syllable.text == "-" {
            if let Some(last) = result.last_mut() {
                last.held = true;
            }
            result.push(Syllable { text: "-".to_string(), held: false });
        } else {
            result.push(syllable);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenises_cjk_without_spaces() {
        let syllables = tokenize_lyrics("你好世界");
        assert_eq!(syllables.len(), 4);
        assert_eq!(syllables[0].text, "你");
        assert_eq!(syllables[1].text, "好");
        assert_eq!(syllables[2].text, "世");
        assert_eq!(syllables[3].text, "界");
    }

    #[test]
    fn tokenises_non_cjk_by_space() {
        let syllables = tokenize_lyrics("he llo world");
        assert_eq!(syllables.len(), 3);
        assert_eq!(syllables[0].text, "he");
        assert_eq!(syllables[1].text, "llo");
        assert_eq!(syllables[2].text, "world");
    }

    #[test]
    fn mixed_cjk_and_latin() {
        let syllables = tokenize_lyrics("你好world");
        assert_eq!(syllables.len(), 3);
        assert_eq!(syllables[0].text, "你");
        assert_eq!(syllables[1].text, "好");
        assert_eq!(syllables[2].text, "world");
    }

    #[test]
    fn spaces_around_cjk_are_ignored() {
        let syllables = tokenize_lyrics("你好 world");
        assert_eq!(syllables.len(), 3);
        assert_eq!(syllables[2].text, "world");
    }

    #[test]
    fn dash_marks_held_syllable() {
        // `he llo - world` → 4 syllables: he, llo (held=true), - (placeholder), world
        let syllables = tokenize_lyrics("he llo - world");
        assert_eq!(syllables.len(), 4);
        assert!(!syllables[0].held);
        assert!(syllables[1].held);
        assert_eq!(syllables[2].text, "-");
        assert!(!syllables[3].held);
    }

    #[test]
    fn held_is_false_by_default() {
        let syllables = tokenize_lyrics("你好");
        assert!(!syllables[0].held);
        assert!(!syllables[1].held);
    }

    #[test]
    fn ignores_leading_trailing_whitespace() {
        let syllables = tokenize_lyrics("  hello  ");
        assert_eq!(syllables.len(), 1);
        assert_eq!(syllables[0].text, "hello");
    }

    // --- new tests ---

    #[test]
    fn empty_string_returns_empty() {
        assert_eq!(tokenize_lyrics(""), Vec::<Syllable>::new());
    }

    #[test]
    fn dash_at_start_no_panic() {
        let syllables = tokenize_lyrics("- hello");
        // first token is "-" (no previous syllable to mark held), second is "hello"
        assert_eq!(syllables.len(), 2);
        assert_eq!(syllables[0].text, "-");
        assert!(!syllables[0].held);
        assert_eq!(syllables[1].text, "hello");
        assert!(!syllables[1].held);
    }

    #[test]
    fn dash_at_end() {
        let syllables = tokenize_lyrics("hello -");
        assert_eq!(syllables.len(), 2);
        assert_eq!(syllables[0].text, "hello");
        assert!(syllables[0].held);
        assert_eq!(syllables[1].text, "-");
        assert!(!syllables[1].held);
    }

    #[test]
    fn consecutive_dashes() {
        // "你 - - 好" → 4 syllables: "你" held=true, "-" held=true, "-", "好"
        let syllables = tokenize_lyrics("你 - - 好");
        assert_eq!(syllables.len(), 4);
        assert_eq!(syllables[0].text, "你");
        assert!(syllables[0].held);
        assert_eq!(syllables[1].text, "-");
        assert!(syllables[1].held);
        assert_eq!(syllables[2].text, "-");
        assert!(!syllables[2].held);
        assert_eq!(syllables[3].text, "好");
        assert!(!syllables[3].held);
    }
}
