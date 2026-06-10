use super::note_head::NoteHead;
use super::{parse_timed_line, GroupStack, LexContext};
use crate::ast::parsed::ScoreEvent;

#[test]
fn parses_spaced_notes() {
    let events =
        parse_timed_line::<NoteHead>("5 0 5", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 3);
}

#[test]
fn parses_single_note() {
    let events =
        parse_timed_line::<NoteHead>("1", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 1);
    matches!(events[0].value, ScoreEvent::Note(_));
}

#[test]
fn parses_rest() {
    let events =
        parse_timed_line::<NoteHead>("0", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].value, ScoreEvent::Rest(_)));
}

#[test]
fn parses_extension() {
    let events =
        parse_timed_line::<NoteHead>("5 -", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[1].value, ScoreEvent::Extension));
}

#[test]
fn parses_bpm_directive() {
    let events =
        parse_timed_line::<NoteHead>("bpm=120", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].value, ScoreEvent::BpmChange(120)));
}

#[test]
fn parses_time_signature() {
    let events =
        parse_timed_line::<NoteHead>("3/4", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].value,
        ScoreEvent::TimeSignatureChange {
            numerator: 3,
            denominator: 4
        }
    ));
}

#[test]
fn parses_key_change() {
    let events =
        parse_timed_line::<NoteHead>("1=C4", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].value, ScoreEvent::KeyChange(_)));
}

#[test]
fn parses_closed_group_applies_tie() {
    use crate::ast::parsed::ParsedNote;
    let events =
        parse_timed_line::<NoteHead>("(5 6)", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 2);
    // First note should be tied (group_continuation > 0).
    if let ScoreEvent::Note(ParsedNote {
        tie,
        group_membership,
        group_continuation,
        ..
    }) = &events[0].value
    {
        assert!(*tie);
        assert_eq!(*group_membership, 1);
        assert_eq!(*group_continuation, 1);
    } else {
        panic!("expected Note");
    }
    // Last note: in group but not tied.
    if let ScoreEvent::Note(ParsedNote {
        tie,
        group_membership,
        group_continuation,
        ..
    }) = &events[1].value
    {
        assert!(!*tie);
        assert_eq!(*group_membership, 1);
        assert_eq!(*group_continuation, 0);
    } else {
        panic!("expected Note");
    }
}

#[test]
fn parses_open_group_all_notes_tied() {
    use crate::ast::parsed::ParsedNote;
    let mut stack = GroupStack::default();
    let events = parse_timed_line::<NoteHead>("(5 6", 0, &mut stack, LexContext::Notes).unwrap();
    assert_eq!(events.len(), 2);
    assert!(stack.is_open(), "stack should still have an open frame");
    for ev in &events {
        if let ScoreEvent::Note(ParsedNote {
            tie,
            group_continuation,
            ..
        }) = &ev.value
        {
            assert!(*tie);
            assert!(*group_continuation > 0);
        } else {
            panic!("expected Note");
        }
    }
}

#[test]
fn parses_spaced_nested_outer_group() {
    // ((1 1) 5 5) should parse to 4 events
    let events = parse_timed_line::<NoteHead>(
        "((1 1) 5 5)",
        0,
        &mut GroupStack::default(),
        LexContext::Notes,
    )
    .unwrap();
    assert_eq!(events.len(), 4);
}

#[test]
fn rejects_single_note_group() {
    // (3) should be rejected — groups must have at least 2 notes
    assert!(
        parse_timed_line::<NoteHead>("(3)", 0, &mut GroupStack::default(), LexContext::Notes)
            .is_err()
    );
}

#[test]
fn cross_bar_open_group_stays_on_stack() {
    // Open group spanning bars: first bar has unclosed group
    let mut stack = GroupStack::default();
    parse_timed_line::<NoteHead>("((1 1", 0, &mut stack, LexContext::Notes).unwrap();
    assert!(stack.is_open());
}

#[test]
fn cross_bar_nested_groups_close_correctly() {
    // Open group spanning bars: second bar closes both
    let mut stack = GroupStack::default();
    parse_timed_line::<NoteHead>("((1 1", 0, &mut stack, LexContext::Notes).unwrap();
    let events = parse_timed_line::<NoteHead>("5 5))", 0, &mut stack, LexContext::Notes).unwrap();
    assert!(!stack.is_open());
    assert_eq!(events.len(), 2);
}

#[test]
fn cross_bar_outer_and_inner() {
    let mut stack = GroupStack::default();
    // Open outer + inner group with some notes
    let _ = parse_timed_line::<NoteHead>("(1 1 (2", 0, &mut stack, LexContext::Notes).unwrap();
    // Close inner then outer
    let events = parse_timed_line::<NoteHead>("3))", 0, &mut stack, LexContext::Notes).unwrap();
    assert!(!stack.is_open());
    assert_eq!(events.len(), 1);
}

#[test]
fn note_duration_suffix_dash_extends() {
    use crate::ast::parsed::{ParsedNote, ScoreEvent};
    // "5-" should produce a note with duration 8 (4 base + 4 per dash).
    let events =
        parse_timed_line::<NoteHead>("5-", 0, &mut GroupStack::default(), LexContext::Notes)
            .unwrap();
    assert_eq!(events.len(), 1);
    if let ScoreEvent::Note(ParsedNote { duration, .. }) = &events[0].value {
        assert_eq!(*duration, 8);
    } else {
        panic!("expected Note");
    }
}
