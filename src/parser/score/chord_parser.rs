use crate::ast::parsed::{
    Accidental, BassDegree, Extension, JianPuPitch, ParsedChordEvent, ParsedChordSymbol,
    TriadQuality,
};
use crate::error::{JianPuError, Span};

#[allow(dead_code)]
pub fn parse(line: &str, line_file_offset: usize) -> Result<Vec<ParsedChordEvent>, JianPuError> {
    let mut events = Vec::new();
    let mut byte_pos = 0usize;

    let bytes = line.as_bytes();
    while byte_pos < line.len() {
        let byte = *bytes.get(byte_pos).ok_or_else(|| {
            JianPuError::new(
                Span::new(line_file_offset + byte_pos, line_file_offset + byte_pos + 1),
                "invalid byte index while parsing chord line",
            )
        })?;
        if byte.is_ascii_whitespace() {
            byte_pos += 1;
            continue;
        }
        let token_start = byte_pos;
        while byte_pos < line.len() {
            let byte = *bytes.get(byte_pos).ok_or_else(|| {
                JianPuError::new(
                    Span::new(line_file_offset + byte_pos, line_file_offset + byte_pos + 1),
                    "invalid byte index while parsing chord line",
                )
            })?;
            if byte.is_ascii_whitespace() {
                break;
            }
            byte_pos += 1;
        }
        let token = line.get(token_start..byte_pos).ok_or_else(|| {
            JianPuError::new(
                Span::new(line_file_offset + token_start, line_file_offset + byte_pos),
                "invalid token range in chord line",
            )
        })?;
        let span = Span::new(line_file_offset + token_start, line_file_offset + byte_pos);

        if token == "|" {
            continue;
        }
        let event = match token {
            "0" => ParsedChordEvent::Rest,
            "-" => ParsedChordEvent::Extend(span),
            _ => ParsedChordEvent::Chord(parse_chord_symbol(token, span)?),
        };
        events.push(event);
    }
    Ok(events)
}

#[allow(dead_code)]
fn parse_chord_symbol(token: &str, span: Span) -> Result<ParsedChordSymbol, JianPuError> {
    let mut chars = token.chars();

    let degree = chars
        .next()
        .and_then(char_to_pitch)
        .ok_or_else(|| JianPuError::new(span.clone(), format!("invalid chord token '{token}'")))?;

    // Peek at remaining string
    let rest: String = chars.collect();
    let mut rest = rest.as_str();

    // Accidental
    let accidental = if let Some(stripped) = rest.strip_prefix('#') {
        rest = stripped;
        Accidental::Sharp
    } else if let Some(stripped) = rest.strip_prefix('b') {
        // 'b' is always consumed as flat before '/' split —
        // bass accidentals only appear after '/', so no ambiguity
        rest = stripped;
        Accidental::Flat
    } else {
        Accidental::Natural
    };

    // Split on first '/' for slash chord
    let (chord_part, bass_str) = match rest.find('/') {
        Some(pos) => (&rest[..pos], Some(&rest[pos + 1..])),
        None => (rest, None),
    };

    // Triad quality — check 'm' before 'o'/'+' to handle 'm7'
    let (triad, ext_str) = if let Some(stripped) = chord_part.strip_prefix('m') {
        (TriadQuality::Minor, stripped)
    } else if let Some(stripped) = chord_part.strip_prefix('o') {
        (TriadQuality::Diminished, stripped)
    } else if let Some(stripped) = chord_part.strip_prefix('+') {
        (TriadQuality::Augmented, stripped)
    } else {
        (TriadQuality::Major, chord_part)
    };

    // Extension — check 'M7' before '7'
    let extension = if ext_str == "M7" {
        Some(Extension::MajorSeventh)
    } else if ext_str == "7" {
        Some(Extension::DominantSeventh)
    } else if ext_str.is_empty() {
        None
    } else {
        return Err(JianPuError::new(
            span,
            format!("unknown chord suffix '{ext_str}' in token '{token}'"),
        ));
    };

    // Bass note — compute a precise span starting at the bass substring within the token
    let bass = bass_str
        .map(|s| {
            let bass_start = span.start + (token.len() - s.len());
            parse_bass(s, Span::new(bass_start, span.end))
        })
        .transpose()?;

    Ok(ParsedChordSymbol {
        degree,
        accidental,
        triad,
        extension,
        bass,
    })
}

#[allow(dead_code)]
fn parse_bass(s: &str, span: Span) -> Result<BassDegree, JianPuError> {
    let mut chars = s.chars();
    let degree = chars
        .next()
        .and_then(char_to_pitch)
        .ok_or_else(|| JianPuError::new(span.clone(), format!("invalid bass note '{s}'")))?;
    let accidental = match chars.next() {
        Some('#') => Accidental::Sharp,
        Some('b') => Accidental::Flat,
        None => Accidental::Natural,
        Some(c) => {
            return Err(JianPuError::new(
                span,
                format!("unexpected character '{c}' in bass note '{s}'"),
            ))
        }
    };
    if chars.next().is_some() {
        return Err(JianPuError::new(
            span,
            format!("bass note '{s}' has trailing characters"),
        ));
    }
    Ok(BassDegree { degree, accidental })
}

