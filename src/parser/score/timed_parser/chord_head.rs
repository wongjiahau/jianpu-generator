use super::TimedUnitHead;
use crate::ast::parsed::{
    Accidental, BassDegree, Extension, JianPuPitch, ParsedChordNote, ParsedRest, ScoreEvent,
    TriadQuality,
};
use crate::error::{JianPuError, Span};

pub struct ChordHead {
    degree: JianPuPitch,
    accidental: Accidental,
    triad: TriadQuality,
    extension: Option<Extension>,
    bass: Option<BassDegree>,
    is_rest: bool,
}

impl TimedUnitHead for ChordHead {
    fn parse_head(
        chars: &[char],
        start: usize,
        span: &Span,
    ) -> Result<(Self, usize, bool), JianPuError> {
        let degree_char = chars[start];
        if !matches!(degree_char, '0'..='7') {
            let pos = span.start + byte_offset_at_char_index_from_chars(chars, start);
            return Err(JianPuError::new(
                Span::new(pos, pos + degree_char.len_utf8()),
                format!("expected chord degree digit (0-7), got: {degree_char}"),
            ));
        }

        if degree_char == '0' {
            return Ok((
                ChordHead {
                    degree: JianPuPitch::One,
                    accidental: Accidental::Natural,
                    triad: TriadQuality::Major,
                    extension: None,
                    bass: None,
                    is_rest: true,
                },
                start + 1,
                true,
            ));
        }

        let head_end = find_symbol_end(chars, start, span)?;
        let token: String = chars[start..head_end].iter().collect();
        let symbol = parse_chord_symbol(&token, span.clone())?;

        Ok((
            ChordHead {
                degree: symbol.degree,
                accidental: symbol.accidental,
                triad: symbol.triad,
                extension: symbol.extension,
                bass: symbol.bass,
                is_rest: false,
            },
            head_end,
            false,
        ))
    }

    fn head_boundary(chars: &[char], i: usize) -> bool {
        matches!(chars[i], '0'..='7')
    }

    fn allows_octave_suffixes() -> bool {
        false
    }

    fn to_event(
        head: &Self,
        duration: u32,
        dotted: bool,
        _octave: i8,
        group_membership: u8,
        group_continuation: u8,
    ) -> ScoreEvent {
        if head.is_rest {
            ScoreEvent::Rest(ParsedRest { duration, dotted })
        } else {
            ScoreEvent::Chord(ParsedChordNote {
                degree: head.degree.clone(),
                accidental: head.accidental.clone(),
                triad: head.triad.clone(),
                extension: head.extension.clone(),
                bass: head.bass.clone(),
                duration,
                tie: group_continuation > 0,
                group_membership,
                group_continuation,
                dotted,
            })
        }
    }
}

struct ParsedChordSymbolFields {
    degree: JianPuPitch,
    accidental: Accidental,
    triad: TriadQuality,
    extension: Option<Extension>,
    bass: Option<BassDegree>,
}

fn find_symbol_end(chars: &[char], start: usize, span: &Span) -> Result<usize, JianPuError> {
    let max_end = chars.len().min(
        chars[start..]
            .iter()
            .position(|&c| matches!(c, '_' | '=' | '.' | '-' | '\'' | ',' | '(' | ')'))
            .map(|p| start + p)
            .unwrap_or(chars.len()),
    );

    for end in (start + 1..=max_end).rev() {
        let token: String = chars[start..end].iter().collect();
        if parse_chord_symbol(&token, span.clone()).is_ok() {
            return Ok(end);
        }
    }

    let token: String = chars[start..start + 1].iter().collect();
    Err(JianPuError::new(
        span.clone(),
        format!("invalid chord token '{token}'"),
    ))
}

