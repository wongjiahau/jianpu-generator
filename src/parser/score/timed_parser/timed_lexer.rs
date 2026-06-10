#![allow(clippy::indexing_slicing)]

use super::directives::{key_change_lexeme_len, parse_key_change_text};
use crate::ast::parsed::KeyChange;
use crate::error::{JianPuError, Span, Spanned};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexContext {
    Notes,
    Chords,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimedLexToken {
    LParen,
    RParen,
    Extension,
    HeadStart { offset: usize },
    Bpm(u32),
    KeyChange(KeyChange),
    TimeSignature { num: u8, den: u8 },
}

pub fn lex_line(
    line: &str,
    base_offset: usize,
    context: LexContext,
) -> Result<Vec<Spanned<TimedLexToken>>, JianPuError> {
    let mut tokens = Vec::new();
    // `at_word_boundary`: true when the next non-whitespace char starts a new "word"
    // (i.e. we are after whitespace, `|`, `(`, or `)`, or at the start of the line).
    let mut at_word_boundary = true;
    let mut i = 0;

    while i < line.len() {
        let (c, len) = match line[i..].chars().next() {
            Some(ch) => (ch, ch.len_utf8()),
            None => break,
        };
        if c.is_whitespace() || c == '|' {
            i += len;
            at_word_boundary = true;
            continue;
        }
        let start = base_offset + i;
        let (token_opt, consumed, new_boundary) =
            lex_one_char(line, i, start, len, c, at_word_boundary, context)?;
        if let Some(tok) = token_opt {
            tokens.push(tok);
        }
        at_word_boundary = new_boundary;
        i += consumed;
    }

    Ok(tokens)
}

/// Lex one non-whitespace character.  Returns `(token, bytes_consumed, new_at_word_boundary)`.
/// When the character is a suffix that belongs to the current head, `token` is `None`.
fn lex_one_char(
    line: &str,
    i: usize,
    start: usize,
    len: usize,
    c: char,
    at_word_boundary: bool,
    context: LexContext,
) -> Result<(Option<Spanned<TimedLexToken>>, usize, bool), JianPuError> {
    match c {
        '(' => Ok((
            Some(Spanned::new(
                TimedLexToken::LParen,
                Span::new(start, start + len),
            )),
            len,
            true,
        )),
        ')' => Ok((
            Some(Spanned::new(
                TimedLexToken::RParen,
                Span::new(start, start + len),
            )),
            len,
            true,
        )),
        '-' if at_word_boundary => Ok((
            Some(Spanned::new(
                TimedLexToken::Extension,
                Span::new(start, start + len),
            )),
            len,
            true,
        )),
        // `-` inside a word: duration-suffix dash; skip it.
        '-' => Ok((None, len, false)),
        '1' if at_word_boundary && line[i..].starts_with("1=") => {
            if let Some((tok, consumed)) = try_lex_key_change(line, i, start)? {
                return Ok((Some(tok), consumed, true));
            }
            // Not a key change — emit HeadStart for digit `1`.
            Ok((
                Some(Spanned::new(
                    TimedLexToken::HeadStart { offset: start },
                    Span::new(start, start + len),
                )),
                len,
                false,
            ))
        }
        '0'..='7' => {
            // Check for time signature only at word boundary and in Notes context.
            if at_word_boundary && context == LexContext::Notes {
                if let Some((tok, consumed)) = try_lex_time_signature(line, i, start)? {
                    return Ok((Some(tok), consumed, true));
                }
            }
            // Emit HeadStart.  If inside a word, the RD parser will skip stale HeadStart tokens
            // that fall within an already-parsed multi-char unit (e.g. `7` in `1m7`).
            Ok((
                Some(Spanned::new(
                    TimedLexToken::HeadStart { offset: start },
                    Span::new(start, start + len),
                )),
                len,
                false,
            ))
        }
        'b' if at_word_boundary && line[i..].starts_with("bpm=") => {
            let (tok, consumed) = lex_bpm(line, i, start)?;
            Ok((Some(tok), consumed, true))
        }
        _ if c.is_ascii_digit() => {
            // Digits 8-9: only valid as time signatures at word boundary in Notes context.
            if at_word_boundary && context == LexContext::Notes {
                if let Some((tok, consumed)) = try_lex_time_signature(line, i, start)? {
                    return Ok((Some(tok), consumed, true));
                }
                return Err(JianPuError::new(
                    Span::new(start, start + len),
                    format!("unexpected character: {c}"),
                ));
            } else if at_word_boundary {
                // In Chords context, digits 8-9 are not valid chord degrees.
                return Err(JianPuError::new(
                    Span::new(start, start + len),
                    format!("unexpected character: {c}"),
                ));
            }
            // Inside a word suffix — skip.
            Ok((None, len, false))
        }
        _ if !at_word_boundary => {
            // Any other suffix character inside a word belongs to the current head; skip it.
            Ok((None, len, false))
        }
        _ => Err(JianPuError::new(
            Span::new(start, start + len),
            format!("unexpected character: {c}"),
        )),
    }
}

/// Lex a `bpm=<number>` directive starting at byte offset `i` within `line`.
/// Returns `(token, bytes_consumed)`.
fn lex_bpm(
    line: &str,
    i: usize,
    start: usize,
) -> Result<(Spanned<TimedLexToken>, usize), JianPuError> {
    // "bpm=" is 4 bytes.
    let prefix_len = 4;
    let rest = &line[i + prefix_len..];
    // Consume ASCII digits.
    let digits: &str = {
        let end = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
        &rest[..end]
    };
    if digits.is_empty() {
        return Err(JianPuError::new(
            Span::new(start, start + prefix_len),
            "expected number after 'bpm='".to_string(),
        ));
    }
    let bpm = digits.parse::<u32>().map_err(|_| {
        JianPuError::new(
            Span::new(start, start + prefix_len + digits.len()),
            format!("invalid bpm value: {digits}"),
        )
    })?;
    let consumed = prefix_len + digits.len();
    let span = Span::new(start, start + consumed);
    Ok((Spanned::new(TimedLexToken::Bpm(bpm), span), consumed))
}

/// Try to lex a `1=<NoteName><accidental?><octave>` key change starting at byte offset `i`.
/// Returns `Some((token, bytes_consumed))` if it looks like a key change, `None` otherwise.
fn try_lex_key_change(
    line: &str,
    i: usize,
    start: usize,
) -> Result<Option<(Spanned<TimedLexToken>, usize)>, JianPuError> {
    // "1=" is 2 bytes.
    let after_eq = &line[i + 2..];

    // Check if the next char is a note name letter.
    let is_note_name = after_eq
        .chars()
        .next()
        .is_some_and(|c| matches!(c, 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G'));

    if !is_note_name {
        return Ok(None);
    }

    // Determine how many bytes the note-name + accidental occupy.
    let head_len = key_change_lexeme_len(after_eq);

    // After the head, consume digits for the octave.
    let after_head = &after_eq[head_len..];
    let octave_len = after_head
        .bytes()
        .take_while(|b| b.is_ascii_digit())
        .count();

    if octave_len == 0 {
        return Ok(None);
    }

    let consumed = 2 + head_len + octave_len; // "1=" + head + octave digits
    let text = &line[i..i + consumed];
    let span = Span::new(start, start + consumed);

    let key_change = parse_key_change_text(text, &span)?;
    Ok(Some((
        Spanned::new(TimedLexToken::KeyChange(key_change), span),
        consumed,
    )))
}

/// Try to lex a `<num>/<den>` time signature starting at byte offset `i`.
/// Returns `Some((token, bytes_consumed))` on success, `None` if the text doesn't look like a
/// time signature (no `/` found).
fn try_lex_time_signature(
    line: &str,
    i: usize,
    start: usize,
) -> Result<Option<(Spanned<TimedLexToken>, usize)>, JianPuError> {
    let slice = &line[i..];

    // Collect numerator digits.
    let num_len = slice.bytes().take_while(|b| b.is_ascii_digit()).count();
    if num_len == 0 {
        return Ok(None);
    }
    // Expect a `/`.
    if slice.as_bytes().get(num_len) != Some(&b'/') {
        return Ok(None);
    }
    // Collect denominator digits.
    let den_start = num_len + 1;
    let den_len = slice[den_start..]
        .bytes()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if den_len == 0 {
        return Ok(None);
    }

    let num_str = &slice[..num_len];
    let den_str = &slice[den_start..den_start + den_len];

    let num = num_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            Span::new(start, start + num_len),
            format!("invalid time signature numerator: {num_str}"),
        )
    })?;
    let den = den_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            Span::new(start + den_start, start + den_start + den_len),
            format!("invalid time signature denominator: {den_str}"),
        )
    })?;

    if den == 0 {
        return Err(JianPuError::new(
            Span::new(start, start + num_len + 1 + den_len),
            "time signature denominator cannot be zero".to_string(),
        ));
    }

    let consumed = num_len + 1 + den_len;
    let span = Span::new(start, start + consumed);
    Ok(Some((
        Spanned::new(TimedLexToken::TimeSignature { num, den }, span),
        consumed,
    )))
}
