use crate::ast::parsed::ScoreEvent;
use crate::error::{JianPuError, Span, Spanned};

const HALF_BAR_BOUNDARY: u32 = 8;

pub fn validate_measure_grouping(
    events: &[Spanned<ScoreEvent>],
    time_num: u8,
    time_den: u8,
) -> Result<(), JianPuError> {
    if time_num != 4 || time_den != 4 {
        return Ok(());
    }

    let mut pos = 0u32;
    let mut index = 0usize;
    while index < events.len() {
        let Some(event) = events.get(index) else {
            break;
        };

        match &event.value {
            ScoreEvent::Note(note) => {
                let total_duration = timed_cluster_duration(events, index);
                let head_duration = timed_head_duration(events, index);
                if note.group_membership == 0
                    && pos > 0
                    && pos < HALF_BAR_BOUNDARY
                    && pos + head_duration > HALF_BAR_BOUNDARY
                {
                    return Err(half_bar_error(&event.span));
                }

                if is_dotted_eighth_at_beat_start(note.dotted, note.duration, pos) {
                    let next_timed = next_timed_index(events, index);
                    validate_dotted_eighth_tail(events, next_timed, &event.span)?;
                    pos += note.duration + 1;
                    index = next_timed.map(|next| next + 1).unwrap_or(events.len());
                    continue;
                }

                pos += total_duration;
                index += timed_cluster_len(events, index);
            }
            ScoreEvent::Chord(chord) => {
                let total_duration = timed_cluster_duration(events, index);
                let head_duration = timed_head_duration(events, index);
                if chord.group_membership == 0
                    && pos > 0
                    && pos < HALF_BAR_BOUNDARY
                    && pos + head_duration > HALF_BAR_BOUNDARY
                {
                    return Err(half_bar_error(&event.span));
                }

                if is_dotted_eighth_at_beat_start(chord.dotted, chord.duration, pos) {
                    let next_timed = next_timed_index(events, index);
                    validate_dotted_eighth_tail(events, next_timed, &event.span)?;
                    pos += chord.duration + 1;
                    index = next_timed.map(|next| next + 1).unwrap_or(events.len());
                    continue;
                }

                pos += total_duration;
                index += timed_cluster_len(events, index);
            }
            ScoreEvent::Rest(rest) => {
                let total_duration = timed_cluster_duration(events, index);
                let head_duration = timed_head_duration(events, index);
                if pos > 0 && pos < HALF_BAR_BOUNDARY && pos + head_duration > HALF_BAR_BOUNDARY {
                    return Err(half_bar_error(&event.span));
                }

                if is_dotted_eighth_at_beat_start(rest.dotted, rest.duration, pos) {
                    let next_timed = next_timed_index(events, index);
                    validate_dotted_eighth_tail(events, next_timed, &event.span)?;
                    pos += rest.duration + 1;
                    index = next_timed.map(|next| next + 1).unwrap_or(events.len());
                    continue;
                }

                pos += total_duration;
                index += timed_cluster_len(events, index);
            }
            _ => index += 1,
        }
    }

    Ok(())
}

fn timed_head_duration(events: &[Spanned<ScoreEvent>], start: usize) -> u32 {
    match events.get(start).map(|e| &e.value) {
        Some(ScoreEvent::Note(note)) => note.duration,
        Some(ScoreEvent::Chord(chord)) => chord.duration,
        Some(ScoreEvent::Rest(rest)) => rest.duration,
        _ => 0,
    }
}

fn timed_cluster_duration(events: &[Spanned<ScoreEvent>], start: usize) -> u32 {
    let Some(event) = events.get(start) else {
        return 0;
    };
    let mut duration = match &event.value {
        ScoreEvent::Note(note) => note.duration,
        ScoreEvent::Chord(chord) => chord.duration,
        ScoreEvent::Rest(rest) => rest.duration,
        _ => return 0,
    };

    let mut index = start + 1;
    while let Some(event) = events.get(index) {
        if matches!(event.value, ScoreEvent::Extension) {
            duration += 4;
            index += 1;
        } else {
            break;
        }
    }

    duration
}

fn timed_cluster_len(events: &[Spanned<ScoreEvent>], start: usize) -> usize {
    let mut len = 1usize;
    let mut index = start + 1;
    while let Some(event) = events.get(index) {
        if matches!(event.value, ScoreEvent::Extension) {
            len += 1;
            index += 1;
        } else {
            break;
        }
    }
    len
}

