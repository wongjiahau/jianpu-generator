use crate::ast::parsed::{
    Accidental, JianPuPitch, KeyChange, Note, NoteName, ParsedNote, ParsedRest, ScoreEvent,
};
use crate::error::{JianPuError, Span, Spanned};
use crate::parser::score::tokenizer::RawToken;

/// Tracks an unfinished `(…` group that continues in a later measure.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GroupParseState {
    pub open: bool,
    pub open_note_count: usize,
}

fn validate_group_note_count(count: usize, span: &Span) -> Result<(), JianPuError> {
    if count < 2 {
        return Err(JianPuError::new(
            span.clone(),
            "tie/slur group `(…)` must contain at least 2 notes".to_string(),
        ));
    }
    Ok(())
}

pub fn parse_tokens(
    tokens: Vec<RawToken>,
    group_state: &mut GroupParseState,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let mut events = Vec::new();

    for token in tokens {
        let span = Span::new(token.offset, token.offset + token.text.len());
        let parsed = parse_single_token(&token.text, span.clone(), group_state)?;
        for event in parsed {
            events.push(Spanned::new(event, span.clone()));
        }
    }

    Ok(events)
}

fn parse_single_token(
    text: &str,
    span: Span,
    group_state: &mut GroupParseState,
) -> Result<Vec<ScoreEvent>, JianPuError> {
    // BPM directive: bpm=N
    if let Some(rest) = text.strip_prefix("bpm=") {
        let bpm = rest
            .parse::<u32>()
            .map_err(|_| JianPuError::new(span.clone(), format!("invalid bpm value: {rest}")))?;
        return Ok(vec![ScoreEvent::BpmChange(bpm)]);
    }

    // Key change directive: 1=C4, 1=Bb4, 1=F#3 (pitch digit 1= is a sixteenth note)
    if text.starts_with("1=") {
        let after_eq = text.get(2..).unwrap_or("");
        if after_eq
            .chars()
            .next()
            .is_some_and(|c| matches!(c, 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G'))
        {
            return Ok(vec![parse_key_change(text, &span)?]);
        }
    }

    // Time signature: N/N
    if text.contains('/') {
        return Ok(vec![parse_time_signature(text, span)?]);
    }

    // Standalone `-` extends the previous note by one beat (4 quarter-beats).
    if text == "-" {
        return Ok(vec![ScoreEvent::Extension]);
    }

    // Note or rest (happi123-style: suffix modifiers, optional () groups)
    parse_note_token(text, span, group_state)
}

struct ParsedAtom {
    pitch_char: char,
    duration: u32,
    octave: i8,
    dotted: bool,
    tie: bool,
}

fn parse_note_token(
    text: &str,
    span: Span,
    group_state: &mut GroupParseState,
) -> Result<Vec<ScoreEvent>, JianPuError> {
    if text.is_empty() {
        return Err(JianPuError::new(span, "empty note token".to_string()));
    }

    if group_state.open && text.starts_with('(') {
        return Err(JianPuError::new(
            span,
            "cannot start a new '(' group while a previous group is still open".to_string(),
        ));
    }

    let mut events = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    if group_state.open {
        if text.contains(')') {
            i = parse_closing_group_segment(&mut events, &chars, text, i, &span, group_state)?;
        } else {
            let added = parse_open_group_continuation(&mut events, &chars, text, i, &span)?;
            group_state.open_note_count += added;
            return Ok(events);
        }
    }

    while i < chars.len() {
        if chars[i] == '(' {
            let inner_start = i + 1;
            if let Some(inner_end) = find_closing_paren(&chars, inner_start) {
                let inner: String = chars[inner_start..inner_end].iter().collect();
                events.extend(parse_closed_group(&inner, &span)?);
                i = inner_end + 1;
            } else {
                let inner: String = chars[inner_start..].iter().collect();
                let group_events = parse_open_group(&inner, &span)?;
                group_state.open_note_count = group_events.len();
                events.extend(group_events);
                group_state.open = true;
                break;
            }
            continue;
        }

        if !matches!(chars[i], '0'..='7') {
            let pos = span.start + byte_offset_at_char_index(text, i);
            return Err(JianPuError::new(
                Span::new(pos, pos + chars[i].len_utf8()),
                format!("expected pitch digit (0-7), got: {}", chars[i]),
            ));
        }

        let (atom, next_i) = parse_one_atom(&chars, i, &span)?;
        events.push(atom_to_event(atom));
        i = next_i;
    }

    Ok(events)
}

fn parse_closing_group_segment(
    events: &mut Vec<ScoreEvent>,
    chars: &[char],
    text: &str,
    mut i: usize,
    span: &Span,
    group_state: &mut GroupParseState,
) -> Result<usize, JianPuError> {
    let mut atoms = Vec::new();

    while i < chars.len() && chars[i] != ')' {
        if chars[i] == '(' {
            return Err(JianPuError::new(
                span.clone(),
                "nested '(' groups are not supported".to_string(),
            ));
        }
        if !matches!(chars[i], '0'..='7') {
            let pos = span.start + byte_offset_at_char_index(text, i);
            return Err(JianPuError::new(
                Span::new(pos, pos + chars[i].len_utf8()),
                format!("expected pitch digit (0-7), got: {}", chars[i]),
            ));
        }
        let (atom, next_i) = parse_one_atom(chars, i, span)?;
        atoms.push(atom);
        i = next_i;
    }

    apply_closing_group_ties(&mut atoms);
    let atom_count = atoms.len();
    events.extend(atoms.into_iter().map(atom_to_event));

    if i < chars.len() && chars[i] == ')' {
        validate_group_note_count(group_state.open_note_count + atom_count, span)?;
        group_state.open = false;
        group_state.open_note_count = 0;
        i += 1;
    } else {
        group_state.open_note_count += atom_count;
        group_state.open = true;
    }

    Ok(i)
}

fn parse_open_group_continuation(
    events: &mut Vec<ScoreEvent>,
    chars: &[char],
    text: &str,
    mut i: usize,
    span: &Span,
) -> Result<usize, JianPuError> {
    let mut atoms = Vec::new();

    while i < chars.len() {
        if chars[i] == '(' {
            return Err(JianPuError::new(
                span.clone(),
                "nested '(' groups are not supported".to_string(),
            ));
        }
        if !matches!(chars[i], '0'..='7') {
            let pos = span.start + byte_offset_at_char_index(text, i);
            return Err(JianPuError::new(
                Span::new(pos, pos + chars[i].len_utf8()),
                format!("expected pitch digit (0-7), got: {}", chars[i]),
            ));
        }
        let (atom, next_i) = parse_one_atom(chars, i, span)?;
        atoms.push(atom);
        i = next_i;
    }

    apply_open_group_ties(&mut atoms);
    let added = atoms.len();
    events.extend(atoms.into_iter().map(atom_to_event));

    Ok(added)
}

fn find_closing_paren(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_atoms_from_text(text: &str, span: &Span) -> Result<Vec<ParsedAtom>, JianPuError> {
    let chars: Vec<char> = text.chars().collect();
    let mut atoms = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '(' {
            return Err(JianPuError::new(
                span.clone(),
                "nested '(' groups are not supported".to_string(),
            ));
        }
        if !matches!(chars[i], '0'..='7') {
            let pos = span.start + byte_offset_at_char_index(text, i);
            return Err(JianPuError::new(
                Span::new(pos, pos + chars[i].len_utf8()),
                format!("expected pitch digit (0-7), got: {}", chars[i]),
            ));
        }
        let (atom, next_i) = parse_one_atom(&chars, i, span)?;
        atoms.push(atom);
        i = next_i;
    }

    Ok(atoms)
}

fn parse_closed_group(inner: &str, span: &Span) -> Result<Vec<ScoreEvent>, JianPuError> {
    let mut atoms = parse_atoms_from_text(inner, span)?;
    validate_group_note_count(atoms.len(), span)?;
    apply_closed_group_ties(&mut atoms);
    Ok(atoms.into_iter().map(atom_to_event).collect())
}

fn parse_open_group(inner: &str, span: &Span) -> Result<Vec<ScoreEvent>, JianPuError> {
    let mut atoms = parse_atoms_from_text(inner, span)?;
    apply_open_group_ties(&mut atoms);
    Ok(atoms.into_iter().map(atom_to_event).collect())
}

fn apply_closed_group_ties(atoms: &mut [ParsedAtom]) {
    for atom in atoms.iter_mut() {
        atom.tie = true;
    }
    if atoms.len() > 1 {
        if let Some(last) = atoms.last_mut() {
            last.tie = false;
        }
    }
}

fn apply_open_group_ties(atoms: &mut [ParsedAtom]) {
    for atom in atoms.iter_mut() {
        atom.tie = true;
    }
}

fn apply_closing_group_ties(atoms: &mut [ParsedAtom]) {
    for atom in atoms.iter_mut() {
        atom.tie = true;
    }
    if let Some(last) = atoms.last_mut() {
        last.tie = false;
    }
}

fn parse_one_atom(
    chars: &[char],
    start: usize,
    span: &Span,
) -> Result<(ParsedAtom, usize), JianPuError> {
    let pitch_char = chars[start];
    let mut i = start + 1;
    let mut duration = 4u32;
    let mut dotted = false;
    let mut octave_up = 0i8;
    let mut octave_down = 0i8;

    while i < chars.len() {
        if matches!(chars[i], '0'..='7') {
            break;
        }

        match chars[i] {
            '_' => {
                duration = duration.min(2);
                i += 1;
            }
            '=' => {
                duration = 1;
                i += 1;
            }
            '\'' => {
                octave_up += 1;
                i += 1;
            }
            ',' => {
                octave_down += 1;
                i += 1;
            }
            '.' => {
                dotted = true;
                i += 1;
            }
            '-' => {
                if pitch_char == '0' {
                    let pos = span.start + byte_offset_at_char_index_from_chars(chars, start, i);
                    return Err(JianPuError::dash_after_rest(Span::new(pos, pos + 1)));
                }
                duration += 4;
                i += 1;
            }
            ')' | '(' => break,
            c => {
                let pos = span.start + byte_offset_at_char_index_from_chars(chars, start, i);
                return Err(JianPuError::new(
                    Span::new(pos, pos + c.len_utf8()),
                    format!("unexpected character in note: {c}"),
                ));
            }
        }
    }

    if octave_up > 0 && octave_down > 0 {
        return Err(JianPuError::new(
            span.clone(),
            "mixed octave markers are invalid (use ' for up, , for down)".to_string(),
        ));
    }

    let octave = if octave_up > 0 {
        octave_up
    } else {
        -octave_down
    };

    if dotted && duration == 1 {
        return Err(JianPuError::new(
            span.clone(),
            "cannot dot a quarter-beat (=) note; use _ or no duration suffix".to_string(),
        ));
    }

    let duration = if dotted {
        duration + duration / 2
    } else {
        duration
    };

    Ok((
        ParsedAtom {
            pitch_char,
            duration,
            octave,
            dotted,
            tie: false,
        },
        i,
    ))
}

fn atom_to_event(atom: ParsedAtom) -> ScoreEvent {
    if atom.pitch_char == '0' {
        ScoreEvent::Rest(ParsedRest {
            duration: atom.duration,
            dotted: atom.dotted,
        })
    } else {
        ScoreEvent::Note(ParsedNote {
            pitch: pitch_char_to_jianpu(atom.pitch_char),
            octave: atom.octave,
            duration: atom.duration,
            tie: atom.tie,
            dotted: atom.dotted,
        })
    }
}

fn pitch_char_to_jianpu(pitch_char: char) -> JianPuPitch {
    match pitch_char {
        '1' => JianPuPitch::One,
        '2' => JianPuPitch::Two,
        '3' => JianPuPitch::Three,
        '4' => JianPuPitch::Four,
        '5' => JianPuPitch::Five,
        '6' => JianPuPitch::Six,
        '7' => JianPuPitch::Seven,
        _ => unreachable!("validated pitch digit"),
    }
}

fn byte_offset_at_char_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

fn byte_offset_at_char_index_from_chars(chars: &[char], start: usize, i: usize) -> usize {
    chars[start..=i].iter().map(|c| c.len_utf8()).sum()
}

fn parse_key_change(text: &str, span: &Span) -> Result<ScoreEvent, JianPuError> {
    let after_eq = text.strip_prefix("1=").ok_or_else(|| {
        JianPuError::new(
            span.clone(),
            format!("expected key change starting with '1=', got: {text}"),
        )
    })?;
    let mut chars = after_eq.chars().peekable();

    let name_char = chars.next().ok_or_else(|| {
        JianPuError::new(
            span.clone(),
            format!("expected note name after '1=', got: {text}"),
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
                format!("invalid note name: {name_char}"),
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
            format!("invalid octave number in key change: {text}"),
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
            span,
            format!("invalid time signature: {text}"),
        ));
    }
    let numerator_str = parts
        .first()
        .ok_or_else(|| JianPuError::new(span.clone(), format!("invalid time signature: {text}")))?;
    let denominator_str = parts
        .get(1)
        .ok_or_else(|| JianPuError::new(span.clone(), format!("invalid time signature: {text}")))?;
    let numerator = numerator_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid time signature numerator: {numerator_str}"),
        )
    })?;
    let denominator = denominator_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid time signature denominator: {denominator_str}"),
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
        parse_tokens(tokenize(input, 0), &mut GroupParseState::default())
    }

    fn parse_with_state(
        input: &str,
        state: &mut GroupParseState,
    ) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
        parse_tokens(tokenize(input, 0), state)
    }

    fn note(events: &[Spanned<ScoreEvent>], i: usize) -> &ParsedNote {
        match &events[i].value {
            ScoreEvent::Note(n) => n,
            _ => panic!("expected Note at index {i}"),
        }
    }

    fn rest(events: &[Spanned<ScoreEvent>], i: usize) -> &ParsedRest {
        match &events[i].value {
            ScoreEvent::Rest(r) => r,
            _ => panic!("expected Rest at index {i}"),
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
        let events = parse("3_").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.pitch, JianPuPitch::Three);
        assert_eq!(n.duration, 2);
    }

    #[test]
    fn parses_quarter_beat_note() {
        let events = parse("5=").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.pitch, JianPuPitch::Five);
        assert_eq!(n.duration, 1);
    }

    #[test]
    fn parses_octave_up() {
        let events = parse("1'").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, 1);
    }

    #[test]
    fn parses_two_octaves_up() {
        let events = parse("1''").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, 2);
    }

    #[test]
    fn parses_octave_down() {
        let events = parse("1,").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, -1);
    }

    #[test]
    fn parses_two_octaves_down() {
        let events = parse("1,,").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.octave, -2);
    }

    #[test]
    fn rejects_mixed_octave_markers() {
        assert!(parse("1',").is_err());
    }

    #[test]
    fn parses_tie_group() {
        let events = parse("(23)").unwrap();
        assert_eq!(events.len(), 2);
        assert!(note(&events, 0).tie);
        assert!(!note(&events, 1).tie);
    }

    #[test]
    fn parses_concatenated_notes() {
        let events = parse("505").unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn parses_standalone_extension() {
        let events = parse("2 - - -").unwrap();
        assert!(matches!(events[0].value, ScoreEvent::Note(_)));
        assert!(matches!(events[1].value, ScoreEvent::Extension));
        assert!(matches!(events[2].value, ScoreEvent::Extension));
        assert!(matches!(events[3].value, ScoreEvent::Extension));
    }

    #[test]
    fn parses_extension_suffix() {
        let events = parse("1---").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.duration, 16);
    }

    #[test]
    fn parses_rest() {
        let events = parse("0").unwrap();
        let r = rest(&events, 0);
        assert_eq!(r.duration, 4);
    }

    #[test]
    fn parses_half_beat_rest() {
        let events = parse("0_").unwrap();
        let r = rest(&events, 0);
        assert_eq!(r.duration, 2);
    }

    #[test]
    fn parses_sequence() {
        let events = parse("1 2_ 3").unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn parses_dotted_half_beat_note() {
        let events = parse("1_.").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.duration, 3);
        assert!(n.dotted);
    }

    #[test]
    fn parses_dotted_full_beat_note() {
        let events = parse("1.").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.duration, 6);
        assert!(n.dotted);
    }

    #[test]
    fn parses_dotted_note_with_lower_octave() {
        let events = parse("1_,.").unwrap();
        let n = note(&events, 0);
        assert_eq!(n.duration, 3);
        assert_eq!(n.octave, -1);
        assert!(n.dotted);
    }

    #[test]
    fn parses_dotted_half_beat_rest() {
        let events = parse("0_.").unwrap();
        let r = rest(&events, 0);
        assert_eq!(r.duration, 3);
        assert!(r.dotted);
    }

    #[test]
    fn rejects_dash_suffix_on_rest() {
        use crate::error::ErrorKind;
        let err = parse("0---").unwrap_err();
        assert_eq!(err.kind, ErrorKind::DashAfterRest);
    }

    #[test]
    fn rejects_dash_suffix_on_rest_in_group() {
        use crate::error::ErrorKind;
        let err = parse("(0-1)").unwrap_err();
        assert_eq!(err.kind, ErrorKind::DashAfterRest);
    }

    #[test]
    fn parses_repeated_quarter_rests() {
        let events = parse("0 0 0 0").unwrap();
        assert_eq!(events.len(), 4);
        for i in 0..4 {
            assert_eq!(rest(&events, i).duration, 4);
        }
    }

    #[test]
    fn rejects_dotted_quarter_beat_note() {
        assert!(parse("1=.").is_err());
    }

    #[test]
    fn non_dotted_note_has_dotted_false() {
        let events = parse("1_").unwrap();
        let n = note(&events, 0);
        assert!(!n.dotted);
    }

    #[test]
    fn rejects_single_note_group() {
        assert!(parse("(3)").is_err());
        assert!(parse("(5)").is_err());
    }

    #[test]
    fn rejects_single_note_cross_measure_group() {
        let mut state = GroupParseState::default();
        parse_with_state("(1", &mut state).unwrap();
        assert!(parse_with_state(")", &mut state).is_err());
    }

    #[test]
    fn parses_group_followed_by_notes() {
        let events = parse("(12)31").unwrap();
        assert_eq!(events.len(), 4);
        assert!(note(&events, 0).tie);
        assert!(!note(&events, 1).tie);
    }

    #[test]
    fn parses_open_group_at_end_of_token() {
        let mut state = GroupParseState::default();
        let events = parse_with_state("111(1", &mut state).unwrap();
        assert_eq!(events.len(), 4);
        assert!(note(&events, 3).tie);
        assert!(state.open);
    }

    #[test]
    fn parses_cross_measure_group_continuation() {
        let mut state = GroupParseState {
            open: true,
            open_note_count: 1,
        };
        let events = parse_with_state("2)345", &mut state).unwrap();
        assert_eq!(events.len(), 4);
        assert!(!note(&events, 0).tie);
        assert!(!state.open);
    }

    #[test]
    fn cross_measure_group_sets_tie_on_opening_note() {
        let mut state = GroupParseState::default();
        parse_with_state("111(1", &mut state).unwrap();
        let events = parse_with_state("2)345", &mut state).unwrap();
        assert!(note(&events, 0).pitch == JianPuPitch::Two);
        assert!(!note(&events, 0).tie);
    }

    #[test]
    fn open_group_continues_across_spaced_tokens_in_same_measure() {
        let mut state = GroupParseState::default();
        parse_with_state("(6", &mut state).unwrap();
        parse_with_state("-", &mut state).unwrap();
        let events = parse_with_state("7", &mut state).unwrap();
        assert!(state.open);
        assert_eq!(events.len(), 1);
        assert!(note(&events, 0).pitch == JianPuPitch::Seven);
        assert!(note(&events, 0).tie);
    }

    #[test]
    fn open_group_closes_on_spaced_tokens_across_measures() {
        let mut state = GroupParseState::default();
        parse_with_state("(6", &mut state).unwrap();
        parse_with_state("-", &mut state).unwrap();
        parse_with_state("7", &mut state).unwrap();
        parse_with_state("-", &mut state).unwrap();
        let events = parse_with_state("7)", &mut state).unwrap();
        assert!(!state.open);
        assert_eq!(events.len(), 1);
        assert!(note(&events, 0).pitch == JianPuPitch::Seven);
        assert!(!note(&events, 0).tie);
    }
}
