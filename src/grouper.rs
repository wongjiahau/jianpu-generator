use crate::ast::grouped::{
    GroupedChordNote, GroupedMeasure, GroupedNote, GroupedPart, GroupedRest, GroupedScore,
    GroupedTrack, MeasureDirectives, Metadata, NoteEvent, Notes, Score, TimeSignature,
};
use crate::ast::parsed::{
    Accidental, KeyChange, Note, NoteName, ParsedChordNote, ParsedDocument, ParsedNote, ParsedRest,
    ParsedTimedTrack, ParsedTrack, PartKind, ScoreEvent, Syllable,
};
use crate::combiner;
use crate::error::{JianPuError, Span};

pub fn group(doc: ParsedDocument) -> Result<Score, JianPuError> {
    let metadata = doc.metadata;
    let mut grouped_tracks = Vec::new();
    for track in doc.tracks {
        grouped_tracks.push(match track {
            ParsedTrack::Timed(part) => GroupedTrack::Timed(group_timed_track(part)?),
        });
    }

    let measure_directives = DirectiveGrouper::new().process_all(&doc.directive_events_per_measure);

    let grouped_score = GroupedScore {
        measure_directives,
        parts: grouped_tracks,
    };

    let measures = combiner::combine(&grouped_score)?;

    Ok(Score {
        metadata: Metadata {
            title: metadata.title,
            subtitle: metadata.subtitle,
            author: metadata.author,
            row_height: metadata.row_height.unwrap_or(24),
            max_columns: metadata.max_columns.unwrap_or(28),
            label_width: metadata.label_width.unwrap_or(40),
            note_number_width: metadata.note_number_width.unwrap_or(8),
        },
        measures,
    })
}

struct DirectiveGrouper {
    current_bpm: u32,
    current_time_sig: TimeSignature,
    current_key: KeyChange,
    bpm_changed: bool,
    time_sig_changed: bool,
    key_changed: bool,
}

impl DirectiveGrouper {
    fn new() -> Self {
        Self {
            current_bpm: 120,
            current_time_sig: TimeSignature {
                numerator: 4,
                denominator: 4,
            },
            current_key: KeyChange {
                note: Note {
                    name: NoteName::C,
                    octave: 4,
                    accidental: Accidental::Natural,
                },
            },
            bpm_changed: true,
            time_sig_changed: true,
            key_changed: true,
        }
    }

    fn process_all(
        mut self,
        directive_events_per_measure: &[Vec<crate::error::Spanned<ScoreEvent>>],
    ) -> Vec<MeasureDirectives> {
        let mut result = Vec::new();
        for events in directive_events_per_measure {
            let mut pending_label: Option<String> = None;
            for event in events {
                match &event.value {
                    ScoreEvent::BpmChange(bpm) => {
                        self.current_bpm = *bpm;
                        self.bpm_changed = true;
                    }
                    ScoreEvent::TimeSignatureChange {
                        numerator,
                        denominator,
                    } => {
                        self.current_time_sig = TimeSignature {
                            numerator: *numerator,
                            denominator: *denominator,
                        };
                        self.time_sig_changed = true;
                    }
                    ScoreEvent::KeyChange(kc) => {
                        self.current_key = kc.clone();
                        self.key_changed = true;
                    }
                    ScoreEvent::LabelChange(text) => {
                        pending_label = Some(text.clone());
                    }
                    _ => {}
                }
            }
            result.push(MeasureDirectives {
                bpm: if self.bpm_changed {
                    Some(self.current_bpm)
                } else {
                    None
                },
                time_signature: if self.time_sig_changed {
                    Some(TimeSignature {
                        numerator: self.current_time_sig.numerator,
                        denominator: self.current_time_sig.denominator,
                    })
                } else {
                    None
                },
                key: if self.key_changed {
                    Some(self.current_key.clone())
                } else {
                    None
                },
                label: pending_label,
            });
            self.bpm_changed = false;
            self.time_sig_changed = false;
            self.key_changed = false;
        }
        result
    }
}

struct PartGrouper {
    part_kind: PartKind,
    current_bpm: u32,
    current_key: KeyChange,
    current_time_sig: TimeSignature,
    bpm_changed: bool,
    key_changed: bool,
    time_sig_changed: bool,
    measures: Vec<GroupedMeasure>,
    current_notes: Vec<NoteEvent>,
    current_beat: u32,
    capacity: u32,
    pending_label: Option<String>,
    part_name: Option<String>,
    part_lyrics: Option<Vec<Syllable>>,
}