fn next_timed_index(events: &[Spanned<ScoreEvent>], start: usize) -> Option<usize> {
    let mut index = start + timed_cluster_len(events, start);
    while index < events.len() {
        if let Some(event) = events.get(index) {
            if matches!(
                event.value,
                ScoreEvent::Note(_) | ScoreEvent::Chord(_) | ScoreEvent::Rest(_)
            ) {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn is_dotted_eighth_at_beat_start(dotted: bool, duration: u32, pos: u32) -> bool {
    dotted && duration == 3 && pos % 4 == 0
}

fn validate_dotted_eighth_tail(
    events: &[Spanned<ScoreEvent>],
    next_timed: Option<usize>,
    span: &Span,
) -> Result<(), JianPuError> {
    let Some(next_index) = next_timed else {
        return Err(dotted_eighth_error(span));
    };
    let Some(event) = events.get(next_index) else {
        return Err(dotted_eighth_error(span));
    };

    let tail_duration = match &event.value {
        ScoreEvent::Note(note) => note.duration,
        ScoreEvent::Chord(chord) => chord.duration,
        ScoreEvent::Rest(rest) => rest.duration,
        _ => return Err(dotted_eighth_error(span)),
    };

    if tail_duration == 1 {
        Ok(())
    } else {
        Err(dotted_eighth_error(span))
    }
}

fn half_bar_error(span: &Span) -> JianPuError {
    JianPuError::new(
        span.clone(),
        "note/rest crosses the half-bar boundary (beat 2→3); use a beam group or tie to show the split"
            .to_string(),
    )
}

fn dotted_eighth_error(span: &Span) -> JianPuError {
    JianPuError::new(
        span.clone(),
        "dotted eighth must be followed by a sixteenth note or rest".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use crate::parser;

    fn parse_score(notes_line: &str) -> Result<(), crate::error::JianPuError> {
        let input = format!(
            concat!(
                "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
                "[score]\n(time=4/4 key=C4 bpm=120)\n",
                "{notes_line}"
            ),
            notes_line = notes_line
        );
        parser::parse(&input, "test.jianpu").map(|_| ())
    }

    #[test]
    fn chord_half_bar_boundary_validation_matches_notes() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "c = chord\n",
            "n = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1. 2. 3_ 4_\n",
            "1 2 3 4\n",
        );
        assert!(parser::parse(input, "t.jianpu").is_err());
    }

    #[test]
    fn rejects_half_bar_crossing() {
        let err = parse_score("1. 2. 3_ 4_\n").unwrap_err();
        assert!(err.message.contains("half-bar boundary"));
    }

    #[test]
    fn rejects_half_bar_crossing_on_half_note() {
        let err = parse_score("1 2- 0_ 0_\n").unwrap_err();
        assert!(err.message.contains("half-bar boundary"));
    }

    #[test]
    fn accepts_half_bar_split_with_beam_group() {
        assert!(parse_score("1. (2_ 2_) 3_ 4_ 0_\n").is_ok());
    }

    #[test]
    fn rejects_dotted_eighth_without_tail_group() {
        use super::validate_measure_grouping;
        use crate::parser::score::token_parser;
        let bar = "1_. 2_ 3_ 4_ 5_ 6_ 7_ 0=";
        let events = token_parser::parse_notes_line(bar, 0, &mut Default::default()).unwrap();
        let err = validate_measure_grouping(&events, 4, 4).unwrap_err();
        assert!(err.message.contains("dotted eighth"));
    }

    #[test]
    fn accepts_dotted_eighth_with_sixteenth_tail() {
        assert!(parse_score("1_. 2= 3_ 4_ 5_ 6_ 7_ 1_\n").is_ok());
    }

    #[test]
    fn rejects_dotted_eighth_rest_without_tail_group() {
        let err = parse_score("0_. 1_ 2_ 3_ 4_ 5_ 6_ 0=\n").unwrap_err();
        assert!(err.message.contains("dotted eighth"));
    }

    #[test]
    fn accepts_extension_notes_that_start_on_beat_three() {
        assert!(parse_score("(6- 7-)\n").is_ok());
    }

    #[test]
    fn skips_validation_for_non_four_four() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
            "[score]\n(time=3/4 key=C4 bpm=120)\n",
            "1 2 3\n",
        );
        assert!(parser::parse(input, "test.jianpu").is_ok());
    }

    #[test]
    fn allows_half_bar_crossing_inside_beam_group() {
        use super::validate_measure_grouping;
        use crate::parser::score::token_parser;
        let mut state = token_parser::GroupStack::default();
        let bar1 = "5_ 5_ 5_ 5= 5= 5_ 3_ 2_ (3_";
        let _ = token_parser::parse_notes_line(bar1, 0, &mut state).unwrap();
        let bar2 = "3_) (1_1-) 0_ 1= 1=";
        let events = token_parser::parse_notes_line(bar2, 0, &mut state).unwrap();
        validate_measure_grouping(&events, 4, 4).expect("grouped crossing should be allowed");
    }
}
