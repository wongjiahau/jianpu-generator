use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use std::collections::HashMap;

use crate::ast::grouped::{NoteEvent, PartRow, Score};
use crate::ast::parsed::{Accidental, JianPuPitch, KeyChange, NoteName};
use crate::error::{JianPuError, Span};

const TPQ: u16 = 480; // ticks per quarter note
const VELOCITY: u8 = 80;
const CHANNEL: u8 = 0;
const PIANO: u8 = 0;

struct RawEvent {
    tick: u32,
    kind: RawKind,
}

enum RawKind {
    Tempo(u32),
    NoteOn(u8),
    NoteOff(u8),
    ProgramChange(u8),
}

pub fn write_midi(score: &Score) -> Result<Vec<u8>, JianPuError> {
    let mut raw: Vec<RawEvent> = Vec::new();

    raw.push(RawEvent {
        tick: 0,
        kind: RawKind::ProgramChange(PIANO),
    });

    let mut current_tick: u32 = 0;
    let mut per_part_ties: Vec<HashMap<u8, u32>> = Vec::new();
    let mut active_key = default_active_key();

    for measure in &score.measures {
        current_tick = process_measure(
            measure,
            current_tick,
            &mut raw,
            &mut per_part_ties,
            &mut active_key,
        )?;
    }

    flush_pending_ties(&mut raw, per_part_ties);
    sort_raw_events(&mut raw);

    let track = build_track_events(&raw);
    write_smf(track)
}

fn default_active_key() -> KeyChange {
    KeyChange {
        note: crate::ast::parsed::Note {
            name: NoteName::C,
            octave: 4,
            accidental: Accidental::Natural,
        },
    }
}

