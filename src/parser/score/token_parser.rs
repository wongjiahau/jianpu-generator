use crate::ast::parsed::{
    Accidental, JianPuPitch, KeyChange, Note, NoteName, ParsedNote, ParsedRest, ScoreEvent,
};
use crate::error::{JianPuError, Span, Spanned};
use crate::parser::score::tokenizer::RawToken;

pub fn parse_tokens(tokens: Vec<RawToken>) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let mut events = Vec::new();

    for token in tokens {
        let span = Span::new(token.offset, token.offset + token.text.len());
        let event = parse_single_token(&token.text, span.clone())?;
        events.push(Spanned::new(event, span));
    }

    Ok(events)
}

fn parse_single_token(text: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    // Extension
    if text == "-" {
        return Ok(ScoreEvent::Extension);
    }

    // Standalone tie/slur marker
    if text == "~" {
        return Ok(ScoreEvent::TieMarker);
    }

    // BPM directive: bpm=N
    if let Some(rest) = text.strip_prefix("bpm=") {
        let bpm = rest
            .parse::<u32>()
            .map_err(|_| JianPuError::new(span.clone(), format!("invalid bpm value: {}", rest)))?;
        return Ok(ScoreEvent::BpmChange(bpm));
    }

    // Key change directive: 1=C4, 1=Bb4, 1=F#3
    if text.starts_with("1=") {
        return parse_key_change(text, span);
    }

    // Time signature: N/N
    if text.contains('/') {
        return parse_time_signature(text, span);
    }

    // Note or rest
    parse_note_or_rest(text, span)
}

fn parse_note_or_rest(text: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let mut chars = text.chars().peekable();

    // Duration prefix
    let duration: u32 = match chars.peek() {
        Some('=') => {
            chars.next();
            1
        }
        Some('_') => {
            chars.next();
            2
        }
        _ => 4,
    };

    // Leading octave dots
    let mut leading_dots = 0i8;
    while chars.peek() == Some(&'.') {
        leading_dots += 1;
        chars.next();
    }

    // Pitch digit
    let pitch_char = chars.next().ok_or_else(|| {
        JianPuError::new(
            span.clone(),
            format!("expected a pitch digit (0-7), got: {}", text),
        )
    })?;

    if !matches!(pitch_char, '0'..='7') {
        return Err(JianPuError::new(
            span.clone(),
            format!("expected pitch digit 0-7, got: {}", pitch_char),
        ));
    }

    // Trailing octave dots and tie
    let mut trailing_dots = 0i8;
    let mut tie = false;
    while let Some(&c) = chars.peek() {
        match c {
            '.' => {
                trailing_dots += 1;
                chars.next();
            }
            '~' => {
                tie = true;
                chars.next();
                break;
            }
            _ => {
                return Err(JianPuError::new(
                    span.clone(),
                    format!("unexpected character after pitch: {}", c),
                ));
            }
        }
    }

    if leading_dots > 0 && trailing_dots > 0 {
        return Err(JianPuError::new(
            span,
            "mixed octave dots are invalid (use leading dots for up, trailing for down)"
                .to_string(),
        ));
    }

    let octave = if leading_dots > 0 {
        leading_dots
    } else {
        -trailing_dots
    };

    if pitch_char == '0' {
        return Ok(ScoreEvent::Rest(ParsedRest { duration }));
    }

    let pitch = match pitch_char {
        '1' => JianPuPitch::One,
        '2' => JianPuPitch::Two,
        '3' => JianPuPitch::Three,
        '4' => JianPuPitch::Four,
        '5' => JianPuPitch::Five,
        '6' => JianPuPitch::Six,
        '7' => JianPuPitch::Seven,
        _ => unreachable!(),
    };

    Ok(ScoreEvent::Note(ParsedNote {
        pitch,
        octave,
        duration,
        tie,
    }))
}

