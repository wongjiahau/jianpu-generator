#![allow(clippy::indexing_slicing)]

use crate::ast::parsed::ScoreEvent;
use crate::error::{JianPuError, Spanned};
use crate::parser::score::timed_parser::{parse_timed_line, ChordHead, LexContext, NoteHead};

pub use crate::parser::score::timed_parser::GroupStack;

pub fn parse_notes_line(
    line: &str,
    base_offset: usize,
    stack: &mut GroupStack,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    parse_timed_line::<NoteHead>(line, base_offset, stack, LexContext::Notes)
}

pub fn parse_chord_line(
    line: &str,
    base_offset: usize,
    stack: &mut GroupStack,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    parse_timed_line::<ChordHead>(line, base_offset, stack, LexContext::Chords)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::{JianPuPitch, ParsedNote, ParsedRest};

    fn parse(input: &str) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
        parse_notes_line(input, 0, &mut GroupStack::default())
    }

    fn parse_with_state(
        input: &str,
        state: &mut GroupStack,
    ) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
        parse_notes_line(input, 0, state)
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
        let mut state = GroupStack::default();
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
        let mut state = GroupStack::default();
        let events = parse_with_state("111(1", &mut state).unwrap();
        assert_eq!(events.len(), 4);
        assert!(note(&events, 3).tie);
        assert!(state.is_open());
    }

    #[test]
    fn parses_cross_measure_group_continuation() {
        let mut state = GroupStack::default();
        // Open a group with one note so state.is_open() is true
        parse_with_state("(1", &mut state).unwrap();
        let events = parse_with_state("2)345", &mut state).unwrap();
        assert_eq!(events.len(), 4);
        assert!(!note(&events, 0).tie);
        assert!(!state.is_open());
    }

    #[test]
    fn cross_measure_group_sets_tie_on_opening_note() {
        let mut state = GroupStack::default();
        parse_with_state("111(1", &mut state).unwrap();
        let events = parse_with_state("2)345", &mut state).unwrap();
        assert!(note(&events, 0).pitch == JianPuPitch::Two);
        assert!(!note(&events, 0).tie);
    }

    #[test]
    fn open_group_continues_across_spaced_tokens_in_same_measure() {
        let mut state = GroupStack::default();
        parse_with_state("(6", &mut state).unwrap();
        parse_with_state("-", &mut state).unwrap();
        let events = parse_with_state("7", &mut state).unwrap();
        assert!(state.is_open());
        assert_eq!(events.len(), 1);
        assert!(note(&events, 0).pitch == JianPuPitch::Seven);
        assert!(note(&events, 0).tie);
    }

    #[test]
    fn parses_nested_tie_group() {
        let mut state = GroupStack::default();
        let events1 = parse_with_state("(3=", &mut state).unwrap();
        assert_eq!(events1.len(), 1);
        assert!(state.is_open());
        let events2 = parse_with_state("(2_1_))", &mut state).unwrap();
        assert_eq!(events2.len(), 2);
        assert!(!state.is_open());
        let events = parse("(3= (2_1_))").unwrap();
        assert_eq!(events.len(), 3);
        let n0 = note(&events, 0);
        assert_eq!(n0.pitch, JianPuPitch::Three);
        assert_eq!(n0.duration, 1);
        assert_eq!(n0.group_membership, 1);
        assert_eq!(n0.group_continuation, 1);
        let n1 = note(&events, 1);
        assert_eq!(n1.pitch, JianPuPitch::Two);
        assert_eq!(n1.duration, 2);
        assert_eq!(n1.group_membership, 2);
        assert_eq!(n1.group_continuation, 2);
        let n2 = note(&events, 2);
        assert_eq!(n2.pitch, JianPuPitch::One);
        assert_eq!(n2.duration, 2);
        assert_eq!(n2.group_membership, 2);
        assert_eq!(n2.group_continuation, 0);
    }

    #[test]
    fn open_group_closes_on_spaced_tokens_across_measures() {
        let mut state = GroupStack::default();
        parse_with_state("(6", &mut state).unwrap();
        parse_with_state("-", &mut state).unwrap();
        parse_with_state("7", &mut state).unwrap();
        parse_with_state("-", &mut state).unwrap();
        let events = parse_with_state("7)", &mut state).unwrap();
        assert!(!state.is_open());
        assert_eq!(events.len(), 1);
        assert!(note(&events, 0).pitch == JianPuPitch::Seven);
        assert!(!note(&events, 0).tie);
    }
}
