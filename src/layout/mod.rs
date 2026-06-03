use crate::ast::grouped::{NoteEvent, Score};
use crate::ast::parsed::JianPuPitch;
use crate::layout::types::*;
use crate::utils::is_cjk_char;

pub mod types;

struct BeamBufferEntry {
    column: u32,
    underline_count: u32,
    duration: u32,
}

fn flush_beam_buffer(
    buffer: &mut Vec<BeamBufferEntry>,
    row_offset: u32,
    elements: &mut Vec<GridElement>,
) {
    if buffer.is_empty() {
        return;
    }
    let levels = compute_underline_levels(buffer);
    elements.push(GridElement {
        position: GridPosition {
            column: buffer[0].column,
            row: row_offset + 2,
        },
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: VerticalAlignment::Top,
        content: GridContent::DurationUnderlines { levels },
    });
    buffer.clear();
}

fn compute_underline_levels(buffer: &[BeamBufferEntry]) -> Vec<UnderlineSpan> {
    let first = &buffer[0];
    let last = &buffer[buffer.len() - 1];
    // Level 1: spans all notes in the group
    let mut levels = vec![UnderlineSpan {
        from_column: first.column,
        to_column: last.column + last.duration,
    }];
    // Level 2+: one span per maximal contiguous sub-run with underline_count >= 2
    let mut run_start: Option<u32> = None;
    let mut run_end: u32 = 0;
    for entry in buffer {
        if entry.underline_count >= 2 {
            if run_start.is_none() {
                run_start = Some(entry.column);
            }
            run_end = entry.column + entry.duration;
        } else if let Some(start) = run_start.take() {
            levels.push(UnderlineSpan { from_column: start, to_column: run_end });
        }
    }
    if let Some(start) = run_start {
        levels.push(UnderlineSpan { from_column: start, to_column: run_end });
    }
    // Identical level-1 and level-2 spans are intentional: they mean "draw this span twice"
    // (e.g. a lone sixteenth note or a pure-sixteenth beat group must render two underlines).
    levels
}

fn compute_prefix_width(
    measure: &crate::ast::grouped::Measure,
    previous_time_signature: Option<(u8, u8)>,
    previous_bpm: Option<u32>,
) -> u32 {
    let mut width = 0;
    if previous_time_signature
        != Some((measure.time_signature.numerator, measure.time_signature.denominator))
    {
        width += 2;
    }
    if previous_bpm != Some(measure.bpm) {
        width += 2;
    }
    width
}

/// Margin on every edge of the page in points (~9 mm).
/// Applied to all four sides: left/right for column fitting, top/bottom for row fitting.
const PAGE_MARGIN: f32 = 25.0;