impl PartGrouper {
    fn new(part: &ParsedTimedTrack) -> Self {
        let default_key = KeyChange {
            note: Note {
                name: NoteName::C,
                octave: 4,
                accidental: Accidental::Natural,
            },
        };
        let current_time_sig = TimeSignature {
            numerator: 4,
            denominator: 4,
        };
        let capacity = Self::measure_capacity(&current_time_sig);

        Self {
            part_kind: part.kind,
            current_bpm: 120,
            current_key: default_key,
            current_time_sig,
            bpm_changed: true,
            key_changed: true,
            time_sig_changed: true,
            measures: Vec::new(),
            current_notes: Vec::new(),
            current_beat: 0,
            capacity,
            pending_label: None,
            part_name: Some(part.abbreviation.clone()),
            part_lyrics: part.lyrics.as_ref().map(|l| l.syllables.clone()),
        }
    }

    fn measure_capacity(ts: &TimeSignature) -> u32 {
        (ts.numerator as u32) * 16 / (ts.denominator as u32)
    }

    // Directive flags reset here are immediately overwritten at directive-change
    // call sites; the resulting assignments are never read before the overwrite.
    #[allow(unused_assignments)]
    fn flush_measure(&mut self) {
        if self.current_notes.is_empty() {
            return;
        }
        self.measures.push(GroupedMeasure {
            time_signature: if self.time_sig_changed {
                Some(TimeSignature {
                    numerator: self.current_time_sig.numerator,
                    denominator: self.current_time_sig.denominator,
                })
            } else {
                None
            },
            bpm: if self.bpm_changed {
                Some(self.current_bpm)
            } else {
                None
            },
            key: if self.key_changed {
                Some(self.current_key.clone())
            } else {
                None
            },
            label: self.pending_label.take(),
            notes: Notes {
                events: std::mem::take(&mut self.current_notes),
            },
        });
        self.current_beat = 0;
        self.bpm_changed = false;
        self.key_changed = false;
        self.time_sig_changed = false;
    }

    fn flush_if_full(&mut self) {
        if self.current_beat >= self.capacity {
            self.flush_measure();
        }
    }

    fn push_timed_event(
        &mut self,
        span: Span,
        duration: u32,
        event: NoteEvent,
        overflow_label: &str,
    ) -> Result<(), JianPuError> {
        self.flush_if_full();
        self.current_notes.push(event);
        self.current_beat += duration;
        if self.current_beat > self.capacity {
            return Err(JianPuError::new(
                span,
                format!(
                    "{overflow_label} duration {duration} overflows the current measure (capacity {} quarter-beats, {} used)",
                    self.capacity, self.current_beat
                ),
            ));
        }
        if self.current_beat == self.capacity {
            self.flush_measure();
        }
        Ok(())
    }

    fn handle_bpm_change(&mut self, bpm: u32) {
        self.flush_measure();
        self.current_bpm = bpm;
        self.bpm_changed = true;
    }

    fn handle_key_change(&mut self, kc: KeyChange) {
        self.flush_measure();
        self.current_key = kc;
        self.key_changed = true;
    }

    fn handle_time_signature_change(&mut self, numerator: u8, denominator: u8) {
        self.flush_measure();
        self.current_time_sig = TimeSignature {
            numerator,
            denominator,
        };
        self.capacity = Self::measure_capacity(&self.current_time_sig);
        self.time_sig_changed = true;
    }

    fn handle_extension(&mut self, span: Span) -> Result<(), JianPuError> {
        match self.current_notes.last_mut() {
            Some(NoteEvent::Note(n)) => {
                n.duration += 4;
                self.current_beat += 4;
            }
            Some(NoteEvent::Chord(c)) => {
                c.duration += 4;
                self.current_beat += 4;
            }
            Some(NoteEvent::Rest(_)) => {
                return Err(JianPuError::dash_after_rest(span));
            }
            None => {
                let message = if self.part_kind == PartKind::Chord {
                    "chord extension '-' with no preceding event".to_string()
                } else {
                    "extension `-` without a preceding note or rest; if it follows a measure boundary, cross-measure extension is not supported".to_string()
                };
                return Err(JianPuError::new(span, message));
            }
        }
        if self.current_beat >= self.capacity {
            self.flush_measure();
        }
        Ok(())
    }

    fn handle_tie_marker(&mut self, span: Span) -> Result<(), JianPuError> {
        let last_event = self.current_notes.last_mut().or_else(|| {
            self.measures
                .last_mut()
                .and_then(|m| m.notes.events.last_mut())
        });
        match last_event {
            Some(NoteEvent::Note(n)) => {
                n.tie = true;
                Ok(())
            }
            Some(NoteEvent::Chord(c)) => {
                c.tie = true;
                Ok(())
            }
            _ => Err(JianPuError::new(
                span,
                "tie `~` without a preceding note".to_string(),
            )),
        }
    }

