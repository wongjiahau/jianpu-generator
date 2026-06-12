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
            ScoreEvent::Rest(ParsedRest {
                duration,
                dotted,
                group_membership: 0,
                group_continuation: 0,
            })
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
                slur_group_close_at_duration: None,
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
mod tests;
