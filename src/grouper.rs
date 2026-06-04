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

    // capacity in sixteenth-note units ("quarter-beats") per measure = numerator * 16 / denominator
    // For 4/4: 4 * 16 / 4 = 16 (four quarter notes × 4 sixteenths each)
    // For 3/4: 3 * 16 / 4 = 12
    // For 6/8: 6 * 16 / 8 = 12 (six eighth notes × 2 sixteenths each)
    let measure_capacity = |ts: &TimeSignature| -> u32 {
        (ts.numerator as u32) * 16 / (ts.denominator as u32)
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
                    key: current_key.clone(),
                    notes: std::mem::take(current_notes),
                });
                *current_beat = 0;
            }
        };

    let first_part = doc.parts.into_iter().next().unwrap();
    let doc_lyrics = first_part.lyrics.map(|l| l.syllables).unwrap_or_default();
    for spanned in first_part.score.events {
        match spanned.value {
            ScoreEvent::BpmChange(bpm) => {
                flush_measure(
                    &mut measures,
                    &mut current_notes,
                    &mut current_beat,
                    current_bpm,
                    &current_key,
                    &current_time_sig,
                );
                current_bpm = bpm;
            }
            ScoreEvent::KeyChange(kc) => {
                flush_measure(
                    &mut measures,
                    &mut current_notes,
                    &mut current_beat,
                    current_bpm,
                    &current_key,
                    &current_time_sig,
                );
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
                            "extension `-` without a preceding note or rest; if it follows a measure boundary, cross-measure extension is not supported".to_string(),
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
                let rest_duration = pr.duration;
                current_notes.push(NoteEvent::Rest(GroupedRest { duration: pr.duration }));
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
            subtitle: doc.metadata.subtitle,
            author: doc.metadata.author,
            cell_size: doc.metadata.cell_size.unwrap_or(24),
        },
        measures,
        lyrics: doc_lyrics,
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

    fn parse_and_group_err(input: &str) -> JianPuError {
        let doc = parser::parse(input, "test.jianpu").unwrap();
        match group(doc) {
            Err(e) => e,
            Ok(_) => panic!("expected group() to return Err, but it returned Ok"),
        }
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

    #[test]
    fn overflow_note_errors() {
        // In 4/4 (capacity=16), three quarter notes fill 12 beats; adding a whole-note (=1, duration 4)
        // would reach 16 — that's fine. But a note with duration > remaining space should error.
        // Use a dotted note workaround: put 4 quarter notes (fills measure), then try one more.
        // Actually let's put 3 quarter notes (12 qb) then a note of duration > 4.
        // The parser only produces durations 1, 2, 4 via prefixes. The Extension token adds 4.
        // Simplest: in 2/4 (capacity=8), place two quarter notes (8 qb fills) then one more — that
        // would overflow. But flush happens at ==capacity before the next note, so no overflow.
        // To actually overflow: place a half note (duration=4) when only 2 qb remain.
        // In 4/4: 3 quarter notes = 12 qb used, 4 remain. Now place a note that is duration 4 — OK (==16).
        // We need remaining < note_duration: 3 quarter notes + 1 quarter note triggers flush... hmm.
        // Best approach: switch to 3/4 (capacity=12). Place two quarter notes (8 qb). 4 remain.
        // A whole-note (duration=4) fits exactly. Need something > 4 remaining.
        // The only way to create an oversized note via the text format is using extension `-`.
        // Place one note `1 - -` in 2/4: note=4, ext=4 (total 8 = capacity, fine).
        // In 2/4, place note `1 -` (total 8) then another note 2 (duration 4): new measure, fine.
        // Actually the simplest overflow: use a 3/4 time (capacity=12 qb).
        // Notes: 1 2 (8 qb) then a note extended via - to become 8 qb -> total 16 > 12.
        // But extension flushes at >= capacity... let's think differently.
        // The overflow guard added is specifically for a *single note* duration > remaining space
        // (not a flush boundary). Current_beat starts at 0 in new measure after flush.
        // In 4/4 (cap=16): place notes that leave e.g. 2 qb, then place a quarter note (dur=4).
        // To have 2 qb remaining: use duration=14 total. 14 = 3*4 + 2 = three quarters + one eighth (dur=2).
        // Let's try: _1 _1 _1 _1 _1 _1 _1 (seven * 2 = 14 qb) then place `1` (quarter, dur=4) -> total 18 > 16.
        let err = parse_and_group_err(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 _1 _1 _1 _1 _1 _1 _1 1\n\n[lyrics]\na b c d e f g h\n",
        );
        assert!(err.message.contains("overflows"), "expected overflow error, got: {}", err.message);
    }

    #[test]
    fn bpm_change_creates_new_measure() {
        // bpm change mid-measure should flush the accumulated notes first,
        // producing a partial measure before the bpm change takes effect.
        let score = parse_and_group(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 1 2 bpm=90 3 4\n\n[lyrics]\na b c d\n",
        );
        // The bpm change after 2 notes should create a new measure boundary.
        assert_eq!(score.measures.len(), 2);
        assert_eq!(score.measures[0].bpm, 120);
        assert_eq!(score.measures[0].notes.len(), 2);
        assert_eq!(score.measures[1].bpm, 90);
        assert_eq!(score.measures[1].notes.len(), 2);
    }
}