    fn handle_note(&mut self, span: Span, pn: ParsedNote) -> Result<(), JianPuError> {
        self.push_timed_event(
            span,
            pn.duration,
            NoteEvent::Note(GroupedNote {
                pitch: pn.pitch,
                octave: pn.octave,
                duration: pn.duration,
                tie: pn.tie,
                group_membership: pn.group_membership,
                group_continuation: pn.group_continuation,
                dotted: pn.dotted,
            }),
            "note",
        )
    }

    fn handle_chord(&mut self, span: Span, pc: ParsedChordNote) -> Result<(), JianPuError> {
        self.push_timed_event(
            span,
            pc.duration,
            NoteEvent::Chord(GroupedChordNote {
                degree: pc.degree,
                accidental: pc.accidental,
                triad: pc.triad,
                extension: pc.extension,
                bass: pc.bass,
                duration: pc.duration,
                tie: pc.tie,
                group_membership: pc.group_membership,
                group_continuation: pc.group_continuation,
                dotted: pc.dotted,
            }),
            "chord",
        )
    }

    fn handle_label_change(&mut self, text: String) {
        self.flush_measure();
        self.pending_label = Some(text);
    }

    fn handle_rest(&mut self, span: Span, pr: &ParsedRest) -> Result<(), JianPuError> {
        self.push_timed_event(
            span,
            pr.duration,
            NoteEvent::Rest(GroupedRest {
                duration: pr.duration,
                dotted: pr.dotted,
            }),
            "rest",
        )
    }

    fn process_event(
        &mut self,
        spanned: crate::error::Spanned<ScoreEvent>,
    ) -> Result<(), JianPuError> {
        match spanned.value {
            ScoreEvent::BpmChange(bpm) => {
                self.handle_bpm_change(bpm);
                Ok(())
            }
            ScoreEvent::KeyChange(kc) => {
                self.handle_key_change(kc);
                Ok(())
            }
            ScoreEvent::TimeSignatureChange {
                numerator,
                denominator,
            } => {
                self.handle_time_signature_change(numerator, denominator);
                Ok(())
            }
            ScoreEvent::Extension => self.handle_extension(spanned.span),
            ScoreEvent::TieMarker => self.handle_tie_marker(spanned.span),
            ScoreEvent::Note(pn) => self.handle_note(spanned.span, pn),
            ScoreEvent::Chord(pc) => self.handle_chord(spanned.span, pc),
            ScoreEvent::LabelChange(text) => {
                self.handle_label_change(text);
                Ok(())
            }
            ScoreEvent::Rest(pr) => self.handle_rest(spanned.span, &pr),
        }
    }

    fn finish(mut self) -> GroupedPart {
        if !self.current_notes.is_empty() {
            self.measures.push(GroupedMeasure {
                time_signature: if self.time_sig_changed {
                    Some(TimeSignature {
                        numerator: self.current_time_sig.numerator,
                        denominator: self.current_time_sig.denominator,
                    })
                } else {
                    None
                },
                bpm: if self.bpm_changed {
                    Some(self.current_bpm)
                } else {
                    None
                },
                key: if self.key_changed {
                    Some(self.current_key.clone())
                } else {
                    None
                },
                label: self.pending_label.take(),
                notes: Notes {
                    events: std::mem::take(&mut self.current_notes),
                },
            });
        }

        GroupedPart {
            name: self.part_name,
            kind: self.part_kind,
            measures: self.measures,
            lyrics: self.part_lyrics,
        }
    }
}

fn group_timed_track(part: ParsedTimedTrack) -> Result<GroupedPart, JianPuError> {
    let mut grouper = PartGrouper::new(&part);
    for spanned in part.score.events {
        grouper.process_event(spanned)?;
    }
    Ok(grouper.finish())
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
        use crate::ast::grouped::PartRow;
        match &score.measures[measure_idx].parts[0] {
            PartRow::Timed(p) => &p.notes.events,
        }
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
        use crate::ast::grouped::PartRow;
        let m0_lyrics = match &score.measures[0].parts[0] {
            PartRow::Timed(p) => p.lyrics.as_ref().unwrap(),
        };
        let m1_lyrics = match &score.measures[1].parts[0] {
            PartRow::Timed(p) => p.lyrics.as_ref().unwrap(),
        };
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
        let PartRow::Timed(slice) = chord_row;
        assert_eq!(slice.notes.events.len(), 1);
        match &slice.notes.events[0] {
            NoteEvent::Chord(c) => {
                assert_eq!(c.duration, 16); // 4 tokens * 4 quarter-beats
            }
            NoteEvent::Note(_) | NoteEvent::Rest(_) => panic!("expected Chord event"),
        }
    }
}
