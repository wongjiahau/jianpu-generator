use crate::ast::grouped::*;
use crate::ast::parsed::{Accidental, NoteName, ParsedDocument};
use crate::error::JianPuError;

pub fn group(doc: ParsedDocument) -> Result<Score, JianPuError> {
    use crate::ast::parsed::{KeyChange, Note, ScoreEvent};

    let default_key = KeyChange {
        note: Note { name: NoteName::C, octave: 4, accidental: Accidental::Natural },
    };

    let mut current_bpm: u32 = 120;
    let mut current_key = default_key;
    let mut current_time_sig = TimeSignature { numerator: 4, denominator: 4 };

    // capacity in quarter-beats = numerator * (4 / denominator) * 4
    // For 4/4: 4 beats * 4 quarter-beats/beat = 16 quarter-beats
    // For 3/4: 3 * 4 = 12 quarter-beats
    let measure_capacity = |ts: &TimeSignature| -> u32 {
        (ts.numerator as u32) * 4 * (4 / ts.denominator as u32)
    };

    let mut measures: Vec<Measure> = Vec::new();
    let mut current_notes: Vec<NoteEvent> = Vec::new();
    let mut current_beat: u32 = 0;
    let mut capacity = measure_capacity(&current_time_sig);

    let flush_measure =
        |measures: &mut Vec<Measure>,
         current_notes: &mut Vec<NoteEvent>,
         current_beat: &mut u32,
         current_bpm: u32,
         current_key: &KeyChange,
         current_time_sig: &TimeSignature| {
            if !current_notes.is_empty() {
                measures.push(Measure {
                    time_signature: TimeSignature {
                        numerator: current_time_sig.numerator,
                        denominator: current_time_sig.denominator,
                    },
                    bpm: current_bpm,
                    key: KeyChange {
                        note: Note {
                            name: current_key.note.name.clone(),
                            octave: current_key.note.octave,
                            accidental: current_key.note.accidental.clone(),
                        },
                    },
                    notes: std::mem::take(current_notes),
                });
                *current_beat = 0;
            }
        };

    for spanned in doc.score_events {
        match spanned.value {
            ScoreEvent::BpmChange(bpm) => {
                current_bpm = bpm;
            }
            ScoreEvent::KeyChange(kc) => {
                current_key = kc;
            }
            ScoreEvent::TimeSignatureChange { numerator, denominator } => {
                flush_measure(
                    &mut measures,
                    &mut current_notes,
                    &mut current_beat,
                    current_bpm,
                    &current_key,
                    &current_time_sig,
                );
                current_time_sig = TimeSignature { numerator, denominator };
                capacity = measure_capacity(&current_time_sig);
            }
            ScoreEvent::Extension => {
                // Add 4 quarter-beats to last note/rest
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
                            "extension `-` without a preceding note or rest".to_string(),
                        ));
                    }
                }
                // Flush if measure is full
                if current_beat >= capacity {
                    flush_measure(
                        &mut measures,
                        &mut current_notes,
                        &mut current_beat,
                        current_bpm,
                        &current_key,
                        &current_time_sig,
                    );
                }
            }
            ScoreEvent::Note(pn) => {
                if current_beat >= capacity {
                    flush_measure(
                        &mut measures,
                        &mut current_notes,
                        &mut current_beat,
                        current_bpm,
                        &current_key,
                        &current_time_sig,
                    );
                }
                current_beat += pn.duration;
                current_notes.push(NoteEvent::Note(GroupedNote {
                    pitch: pn.pitch,
                    octave: pn.octave,
                    duration: pn.duration,
                    tie: pn.tie,
                }));
                if current_beat >= capacity {
                    flush_measure(
                        &mut measures,
                        &mut current_notes,
                        &mut current_beat,
                        current_bpm,
                        &current_key,
                        &current_time_sig,
                    );
                }
            }
            ScoreEvent::Rest(pr) => {
                if current_beat >= capacity {
                    flush_measure(
                        &mut measures,
                        &mut current_notes,
                        &mut current_beat,
                        current_bpm,
                        &current_key,
                        &current_time_sig,
                    );
                }
                current_beat += pr.duration;
                current_notes.push(NoteEvent::Rest(GroupedRest { duration: pr.duration }));
                if current_beat >= capacity {
                    flush_measure(
                        &mut measures,
                        &mut current_notes,
                        &mut current_beat,
                        current_bpm,
                        &current_key,
                        &current_time_sig,
                    );
                }
            }
        }
    }

    // Flush any remaining notes as a partial measure
    if !current_notes.is_empty() {
        flush_measure(
            &mut measures,
            &mut current_notes,
            &mut current_beat,
            current_bpm,
            &current_key,
            &current_time_sig,
        );
    }

    Ok(Score {
        metadata: Metadata {
            title: doc.metadata.title,
            author: doc.metadata.author,
            cell_size: doc.metadata.cell_size.unwrap_or(24),
        },
        measures,
        lyrics: doc.lyrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn parse_and_group(input: &str) -> Score {
        let doc = parser::parse(input, "test.jianpu").unwrap();
        group(doc).unwrap()
    }

    #[test]
    fn groups_four_four_into_single_measure() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4\n\n[lyrics]\na b c d\n",
        );
        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].notes.len(), 4);
    }

    #[test]
    fn splits_into_two_measures_at_bar_boundary() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4 5 6 7 1\n\n[lyrics]\na b c d e f g h\n",
        );
        assert_eq!(score.measures.len(), 2);
    }

    #[test]
    fn extension_adds_to_previous_note_duration() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 - 3 4\n\n[lyrics]\na - b c\n",
        );
        match &score.measures[0].notes[0] {
            NoteEvent::Note(n) => assert_eq!(n.duration, 8), // 4 + 4
            _ => panic!("expected Note"),
        }
    }

    #[test]
    fn default_bpm_is_120() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4\n\n[lyrics]\na b c d\n",
        );
        assert_eq!(score.measures[0].bpm, 120);
    }

    #[test]
    fn bpm_change_propagates_to_next_measure() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4 bpm=90 5 6 7 1\n\n[lyrics]\na b c d e f g h\n",
        );
        assert_eq!(score.measures[0].bpm, 120);
        assert_eq!(score.measures[1].bpm, 90);
    }

    #[test]
    fn key_change_propagates() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1=G4 1 2 3 4\n\n[lyrics]\na b c d\n",
        );
        assert_eq!(score.measures[0].key.note.name, NoteName::G);
    }

    #[test]
    fn cell_size_defaults_to_24() {
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 3 4\n\n[lyrics]\na b c d\n",
        );
        assert_eq!(score.metadata.cell_size, 24);
    }

    #[test]
    fn half_beat_notes_accumulate_correctly() {
        // 4/4 = 16 quarter-beats; eight half-beat notes = 8*2 = 16 quarter-beats (one full measure)
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 _1 _2 _3 _4 _5 _6 _7 _1\n\n[lyrics]\na b c d e f g h\n",
        );
        assert_eq!(score.measures.len(), 1); // 8 * 2 = 16 quarter-beats = one 4/4 measure
    }
}