fn parse_key_change(text: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let after_eq = text.strip_prefix("1=").unwrap();
    let mut chars = after_eq.chars().peekable();

    let name_char = chars.next().ok_or_else(|| {
        JianPuError::new(
            span.clone(),
            format!("expected note name after '1=', got: {}", text),
        )
    })?;

    let name = match name_char {
        'A' => NoteName::A,
        'B' => NoteName::B,
        'C' => NoteName::C,
        'D' => NoteName::D,
        'E' => NoteName::E,
        'F' => NoteName::F,
        'G' => NoteName::G,
        _ => {
            return Err(JianPuError::new(
                span.clone(),
                format!("invalid note name: {}", name_char),
            ))
        }
    };

    let accidental = match chars.peek() {
        Some('b') => {
            chars.next();
            Accidental::Flat
        }
        Some('#') => {
            chars.next();
            Accidental::Sharp
        }
        _ => Accidental::Natural,
    };

    let octave_str: String = chars.collect();
    let octave = octave_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid octave number in key change: {}", text),
        )
    })?;

    Ok(ScoreEvent::KeyChange(KeyChange {
        note: Note {
            name,
            octave,
            accidental,
        },
    }))
}

fn parse_time_signature(text: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let parts: Vec<&str> = text.split('/').collect();
    if parts.len() != 2 {
        return Err(JianPuError::new(
            span.clone(),
            format!("invalid time signature: {}", text),
        ));
    }
    let numerator = parts[0].parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid time signature numerator: {}", parts[0]),
        )
    })?;
    let denominator = parts[1].parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid time signature denominator: {}", parts[1]),
        )
    })?;
    if denominator == 0 {
        return Err(JianPuError::new(
            span,
            "time signature denominator cannot be zero".to_string(),
        ));
    }
    Ok(ScoreEvent::TimeSignatureChange {
        numerator,
        denominator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::score::tokenizer::tokenize;

    fn parse(input: &str) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
        parse_tokens(tokenize(input, 0))
    }

    fn note(events: &[Spanned<ScoreEvent>], i: usize) -> &ParsedNote {
        match &events[i].value {
            ScoreEvent::Note(n) => n,
            _ => panic!("expected Note at index {}", i),
        }
    }

    fn rest(events: &[Spanned<ScoreEvent>], i: usize) -> &ParsedRest {
        match &events[i].value {
            ScoreEvent::Rest(r) => r,
            _ => panic!("expected Rest at index {}", i),
        }
    }

    #[test]
    fn parses_full_beat_note() {
        let events = parse("1").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.pitch, JianPuPitch::One);
        assert_eq!(n.duration, 4);
        assert_eq!(n.octave, 0);
        assert!(!n.tie);
    }

    #[test]
    fn parses_half_beat_note() {
        let events = parse("_3").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.pitch, JianPuPitch::Three);
        assert_eq!(n.duration, 2);
    }

    #[test]
    fn parses_quarter_beat_note() {
        let events = parse("=5").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.pitch, JianPuPitch::Five);
        assert_eq!(n.duration, 1);
    }

    #[test]
    fn parses_octave_up() {
        let events = parse(".1").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, 1);
    }

    #[test]
    fn parses_two_octaves_up() {
        let events = parse("..1").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, 2);
    }

    #[test]
    fn parses_octave_down() {
        let events = parse("1.").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, -1);
    }

    #[test]
    fn parses_two_octaves_down() {
        let events = parse("1..").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, -2);
    }

    #[test]
    fn rejects_mixed_octave_dots() {
        assert!(parse(".1.").is_err());
    }

    #[test]
    fn parses_tie() {
        let events = parse("2~").unwrap();
        let n = note(&events, 0);
        assert!(n.tie);
    }

    #[test]
    fn parses_rest() {
        let events = parse("0").unwrap();
        let r = rest(&events, 0);
        assert_eq!(r.duration, 4);
    }

    #[test]
    fn parses_half_beat_rest() {
        let events = parse("_0").unwrap();
        let r = rest(&events, 0);
        assert_eq!(r.duration, 2);
    }

    #[test]
    fn parses_extension() {
        let events = parse("-").unwrap();
        assert!(matches!(events[0].value, ScoreEvent::Extension));
    }

    #[test]
    fn parses_sequence() {
        let events = parse("1 _2 3").unwrap();
        assert_eq!(events.len(), 3);
    }
}
