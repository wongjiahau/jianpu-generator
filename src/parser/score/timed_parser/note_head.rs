use super::TimedUnitHead;
use crate::ast::parsed::{JianPuPitch, ParsedNote, ParsedRest, ScoreEvent};
use crate::error::{JianPuError, Span};

pub struct NoteHead {
    pitch: JianPuPitch,
    is_rest: bool,
}

impl TimedUnitHead for NoteHead {
    fn parse_head(
        chars: &[char],
        start: usize,
        span: &Span,
    ) -> Result<(Self, usize, bool), JianPuError> {
        let pitch_char = chars[start];
        if !matches!(pitch_char, '0'..='7') {
            let pos = span.start + byte_offset_at_char_index_from_chars(chars, start);
            return Err(JianPuError::new(
                Span::new(pos, pos + pitch_char.len_utf8()),
                format!("expected pitch digit (0-7), got: {pitch_char}"),
            ));
        }
        let is_rest = pitch_char == '0';
        Ok((
            NoteHead {
                pitch: if is_rest {
                    JianPuPitch::One
                } else {
                    pitch_char_to_jianpu(pitch_char)
                },
                is_rest,
            },
            start + 1,
            is_rest,
        ))
    }

    fn head_boundary(chars: &[char], i: usize) -> bool {
        matches!(chars[i], '0'..='7')
    }

    fn allows_octave_suffixes() -> bool {
        true
    }

    fn to_event(
        head: &Self,
        duration: u32,
        dotted: bool,
        octave: i8,
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
            ScoreEvent::Note(ParsedNote {
                pitch: head.pitch.clone(),
                octave,
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

fn byte_offset_at_char_index_from_chars(chars: &[char], char_index: usize) -> usize {
    chars[..char_index].iter().map(|c| c.len_utf8()).sum()
}