/// A4 in points: 595 × 842.
/// Column width = cell_size, row height = cell_size.
pub fn layout(score: &Score, page_width_pt: f32, page_height_pt: f32) -> Vec<Page> {
    let cell = score.metadata.cell_size as f32;
    // Reserve space for left+right margins before fitting columns.
    let usable_width = page_width_pt - 2.0 * PAGE_MARGIN;
    let columns_per_page = (usable_width / cell) as u32;
    // Each row-group uses 4 rows (octave-up, note, octave-down+underline, lyric)
    let row_group_height: u32 = 4;

    let header_rows: u32 = if score.metadata.subtitle.is_some() { 3 } else { 2 };
    let footer_rows: u32 = 1;
    let reserved_rows = header_rows + footer_rows;
    // Subtract top+bottom margins before counting how many rows fit vertically.
    let usable_height = page_height_pt - 2.0 * PAGE_MARGIN;
    let row_groups_per_page = ((usable_height / cell) as u32 - reserved_rows) / row_group_height;

    let make_header = || Header {
        title: score.metadata.title.clone(),
        subtitle: score.metadata.subtitle.clone(),
        author: score.metadata.author.clone(),
    };

    let mut pages: Vec<Page> = Vec::new();
    let mut current_page_row_groups: Vec<RowGroup> = Vec::new();
    let mut current_elements: Vec<GridElement> = Vec::new();
    let mut current_col: u32 = 0;
    // current_row_offset is the absolute grid row where the current row-group starts.
    // Row-group rows are: +0 (octave up, via NoteHead.octave), +1 (note head), +2 (duration underlines / octave down), +3 (lyrics)
    let mut current_row_offset: u32 = header_rows;

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

    let mut beam_buffer: Vec<BeamBufferEntry> = Vec::new();
    let mut previous_time_signature: Option<(u8, u8)> = None;
    let mut previous_bpm: Option<u32> = None;

    for measure in &score.measures {
        let measure_width = measure_column_width(measure);
        if current_col + compute_prefix_width(measure, previous_time_signature, previous_bpm) + measure_width > columns_per_page {
            flush_beam_buffer(&mut beam_buffer, current_row_offset, &mut current_elements);
            pending_chain.clear();
            prev_tie = false;
            if !current_elements.is_empty() {
                current_page_row_groups.push(RowGroup {
                    elements: std::mem::take(&mut current_elements),
                    height_in_rows: row_group_height,
                    width_in_columns: current_col,
                });
            }
            current_col = 0;
            current_row_offset += row_group_height;

            if current_page_row_groups.len() >= row_groups_per_page as usize {
                if !current_page_row_groups.is_empty() {
                    pages.push(Page {
                        header: make_header(),
                        footer: Footer { page: pages.len() as u32 + 1, total: 0 },
                        row_groups: std::mem::take(&mut current_page_row_groups),
                        page_width_pt,
                    });
                }
                current_row_offset = header_rows;
            }
        }

        // Flush any leftover buffer from the previous measure (partial-beat edge case)
        flush_beam_buffer(&mut beam_buffer, current_row_offset, &mut current_elements);

        // Emit time signature label if new or changed
        if previous_time_signature
            != Some((measure.time_signature.numerator, measure.time_signature.denominator))
        {
            current_elements.push(GridElement {
                position: GridPosition { column: current_col, row: current_row_offset + 1 },
                horizontal_alignment: HorizontalAlignment::Center,
                vertical_alignment: VerticalAlignment::Center,
                content: GridContent::TimeSignatureLabel {
                    numerator: measure.time_signature.numerator,
                    denominator: measure.time_signature.denominator,
                },
            });
            current_col += 2;
            previous_time_signature = Some((
                measure.time_signature.numerator,
                measure.time_signature.denominator,
            ));
        }

        // Emit BPM label if new or changed
        if previous_bpm != Some(measure.bpm) {
            current_elements.push(GridElement {
                position: GridPosition { column: current_col, row: current_row_offset + 1 },
                horizontal_alignment: HorizontalAlignment::Center,
                vertical_alignment: VerticalAlignment::Center,
                content: GridContent::BpmLabel { bpm: measure.bpm },
            });
            current_col += 2;
            previous_bpm = Some(measure.bpm);
        }

        let measure_col_start = current_col;

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

                    // Lower octave dots (row 2, below any underlines)
                    if note.octave < 0 {
                        current_elements.push(GridElement {
                            position: GridPosition { column: current_col, row: current_row_offset + 2 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Bottom,
                            content: GridContent::LowerOctaveDots { count: (-note.octave) as u32 },
                        });
                    }

                    // Extension dashes (row 1)
                    if note.duration > 4 {
                        let extra_beats = (note.duration - 4) / 4;
                        for i in 0..extra_beats {
                            let ext_col = current_col + 4 + i * 4;
                            current_elements.push(GridElement {
                                position: GridPosition { column: ext_col, row: current_row_offset + 1 },
                                horizontal_alignment: HorizontalAlignment::Center,
                                vertical_alignment: VerticalAlignment::Center,
                                content: GridContent::Extension,
                            });
                        }
                    }

                    // Beam buffer logic (row 2)
                    let underline_count = match note.duration {
                        1 => 2, // sixteenth note
                        2 => 1, // eighth note
                        _ => 0, // quarter or longer — flush any pending group first
                    };

                    if underline_count == 0 {
                        // Trigger 2: quarter-or-longer note ends any open beam group
                        flush_beam_buffer(&mut beam_buffer, current_row_offset, &mut current_elements);
                    }

                    // Chain tracking for tie/slur arcs
                    if pending_chain.is_empty() {
                        chain_row = current_row_offset + 1;
                    }
                    pending_chain.push((current_col, note.pitch.clone()));

                    // Lyric (row 3)
                    let is_tie_continuation = prev_tie && prev_pitch.as_ref() == Some(&note.pitch);
                    if !is_tie_continuation {
                        if let Some(syllable) = lyrics_iter.next() {
                            let is_cjk = syllable.text.chars().next().map(|c| is_cjk_char(c)).unwrap_or(false);
                            current_elements.push(GridElement {
                                position: GridPosition { column: current_col, row: current_row_offset + 3 },
                                horizontal_alignment: HorizontalAlignment::Center,
                                vertical_alignment: VerticalAlignment::Top,
                                content: GridContent::Lyric { text: syllable.text.clone(), is_cjk },
                            });
                        }
                    }
                    prev_tie = note.tie;
                    prev_pitch = Some(note.pitch.clone());

                    if underline_count > 0 {
                        beam_buffer.push(BeamBufferEntry {
                            column: current_col,
                            underline_count,
                            duration: note.duration,
                        });
                    }

                    current_col += note.duration;

                    // Trigger 3: flush when new position lands on a beat boundary
                    let beat_position = current_col - measure_col_start;
                    if underline_count > 0 && beat_position % 4 == 0 {
                        flush_beam_buffer(&mut beam_buffer, current_row_offset, &mut current_elements);
                    }

                    if !note.tie {
                        flush_chain(&pending_chain, chain_row, &mut current_elements);
                        pending_chain.clear();
                    }
                }
                NoteEvent::Rest(rest) => {
                    // Trigger 1: any rest ends an open beam group
                    flush_beam_buffer(&mut beam_buffer, current_row_offset, &mut current_elements);

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

        // Flush any beam group open at the end of the measure
        flush_beam_buffer(&mut beam_buffer, current_row_offset, &mut current_elements);

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
            width_in_columns: current_col,
        });
    }
    if !current_page_row_groups.is_empty() {
        pages.push(Page {
            header: make_header(),
            footer: Footer { page: pages.len() as u32 + 1, total: 0 },
            row_groups: std::mem::take(&mut current_page_row_groups),
            page_width_pt,
        });
    }

    if pages.is_empty() {
        pages.push(Page {
            header: make_header(),
            footer: Footer { page: 1, total: 1 },
            row_groups: Vec::new(),
            page_width_pt,
        });
    }

    // Second pass: fill in total page count
    let total = pages.len() as u32;
    for page in &mut pages {
        page.footer.total = total;
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

    fn make_score_raw(score_section: &str, lyrics_str: &str) -> Score {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n{}\n\n[lyrics]\n{}\n",
            score_section, lyrics_str
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    #[test]
    fn first_measure_emits_time_signature_label_at_column_zero() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].position.column, 0);
        if let GridContent::TimeSignatureLabel { numerator, denominator } = &labels[0].content {
            assert_eq!(*numerator, 4);
            assert_eq!(*denominator, 4);
        } else {
            panic!("expected TimeSignatureLabel");
        }
    }

    #[test]
    fn first_measure_emits_bpm_label_at_column_two() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].position.column, 2);
        if let GridContent::BpmLabel { bpm } = &labels[0].content {
            assert_eq!(*bpm, 120);
        } else {
            panic!("expected BpmLabel");
        }
    }

    #[test]
    fn note_heads_start_after_both_label_columns() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let note_heads: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads[0].position.column, 4);
    }

    #[test]
    fn unchanged_time_signature_emits_no_second_label() {
        let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1, "only one time signature label expected for two measures with identical time signature on the same line");
    }

    #[test]
    fn unchanged_bpm_emits_no_second_label() {
        let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1, "only one BPM label expected for two measures with identical BPM on the same line");
    }

    #[test]
    fn time_signature_change_emits_second_label() {
        let score = make_score_raw("4/4 1 2 3 4 3/4 1 2 3", "a b c d e f g");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 2, "expected one label per distinct time signature");
    }

    #[test]
    fn bpm_change_emits_second_label() {
        let score = make_score_raw("4/4 bpm=120 1 2 3 4 bpm=90 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 2, "expected one BPM label per distinct BPM value");
    }

    const A4_WIDTH: f32 = 595.0;  // points
    const A4_HEIGHT: f32 = 842.0; // points

    #[test]
    fn header_is_populated_on_every_page() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert!(!pages.is_empty());
        for page in &pages {
            assert_eq!(page.header.title, "t");
            assert_eq!(page.header.author, "a");
            assert_eq!(page.header.subtitle, None);
        }
    }

    #[test]
    fn footer_page_numbers_are_correct() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let total = pages.len() as u32;
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page.footer.page, i as u32 + 1);
            assert_eq!(page.footer.total, total);
        }
    }

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
        assert_eq!(curves[0], (4, 8));
    }

    #[test]
    fn three_note_slur_emits_one_curve() {
        // 3~2~1: all different pitches → one slur from col 0 to col 8
        let score = make_score("3~2~1 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (4, 12));
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
        assert_eq!(curves[0], (4, 12)); // slur
        assert_eq!(curves[1], (8, 12)); // tie
    }

    #[test]
    fn same_pitch_chain_emits_only_tie() {
        // 1~1 2 3: same pitches → one tie, no slur
        let score = make_score("1~1 2 3", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (4, 8));
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

    fn collect_underline_levels(pages: &[Page]) -> Vec<Vec<UnderlineSpan>> {
        pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::DurationUnderlines { levels } => Some(levels.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn consecutive_eighth_notes_at_beat_start_share_one_underline() {
        // _2 _2 fills beat 1 (qb 0–3); 0 0 0 are quarter rests filling the rest of 4/4
        // Total: 2+2+4+4+4 = 16 quarter-beats ✓
        let score = make_score("_2 _2 0 0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group");
        assert_eq!(groups[0].len(), 1, "expected one underline level");
        assert_eq!(groups[0][0].from_column, 4);
        assert_eq!(groups[0][0].to_column, 8);
    }

    #[test]
    fn eighth_notes_straddling_beat_boundary_produce_separate_underlines() {
        // 0(4qb) _0(2qb) _2(2qb) _2(2qb) _0(2qb) 0(4qb) = 16qb ✓
        // First _2 starts at qb 6 (mid-beat-2), ends at qb 8 (beat boundary) → flushed alone
        // Second _2 starts at qb 8, ends at qb 10 → flushed alone when _0 rest arrives
        let score = make_score("0 _0 _2 _2 _0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 2, "expected two separate underline groups");
        assert_eq!(groups[0][0].from_column, 10);
        assert_eq!(groups[0][0].to_column, 12);
        assert_eq!(groups[1][0].from_column, 12);
        assert_eq!(groups[1][0].to_column, 14);
    }

    #[test]
    fn mixed_eighth_and_sixteenth_notes_produce_two_underline_levels() {
        // _1(2qb) =2(1qb) =3(1qb) fills beat 1 exactly; 0 0 0 fill 12 more qb = 16 total ✓
        // Level 1: spans all three notes (col 0–4)
        // Level 2: spans only the sixteenth sub-run =2,=3 (col 2–4)
        let score = make_score("_1 =2 =3 0 0 0", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group");
        assert_eq!(groups[0].len(), 2, "expected two underline levels");
        assert_eq!(groups[0][0].from_column, 4);
        assert_eq!(groups[0][0].to_column, 8);
        assert_eq!(groups[0][1].from_column, 6);
        assert_eq!(groups[0][1].to_column, 8);
    }

    #[test]
    fn lone_sixteenth_note_has_two_underlines() {
        // =1(1qb) then 15 quarter-beats of rest to fill 4/4 (15 = not valid as quarter notes...)
        // Use =1 then rests: =1(1qb) + 0(4qb)*3 + rest to fill. Actually 1+4+4+4+3 doesn't work.
        // Fill with: =1 0 0 0 and pad the score to 4/4 = 16qb: =1(1) + 0(4)+0(4)+0(4) = 13, need 3 more.
        // Use: =1 =0 0 0 0 0 = 1+1+4+4+4+... let's just use a full measure with rests.
        // =1(1qb) and fill with quarter rests: need 15 more qb but rests are 4qb each = can't hit 16.
        // Use: =1 =0 =0 =0 0 0 0 = 1+1+1+1+4+4+4 = 16qb ✓
        // Only =1 is a note (pitch); =0 are sixteenth rests → flush before each rest.
        // So =1 is a lone sixteenth in the buffer → produces level-1 and level-2 spans both {0,1}.
        let score = make_score("=1 =0 =0 =0 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group for the lone sixteenth");
        assert_eq!(groups[0].len(), 2, "lone sixteenth must produce two underline levels");
        assert_eq!(groups[0][0], UnderlineSpan { from_column: 4, to_column: 5 });
        assert_eq!(groups[0][1], UnderlineSpan { from_column: 4, to_column: 5 });
    }

    #[test]
    fn pure_sixteenth_beat_group_has_two_underlines() {
        // =1 =2 =3 =4 fills one beat exactly (4×1qb = 4qb); 0 0 0 fills 12 more qb = 16 total ✓
        // All four notes are sixteenth (underline_count=2): level-1 spans 0–4, level-2 also 0–4.
        let score = make_score("=1 =2 =3 =4 0 0 0", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group spanning the beat");
        assert_eq!(groups[0].len(), 2, "pure-sixteenth group must produce two underline levels");
        assert_eq!(groups[0][0], UnderlineSpan { from_column: 4, to_column: 8 });
        assert_eq!(groups[0][1], UnderlineSpan { from_column: 4, to_column: 8 });
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
            vec![(4, "a".to_string()), (12, "b".to_string()), (16, "c".to_string())],
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
            vec![(4, "a".to_string()), (8, "b".to_string()), (16, "c".to_string())],
        );
    }

    #[test]
    fn dash_lyric_is_rendered() {
        // "1 2 3 4" with lyrics "你 - 好 a": note 1→"你", note 2→"-", note 3→"好", note 4→"a"
        let score = make_score("1 2 3 4", "你 - 好 a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![(4, "你".to_string()), (8, "-".to_string()), (12, "好".to_string()), (16, "a".to_string())],
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
            .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if levels.len() == 1))
            .collect();
        assert_eq!(underlines.len(), 2); // _1 and _4
    }

    #[test]
    fn lower_octave_note_emits_lower_octave_dots_element() {
        // "1." means pitch 1 with one trailing dot = octave -1
        let score = make_score("1. 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let lower_dots: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::LowerOctaveDots { .. }))
            .collect();
        assert_eq!(lower_dots.len(), 1, "expected one LowerOctaveDots element");
        if let GridContent::LowerOctaveDots { count } = &lower_dots[0].content {
            assert_eq!(*count, 1);
        }
        // First row-group starts at row offset = header_rows (2), so underline sub-row (+2) → absolute row 4
        assert_eq!(lower_dots[0].position.row, 4, "LowerOctaveDots must be in absolute row 4 (header_rows + underline sub-row)");
        assert_eq!(lower_dots[0].vertical_alignment, VerticalAlignment::Bottom);
    }

    #[test]
    fn unchanged_labels_do_not_repeat_after_line_wrap() {
        // Use a narrow page so measures wrap across multiple row groups.
        // With cell_size=24 and page_width=300: columns_per_page = 12.
        // First measure: 2+2+16+1 = 21 > 12 → wraps before placing notes.
        // After wrap the first measure is placed (still same time sig, same BPM).
        // Second measure: same time sig, same BPM → no prefix labels.
        // Total TimeSignatureLabel count across the whole score should be exactly 1.
        let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let time_sig_labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(
            time_sig_labels.len(),
            1,
            "time signature label must not repeat on wrapped lines, got {}",
            time_sig_labels.len()
        );
    }
}
