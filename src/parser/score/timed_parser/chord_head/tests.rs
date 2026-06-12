use super::*;
use crate::parser::score::timed_parser::{parse_timed_line, GroupStack, LexContext};

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
        slur_group_close_at_duration: None,
    })
}

fn try_parse_symbol(token: &str) -> Result<ScoreEvent, JianPuError> {
    let events =
        parse_timed_line::<ChordHead>(token, 0, &mut GroupStack::default(), LexContext::Chords)?;
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
    parse_timed_line::<ChordHead>(line, 0, &mut GroupStack::default(), LexContext::Chords)
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

#[test]
fn parses_slash_chord() {
    let bass = BassDegree {
        degree: JianPuPitch::Five,
        accidental: Accidental::Natural,
    };
    // Goes through the full pipeline (including the lexer in Chords context) so that
    // `1/5` is not mistakenly consumed as a time signature.
    assert_eq!(
        parse_symbol("1/5"),
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
        parse_symbol("1/4b"),
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
    assert!(
        parse_timed_line::<ChordHead>("X", 0, &mut GroupStack::default(), LexContext::Chords)
            .is_err()
    );
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
        parse_timed_line::<ChordHead>("(1-6m-)", 0, &mut GroupStack::default(), LexContext::Chords)
            .unwrap();
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
        let events =
            parse_timed_line::<ChordHead>(token, 0, &mut state, LexContext::Chords).unwrap();
        chord_count += events
            .iter()
            .filter(|e| matches!(e.value, ScoreEvent::Chord(_)))
            .count();
    }
    assert_eq!(chord_count, 2, "expected chord 1 and 6m in group");
    assert!(!state.is_open());
}
