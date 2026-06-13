use crate::ast::grouped::{
    GroupedMeasure, GroupedScore, GroupedTrack, Lyrics, MultiPartMeasure, NoteEvent, Notes,
    PartRow, PartSlice,
};
use crate::ast::parsed::{JianPuPitch, PartKind, Syllable};
use crate::error::{JianPuError, Span};

pub(crate) fn combine(grouped_score: &GroupedScore) -> Result<Vec<MultiPartMeasure>, JianPuError> {
    if grouped_score.parts.is_empty() {
        return Ok(Vec::new());
    }

    let expected_len = grouped_score
        .parts
        .first()
        .map(GroupedTrack::measure_count)
        .unwrap_or(0);
    validate_measure_counts(&grouped_score.parts, expected_len)?;

    let lyrics_per_track: Vec<Vec<Vec<Syllable>>> = grouped_score
        .parts
        .iter()
        .map(|track| match track {
            GroupedTrack::Timed(part) => match part.kind {
                PartKind::NotesWithLyrics => part
                    .lyrics
                    .as_deref()
                    .map(|lyrics| distribute_lyrics(&part.measures, lyrics))
                    .unwrap_or_else(|| vec![vec![]; part.measures.len()]),
                PartKind::Chord | PartKind::Notes => {
                    vec![vec![]; part.measures.len()]
                }
            },
        })
        .collect();

    let mut combined = Vec::with_capacity(expected_len);
    for measure_idx in 0..expected_len {
        let directives = grouped_score
            .measure_directives
            .get(measure_idx)
            .ok_or_else(|| {
                JianPuError::new(
                    Span::new(0, 0),
                    "internal invariant: measure_directives shorter than measure count",
                )
            })?;
        let part_rows = build_part_rows(&grouped_score.parts, measure_idx, &lyrics_per_track)?;
        let source_span = grouped_score
            .parts
            .first()
            .and_then(|track| match track {
                GroupedTrack::Timed(part) => part.measures.get(measure_idx),
            })
            .map(|m| m.source_span.clone())
            .unwrap_or_else(|| Span::new(0, 0));
        combined.push(MultiPartMeasure {
            time_signature: directives.time_signature.clone(),
            bpm: directives.bpm,
            key: directives.key.clone(),
            label: directives.label.clone(),
            parts: part_rows,
            source_span,
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
            GroupedTrack::Timed(part) => {
                let measure = part.measures.get(measure_idx).ok_or_else(|| {
                    JianPuError::new(
                        Span::new(0, 0),
                        "internal invariant: timed part measure missing",
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
                let lyrics = match part.kind {
                    PartKind::NotesWithLyrics => Some(Lyrics { syllables }),
                    PartKind::Chord | PartKind::Notes => None,
                };
                let mut slice = PartSlice {
                    name: part.name.clone(),
                    kind: part.kind,
                    notes: Notes {
                        events: measure.notes.events.clone(),
                    },
                    lyrics,
                };
                let is_ditto = part
                    .ditto_measures
                    .get(measure_idx)
                    .copied()
                    .unwrap_or(false);
                let lyrics_ditto = part
                    .lyrics_ditto_measures
                    .get(measure_idx)
                    .copied()
                    .unwrap_or(false);
                // A ditto'd lyric line duplicates the part above's lyrics, so
                // render this measure as a plain notes part: the copied
                // syllables are not shown and the lyric row is reclaimed.
                if lyrics_ditto && !is_ditto && matches!(slice.kind, PartKind::NotesWithLyrics) {
                    slice.kind = PartKind::Notes;
                    slice.lyrics = None;
                }
                part_rows.push(if is_ditto {
                    PartRow::Ditto(slice)
                } else {
                    PartRow::Timed(slice)
                });
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
                NoteEvent::Rest(_) | NoteEvent::Chord(_) => {
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
