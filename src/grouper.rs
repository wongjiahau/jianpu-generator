use crate::ast::grouped::*;
use crate::ast::parsed::{Accidental, NoteName, ParsedDocument, ParsedPart};
use crate::combiner;
use crate::error::JianPuError;

pub fn group(doc: ParsedDocument) -> Result<Score, JianPuError> {
    let mut grouped_parts = Vec::new();
    for part in doc.parts {
        grouped_parts.push(group_part(part)?);
    }

    let measures = combiner::combine(grouped_parts)?;

    Ok(Score {
        metadata: Metadata {
            title: doc.metadata.title,
            subtitle: doc.metadata.subtitle,
            author: doc.metadata.author,
            row_height: doc.metadata.row_height.unwrap_or(24),
            max_columns: doc.metadata.max_columns.unwrap_or(28),
            label_width: doc.metadata.label_width.unwrap_or(40),
        },
        measures,
    })
}

// The flush_measure! macro resets directive flags that are immediately overwritten
// at directive-change call sites; the resulting assignments are never read before
// the overwrite, which is intentional, not a bug.
#[allow(unused_assignments)]
fn group_part(part: ParsedPart) -> Result<GroupedPart, JianPuError> {
    use crate::ast::parsed::{KeyChange, Note, ScoreEvent};

    let default_key = KeyChange {
        note: Note {
            name: NoteName::C,
            octave: 4,
            accidental: Accidental::Natural,
        },
    };

    let mut current_bpm: u32 = 120;
    let mut current_key = default_key;
    let mut current_time_sig = TimeSignature {
        numerator: 4,
        denominator: 4,
    };

    let measure_capacity =
        |ts: &TimeSignature| -> u32 { (ts.numerator as u32) * 16 / (ts.denominator as u32) };

    // Track whether each directive was explicitly set since the last measure boundary.
    // All start as true so the first measure always gets Some(_) for all directives.
    let mut bpm_changed = true;
    let mut key_changed = true;
    let mut time_sig_changed = true;

    let mut measures: Vec<GroupedMeasure> = Vec::new();
    let mut current_notes: Vec<NoteEvent> = Vec::new();
    let mut current_beat: u32 = 0;
    let mut capacity = measure_capacity(&current_time_sig);

    macro_rules! flush_measure {
        () => {
            if !current_notes.is_empty() {
                measures.push(GroupedMeasure {
                    time_signature: if time_sig_changed {
                        Some(TimeSignature {
                            numerator: current_time_sig.numerator,
                            denominator: current_time_sig.denominator,
                        })
                    } else {
                        None
                    },
                    bpm: if bpm_changed { Some(current_bpm) } else { None },
                    key: if key_changed {
                        Some(current_key.clone())
                    } else {
                        None
                    },
                    notes: Notes {
                        events: std::mem::take(&mut current_notes),
                    },
                });
                current_beat = 0;
                bpm_changed = false;
                key_changed = false;
                time_sig_changed = false;
            }
        };
    }

    for spanned in part.score.events {
        match spanned.value {
            ScoreEvent::BpmChange(bpm) => {
                flush_measure!();
                current_bpm = bpm;
                bpm_changed = true;
            }
            ScoreEvent::KeyChange(kc) => {
                flush_measure!();
                current_key = kc;
                key_changed = true;
            }
            ScoreEvent::TimeSignatureChange {
                numerator,
                denominator,
            } => {
                flush_measure!();
                current_time_sig = TimeSignature {
                    numerator,
                    denominator,
                };
                capacity = measure_capacity(&current_time_sig);
                time_sig_changed = true;
            }
            ScoreEvent::Extension => {
                match current_notes.last_mut() {
                    Some(NoteEvent::Note(n)) => {
                        n.duration += 4;
                        current_beat += 4;
                    }
                    Some(NoteEvent::Rest(r)) => {
                        r.duration += 4;
                        current_beat += 4;
                    }
                    None => {
                        return Err(JianPuError::new(
                            spanned.span,
                            "extension `-` without a preceding note or rest; if it follows a measure boundary, cross-measure extension is not supported".to_string(),
                        ));
                    }
                }
                if current_beat >= capacity {
                    flush_measure!();
                }
            }
            ScoreEvent::Note(pn) => {
                if current_beat >= capacity {
                    flush_measure!();
                }
                let note_duration = pn.duration;
                current_notes.push(NoteEvent::Note(GroupedNote {
                    pitch: pn.pitch,
                    octave: pn.octave,
                    duration: pn.duration,
                    tie: pn.tie,
                }));
                current_beat += note_duration;
                if current_beat > capacity {
                    return Err(JianPuError::new(
                        spanned.span,
                        format!(
                            "note duration {} overflows the current measure (capacity {} quarter-beats, {} used)",
                            note_duration, capacity, current_beat
                        ),
                    ));
                }
                if current_beat == capacity {
                    flush_measure!();
                }
            }
            ScoreEvent::Rest(pr) => {
                if current_beat >= capacity {
                    flush_measure!();
                }
                let rest_duration = pr.duration;
                current_notes.push(NoteEvent::Rest(GroupedRest {
                    duration: pr.duration,
                }));
                current_beat += rest_duration;
                if current_beat > capacity {
                    return Err(JianPuError::new(
                        spanned.span,
                        format!(
                            "rest duration {} overflows the current measure (capacity {} quarter-beats, {} used)",
                            rest_duration, capacity, current_beat
                        ),
                    ));
                }
                if current_beat == capacity {
                    flush_measure!();
                }
            }
        }
    }

    // Flush any remaining notes as a partial measure (inline to avoid spurious unused-assignment warnings)
    if !current_notes.is_empty() {
        measures.push(GroupedMeasure {
            time_signature: if time_sig_changed {
                Some(TimeSignature {
                    numerator: current_time_sig.numerator,
                    denominator: current_time_sig.denominator,
                })
            } else {
                None
            },
            bpm: if bpm_changed { Some(current_bpm) } else { None },
            key: if key_changed {
                Some(current_key.clone())
            } else {
                None
            },
            notes: Notes {
                events: std::mem::take(&mut current_notes),
            },
        });
    }

    Ok(GroupedPart {
        name: part.name,
        measures,
        lyrics: part.lyrics.map(|l| l.syllables),
    })
}

