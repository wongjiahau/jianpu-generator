use super::*;
use crate::ast::parsed::{Accidental, JianPuPitch, ParsedChordNote, ScoreEvent, TriadQuality};

use super::test_helpers::{chord_track, decl, notes_track, parse};

#[test]
fn chord_line_parses_spaced_slur_group() {
    let input = concat!(
        "[metadata]\n",
        "title = \"t\"\n",
        "author = \"a\"\n",
        "\n",
        "[parts]\n",
        "Chord = chord\n",
        "Melody = notes\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "(1 - 6m -)\n",
        "1 1 5 5\n",
    );
    let doc = crate::parser::parse(input, "test.jianpu").unwrap();
    let chord_events: Vec<_> = chord_track(&doc.tracks, "Chord")
        .score
        .events
        .iter()
        .filter(|e| matches!(e.value, ScoreEvent::Chord(_)))
        .collect();
    assert_eq!(chord_events.len(), 2, "expected chord 1 and 6m in group");
}

#[test]
fn chord_column_events_are_parsed() {
    let declarations = vec![decl("main", PartKind::Chord), decl("main", PartKind::Notes)];
    let content = "(time=4/4 key=C4 bpm=120)\n1 - - -\n1---\n";
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(tracks.len(), 2);
    let chord = chord_track(&tracks, "main");
    let events: Vec<_> = chord.score.events.iter().map(|e| &e.value).collect();
    assert_eq!(
        events[0],
        &ScoreEvent::Chord(ParsedChordNote {
            degree: JianPuPitch::One,
            accidental: Accidental::Natural,
            triad: TriadQuality::Major,
            extension: None,
            bass: None,
            duration: 4,
            tie: false,
            group_membership: 0,
            group_continuation: 0,
            dotted: false,
            slur_group_close_at_duration: None,
        })
    );
    assert!(matches!(events[1], ScoreEvent::Extension));
    assert_eq!(notes_track(&tracks, "main").score.events.len(), 4);
}

#[test]
fn single_unnamed_part_no_lyrics() {
    let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
    let declarations = vec![decl("", PartKind::Notes)];
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(tracks.len(), 1);
    let notes = notes_track(&tracks, "");
    assert!(notes.lyrics.is_none());
    assert_eq!(notes.score.events.len(), 7);
}

#[test]
fn single_part_with_lyrics() {
    let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\ndo re mi fa\n";
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(tracks.len(), 1);
    let notes = notes_track(&tracks, "");
    assert!(notes.lyrics.is_some());
    assert_eq!(notes.lyrics.as_ref().unwrap().syllables.len(), 4);
}

#[test]
fn two_parts_two_bars() {
    let content = concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "5 6 7 1\n",
        "\n",
        "1 2 3 4\n",
        "5 6 7 1\n",
    );
    let declarations = vec![
        decl("Soprano", PartKind::Notes),
        decl("Alto", PartKind::Notes),
    ];
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(notes_track(&tracks, "Soprano").score.events.len(), 11);
    assert_eq!(notes_track(&tracks, "Alto").score.events.len(), 8);
}

#[test]
fn rejects_too_many_lines_in_group() {
    let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\nextra line\n";
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let err = parse(content, 0, &declarations).unwrap_err();
    assert!(err.message.contains("lines") && err.message.contains("expected"));
}

#[test]
fn underscore_on_lyrics_line_means_no_lyrics_for_that_bar() {
    let content = concat!(
        "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
        "\n",
        "5 6 7 1\n",
        "_\n",
    );
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(
        notes_track(&tracks, "")
            .lyrics
            .as_ref()
            .unwrap()
            .syllables
            .len(),
        4
    );
}

#[test]
fn rejects_too_few_lyrics_syllables_for_notes() {
    let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c\n";
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let err = parse(content, 0, &declarations).unwrap_err();
    assert!(
        err.message
            .contains("lyrics has 3 syllables but notes need 4"),
        "got: {}",
        err.message
    );
}

#[test]
fn rejects_too_many_lyrics_syllables_for_notes() {
    let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d e\n";
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let err = parse(content, 0, &declarations).unwrap_err();
    assert!(
        err.message
            .contains("lyrics has 5 syllables but notes need 4"),
        "got: {}",
        err.message
    );
}

