use crate::ast::grouped::{NoteEvent, Score};
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
    let mut current_row_offset: u32 = 0;

    let mut lyrics_iter = score.lyrics.iter();

    for measure in &score.measures {
        // Check if this measure fits in the current row-group
        let measure_width = measure_column_width(measure);
        if current_col + measure_width > columns_per_page {
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
                        1 => 2, // quarter-beat = 2 underlines
                        2 => 1, // half-beat = 1 underline
                        _ => 0,
                    };
                    if underline_count > 0 {
                        current_elements.push(GridElement {
                            position: GridPosition { column: current_col, row: current_row_offset + 2 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Top,
                            content: GridContent::DurationUnderlines { count: underline_count },
                        });
                    }

                    // Lyric (row 3)
                    if let Some(syllable) = lyrics_iter.next() {
                        let is_cjk = syllable.text.chars().next().map(|c| is_cjk_char(c)).unwrap_or(false);
                        current_elements.push(GridElement {
                            position: GridPosition { column: current_col, row: current_row_offset + 3 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Top,
                            content: GridContent::Lyric { text: syllable.text.clone(), is_cjk },
                        });
                    }

                    current_col += note.duration; // each quarter-beat = 1 column
                }
                NoteEvent::Rest(rest) => {
                    current_elements.push(GridElement {
                        position: GridPosition { column: current_col, row: current_row_offset + 1 },
                        horizontal_alignment: HorizontalAlignment::Center,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::Rest,
                    });
                    if let Some(syllable) = lyrics_iter.next() {
                        let is_cjk = syllable.text.chars().next().map(|c| is_cjk_char(c)).unwrap_or(false);
                        current_elements.push(GridElement {
                            position: GridPosition { column: current_col, row: current_row_offset + 3 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Top,
                            content: GridContent::Lyric { text: syllable.text.clone(), is_cjk },
                        });
                    }
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
