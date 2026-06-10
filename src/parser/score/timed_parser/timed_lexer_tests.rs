use super::timed_lexer::{lex_line, LexContext, TimedLexToken};

fn kinds(line: &str) -> Vec<TimedLexToken> {
    lex_line(line, 0, LexContext::Notes)
        .unwrap()
        .into_iter()
        .map(|t| t.value)
        .collect()
}

#[test]
fn skips_whitespace_and_bar_lines() {
    assert_eq!(
        kinds("1 2 | 3"),
        vec![
            TimedLexToken::HeadStart { offset: 0 },
            TimedLexToken::HeadStart { offset: 2 },
            TimedLexToken::HeadStart { offset: 6 },
        ]
    );
}

#[test]
fn lexes_spaced_nested_groups() {
    assert_eq!(
        kinds("((1 1) 5 5)"),
        vec![
            TimedLexToken::LParen,
            TimedLexToken::LParen,
            TimedLexToken::HeadStart { offset: 2 },
            TimedLexToken::HeadStart { offset: 4 },
            TimedLexToken::RParen,
            TimedLexToken::HeadStart { offset: 7 },
            TimedLexToken::HeadStart { offset: 9 },
            TimedLexToken::RParen,
        ]
    );
}

#[test]
fn extension_vs_suffix_dash() {
    assert_eq!(kinds("2---"), vec![TimedLexToken::HeadStart { offset: 0 }]);
    assert_eq!(
        kinds("2 - - -"),
        vec![
            TimedLexToken::HeadStart { offset: 0 },
            TimedLexToken::Extension,
            TimedLexToken::Extension,
            TimedLexToken::Extension,
        ]
    );
}

#[test]
fn sixteenth_note_not_key_change() {
    // "1=C" has a note name but no octave digit, so try_lex_key_change returns None
    // and "1" is emitted as a HeadStart; the trailing "=C" is skipped at the lex stage
    // and the error is caught at parse time (the 'C' is invalid in parse_duration_suffixes).
    let tokens: Vec<_> = lex_line("1=C", 0, LexContext::Notes)
        .unwrap()
        .into_iter()
        .map(|t| t.value)
        .collect();
    assert!(matches!(tokens[0], TimedLexToken::HeadStart { offset: 0 }));

    // A proper key change (with octave digit) must succeed.
    let tokens: Vec<_> = lex_line("1=C4", 0, LexContext::Notes)
        .unwrap()
        .into_iter()
        .map(|t| t.value)
        .collect();
    assert!(matches!(tokens[0], TimedLexToken::KeyChange(_)));
}

#[test]
fn lexes_directives() {
    use TimedLexToken::*;
    let tokens = kinds("bpm=120 4/4");
    assert!(matches!(tokens[0], Bpm(120)));
    assert!(matches!(tokens[1], TimeSignature { num: 4, den: 4 }));
}

#[test]
fn time_signature_with_low_digit() {
    // digits 0-7 at atom boundary should be parsed as time signatures when followed by /
    use TimedLexToken::*;
    let tokens = kinds("4/4");
    assert_eq!(tokens, vec![TimeSignature { num: 4, den: 4 }]);
}

#[test]
fn digits_8_9_without_slash_error() {
    // digits 8-9 without a slash are invalid
    assert!(lex_line("8", 0, LexContext::Notes).is_err());
    assert!(lex_line("9", 0, LexContext::Notes).is_err());
}

#[test]
fn time_signature_zero_denominator_errors() {
    assert!(lex_line("4/0", 0, LexContext::Notes).is_err());
}

#[test]
fn chord_context_treats_slash_as_part_of_chord() {
    // In Chords context, `1/5` must NOT be consumed as a TimeSignature.
    // The lexer should emit a HeadStart for `1`, and leave `/5` for the RD parser
    // (which will handle it via parse_head / find_symbol_end).
    let tokens = lex_line("1/5", 0, LexContext::Chords).unwrap();
    assert!(
        matches!(tokens[0].value, TimedLexToken::HeadStart { .. }),
        "expected HeadStart, got {:?}",
        tokens[0].value
    );
}
