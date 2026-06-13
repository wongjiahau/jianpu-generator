use super::*;
use crate::ast::parsed::NoteName;
use crate::parser;

fn parse_and_group(input: &str) -> Score {
    let doc = parser::parse(input, "test.jianpu").unwrap();
    group(doc).unwrap()
}

fn parse_and_group_err(input: &str) -> JianPuError {
    let doc = parser::parse(input, "test.jianpu").unwrap();
    match group(doc) {
        Err(e) => e,
        Ok(_) => panic!("expected group() to return Err, but it returned Ok"),
    }
}

fn first_part_notes(score: &Score, measure_idx: usize) -> &Vec<NoteEvent> {
    &score.measures[measure_idx].parts[0].slice().notes.events
}

#[test]
fn groups_four_four_into_single_measure() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
    ));
    assert_eq!(score.measures.len(), 1);
    assert_eq!(first_part_notes(&score, 0).len(), 4);
}

#[test]
fn splits_into_two_measures_at_bar_boundary() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
    ));
    assert_eq!(score.measures.len(), 2);
}

#[test]
fn extension_adds_to_previous_note_duration() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1- 3 4\na - b\n",
    ));
    match &first_part_notes(&score, 0)[0] {
        NoteEvent::Note(n) => assert_eq!(n.duration, 8),
        NoteEvent::Rest(_) | NoteEvent::Chord(_) => panic!("expected Note"),
    }
}

#[test]
fn rejects_standalone_dash_after_rest() {
    use crate::error::ErrorKind;
    let err = parse_and_group_err(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n0 - - -\n_\n",
    ));
    assert_eq!(err.kind, ErrorKind::DashAfterRest);
}

#[test]
fn first_measure_has_bpm_some() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
    ));
    assert_eq!(score.measures[0].bpm, Some(120));
}

#[test]
fn bpm_change_sets_some_on_next_measure() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(bpm=90)\n5 6 7 1\ne f g h\n",
    ));
    assert_eq!(score.measures[0].bpm, Some(120));
    assert_eq!(score.measures[1].bpm, Some(90));
}

#[test]
fn unchanged_bpm_is_none_on_second_measure() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
    ));
    assert_eq!(score.measures[0].bpm, Some(120));
    assert_eq!(score.measures[1].bpm, None);
}

#[test]
fn key_change_propagates() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=G4 bpm=120)\n1 2 3 4\na b c d\n",
    ));
    assert_eq!(
        score.measures[0].key.as_ref().unwrap().note.name,
        NoteName::G
    );
}

#[test]
fn row_height_defaults_to_24() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
    ));
    assert_eq!(score.metadata.row_height, 24);
}

#[test]
fn max_columns_defaults_to_28() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
    ));
    assert_eq!(score.metadata.max_columns, 28);
}

#[test]
fn half_beat_notes_accumulate_correctly() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1_ 2_ 3_ 4_ 5_ 6_ 7_ 1_\na b c d e f g h\n",
    ));
    assert_eq!(score.measures.len(), 1);
}

#[test]
fn overflow_note_errors() {
    // The interleaved parser validates beats per bar — overfull bar is rejected at parse time.
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1_ 1_ 1_ 1_ 1_ 1_ 1_ 1\na b c d e f g h\n",
    );
    assert!(
        parser::parse(input, "test.jianpu").is_err(),
        "expected parse error for overfull measure",
    );
}

#[test]
fn bpm_change_creates_new_measure() {
    // Bar 1 (bpm=120): 1 2 3 4; Bar 2 (bpm=90): 5 6 7 1
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(bpm=90)\n5 6 7 1\ne f g h\n",
    ));
    assert_eq!(score.measures.len(), 2);
    assert_eq!(score.measures[0].bpm, Some(120));
    assert_eq!(first_part_notes(&score, 0).len(), 4);
    assert_eq!(score.measures[1].bpm, Some(90));
    assert_eq!(first_part_notes(&score, 1).len(), 4);
}

#[test]
fn two_part_score_has_two_part_slices_per_measure() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nSoprano = notes\nAlto = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n5 6 7 1\n",
    );
    let doc = parser::parse(input, "test.jianpu").unwrap();
    let score = group(doc).unwrap();
    assert_eq!(score.measures.len(), 1);
    assert_eq!(score.measures[0].parts.len(), 2);
    assert_eq!(
        score.measures[0].parts[0].name(),
        Some(&"Soprano".to_string())
    );
    assert_eq!(score.measures[0].parts[1].name(), Some(&"Alto".to_string()));
}

#[test]
fn label_directive_propagates_to_measure() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n",
    ));
    assert_eq!(score.measures[0].label, Some("Verse 1".to_string()));
}

#[test]
fn label_is_none_when_not_declared() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
    ));
    assert_eq!(score.measures[0].label, None);
}

#[test]
fn label_does_not_persist_to_next_measure() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n\n5 6 7 1\n",
    ));
    assert_eq!(score.measures[0].label, Some("Verse 1".to_string()));
    assert_eq!(score.measures[1].label, None);
}