fn parse_chord_symbol(token: &str, span: Span) -> Result<ParsedChordSymbolFields, JianPuError> {
    let mut chars = token.chars();

    let degree = chars
        .next()
        .and_then(char_to_pitch)
        .ok_or_else(|| JianPuError::new(span.clone(), format!("invalid chord token '{token}'")))?;

    let rest: String = chars.collect();
    let mut rest = rest.as_str();

    let accidental = if let Some(stripped) = rest.strip_prefix('#') {
        rest = stripped;
        Accidental::Sharp
    } else if let Some(stripped) = rest.strip_prefix('b') {
        rest = stripped;
        Accidental::Flat
    } else {
        Accidental::Natural
    };

    let (chord_part, bass_str) = match rest.find('/') {
        Some(pos) => (&rest[..pos], Some(&rest[pos + 1..])),
        None => (rest, None),
    };

    let (triad, ext_str) = if let Some(stripped) = chord_part.strip_prefix('m') {
        (TriadQuality::Minor, stripped)
    } else if let Some(stripped) = chord_part.strip_prefix('o') {
        (TriadQuality::Diminished, stripped)
    } else if let Some(stripped) = chord_part.strip_prefix('+') {
        (TriadQuality::Augmented, stripped)
    } else {
        (TriadQuality::Major, chord_part)
    };

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

    let bass = bass_str.map(|s| parse_bass(s, span.clone())).transpose()?;

    Ok(ParsedChordSymbolFields {
        degree,
        accidental,
        triad,
        extension,
        bass,
    })
}

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

