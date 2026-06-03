use crate::ast::grouped::{NoteEvent, Score};
use crate::ast::parsed::JianPuPitch;
use crate::layout::types::*;
use crate::utils::is_cjk_char;

pub mod types;

/// A4 in points: 595 × 842.
/// Column width = cell_size, row height = cell_size.
pub fn layout(score: &Score, page_width_pt: f32, page_height_pt: f32) -> Vec<Page> {
    let cell = score.metadata.cell_size as f32;
    let columns_per_page = (page_width_pt / cell) as u32;
    // Each row-group uses 4 rows (octave-up, note, octave-down+underline, lyric)
    let row_group_height: u32 = 4;
    let row_groups_per_page = ((page_height_pt / cell) as u32) / row_group_height;

    let mut pages: Vec<Page> = Vec::new();
    let mut current_page_row_groups: Vec<RowGroup> = Vec::new();
    let mut current_elements: Vec<GridElement> = Vec::new();
    let mut current_col: u32 = 0;
    // current_row_offset is the absolute grid row where the current row-group starts.
    // Row-group rows are: +0 (octave up, via NoteHead.octave), +1 (note head), +2 (duration underlines / octave down), +3 (lyrics)
    let mut current_row_offset: u32 = 0;

    let mut lyrics_iter = score.lyrics.iter();

    // Chain tracking: consecutive notes connected by `~`.
    // Each entry is (column, pitch) of a note in the current chain.
    let mut pending_chain: Vec<(u32, JianPuPitch)> = Vec::new();
    let mut chain_row: u32 = 0;

    // Lyric consumption: a note skips its syllable only when the previous note
    // tied to it with the same pitch (a tie, not a slur). Slurred notes (different
    // pitches connected by ~) each get their own syllable.
    let mut prev_tie: bool = false;
    let mut prev_pitch: Option<JianPuPitch> = None;

    for measure in &score.measures {
        // Check if this measure fits in the current row-group
        let measure_width = measure_column_width(measure);
        if current_col + measure_width > columns_per_page {
            // Abandon any cross-row-group chain (cannot draw arc across rows)
            pending_chain.clear();
            prev_tie = false;
            // Start a new row-group
            if !current_elements.is_empty() {
                current_page_row_groups.push(RowGroup {
                    elements: std::mem::take(&mut current_elements),
                    height_in_rows: row_group_height,
                });
            }
            current_col = 0;
            current_row_offset += row_group_height;

            if current_page_row_groups.len() >= row_groups_per_page as usize {
                if !current_page_row_groups.is_empty() {
                    pages.push(Page {
                        row_groups: std::mem::take(&mut current_page_row_groups),
                    });
                }
                current_row_offset = 0;
            }
        }

        for note_event in &measure.notes {
            match note_event {
                NoteEvent::Note(note) => {
                    // Row 1: note head
                    current_elements.push(GridElement {
                        position: GridPosition { column: current_col, row: current_row_offset + 1 },
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::NoteHead {
                            pitch: note.pitch.clone(),
                            octave: note.octave,
                        },
                    });

                    // Duration underlines (row 2)
                    let underline_count = match note.duration {
                        1 => 2, // sixteenth note (= prefix) — 2 underlines
                        2 => 1, // eighth note (_ prefix) — 1 underline
                        _ => 0, // quarter note or longer — no underlines
                    };
                    if underline_count > 0 {
                        current_elements.push(GridElement {
                            position: GridPosition { column: current_col, row: current_row_offset + 2 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Top,
                            content: GridContent::DurationUnderlines { count: underline_count },
                        });
                    }

                    // Chain tracking for tie/slur arcs.
                    if pending_chain.is_empty() {
                        chain_row = current_row_offset + 1;
                    }
                    pending_chain.push((current_col, note.pitch.clone()));

                    // Lyric (row 3).
                    // A note is a tie continuation (same pitch as the note that ties into it)
                    // and should skip the syllable. A slur continuation (different pitch) still
                    // gets its own syllable.
                    let is_tie_continuation = prev_tie && prev_pitch.as_ref() == Some(&note.pitch);
                    if !is_tie_continuation {
                        if let Some(syllable) = lyrics_iter.next() {
                            if syllable.text != "-" {
                                let is_cjk = syllable.text.chars().next().map(|c| is_cjk_char(c)).unwrap_or(false);
                                current_elements.push(GridElement {
                                    position: GridPosition { column: current_col, row: current_row_offset + 3 },
                                    horizontal_alignment: HorizontalAlignment::Center,
                                    vertical_alignment: VerticalAlignment::Top,
                                    content: GridContent::Lyric { text: syllable.text.clone(), is_cjk },
                                });
                            }
                        }
                    }
                    prev_tie = note.tie;
                    prev_pitch = Some(note.pitch.clone());

                    current_col += note.duration;

                    if !note.tie {
                        flush_chain(&pending_chain, chain_row, &mut current_elements);
                        pending_chain.clear();
                    }
                }
                NoteEvent::Rest(rest) => {
                    current_elements.push(GridElement {
                        position: GridPosition { column: current_col, row: current_row_offset + 1 },
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::Rest,
                    });
                    current_col += rest.duration;
                }
            }
        }

        // Bar line after measure
        current_elements.push(GridElement {
            position: GridPosition { column: current_col, row: current_row_offset + 1 },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Center,
            content: GridContent::BarLine,
        });
        current_col += 1;
    }

    // Flush remaining elements
    if !current_elements.is_empty() {
        current_page_row_groups.push(RowGroup {
            elements: std::mem::take(&mut current_elements),
            height_in_rows: row_group_height,
        });
    }
    if !current_page_row_groups.is_empty() {
        pages.push(Page {
            row_groups: std::mem::take(&mut current_page_row_groups),
        });
    }

    if pages.is_empty() {
        pages.push(Page { row_groups: Vec::new() });
    }

    pages
}

/// Emit tie/slur arcs for a completed chain of `~`-connected notes.
///
/// Rules:
/// - If the chain contains any pitch change → one **slur** arc from first to last note.
/// - For every consecutive same-pitch pair within the chain → one **tie** arc between them.
fn flush_chain(chain: &[(u32, JianPuPitch)], chain_row: u32, elements: &mut Vec<GridElement>) {
    if chain.len() <= 1 {
        return;
    }

    let has_pitch_change = chain.windows(2).any(|w| w[0].1 != w[1].1);

    if has_pitch_change {
        // One slur spanning the entire chain
        elements.push(GridElement {
            position: GridPosition { column: chain[0].0, row: chain_row },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            content: GridContent::TieOrSlurCurve {
                from_column: chain[0].0,
                to_column: chain.last().unwrap().0,
            },
        });
    }

    // Tie arc for each consecutive same-pitch pair
    for w in chain.windows(2) {
        if w[0].1 == w[1].1 {
            elements.push(GridElement {
                position: GridPosition { column: w[0].0, row: chain_row },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Top,
                content: GridContent::TieOrSlurCurve {
                    from_column: w[0].0,
                    to_column: w[1].0,
                },
            });
        }
    }
}

fn measure_column_width(measure: &crate::ast::grouped::Measure) -> u32 {
    let notes_width: u32 = measure.notes.iter().map(|n| match n {
        NoteEvent::Note(note) => note.duration,
        NoteEvent::Rest(rest) => rest.duration,
    }).sum();
    notes_width + 1 // +1 for bar line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::grouper;

    fn make_score(score_str: &str, lyrics_str: &str) -> Score {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 {}\n\n[lyrics]\n{}\n",
            score_str, lyrics_str
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    const A4_WIDTH: f32 = 595.0;  // points
    const A4_HEIGHT: f32 = 842.0; // points

    #[test]
    fn produces_at_least_one_page() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert!(!pages.is_empty());
    }

    #[test]
    fn note_heads_are_present() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let note_heads: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads.len(), 4);
    }

    #[test]
    fn lyrics_are_present() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let lyrics: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::Lyric { .. }))
            .collect();
        assert_eq!(lyrics.len(), 4);
    }

    #[test]
    fn two_different_notes_emit_one_slur() {
        // 1~ 2: different pitches → one slur from col 0 to col 4
        let score = make_score("1~ 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (0, 4));
    }

    #[test]
    fn three_note_slur_emits_one_curve() {
        // 3~2~1: all different pitches → one slur from col 0 to col 8
        let score = make_score("3~2~1 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (0, 8));
    }

    #[test]
    fn mixed_chain_emits_slur_and_tie() {
        // 4~3~3 2: chain [4@0, 3@4, 3@8]
        // → one slur from 0 to 8 (pitch change exists)
        // → one tie from 4 to 8 (same-pitch pair 3~3)
        let score = make_score("4~3~3 2", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let mut curves = collect_curves(&pages);
        curves.sort();
        assert_eq!(curves.len(), 2);
        assert_eq!(curves[0], (0, 8)); // slur
        assert_eq!(curves[1], (4, 8)); // tie
    }

    #[test]
    fn same_pitch_chain_emits_only_tie() {
        // 1~1 2 3: same pitches → one tie, no slur
        let score = make_score("1~1 2 3", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (0, 4));
    }

    fn collect_curves(pages: &[Page]) -> Vec<(u32, u32)> {
        pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::TieOrSlurCurve { from_column, to_column } => Some((*from_column, *to_column)),
                _ => None,
            })
            .collect()
    }

    fn collect_lyric_positions(pages: &[Page]) -> Vec<(u32, String)> {
        pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::Lyric { text, .. } => Some((e.position.column, text.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tied_notes_share_one_lyric_syllable() {
        // 3~3 is a tie (same pitch): both notes share one syllable.
        // 3~3 1 2 with lyrics "a b c":
        //   3 (col 0) → "a",  second 3 (col 4) → no lyric,  1 (col 8) → "b",  2 (col 12) → "c"
        let score = make_score("3~3 1 2", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![(0, "a".to_string()), (8, "b".to_string()), (12, "c".to_string())],
        );
    }

    #[test]
    fn slurred_notes_each_get_a_lyric_syllable() {
        // 4~3~3: 4→3 is a slur (different pitch, each gets a syllable),
        //        3→3 is a tie (same pitch, second 3 shares the syllable of first 3).
        // So "4~3~3 2" with lyrics "a b c" assigns: 4→"a", first 3→"b", second 3→no lyric, 2→"c"
        let score = make_score("4~3~3 2", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![(0, "a".to_string()), (4, "b".to_string()), (12, "c".to_string())],
        );
    }

    #[test]
    fn half_beat_note_has_duration_underline() {
        let score = make_score("_1 2 3 _4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let underlines: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::DurationUnderlines { count: 1 }))
            .collect();
        assert_eq!(underlines.len(), 2); // _1 and _4
    }
}