fn process_measure(
    measure: &crate::ast::grouped::MultiPartMeasure,
    current_tick: u32,
    raw: &mut Vec<RawEvent>,
    per_part_ties: &mut Vec<HashMap<u8, u32>>,
    active_key: &mut KeyChange,
) -> Result<u32, JianPuError> {
    if let Some(bpm) = measure.bpm {
        let micros = 60_000_000 / bpm;
        raw.push(RawEvent {
            tick: current_tick,
            kind: RawKind::Tempo(micros),
        });
    }

    if let Some(key) = &measure.key {
        *active_key = key.clone();
    }

    let mut measure_duration: u32 = 0;

    let notes_parts: Vec<&crate::ast::grouped::PartSlice> = measure
        .parts
        .iter()
        .filter_map(|r| {
            if let PartRow::Notes(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect();

    while per_part_ties.len() < notes_parts.len() {
        per_part_ties.push(HashMap::new());
    }

    for (part_idx, part) in notes_parts.iter().enumerate() {
        let part_duration =
            process_measure_notes(part, part_idx, current_tick, raw, per_part_ties, active_key)?;
        if part_duration > measure_duration {
            measure_duration = part_duration;
        }
    }

    for row in &measure.parts {
        if let PartRow::Chord(chord_slice) = row {
            let chord_duration = process_chord_events(chord_slice, current_tick, raw, active_key);
            if chord_duration > measure_duration {
                measure_duration = chord_duration;
            }
        }
    }

    Ok(current_tick + measure_duration)
}

fn process_measure_notes(
    part: &crate::ast::grouped::PartSlice,
    part_idx: usize,
    current_tick: u32,
    raw: &mut Vec<RawEvent>,
    per_part_ties: &mut [HashMap<u8, u32>],
    active_key: &KeyChange,
) -> Result<u32, JianPuError> {
    let pending_ties = per_part_ties.get_mut(part_idx).ok_or_else(|| {
        JianPuError::new(
            Span::new(0, 0),
            format!("internal error: missing MIDI tie state for part {part_idx}"),
        )
    })?;
    let mut part_tick = current_tick;

    for event in &part.notes.events {
        match event {
            NoteEvent::Note(note) => {
                let ticks = duration_to_ticks(note.duration);
                let midi_note = resolve_midi_note(&note.pitch, note.octave, active_key);
                let note_off_tick = part_tick + ticks;

                let is_tie_continuation = pending_ties.remove(&midi_note).is_some();
                flush_pending_ties_at_tick(pending_ties, part_tick, raw);

                if !is_tie_continuation {
                    raw.push(RawEvent {
                        tick: part_tick,
                        kind: RawKind::NoteOn(midi_note),
                    });
                }

                if note.tie {
                    pending_ties.insert(midi_note, note_off_tick);
                } else {
                    raw.push(RawEvent {
                        tick: note_off_tick,
                        kind: RawKind::NoteOff(midi_note),
                    });
                }

                part_tick += ticks;
            }
            NoteEvent::Rest(rest) => {
                flush_pending_ties_at_tick(pending_ties, part_tick, raw);
                part_tick += duration_to_ticks(rest.duration);
            }
        }
    }

    Ok(part_tick - current_tick)
}

fn flush_pending_ties_at_tick(
    pending_ties: &mut HashMap<u8, u32>,
    tick: u32,
    raw: &mut Vec<RawEvent>,
) {
    for (slurred_note, _) in pending_ties.drain() {
        raw.push(RawEvent {
            tick,
            kind: RawKind::NoteOff(slurred_note),
        });
    }
}

fn process_chord_events(
    chord_slice: &crate::ast::grouped::ChordSlice,
    current_tick: u32,
    raw: &mut Vec<RawEvent>,
    active_key: &KeyChange,
) -> u32 {
    let mut chord_tick = current_tick;

    for event in &chord_slice.events {
        match event {
            crate::ast::grouped::GroupedChordEvent::Chord(chord) => {
                let ticks = duration_to_ticks(chord.duration);
                let notes_to_play = chord_midi_notes(chord, active_key);

                for &midi_note in &notes_to_play {
                    raw.push(RawEvent {
                        tick: chord_tick,
                        kind: RawKind::NoteOn(midi_note),
                    });
                }
                let off_tick = chord_tick + ticks;
                for &midi_note in &notes_to_play {
                    raw.push(RawEvent {
                        tick: off_tick,
                        kind: RawKind::NoteOff(midi_note),
                    });
                }

                chord_tick += ticks;
            }
            crate::ast::grouped::GroupedChordEvent::Rest(dur) => {
                chord_tick += duration_to_ticks(*dur);
            }
        }
    }

    chord_tick - current_tick
}

fn chord_midi_notes(chord: &crate::ast::grouped::GroupedChord, active_key: &KeyChange) -> Vec<u8> {
    let base_root = resolve_midi_note(&chord.degree, 0, active_key);
    let acc_delta = accidental_offset(&chord.accidental);
    let root = (base_root as i32 + acc_delta).clamp(0, 127) as u8;

    let triad_offsets: &[i32] = match chord.triad {
        crate::ast::parsed::TriadQuality::Major => &[0, 4, 7],
        crate::ast::parsed::TriadQuality::Minor => &[0, 3, 7],
        crate::ast::parsed::TriadQuality::Diminished => &[0, 3, 6],
        crate::ast::parsed::TriadQuality::Augmented => &[0, 4, 8],
    };

    let ext_offset: Option<i32> = match &chord.extension {
        Some(crate::ast::parsed::Extension::DominantSeventh) => Some(10),
        Some(crate::ast::parsed::Extension::MajorSeventh) => Some(11),
        None => None,
    };

    let mut notes_to_play: Vec<u8> = triad_offsets
        .iter()
        .map(|&off| (root as i32 + off).clamp(0, 127) as u8)
        .collect();
    if let Some(off) = ext_offset {
        notes_to_play.push((root as i32 + off).clamp(0, 127) as u8);
    }

    if let Some(bass) = &chord.bass {
        let base_bass = resolve_midi_note(&bass.degree, 0, active_key);
        let bass_acc = accidental_offset(&bass.accidental);
        let bass_note = ((base_bass as i32 + bass_acc) - 12).clamp(0, 127) as u8;
        notes_to_play.push(bass_note);
    }

    notes_to_play
}

fn flush_pending_ties(raw: &mut Vec<RawEvent>, per_part_ties: Vec<HashMap<u8, u32>>) {
    for pending_ties in per_part_ties {
        for (midi_note, note_off_tick) in pending_ties {
            raw.push(RawEvent {
                tick: note_off_tick,
                kind: RawKind::NoteOff(midi_note),
            });
        }
    }
}

fn sort_raw_events(raw: &mut [RawEvent]) {
    raw.sort_by_key(|e| {
        let priority: u8 = match e.kind {
            RawKind::Tempo(_) | RawKind::ProgramChange(_) => 0,
            RawKind::NoteOff(_) => 1,
            RawKind::NoteOn(_) => 2,
        };
        (e.tick, priority)
    });
}

fn build_track_events(raw: &[RawEvent]) -> Vec<TrackEvent<'static>> {
    let mut track: Vec<TrackEvent> = Vec::new();
    let mut last_tick: u32 = 0;

    for event in raw {
        let delta = event.tick - last_tick;
        last_tick = event.tick;
        track.push(raw_event_to_track_event(event, delta));
    }

    track.push(TrackEvent {
        delta: u28::from(0u32),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    track
}

fn raw_event_to_track_event(event: &RawEvent, delta: u32) -> TrackEvent<'static> {
    match &event.kind {
        RawKind::Tempo(micros) => TrackEvent {
            delta: u28::from(delta),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(*micros))),
        },
        RawKind::ProgramChange(program) => TrackEvent {
            delta: u28::from(delta),
            kind: TrackEventKind::Midi {
                channel: u4::from(CHANNEL),
                message: MidiMessage::ProgramChange {
                    program: u7::from(*program),
                },
            },
        },
        RawKind::NoteOn(note) => TrackEvent {
            delta: u28::from(delta),
            kind: TrackEventKind::Midi {
                channel: u4::from(CHANNEL),
                message: MidiMessage::NoteOn {
                    key: u7::from(*note),
                    vel: u7::from(VELOCITY),
                },
            },
        },
        RawKind::NoteOff(note) => TrackEvent {
            delta: u28::from(delta),
            kind: TrackEventKind::Midi {
                channel: u4::from(CHANNEL),
                message: MidiMessage::NoteOff {
                    key: u7::from(*note),
                    vel: u7::from(0u8),
                },
            },
        },
    }
}

fn write_smf(track: Vec<TrackEvent<'static>>) -> Result<Vec<u8>, JianPuError> {
    let smf = Smf {
        header: Header {
            format: Format::SingleTrack,
            timing: Timing::Metrical(u15::from(TPQ)),
        },
        tracks: vec![track],
    };

    let mut buf = Vec::new();
    smf.write_std(&mut buf)
        .map_err(|_| JianPuError::new(Span::new(0, 0), "failed to write MIDI data"))?;
    Ok(buf)
}

fn note_name_to_semitone(name: &NoteName) -> i32 {
    match name {
        NoteName::C => 0,
        NoteName::D => 2,
        NoteName::E => 4,
        NoteName::F => 5,
        NoteName::G => 7,
        NoteName::A => 9,
        NoteName::B => 11,
    }
}

fn pitch_to_scale_offset(pitch: &JianPuPitch) -> i32 {
    match pitch {
        JianPuPitch::One => 0,
        JianPuPitch::Two => 2,
        JianPuPitch::Three => 4,
        JianPuPitch::Four => 5,
        JianPuPitch::Five => 7,
        JianPuPitch::Six => 9,
        JianPuPitch::Seven => 11,
    }
}

fn accidental_offset(acc: &Accidental) -> i32 {
    match acc {
        Accidental::Sharp => 1,
        Accidental::Flat => -1,
        Accidental::Natural => 0,
    }
}

pub(crate) fn resolve_midi_note(pitch: &JianPuPitch, octave: i8, key: &KeyChange) -> u8 {
    let root = 12 * (key.note.octave as i32 + 1)
        + note_name_to_semitone(&key.note.name)
        + accidental_offset(&key.note.accidental);
    let midi = root + pitch_to_scale_offset(pitch) + (octave as i32) * 12;
    midi.clamp(0, 127) as u8
}

pub(crate) fn duration_to_ticks(quarter_beats: u32) -> u32 {
    quarter_beats * (TPQ as u32) / 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::{Accidental, KeyChange, Note, NoteName};

    #[test]
    fn chord_major_expands_to_three_notes() {
        use crate::ast::grouped::{
            ChordSlice, GroupedChord, GroupedChordEvent, Metadata, MultiPartMeasure, PartRow,
            Score, TimeSignature,
        };
        use crate::ast::parsed::{
            Accidental, JianPuPitch, KeyChange, Note, NoteName, TriadQuality,
        };

        let key = KeyChange {
            note: Note {
                name: NoteName::C,
                octave: 4,
                accidental: Accidental::Natural,
            },
        };
        let chord = GroupedChord {
            degree: JianPuPitch::One,
            accidental: Accidental::Natural,
            triad: TriadQuality::Major,
            extension: None,
            bass: None,
            duration: 16,
        };
        let score = Score {
            metadata: Metadata {
                title: String::new(),
                subtitle: None,
                author: String::new(),
                row_height: 24,
                max_columns: 28,
                label_width: 40,
                note_number_width: 8,
            },
            measures: vec![MultiPartMeasure {
                time_signature: Some(TimeSignature {
                    numerator: 4,
                    denominator: 4,
                }),
                bpm: Some(120),
                key: Some(key),
                label: None,
                parts: vec![PartRow::Chord(ChordSlice {
                    name: None,
                    events: vec![GroupedChordEvent::Chord(chord)],
                })],
            }],
        };
        let midi_bytes = write_midi(&score).unwrap();
        // MIDI bytes must be non-empty and start with MThd
        assert!(midi_bytes.starts_with(b"MThd"), "expected MIDI header");
        assert!(midi_bytes.len() > 20);
    }

    fn key(name: NoteName, octave: u8) -> KeyChange {
        KeyChange {
            note: Note {
                name,
                octave,
                accidental: Accidental::Natural,
            },
        }
    }

    #[test]
    fn middle_c_degree_one() {
        assert_eq!(
            resolve_midi_note(&JianPuPitch::One, 0, &key(NoteName::C, 4)),
            60
        );
    }

    #[test]
    fn degree_five_c4_is_g4() {
        assert_eq!(
            resolve_midi_note(&JianPuPitch::Five, 0, &key(NoteName::C, 4)),
            67
        );
    }

    #[test]
    fn octave_up_shifts_by_12() {
        assert_eq!(
            resolve_midi_note(&JianPuPitch::One, 1, &key(NoteName::C, 4)),
            72
        );
    }

    #[test]
    fn key_g4_degree_one_is_midi_67() {
        assert_eq!(
            resolve_midi_note(&JianPuPitch::One, 0, &key(NoteName::G, 4)),
            67
        );
    }

    #[test]
    fn duration_quarter_note_is_480_ticks() {
        assert_eq!(duration_to_ticks(4), 480);
    }

    #[test]
    fn duration_eighth_note_is_240_ticks() {
        assert_eq!(duration_to_ticks(2), 240);
    }

    #[test]
    fn duration_half_note_is_960_ticks() {
        assert_eq!(duration_to_ticks(8), 960);
    }
}
