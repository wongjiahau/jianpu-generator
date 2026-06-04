use crate::ast::grouped::*;
use crate::ast::parsed::{JianPuPitch, Syllable};
use crate::error::{JianPuError, Span};

pub fn combine(parts: Vec<GroupedPart>) -> Result<Vec<MultiPartMeasure>, JianPuError> {
    if parts.is_empty() {
        return Ok(Vec::new());
    }

    let expected_len = parts[0].measures.len();
    for part in &parts[1..] {
        if part.measures.len() != expected_len {
            return Err(JianPuError::new(
                Span::new(0, 0),
                format!(
                    "part {:?} has {} measures but the first part has {}; all parts must have the same number of measures",
                    part.name, part.measures.len(), expected_len
                ),
            ));
        }
    }

    let lyrics_per_part: Vec<Vec<Vec<Syllable>>> = parts
        .iter()
        .map(|p| {
            p.lyrics
                .as_deref()
                .map(|lyrics| distribute_lyrics(&p.measures, lyrics))
                .unwrap_or_else(|| vec![vec![]; p.measures.len()])
        })
        .collect();

    let num_measures = expected_len;
    let mut combined = Vec::with_capacity(num_measures);

    for measure_idx in 0..num_measures {
        let first = &parts[0].measures[measure_idx];

        let part_slices = parts
            .iter()
            .enumerate()
            .map(|(part_idx, part)| {
                let measure = &part.measures[measure_idx];
                let syllables = lyrics_per_part[part_idx][measure_idx].clone();
                let lyrics = if part.lyrics.is_some() {
                    Some(Lyrics { syllables })
                } else {
                    None
                };
                PartSlice {
                    name: part.name.clone(),
                    notes: Notes { events: measure.notes.events.clone() },
                    lyrics,
                }
            })
            .collect();

        combined.push(MultiPartMeasure {
            time_signature: first.time_signature.clone(),
            bpm: first.bpm,
            key: first.key.clone(),
            parts: part_slices,
        });
    }

    Ok(combined)
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
                    if !is_continuation && syllable_idx < lyrics.len() {
                        measure_syllables.push(lyrics[syllable_idx].clone());
                        syllable_idx += 1;
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{}\n{}\n",
            soprano, alto
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
        // Both parts in one group must have the same beat count.
        // Alto row has too many beats — interleaved parser rejects it.
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1 5\n",
        );
        assert!(parser::parse(input, "test.jianpu").is_err());
    }
}