#[test]
fn label_on_second_measure_not_first() {
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n\n(label=\"Chorus\")\n5 6 7 1\n",
    ));
    assert_eq!(score.measures[0].label, None);
    assert_eq!(score.measures[1].label, Some("Chorus".to_string()));
}

#[test]
fn lyrics_distributed_per_measure() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
    );
    let doc = parser::parse(input, "test.jianpu").unwrap();
    let score = group(doc).unwrap();
    assert_eq!(score.measures.len(), 2);
    let m0_lyrics = score.measures[0].parts[0].slice().lyrics.as_ref().unwrap();
    let m1_lyrics = score.measures[1].parts[0].slice().lyrics.as_ref().unwrap();
    assert_eq!(m0_lyrics.syllables.len(), 4);
    assert_eq!(m1_lyrics.syllables.len(), 4);
}

#[test]
fn standalone_tie_marker_after_extension_that_flushes_measure() {
    // `(6---` fills a 4/4 measure exactly; `7)` closes the cross-measure group.
    // The outgoing tie on 6 must carry into the next measure.
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n(6---\n\n7) 0 0 0\n",
    ));
    let notes_m0 = first_part_notes(&score, 0);
    match notes_m0.last().unwrap() {
        NoteEvent::Note(n) => assert!(n.tie, "note 6 in measure 0 should be tied"),
        NoteEvent::Rest(_) | NoteEvent::Chord(_) => panic!("expected Note"),
    }
}

#[test]
fn standalone_tie_marker_sets_tie_on_preceding_note() {
    // `(6-7)` means note 6 extended by one beat, slurred into note 7
    let score = parse_and_group(concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n(6-7) 0\n",
    ));
    let notes = first_part_notes(&score, 0);
    match &notes[0] {
        NoteEvent::Note(n) => {
            assert_eq!(n.duration, 8, "note 6 should be extended to 2 beats");
            assert!(n.tie, "note 6 should have tie=true");
        }
        NoteEvent::Rest(_) | NoteEvent::Chord(_) => panic!("expected Note"),
    }
    match &notes[1] {
        NoteEvent::Note(n) => assert_eq!(n.pitch, crate::ast::parsed::JianPuPitch::Seven),
        NoteEvent::Rest(_) | NoteEvent::Chord(_) => panic!("expected Note"),
    }
}

#[test]
fn chord_extend_with_no_preceding_event_reports_token_span() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nc = chord\nn = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n- 1 - -\n1 2 3 4\n",
    );
    let err = parse_and_group_err(input);
    assert!(
        err.span.start > 0 || err.span.end > 0,
        "expected a non-zero span for the '-' token, got start={} end={}",
        err.span.start,
        err.span.end,
    );
    assert!(err.message.contains("chord extension"));
}

#[test]
fn chord_part_produces_one_chord_event_per_measure() {
    use crate::ast::grouped::PartRow;
    let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nchord = chord\nMelody = notes\n\n[score]\n(time=4/4 key=C4 bpm=120)\n1 - - -\n1---\n";
    let doc = parser::parse(input, "test.jianpu").unwrap();
    let score = group(doc).unwrap();
    let measure = &score.measures[0];
    let chord_row = measure
        .parts
        .iter()
        .find(|r| {
            matches!(
                r,
                PartRow::Timed(p) if p.kind == crate::ast::parsed::PartKind::Chord
            )
        })
        .unwrap();
    let slice = chord_row.slice();
    assert_eq!(slice.notes.events.len(), 1);
    match &slice.notes.events[0] {
        NoteEvent::Chord(c) => {
            assert_eq!(c.duration, 16); // 4 tokens * 4 quarter-beats
        }
        NoteEvent::Note(_) | NoteEvent::Rest(_) => panic!("expected Chord event"),
    }
}

#[test]
fn measure_span_covers_first_note_byte_offset() {
    let source = concat!(
        "[metadata]\n",
        "title = \"t\"\n",
        "author = \"a\"\n",
        "\n",
        "[parts]\n",
        "Melody = notes\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
    );
    let score = parse_and_group(source);
    let span = &score.measures[0].source_span;
    let first_note_offset = source.find("1 2 3 4").unwrap();
    assert!(
        span.start <= first_note_offset && first_note_offset < span.end,
        "span {:?} should contain first note offset {}",
        span,
        first_note_offset
    );
}

#[test]
fn second_measure_span_covers_its_first_note() {
    let source = concat!(
        "[metadata]\n",
        "title = \"t\"\n",
        "author = \"a\"\n",
        "\n",
        "[parts]\n",
        "Melody = notes\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "\n",
        "5 6 7 1\n",
    );
    let score = parse_and_group(source);
    assert_eq!(score.measures.len(), 2);
    let span = &score.measures[1].source_span;
    let second_note_offset = source.rfind("5 6 7 1").unwrap();
    assert!(
        span.start <= second_note_offset && second_note_offset < span.end,
        "span {:?} should contain second measure offset {}",
        span,
        second_note_offset
    );
    // Second measure span must not overlap with first
    assert!(span.start > score.measures[0].source_span.start);
}