#[cfg(test)]
mod tests {
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
        &score.measures[measure_idx].parts[0].notes.events
    }

    #[test]
    fn groups_four_four_into_single_measure() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
        ));
        assert_eq!(score.measures.len(), 1);
        assert_eq!(first_part_notes(&score, 0).len(), 4);
    }

    #[test]
    fn splits_into_two_measures_at_bar_boundary() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
        ));
        assert_eq!(score.measures.len(), 2);
    }

    #[test]
    fn extension_adds_to_previous_note_duration() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 - 3 4\na - b c\n",
        ));
        match &first_part_notes(&score, 0)[0] {
            NoteEvent::Note(n) => assert_eq!(n.duration, 8),
            _ => panic!("expected Note"),
        }
    }

    #[test]
    fn first_measure_has_bpm_some() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
        ));
        assert_eq!(score.measures[0].bpm, Some(120));
    }

    #[test]
    fn bpm_change_sets_some_on_next_measure() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(bpm=90)\n5 6 7 1\ne f g h\n",
        ));
        assert_eq!(score.measures[0].bpm, Some(120));
        assert_eq!(score.measures[1].bpm, Some(90));
    }

    #[test]
    fn unchanged_bpm_is_none_on_second_measure() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
        ));
        assert_eq!(score.measures[0].bpm, Some(120));
        assert_eq!(score.measures[1].bpm, None);
    }

    #[test]
    fn key_change_propagates() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
        ));
        assert_eq!(score.metadata.row_height, 24);
    }

    #[test]
    fn max_columns_defaults_to_28() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
        ));
        assert_eq!(score.metadata.max_columns, 28);
    }

    #[test]
    fn half_beat_notes_accumulate_correctly() {
        let score = parse_and_group(concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n_1 _2 _3 _4 _5 _6 _7 _1\na b c d e f g h\n",
        ));
        assert_eq!(score.measures.len(), 1);
    }

    #[test]
    fn overflow_note_errors() {
        // The interleaved parser validates beats per bar — overfull bar is rejected at parse time.
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n_1 _1 _1 _1 _1 _1 _1 1\na b c d e f g h\n",
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n5 6 7 1\n",
        );
        let doc = parser::parse(input, "test.jianpu").unwrap();
        let score = group(doc).unwrap();
        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].parts.len(), 2);
        assert_eq!(score.measures[0].parts[0].name, Some("Soprano".to_string()));
        assert_eq!(score.measures[0].parts[1].name, Some("Alto".to_string()));
    }

    #[test]
    fn lyrics_distributed_per_measure() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n5 6 7 1\ne f g h\n",
        );
        let doc = parser::parse(input, "test.jianpu").unwrap();
        let score = group(doc).unwrap();
        assert_eq!(score.measures.len(), 2);
        let m0_lyrics = score.measures[0].parts[0].lyrics.as_ref().unwrap();
        let m1_lyrics = score.measures[1].parts[0].lyrics.as_ref().unwrap();
        assert_eq!(m0_lyrics.syllables.len(), 4);
        assert_eq!(m1_lyrics.syllables.len(), 4);
    }
}
