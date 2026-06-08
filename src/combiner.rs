use crate::ast::grouped::{
    GroupedMeasure, GroupedTrack, Lyrics, MultiPartMeasure, NoteEvent, Notes, PartRow, PartSlice,
};
use crate::ast::parsed::{JianPuPitch, Syllable};
use crate::error::{JianPuError, Span};

pub fn combine(grouped_tracks: &[GroupedTrack]) -> Result<Vec<MultiPartMeasure>, JianPuError> {
    if grouped_tracks.is_empty() {
        return Ok(Vec::new());
    }

    let expected_len = grouped_tracks
        .first()
        .map(GroupedTrack::measure_count)
        .unwrap_or(0);
    validate_measure_counts(grouped_tracks, expected_len)?;

    let lyrics_per_track: Vec<Vec<Vec<Syllable>>> = grouped_tracks
        .iter()
        .map(|track| match track {
            GroupedTrack::Notes(part) => part
                .lyrics
                .as_deref()
                .map(|lyrics| distribute_lyrics(&part.measures, lyrics))
                .unwrap_or_else(|| vec![vec![]; part.measures.len()]),
            GroupedTrack::Chord(_) => vec![vec![]; expected_len],
        })
        .collect();

    let mut combined = Vec::with_capacity(expected_len);
    for measure_idx in 0..expected_len {
        let first_notes_measure = grouped_tracks
            .iter()
            .find_map(|track| match track {
                GroupedTrack::Notes(part) => part.measures.get(measure_idx),
                GroupedTrack::Chord(_) => None,
            })
            .ok_or_else(|| {
                JianPuError::new(
                    Span::new(0, 0),
                    "internal invariant: no notes track for measure metadata",
                )
            })?;
        let part_rows = build_part_rows(grouped_tracks, measure_idx, &lyrics_per_track)?;
        combined.push(MultiPartMeasure {
            time_signature: first_notes_measure.time_signature.clone(),
            bpm: first_notes_measure.bpm,
            key: first_notes_measure.key.clone(),
            label: first_notes_measure.label.clone(),
            parts: part_rows,
        });
    }

    Ok(combined)
}

fn validate_measure_counts(
    grouped_tracks: &[GroupedTrack],
    expected_len: usize,
) -> Result<(), JianPuError> {
    for track in grouped_tracks.iter().skip(1) {
        if track.measure_count() != expected_len {
            return Err(JianPuError::new(
                Span::new(0, 1),
                format!(
                    "part {:?} has {} measures but the first part has {}; all parts must have the same number of measures",
                    track.track_name(),
                    track.measure_count(),
                    expected_len
                ),
            ));
        }
    }
    Ok(())
}

fn build_part_rows(
    grouped_tracks: &[GroupedTrack],
    measure_idx: usize,
    lyrics_per_track: &[Vec<Vec<Syllable>>],
) -> Result<Vec<PartRow>, JianPuError> {
    let mut part_rows = Vec::new();

    for (track_idx, track) in grouped_tracks.iter().enumerate() {
        match track {
            GroupedTrack::Notes(part) => {
                let measure = part.measures.get(measure_idx).ok_or_else(|| {
                    JianPuError::new(
                        Span::new(0, 0),
                        "internal invariant: notes part measure missing",
                    )
                })?;
                let syllables = lyrics_per_track
                    .get(track_idx)
                    .and_then(|lyrics| lyrics.get(measure_idx))
                    .ok_or_else(|| {
                        JianPuError::new(
                            Span::new(0, 0),
                            "internal invariant: lyrics distribution missing",
                        )
                    })?
                    .clone();
                let lyrics = if part.lyrics.is_some() {
                    Some(Lyrics { syllables })
                } else {
                    None
                };
                part_rows.push(PartRow::Notes(PartSlice {
                    name: part.name.clone(),
                    notes: Notes {
                        events: measure.notes.events.clone(),
                    },
                    lyrics,
                }));
            }
            GroupedTrack::Chord(part) => {
                let chord_measure = part.measures.get(measure_idx).ok_or_else(|| {
                    JianPuError::new(
                        Span::new(0, 0),
                        "internal invariant: chord part measure missing",
                    )
                })?;
                part_rows.push(PartRow::Chord(chord_measure.clone()));
            }
        }
    }

    Ok(part_rows)
}

fn distribute_lyrics(measures: &[GroupedMeasure], lyrics: &[Syllable]) -> Vec<Vec<Syllable>> {
    let mut syllable_idx = 0;
    let mut prev_tie = false;
    let mut prev_pitch: Option<JianPuPitch> = None;

    let mut result = Vec::with_capacity(measures.len());
    for measure in measures {
        let mut measure_syllables = Vec::new();
        for event in &measure.notes.events {
            match event {
                NoteEvent::Note(note) => {
                    let is_continuation = prev_tie && prev_pitch.as_ref() == Some(&note.pitch);
                    if !is_continuation {
                        if let Some(syllable) = lyrics.get(syllable_idx) {
                            measure_syllables.push(syllable.clone());
                            syllable_idx += 1;
                        }
                    }
                    prev_tie = note.tie;
                    prev_pitch = Some(note.pitch.clone());
                }
                NoteEvent::Rest(_) => {
                    prev_tie = false;
                }
            }
        }
        result.push(measure_syllables);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{grouper, parser};

    fn make_two_part_score(soprano: &str, alto: &str) -> Vec<MultiPartMeasure> {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nSoprano = notes\nAlto = notes\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{soprano}\n{alto}\n"
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap().measures
    }

    #[test]
    fn combines_two_parts_into_measures() {
        let measures = make_two_part_score("1 2 3 4", "5 6 7 1");
        assert_eq!(measures.len(), 1);
        assert_eq!(measures[0].parts.len(), 2);
    }

    #[test]
    fn directives_come_from_first_part() {
        let measures = make_two_part_score("1 2 3 4", "5 6 7 1");
        assert_eq!(measures[0].bpm, Some(120));
        assert!(measures[0].time_signature.is_some());
    }

    #[test]
    fn rejects_parts_with_different_measure_counts() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nSoprano = notes\nAlto = notes\n\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1 5\n",
        );
        assert!(parser::parse(input, "test.jianpu").is_err());
    }
}