fn byte_offset_at_char_index_from_chars(chars: &[char], char_index: usize) -> usize {
    chars[..char_index].iter().map(|c| c.len_utf8()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::score::timed_parser::{parse_timed_line, GroupStack};

    fn chord(
        degree: JianPuPitch,
        acc: Accidental,
        triad: TriadQuality,
        ext: Option<Extension>,
        bass: Option<BassDegree>,
    ) -> ScoreEvent {
        ScoreEvent::Chord(ParsedChordNote {
            degree,
            accidental: acc,
            triad,
            extension: ext,
            bass,
            duration: 4,
            tie: false,
            group_membership: 0,
            group_continuation: 0,
            dotted: false,
        })
    }

    fn try_parse_symbol(token: &str) -> Result<ScoreEvent, JianPuError> {
        let events = parse_timed_line::<ChordHead>(token, 0, &mut GroupStack::default())?;
        if events.len() != 1 {
            return Err(JianPuError::new(
                Span::new(0, token.len()),
                format!("expected one event, got {}", events.len()),
            ));
        }
        Ok(events.into_iter().next().unwrap().value)
    }

    fn parse_symbol(token: &str) -> ScoreEvent {
        try_parse_symbol(token).unwrap()
    }

    fn parse_line(line: &str) -> Vec<ScoreEvent> {
        parse_timed_line::<ChordHead>(line, 0, &mut GroupStack::default())
            .unwrap()
            .into_iter()
            .map(|e| e.value)
            .collect()
    }

    #[test]
    fn parses_major_chord() {
        assert_eq!(
            parse_symbol("1"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                None
            )
        );
    }

    #[test]
    fn parses_minor_chord() {
        assert_eq!(
            parse_symbol("1m"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Minor,
                None,
                None
            )
        );
    }

    #[test]
    fn parses_diminished() {
        assert_eq!(
            parse_symbol("1o"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Diminished,
                None,
                None
            )
        );
    }

    #[test]
    fn parses_augmented() {
        assert_eq!(
            parse_symbol("1+"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Augmented,
                None,
                None
            )
        );
    }

    #[test]
    fn parses_dominant_seventh() {
        assert_eq!(
            parse_symbol("17"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                Some(Extension::DominantSeventh),
                None
            )
        );
    }

    #[test]
    fn parses_major_seventh() {
        assert_eq!(
            parse_symbol("1M7"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                Some(Extension::MajorSeventh),
                None
            )
        );
    }

    #[test]
    fn parses_minor_dominant_seventh() {
        assert_eq!(
            parse_symbol("1m7"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Minor,
                Some(Extension::DominantSeventh),
                None
            )
        );
    }

    #[test]
    fn parses_sharp_accidental() {
        assert_eq!(
            parse_symbol("1#"),
            chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Major,
                None,
                None
            )
        );
    }

    #[test]
    fn parses_flat_accidental() {
        assert_eq!(
            parse_symbol("3b"),
            chord(
                JianPuPitch::Three,
                Accidental::Flat,
                TriadQuality::Major,
                None,
                None
            )
        );
    }

    /// Helper that parses a chord symbol directly (bypassing the lexer) to test the symbol parser
    /// in isolation. Tokens like `1/5` would be lexed as a time-signature by the timed pipeline,
    /// so slash-chord unit tests must go through this lower-level entry point.
    fn parse_chord_symbol_as_event(token: &str) -> ScoreEvent {
        let span = Span::new(0, token.len());
        let sym = parse_chord_symbol(token, span).unwrap();
        ScoreEvent::Chord(ParsedChordNote {
            degree: sym.degree,
            accidental: sym.accidental,
            triad: sym.triad,
            extension: sym.extension,
            bass: sym.bass,
            duration: 4,
            tie: false,
            group_membership: 0,
            group_continuation: 0,
            dotted: false,
        })
    }

    #[test]
    fn parses_slash_chord() {
        let bass = BassDegree {
            degree: JianPuPitch::Five,
            accidental: Accidental::Natural,
        };
        assert_eq!(
            parse_chord_symbol_as_event("1/5"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                Some(bass)
            )
        );
    }

    #[test]
    fn parses_slash_chord_with_accidental_bass() {
        let bass = BassDegree {
            degree: JianPuPitch::Four,
            accidental: Accidental::Flat,
        };
        assert_eq!(
            parse_chord_symbol_as_event("1/4b"),
            chord(
                JianPuPitch::One,
                Accidental::Natural,
                TriadQuality::Major,
                None,
                Some(bass)
            )
        );
    }

    #[test]
    fn parses_complex_slash_chord() {
        let bass = BassDegree {
            degree: JianPuPitch::Five,
            accidental: Accidental::Natural,
        };
        assert_eq!(
            parse_symbol("6m/5"),
            chord(
                JianPuPitch::Six,
                Accidental::Natural,
                TriadQuality::Minor,
                None,
                Some(bass)
            )
        );
    }

    #[test]
    fn parses_rest() {
        let events = parse_line("0");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ScoreEvent::Rest(_)));
    }

    #[test]
    fn parses_extend() {
        let events = parse_line("1 -");
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
        assert!(matches!(events[1], ScoreEvent::Extension));
    }

    #[test]
    fn parses_multiple_tokens() {
        assert_eq!(
            parse_line("1 4m 5"),
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
        assert_eq!(
            parse_line("1 | 4m"),
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
        assert_eq!(
            parse_symbol("1#7"),
            chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Major,
                Some(Extension::DominantSeventh),
                None
            )
        );
    }

    #[test]
    fn parses_flat_with_major_seventh() {
        assert_eq!(
            parse_symbol("3bM7"),
            chord(
                JianPuPitch::Three,
                Accidental::Flat,
                TriadQuality::Major,
                Some(Extension::MajorSeventh),
                None
            )
        );
    }

    #[test]
    fn parses_sharp_minor_dominant_seventh() {
        assert_eq!(
            parse_symbol("1#m7"),
            chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Minor,
                Some(Extension::DominantSeventh),
                None
            )
        );
    }

    #[test]
    fn parses_sharp_with_slash_chord() {
        let bass = BassDegree {
            degree: JianPuPitch::Five,
            accidental: Accidental::Natural,
        };
        assert_eq!(
            parse_symbol("1#/5"),
            chord(
                JianPuPitch::One,
                Accidental::Sharp,
                TriadQuality::Major,
                None,
                Some(bass)
            )
        );
    }

    #[test]
    fn rejects_invalid_token() {
        assert!(parse_timed_line::<ChordHead>("X", 0, &mut GroupStack::default()).is_err());
    }

    #[test]
    fn rejects_unknown_suffix() {
        assert!(try_parse_symbol("1z").is_err());
    }

    #[test]
    fn rejects_octave_suffix() {
        assert!(try_parse_symbol("1'").is_err());
        assert!(try_parse_symbol("1,").is_err());
    }

    #[test]
    fn parses_compact_slur_group() {
        let events =
            parse_timed_line::<ChordHead>("(1-6m-)", 0, &mut GroupStack::default()).unwrap();
        let chord_count = events
            .iter()
            .filter(|e| matches!(e.value, ScoreEvent::Chord(_)))
            .count();
        assert_eq!(chord_count, 2, "expected chord 1 and 6m in group");
    }

    #[test]
    fn parses_spaced_slur_group_across_tokens() {
        let mut state = GroupStack::default();
        let mut chord_count = 0usize;
        for token in ["(1", "-", "6m", "-)"] {
            let events = parse_timed_line::<ChordHead>(token, 0, &mut state).unwrap();
            chord_count += events
                .iter()
                .filter(|e| matches!(e.value, ScoreEvent::Chord(_)))
                .count();
        }
        assert_eq!(chord_count, 2, "expected chord 1 and 6m in group");
        assert!(!state.is_open());
    }
}