#[test]
fn cross_measure_paren_group_parses() {
    let content = concat!("(time=4/4 key=C4 bpm=120)\n", "111(1\n", "\n", "2)345\n",);
    let declarations = vec![decl("", PartKind::Notes)];
    let tracks = parse(content, 0, &declarations).unwrap();
    let notes = notes_track(&tracks, "");
    let note_events: Vec<_> = notes
        .score
        .events
        .iter()
        .filter_map(|e| match &e.value {
            ScoreEvent::Note(n) => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(note_events.len(), 8);
    assert!(note_events[3].tie);
    assert!(!note_events[4].tie);
}

#[test]
fn rejects_unclosed_paren_group_at_eof() {
    let content = "(time=4/4 key=C4 bpm=120)\n111(1\n";
    let declarations = vec![decl("", PartKind::Notes)];
    let err = parse(content, 0, &declarations).unwrap_err();
    assert!(err.message.contains("unclosed '(' group"));
}

#[test]
fn tied_notes_share_one_lyric_slot_in_bar() {
    let content = "(time=4/4 key=C4 bpm=120)\n(33) 1 2\na b c\n";
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(
        notes_track(&tracks, "")
            .lyrics
            .as_ref()
            .unwrap()
            .syllables
            .len(),
        3
    );
}

#[test]
fn cross_measure_tie_continuation_needs_fewer_lyrics() {
    let content = concat!(
        "(time=4/4 key=C4 bpm=120)\n0 0 0 (3\na\n",
        "\n",
        "3) 0 0 0\n",
        "_\n",
    );
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let tracks = parse(content, 0, &declarations).unwrap();
    assert_eq!(
        notes_track(&tracks, "")
            .lyrics
            .as_ref()
            .unwrap()
            .syllables
            .len(),
        1
    );
}

#[test]
fn spaced_open_group_cross_measure_lyrics() {
    let content = concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 - 6m -\n",
        "(6- 7-\n",
        "慈 -\n",
        "\n",
        "1 - 6m -\n",
        "7) 1 2 3\n",
        "光 - 光\n",
    );
    let declarations = vec![
        decl("main", PartKind::Chord),
        decl("S1", PartKind::NotesWithLyrics),
    ];
    let tracks = parse(content, 0, &declarations).unwrap();
    let s1 = notes_track(&tracks, "S1");
    assert_eq!(s1.lyrics.as_ref().unwrap().syllables.len(), 5);
}

#[test]
fn rejects_omitted_trailing_lyrics_without_precedent() {
    let content = concat!(
        "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
        "\n",
        "5 6 7 1\n",
    );
    let declarations = vec![decl("", PartKind::NotesWithLyrics)];
    let err = parse(content, 0, &declarations).unwrap_err();
    assert!(
        err.message.contains("expected lyrics line"),
        "got: {}",
        err.message
    );
}

#[test]
fn partial_measure_still_needs_ditto_before_diverging_middle_columns() {
    let content = concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 - 6m -\n",
        "6. 6= 6= 6_ 5_ 3= (2_=2_)\n",
        "a b c d e f g\n",
        "4. 4= 4= 4_ 3_ 1= (2_=2_)\n",
        "\"\n",
        "6- 5-\n",
        "alto lyrics\n",
    );
    let declarations = vec![
        decl("main", PartKind::Chord),
        decl("A1", PartKind::NotesWithLyrics),
        decl("A2", PartKind::NotesWithLyrics),
        decl("S1", PartKind::NotesWithLyrics),
        decl("S2", PartKind::NotesWithLyrics),
    ];
    let tracks = parse(content, 0, &declarations).unwrap();
    let s1 = notes_track(&tracks, "S1");
    assert_eq!(s1.lyrics.as_ref().unwrap().syllables[0].text, "alto");
}

#[test]
fn implicit_trailing_ditto_matches_explicit_ditto() {
    let explicit = concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 - - -\n",
        "1 2 3 4\n",
        "do re mi fa\n",
        "\"\n",
        "\"\n",
    );
    let implicit = concat!(
        "(time=4/4 key=C4 bpm=120)\n",
        "1 - - -\n",
        "1 2 3 4\n",
        "do re mi fa\n",
    );
    let declarations = vec![
        decl("main", PartKind::Chord),
        decl("A", PartKind::NotesWithLyrics),
        decl("B", PartKind::NotesWithLyrics),
    ];
    let explicit_tracks = parse(explicit, 0, &declarations).unwrap();
    let implicit_tracks = parse(implicit, 0, &declarations).unwrap();
    let explicit_a = notes_track(&explicit_tracks, "A");
    let implicit_a = notes_track(&implicit_tracks, "A");
    let explicit_b = notes_track(&explicit_tracks, "B");
    let implicit_b = notes_track(&implicit_tracks, "B");
    assert_eq!(explicit_a.score.events.len(), implicit_a.score.events.len());
    assert_eq!(explicit_b.score.events.len(), implicit_b.score.events.len());
    assert_eq!(
        explicit_a.lyrics.as_ref().unwrap().syllables.len(),
        implicit_a.lyrics.as_ref().unwrap().syllables.len()
    );
    assert_eq!(
        explicit_b.lyrics.as_ref().unwrap().syllables.len(),
        implicit_b.lyrics.as_ref().unwrap().syllables.len()
    );
}