#[allow(dead_code)]
fn char_to_pitch(c: char) -> Option<JianPuPitch> {
    match c {
        '1' => Some(JianPuPitch::One),
        '2' => Some(JianPuPitch::Two),
        '3' => Some(JianPuPitch::Three),
        '4' => Some(JianPuPitch::Four),
        '5' => Some(JianPuPitch::Five),
        '6' => Some(JianPuPitch::Six),
        '7' => Some(JianPuPitch::Seven),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(
        degree: JianPuPitch,
        acc: Accidental,
        triad: TriadQuality,
        ext: Option<Extension>,
        bass: Option<BassDegree>,
    ) -> ParsedChordEvent {
        ParsedChordEvent::Chord(ParsedChordSymbol {
            degree,
            accidental: acc,
            triad,
            extension: ext,
            bass,
        })
    }

    #[test]
    fn parses_major_chord() {
        let events = parse("1", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                None
            )]
        );
    }

    #[test]
    fn parses_minor_chord() {
        let events = parse("1m", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Minor,
                None,
                None
            )]
        );
    }

    #[test]
    fn parses_diminished() {
        let events = parse("1o", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Diminished,
                None,
                None
            )]
        );
    }

    #[test]
    fn parses_augmented() {
        let events = parse("1+", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Augmented,
                None,
                None
            )]
        );
    }

    #[test]
    fn parses_dominant_seventh() {
        let events = parse("17", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                Some(Extension::DominantSeventh),
                None
            )]
        );
    }

    #[test]
    fn parses_major_seventh() {
        let events = parse("1M7", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                Some(Extension::MajorSeventh),
                None
            )]
        );
    }

    #[test]
    fn parses_minor_dominant_seventh() {
        let events = parse("1m7", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Minor,
                Some(Extension::DominantSeventh),
                None
            )]
        );
    }

    #[test]
    fn parses_sharp_accidental() {
        let events = parse("1#", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Major,
                None,
                None
            )]
        );
    }

    #[test]
    fn parses_flat_accidental() {
        let events = parse("3b", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::Three,
                Accidental::Flat,
                TriadQuality::Major,
                None,
                None
            )]
        );
    }

    #[test]
    fn parses_slash_chord() {
        let events = parse("1/5", 0).unwrap();
        let bass = BassDegree {
            degree: JianPuPitch::Five,
            accidental: Accidental::Natural,
        };
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                Some(bass)
            )]
        );
    }

    #[test]
    fn parses_slash_chord_with_accidental_bass() {
        let events = parse("1/4b", 0).unwrap();
        let bass = BassDegree {
            degree: JianPuPitch::Four,
            accidental: Accidental::Flat,
        };
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                Some(bass)
            )]
        );
    }

    #[test]
    fn parses_complex_slash_chord() {
        let events = parse("6m/5", 0).unwrap();
        let bass = BassDegree {
            degree: JianPuPitch::Five,
            accidental: Accidental::Natural,
        };
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::Six,
                Accidental::Natural,
                TriadQuality::Minor,
                None,
                Some(bass)
            )]
        );
    }

    #[test]
    fn parses_rest() {
        let events = parse("0", 0).unwrap();
        assert_eq!(events, vec![ParsedChordEvent::Rest]);
    }

    #[test]
    fn parses_extend() {
        let events = parse("1 -", 0).unwrap();
        assert_eq!(
            events[0],
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                None
            )
        );
        assert!(matches!(events[1], ParsedChordEvent::Extend(_)));
    }

    #[test]
    fn parses_multiple_tokens() {
        let events = parse("1 4m 5", 0).unwrap();
        assert_eq!(
            events,
            vec![
                chord(
                    JianPuPitch::One,
                    Accidental::Natural,
                    TriadQuality::Major,
                    None,
                    None
                ),
                chord(
                    JianPuPitch::Four,
                    Accidental::Natural,
                    TriadQuality::Minor,
                    None,
                    None
                ),
                chord(
                    JianPuPitch::Five,
                    Accidental::Natural,
                    TriadQuality::Major,
                    None,
                    None
                ),
            ]
        );
    }

    #[test]
    fn skips_bar_lines() {
        let events = parse("1 | 4m", 0).unwrap();
        assert_eq!(
            events,
            vec![
                chord(
                    JianPuPitch::One,
                    Accidental::Natural,
                    TriadQuality::Major,
                    None,
                    None
                ),
                chord(
                    JianPuPitch::Four,
                    Accidental::Natural,
                    TriadQuality::Minor,
                    None,
                    None
                ),
            ]
        );
    }

    #[test]
    fn parses_sharp_with_dominant_seventh() {
        let events = parse("1#7", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Major,
                Some(Extension::DominantSeventh),
                None
            )]
        );
    }

    #[test]
    fn parses_flat_with_major_seventh() {
        let events = parse("3bM7", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::Three,
                Accidental::Flat,
                TriadQuality::Major,
                Some(Extension::MajorSeventh),
                None
            )]
        );
    }

    #[test]
    fn parses_sharp_minor_dominant_seventh() {
        let events = parse("1#m7", 0).unwrap();
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Minor,
                Some(Extension::DominantSeventh),
                None
            )]
        );
    }

    #[test]
    fn parses_sharp_with_slash_chord() {
        let events = parse("1#/5", 0).unwrap();
        let bass = BassDegree {
            degree: JianPuPitch::Five,
            accidental: Accidental::Natural,
        };
        assert_eq!(
            events,
            vec![chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Major,
                None,
                Some(bass)
            )]
        );
    }

    #[test]
    fn rejects_invalid_token() {
        assert!(parse("X", 0).is_err());
    }

    #[test]
    fn rejects_unknown_suffix() {
        assert!(parse("1z", 0).is_err());
    }
}
